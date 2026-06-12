// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C In-Band-Interrupt test — target side (device B).
//!
//! Faithful openprot port of aspeed-rust `tests-hw/src/i3c_test.rs::test_i3c_target`
//! (@ ce3b567). Companion to `target.rs`; runs on the AST1060 Test Harness on
//! I3C **bus 2** (PAC `I3c2`) HV pads (`PINCTRL_HVI3C2`).
//!
//! Boot order (mirrors the reference): power **the controller first**, then this
//! target — the controller must already be draining the IBI work queue when this
//! target raises its Hot-Join.
//!
//! Flow (mirrors the reference): come up in secondary mode, attach a device,
//! raise a Hot-Join, wait for the controller to assign a dynamic address, then
//! send 10 IBIs (each making a 16-byte payload available for the controller to
//! read). Panic-hygiene-only differences from the reference (Delta D9).
//!
//! Under QEMU this image is build- + `no_panics`-checked; the real exchange runs
//! under the `hardware`-tagged `irq_test` (`--config=k_ast1060_evb`).

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i3c::{
    i3c_ibi_workq_consumer, Ast1060I3c, I3cConfig, I3cController, I3cTargetConfig, IbiConsumer,
    IbiWork,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use cortex_m::peripheral::NVIC;
use entry as _;
use kernel::Kernel;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

/// One driver type serves every bus; the instance is selected at runtime.
type I3cHw = Ast1060I3c<fn(u32)>;

/// Bus index under test (PAC `I3c2`, HV pads).
const I3C_BUS: u8 = 2;

/// Number of IBIs the target raises once it has a dynamic address.
const MAX_IBIS: u32 = 10;
/// Give the controller time to finish init and open the hot-join ACK window.
const HOT_JOIN_STARTUP_DELAY_SPINS: u32 = 0x1000_0000;
/// Re-raise hot-join while waiting in case the first request hit the NACK window.
const HOT_JOIN_RETRY_SPINS: u32 = 0x0400_0000;
const WAIT_MASTER_WRITE_SPINS: u32 = 0x0400_0000;
const XFER_DATA_LEN: usize = 16;

fn spin_wait(mut cycles: u32) {
    while cycles != 0 {
        core::hint::spin_loop();
        cycles = cycles.wrapping_sub(1);
    }
}

/// Read-only register snapshot for debugging (never pops a queue).
fn dump_slave_i3c2(label: u32) {
    let regs = unsafe { &*ast1060_pac::I3c2::ptr() };
    let status = regs.i3cd03c().read().bits();
    let queue = regs.i3cd04c().read().bits();
    let present = regs.i3cd054().read().bits();
    let event_ctrl = regs.i3cd038().read().bits();
    let dev_addr = regs.i3cd004().read().bits();
    pw_log::info!(
        "[SDUMP{}] status={:08x} queue={:08x}",
        label as u32,
        status as u32,
        queue as u32
    );
    pw_log::info!(
        "[SDUMP{}] present={:08x} event_ctrl={:08x}",
        label as u32,
        present as u32,
        event_ctrl as u32
    );
    pw_log::info!("[SDUMP{}] dev_addr={:08x}", label as u32, dev_addr as u32);
}

fn log_target_hj_state(label: u32) {
    let regs = unsafe { &*ast1060_pac::I3c2::ptr() };
    let dev_addr = regs.i3cd004().read().bits();
    let event_ctrl = regs.i3cd038().read().bits();
    let device_char = regs.i3cd008().read().bits();
    pw_log::info!(
        "[DBG] target hj label={} dev_addr={}",
        label as u32,
        dev_addr as u32
    );
    pw_log::info!(
        "[DBG] target hj event_ctrl={} device_char={}",
        event_ctrl as u32,
        device_char as u32
    );
}

/// Logs the first [`XFER_DATA_LEN`] bytes of a received master write. The
/// work item's inline buffer is larger (`IBI_MWR_DATA_MAX`); this test only
/// exchanges 16-byte payloads.
fn log_target_master_write(exchange: u32, len: u8, data: &[u8]) {
    let mut d = [0u8; XFER_DATA_LEN];
    let take = data.len().min(d.len());
    d[..take].copy_from_slice(&data[..take]);
    let w0 = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
    let w1 = u32::from_be_bytes([d[4], d[5], d[6], d[7]]);
    let w2 = u32::from_be_bytes([d[8], d[9], d[10], d[11]]);
    let w3 = u32::from_be_bytes([d[12], d[13], d[14], d[15]]);
    pw_log::info!(
        "TARGET_RX_FROM_MASTER #{} {}B {:08x} {:08x} {:08x} {:08x}",
        exchange as u32,
        len as u32,
        w0 as u32,
        w1 as u32,
        w2 as u32,
        w3 as u32
    );
}

fn log_target_read_payload(ibi_count: u32, data: &[u8; XFER_DATA_LEN]) {
    let w0 = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let w1 = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let w2 = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let w3 = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    pw_log::info!(
        "TARGET_TX_TO_MASTER #{} 16B {:08x} {:08x} {:08x} {:08x}",
        ibi_count as u32,
        w0 as u32,
        w1 as u32,
        w2 as u32,
        w3 as u32
    );
}

fn wait_for_master_write(ibi_cons: &mut IbiConsumer, exchange: u32) -> Result<(), &'static str> {
    let mut spin_count = 0u32;
    loop {
        let Some(work) = ibi_cons.dequeue() else {
            core::hint::spin_loop();
            spin_count = spin_count.wrapping_add(1);
            if spin_count >= WAIT_MASTER_WRITE_SPINS {
                return Err("master write not received");
            }
            continue;
        };

        match work {
            IbiWork::TargetMasterWrite { len, data } => {
                log_target_master_write(exchange, len, &data);
                return Ok(());
            }
            IbiWork::TargetDaAssignment => pw_log::info!("[IBI] TargetDaAssignment"),
            IbiWork::HotJoin => pw_log::info!("[IBI] hotjoin"),
            IbiWork::Sirq { addr, len, .. } => {
                pw_log::info!("[IBI] SIRQ from 0x{:02x} len {}", addr as u32, len as u32);
            }
        }
    }
}

/// Calibrated busy-wait used as the driver's yield/delay hook. Mirrors the
/// reference `DummyDelay::delay_ns` (busy-loop of ~`ns / 100` nops). A named
/// `fn` (not a closure) keeps the driver type nameable.
fn yield_delay(ns: u32) {
    for _ in 0..(ns / 100) {
        core::hint::spin_loop();
    }
}

/// Build + validate the configuration in its own `#[inline(never)]` frame so
/// builder temporaries are freed on return — the kernel bootstrap stack is
/// only 2 KiB and the config embeds the ~0.5 KiB device tables. The caller
/// keeps the single live config and lends it to the controller (`&mut`).
#[inline(never)]
fn build_config() -> Result<I3cConfig, &'static str> {
    // Secondary (target) timing — identical to the reference target.
    let mut config = I3cConfig::new()
        .core_clk_hz(200_000_000)
        .secondary(true)
        .i2c_scl_hz(1_000_000)
        .i3c_scl_hz(12_500_000)
        .i3c_pp_scl_hi_period_ns(36)
        .i3c_pp_scl_lo_period_ns(36)
        .i3c_od_scl_hi_period_ns(0)
        .i3c_od_scl_lo_period_ns(0)
        .sda_tx_hold_ns(0)
        .dcr(0xcc)
        .target_config(I3cTargetConfig::new(0, Some(0), 0xae));
    config.init_runtime_fields();
    config
        .validate_clock()
        .map_err(|_| "invalid clock configuration")?;
    Ok(config)
}

fn run_target() -> Result<(), &'static str> {
    pw_log::info!("####### I3C target test #######");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_HVI3C2],
        i2c_buses: &[],
    });
    // SAFETY: single call at boot with exclusive access to the board.
    unsafe { board.init() }.expect("board init failed");

    // Build the config in a separate (never-inlined) frame, keep the single
    // live copy here, and lend it to the controller — see `build_config`.
    let mut config = build_config()?;
    // SAFETY: the test owns I3C bus 2 and uses the matching PAC blocks.
    let hw = unsafe { I3cHw::new(I3C_BUS, yield_delay) }.ok_or("invalid I3C bus index")?;
    let mut ctrl = I3cController::new(hw, &mut config)
        .start()
        .map_err(|_| "controller start failed")?;
    let bus = ctrl.bus_num() as usize;
    let mut ibi_cons = i3c_ibi_workq_consumer(bus).ok_or("IBI consumer unavailable")?;
    pw_log::info!("IBI work queue ready on bus {}", bus as u32);

    // Kernel vector is in place and the handler is registered; this
    // integration layer owns the NVIC line for the bus it selected
    // (`I3C_BUS` = 2 -> `Interrupt::i3c2`), so unmask it now.
    // SAFETY: handler registered and hardware initialized (Ready state);
    // unmasking cannot deliver an IRQ into partially-initialized state.
    unsafe { NVIC::unmask(ast1060_pac::Interrupt::i3c2) };

    let dyn_addr = 8u8;
    let dev_idx = 0usize;
    let _ = ctrl.attach_i3c_dev(0, dyn_addr, dev_idx as u8);
    pw_log::info!(
        "target dev at slot {}, dyn addr {}",
        dev_idx as u32,
        dyn_addr as u32
    );

    pw_log::info!("waiting before hot-join...");
    spin_wait(HOT_JOIN_STARTUP_DELAY_SPINS);
    pw_log::info!("raising hot-join; waiting for dynamic address assignment...");
    let hj_ok = ctrl.target_raise_hot_join().is_ok();
    pw_log::info!("[DBG] hot-join raise ok={}", hj_ok as u32);
    log_target_hj_state(0);

    // Wait for the controller to assign our dynamic address.
    let mut spin_count = 0u32;
    loop {
        let Some(work) = ibi_cons.dequeue() else {
            core::hint::spin_loop();
            spin_count = spin_count.wrapping_add(1);
            if spin_count & (HOT_JOIN_RETRY_SPINS - 1) == 0 {
                pw_log::info!("[DBG] retry hot-join");
                let hj_ok = ctrl.target_raise_hot_join().is_ok();
                pw_log::info!("[DBG] hot-join retry ok={}", hj_ok as u32);
                log_target_hj_state(1);
            }
            continue;
        };
        match work {
            IbiWork::TargetDaAssignment => {
                let da = ctrl.target_dynamic_address();
                if let Some(da) = da {
                    pw_log::info!("[IBI] dyn addr 0x{:02x} assigned by master", da as u32);
                }
                ctrl.target_on_dynamic_address_assigned();
                break;
            }
            IbiWork::HotJoin => pw_log::info!("[IBI] hotjoin"),
            IbiWork::Sirq { addr, len, .. } => {
                pw_log::info!("[IBI] SIRQ from 0x{:02x} len {}", addr as u32, len as u32);
            }
            IbiWork::TargetMasterWrite { len, data } => {
                log_target_master_write(0, len, &data);
            }
        }
    }

    // Raise IBIs, each presenting a 16-byte incrementing payload for the master.
    let mut ibi_count = 0u32;
    while ibi_count < MAX_IBIS {
        let mut data = [0u8; XFER_DATA_LEN];
        for (i, b) in data.iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap_or(0);
        }
        dump_slave_i3c2(ibi_count);
        if ctrl.target_get_ibi_payload(&mut data).is_err() {
            dump_slave_i3c2(0xdead);
            return Err("target_get_ibi_payload failed");
        }
        log_target_read_payload(ibi_count, &data);
        wait_for_master_write(&mut ibi_cons, ibi_count)?;
        ibi_count += 1;
    }

    pw_log::info!("I3C target test done");
    Ok(())
}

pub fn i3c2_irq<K: Kernel>(_k: K) {
    ast10x0_peripherals::i3c::dispatch_i3c_irq(2);
}

codegen::declare_kernel_interrupt_handlers!();

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel I3C IBI (target)";

    fn main() -> ! {
        let sentinel: &[u8] = match run_target() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("I3C IBI target test failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
