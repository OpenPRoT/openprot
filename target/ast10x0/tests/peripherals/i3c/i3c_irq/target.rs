// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C In-Band-Interrupt test — controller side (device A).
//!
//! Faithful openprot port of aspeed-rust `tests-hw/src/i3c_test.rs::test_i3c_master`
//! (@ ce3b567). Runs on the AST1060 Test Harness with I3C **bus 2** (PAC `I3c2`)
//! wired between device A and device B on the **HV** pads (`PINCTRL_HVI3C2`), the
//! same bus/pad set the reference uses. Load the `:slave` image on device B.
//!
//! Boot order (mirrors the reference): bring up **this controller first** so it
//! is already draining the IBI work queue, then power the target — the target
//! raises a Hot-Join which this controller answers by assigning a dynamic
//! address.
//!
//! Flow (mirrors the reference): bring up the controller, pre-attach a device by
//! PID, enable its IBI, then drain the IBI work queue — on Hot-Join assign a
//! dynamic address; on a target SIR do a private read followed by a private
//! write; stop after 10 exchanges.
//!
//! Differences from the reference are panic-hygiene only (Delta D9): `unwrap`s
//! become `?`/`pw_log`, and `DummyDelay` (a no-op in the reference) is dropped.
//! Under QEMU this image is build- + `no_panics`-checked; the two-device run is
//! the `hardware`-tagged `irq_test` (`--config=k_ast1060_evb`).

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::i3c::{
    i3c_ibi_workq_consumer, Ast1060I3c, I3cConfig, I3cController, IbiWork, Ready,
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
type I3c2Controller<'c> = I3cController<'c, I3cHw, Ready>;

/// Bus index under test (PAC `I3c2`, HV pads).
const I3C_BUS: u8 = 2;

/// PID of the peer target (matches the `:slave` image / the reference).
const KNOWN_PID: u64 = 0x07ec_a003_2000;
/// Stop after this many master<->target exchanges.
const MAX_EXCHANGES: u32 = 10;
const XFER_DATA_LEN: usize = 16;
const WAIT_LOG_SPINS: u32 = 0x0400_0000;

/// Calibrated busy-wait used as the driver's yield/delay hook. Mirrors the
/// reference `DummyDelay::delay_ns` (busy-loop of ~`ns / 100` nops). A named
/// `fn` (not a closure) keeps the driver type nameable.
fn yield_delay(ns: u32) {
    for _ in 0..(ns / 100) {
        core::hint::spin_loop();
    }
}

fn log_master_read_payload(exchange: u32, len: u32, data: &[u8; XFER_DATA_LEN]) {
    let w0 = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let w1 = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let w2 = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let w3 = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    pw_log::info!(
        "MASTER_RX_FROM_TARGET #{} {}B {:08x} {:08x} {:08x} {:08x}",
        exchange as u32,
        len as u32,
        w0 as u32,
        w1 as u32,
        w2 as u32,
        w3 as u32
    );
}

fn log_master_write_payload(exchange: u32, data: &[u8; XFER_DATA_LEN]) {
    let w0 = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let w1 = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let w2 = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let w3 = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    pw_log::info!(
        "MASTER_TX_TO_TARGET #{} 16B {:08x} {:08x} {:08x} {:08x}",
        exchange as u32,
        w0 as u32,
        w1 as u32,
        w2 as u32,
        w3 as u32
    );
}

/// Build + validate the configuration in its own `#[inline(never)]` frame so
/// builder temporaries are freed on return — the kernel bootstrap stack is
/// only 2 KiB and the config embeds the ~0.5 KiB device tables. The caller
/// keeps the single live config and lends it to the controller (`&mut`).
#[inline(never)]
fn build_config() -> Result<I3cConfig, &'static str> {
    // Controller (primary) timing — identical to the reference master.
    let mut config = I3cConfig::new()
        .core_clk_hz(200_000_000)
        .secondary(false)
        .i2c_scl_hz(1_000_000)
        .i3c_scl_hz(12_500_000)
        .i3c_pp_scl_hi_period_ns(250)
        .i3c_pp_scl_lo_period_ns(250)
        .i3c_od_scl_hi_period_ns(0)
        .i3c_od_scl_lo_period_ns(0)
        .sda_tx_hold_ns(20);
    config.init_runtime_fields();
    config
        .validate_clock()
        .map_err(|_| "invalid clock configuration")?;
    Ok(config)
}

#[inline(never)]
fn master_read_from_target(
    ctrl: &mut I3c2Controller<'_>,
) -> Result<(u32, [u8; XFER_DATA_LEN]), &'static str> {
    let mut rx_buf = [0u8; 128];
    let actual_len = ctrl
        .priv_read(KNOWN_PID, &mut rx_buf)
        .map_err(|_| "private read failed")?;
    let mut data = [0u8; XFER_DATA_LEN];
    let take = (actual_len as usize).min(data.len()).min(rx_buf.len());
    data[..take].copy_from_slice(&rx_buf[..take]);
    Ok((actual_len, data))
}

#[inline(never)]
fn master_write_to_target(
    ctrl: &mut I3c2Controller<'_>,
    exchange: u32,
) -> Result<(), &'static str> {
    let mut tx_buf: [u8; XFER_DATA_LEN] = [
        0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ];
    ctrl.priv_write(KNOWN_PID, &mut tx_buf)
        .map_err(|_| "private write failed")?;
    log_master_write_payload(exchange, &tx_buf);
    Ok(())
}

fn run_controller() -> Result<(), &'static str> {
    pw_log::info!("####### I3C master test #######");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_HVI3C2],
    });
    // SAFETY: single call at boot with exclusive access to the board.
    unsafe { board.init() };

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

    // The kernel vector (system.json5 IRQ 104 -> `i3c2_irq`) is in place and
    // the handler is registered; this integration layer owns the NVIC line
    // for the bus it selected (`I3C_BUS` = 2 -> `Interrupt::i3c2`), so unmask
    // it now.
    // SAFETY: handler registered and hardware initialized (Ready state);
    // unmasking cannot deliver an IRQ into partially-initialized state.
    unsafe { NVIC::unmask(ast1060_pac::Interrupt::i3c2) };
    pw_log::info!("I3C2 controller ready");

    let dyn_addr = ctrl
        .alloc_dynamic_address_from(8)
        .ok_or("no dynamic address available")?;
    ctrl.attach_i3c_dev(KNOWN_PID, dyn_addr, 0)
        .map_err(|_| "attach_i3c_dev failed")?;
    ctrl.enable_ibi(dyn_addr, 0)
        .map_err(|_| "ibi_enable failed")?;
    pw_log::info!("pre-attached dev at slot 0, dyn addr {}", dyn_addr as u32);

    let mut received = 0u32;
    let mut spin_count = 0u32;
    loop {
        let Some(work) = ibi_cons.dequeue() else {
            core::hint::spin_loop();
            spin_count = spin_count.wrapping_add(1);
            if spin_count & (WAIT_LOG_SPINS - 1) == 0 {
                let irq_count = I3C2_IRQ_COUNT.load(core::sync::atomic::Ordering::Relaxed);
                let status = I3C2_LAST_STATUS.load(core::sync::atomic::Ordering::Relaxed);
                let queue_status =
                    I3C2_LAST_QUEUE_STATUS.load(core::sync::atomic::Ordering::Relaxed);
                let status_en = I3C2_LAST_STATUS_EN.load(core::sync::atomic::Ordering::Relaxed);
                let signal_en = I3C2_LAST_SIGNAL_EN.load(core::sync::atomic::Ordering::Relaxed);
                let ibi_count = (queue_status >> 24) & 0x1f;
                let ibi_buf_blr = (queue_status >> 16) & 0xff;
                let resp_blr = (queue_status >> 8) & 0xff;
                pw_log::info!(
                    "[DBG] waiting irq_count={} spin={}",
                    irq_count as u32,
                    spin_count as u32
                );
                pw_log::info!(
                    "[DBG] i3c2 status={} queue={}",
                    status as u32,
                    queue_status as u32
                );
                pw_log::info!(
                    "[DBG] i3c2 ibi_count={} ibi_buf_blr={}",
                    ibi_count as u32,
                    ibi_buf_blr as u32
                );
                pw_log::info!(
                    "[DBG] i3c2 resp_blr={} status_en={}",
                    resp_blr as u32,
                    status_en as u32
                );
                pw_log::info!(
                    "[DBG] i3c2 signal_en={} reserved={}",
                    signal_en as u32,
                    0 as u32
                );
            }
            continue;
        };
        match work {
            IbiWork::HotJoin => {
                pw_log::info!("[IBI] hotjoin");
                let _ = ctrl.handle_hot_join();
                let _ = ctrl.assign_dynamic_address(dyn_addr);
            }
            IbiWork::Sirq { addr, len, .. } => {
                pw_log::info!("[IBI] SIRQ from 0x{:02x} len {}", addr as u32, len as u32);
                if ctrl.acknowledge_ibi(addr).is_err() {
                    pw_log::error!("acknowledge_ibi failed");
                }

                let exchange = received;
                let (read_len, read_data) = master_read_from_target(&mut ctrl)?;
                log_master_read_payload(exchange, read_len, &read_data);

                master_write_to_target(&mut ctrl, exchange)?;
                received += 1;

                if received >= MAX_EXCHANGES {
                    pw_log::info!("I3C master test done");
                    return Ok(());
                }
            }
            IbiWork::TargetDaAssignment => pw_log::info!("[IBI] TargetDaAssignment"),
            IbiWork::TargetMasterWrite { len, .. } => {
                pw_log::info!("[IBI] TargetMasterWrite len {}", len as u32);
            }
        }
    }
}

static I3C2_IRQ_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static I3C2_LAST_STATUS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static I3C2_LAST_QUEUE_STATUS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
static I3C2_LAST_STATUS_EN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
static I3C2_LAST_SIGNAL_EN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

pub fn i3c2_irq<K: Kernel>(_k: K) {
    // Do not read i3cd018 here: that register pops the IBI queue entry.
    let regs = unsafe { &*ast1060_pac::I3c2::ptr() };
    I3C2_LAST_STATUS.store(
        regs.i3cd03c().read().bits(),
        core::sync::atomic::Ordering::Relaxed,
    );
    I3C2_LAST_QUEUE_STATUS.store(
        regs.i3cd04c().read().bits(),
        core::sync::atomic::Ordering::Relaxed,
    );
    I3C2_LAST_STATUS_EN.store(
        regs.i3cd040().read().bits(),
        core::sync::atomic::Ordering::Relaxed,
    );
    I3C2_LAST_SIGNAL_EN.store(
        regs.i3cd044().read().bits(),
        core::sync::atomic::Ordering::Relaxed,
    );
    I3C2_IRQ_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    ast10x0_peripherals::i3c::dispatch_i3c_irq(2);
}

codegen::declare_kernel_interrupt_handlers!();

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 Kernel I3C IBI (controller)";

    fn main() -> ! {
        let sentinel: &[u8] = match run_controller() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("I3C IBI controller test failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };
        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
