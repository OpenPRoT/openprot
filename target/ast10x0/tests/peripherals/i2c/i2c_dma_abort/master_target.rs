// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! DMA-guard abort test — master side (device A).
//!
//! Exercises [`ArmedDma`](ast10x0_peripherals::i2c) black-box through the driver's
//! master DMA write path (I2C2, DMA mode). Two phases against device B on Bus 2:
//!
//! - **Phase 1 (commit / no-op):** a DMA `write()` to a responsive slave `0x42`
//!   completes with `Ok`. The transfer's `wait_completion` returns cleanly, so the
//!   guard is `commit()`ed and its teardown never runs — the happy path still works
//!   and the engine is not spuriously reset.
//! - **Phase 2 (timeout → auto-teardown):** device B stops servicing and holds SCL
//!   low. The DMA `write()` cannot complete, `wait_completion` times out, and the
//!   uncommitted guard drops → controller soft-reset. We then read our own I2C2
//!   registers to prove the teardown ran: function-control was restored
//!   (`i2cc00.enbl_master_fn` set) and latched interrupts cleared (`i2cm14 == 0`).
//!   The call *returning at all* (Err, not a hang) demonstrates the bounded
//!   busy-wait in the guard's Drop.
//!
//! Device B must be running its stretcher image before this image is loaded.

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i2c::{
    Ast1060I2c, Ast1060I2cRegisters, ClockConfig, I2cConfig, I2cError, I2cSpeed, I2cXferMode,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const SLAVE_ADDR: u8 = 0x42;
const PAYLOAD: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];

/// Bounded retry budget for phase 1, absorbing device B's bring-up latency
/// (there is no explicit two-node handshake). Kept small so a genuine failure
/// surfaces quickly instead of burning the whole test timeout on retries.
const PHASE1_ATTEMPTS: u32 = 30;

// Master TX staging buffer. A master DMA transfer points the engine (an AHB bus
// master) at this buffer, so it must live in non-cached SRAM the DMA engine and
// CPU observe coherently. The slave buffer is unused here but required by the
// DMA constructor.
#[unsafe(link_section = ".ram_nc")]
static mut MASTER_DMA_BUF: [u8; 4096] = [0u8; 4096];
#[unsafe(link_section = ".ram_nc")]
static mut SLAVE_DMA_BUF: [u8; 256] = [0u8; 256];

fn i2c2_dma_config() -> I2cConfig {
    I2cConfig {
        xfer_mode: I2cXferMode::DmaMode,
        speed: I2cSpeed::Standard,
        multi_master: false,
        smbus_timeout: false,
        smbus_alert: false,
        clock_config: ClockConfig::ast1060_default(),
    }
}

fn i2c_error_str(error: I2cError) -> &'static str {
    match error {
        I2cError::Overrun => "Overrun",
        I2cError::NoAcknowledge => "NoAcknowledge",
        I2cError::Timeout => "Timeout",
        I2cError::BusRecoveryFailed => "BusRecoveryFailed",
        I2cError::Bus => "Bus",
        I2cError::Busy => "Busy",
        I2cError::Invalid => "Invalid",
        I2cError::Abnormal => "Abnormal",
        I2cError::ArbitrationLoss => "ArbitrationLoss",
        I2cError::SlaveError => "SlaveError",
        I2cError::InvalidAddress => "InvalidAddress",
    }
}

/// Dump the master's I2C2 status registers for post-mortem diagnosis.
fn dump_master_regs(context: &str) {
    // SAFETY: the test owns I2C2; read-only view of the same registers.
    let regs = unsafe { &*ast1060_pac::I2c2::ptr() };
    pw_log::error!(
        "{}: i2cc00=0x{:08x} i2cc08=0x{:08x} i2cm14=0x{:08x}",
        context as &str,
        regs.i2cc00().read().bits() as u32,
        regs.i2cc08().read().bits() as u32,
        regs.i2cm14().read().bits() as u32
    );
}

fn run_master() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 I2C DMA-guard abort test (master, Bus 2) ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_I2C2],
        i2c_buses: &[],
    });
    // SAFETY: single call at boot with exclusive access to SCU/I2C global regs.
    unsafe { board.init() }.map_err(|_| "board init failed")?;

    // SAFETY: I2C2 registers accessed only through `master` for this test.
    let mmio =
        unsafe { Ast1060I2cRegisters::new(ast1060_pac::I2c2::ptr(), ast1060_pac::I2cbuff2::ptr()) };
    // SAFETY: both buffers are non-cached SRAM statics uniquely owned by this
    // driver for the test's lifetime.
    let master_dma_buf: &'static mut [u8] =
        unsafe { &mut *core::ptr::addr_of_mut!(MASTER_DMA_BUF) };
    let slave_dma_buf: &'static mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(SLAVE_DMA_BUF) };
    let mut master = Ast1060I2c::new_with_dma(
        mmio,
        &i2c2_dma_config(),
        master_dma_buf,
        slave_dma_buf,
        |_| core::hint::spin_loop(),
    )
    .map_err(|_| "I2C2 master DMA init failed")?;

    // -- Phase 1: commit / no-op path. Device B services exactly one write. --
    let mut attempts = PHASE1_ATTEMPTS;
    loop {
        match master.write(SLAVE_ADDR, PAYLOAD) {
            Ok(()) => {
                pw_log::info!("phase 1: committed DMA write OK (guard defused, no reset)");
                break;
            }
            Err(_) if attempts > 0 => {
                attempts -= 1;
                for _ in 0..10_000 {
                    core::hint::spin_loop();
                }
            }
            Err(e) => {
                pw_log::error!("phase 1 DMA write failed: {}", i2c_error_str(e) as &str);
                dump_master_regs("phase 1 failure");
                return Err("phase 1 commit path failed (device B not responding?)");
            }
        }
    }

    // -- Phase 2: timeout → auto-teardown. Device B now wedges, holding SCL. --
    // The DMA write cannot complete; wait_completion times out; the uncommitted
    // ArmedDma drops and soft-resets the controller. Returning (not hanging)
    // demonstrates the bounded busy-wait in the guard's Drop.
    match master.write(SLAVE_ADDR, PAYLOAD) {
        Err(I2cError::Timeout) => {
            pw_log::info!("phase 2: DMA write timed out as expected (guard drop → teardown)");
        }
        Err(other) => {
            pw_log::error!(
                "phase 2: expected Timeout, got {}",
                i2c_error_str(other) as &str
            );
            return Err("phase 2 did not time out (stretch recipe needs tuning on the rig)");
        }
        Ok(()) => {
            return Err("phase 2 unexpectedly succeeded (device B did not stall)");
        }
    }

    // The guard's Drop soft-resets I2CC00 (clear → restore) and clears I2CM14.
    // Read our own registers to confirm the teardown actually ran.
    // SAFETY: the test owns I2C2; a read-only view of the same registers.
    let regs = unsafe { &*ast1060_pac::I2c2::ptr() };
    if !regs.i2cc00().read().enbl_master_fn().bit() {
        pw_log::error!(
            "teardown check: i2cc00=0x{:08x} master-enable not restored",
            regs.i2cc00().read().bits() as u32
        );
        return Err("teardown did not restore i2cc00 master-enable");
    }
    let m14 = regs.i2cm14().read().bits();
    if m14 != 0 {
        pw_log::error!("teardown check: i2cm14=0x{:08x} not cleared", m14 as u32);
        return Err("teardown did not clear i2cm14 latched interrupts");
    }
    pw_log::info!("phase 2: teardown verified (i2cc00 master-enable restored, i2cm14 clear)");

    pw_log::info!("=== AST10x0 I2C DMA-guard abort test PASSED ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 I2C DMA Abort Master";

    fn main() -> ! {
        let sentinel: &[u8] = match run_master() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(e) => {
                pw_log::error!("DMA abort master failed: {}", e as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
