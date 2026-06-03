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
    i3c_ibi_workq_consumer, Ast1060I3c, HardwareCore, HardwareTransfer, I3cConfig, I3cController,
    I3cMsg, IbiWork, I3C_MSG_READ, I3C_MSG_STOP, I3C_MSG_WRITE,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

/// PID of the peer target (matches the `:slave` image / the reference).
const KNOWN_PID: u64 = 0x07ec_a003_2000;
/// Stop after this many master<->target exchanges.
const MAX_EXCHANGES: u32 = 10;

fn run_controller() -> Result<(), &'static str> {
    pw_log::info!("####### I3C master test #######");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_HVI3C2],
    });
    // SAFETY: single call at boot with exclusive access to the board.
    unsafe { board.init() };

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

    // SAFETY: the test owns I3C bus 2 and uses the matching PAC blocks; the
    // busy-spin closure is the bare-metal wait policy.
    let hw = unsafe { Ast1060I3c::<ast1060_pac::I3c2, _>::new(|_| core::hint::spin_loop()) };
    let mut ctrl = I3cController::new(hw, config);
    ctrl.init_hardware();

    let bus = ctrl.hw.bus_num() as usize;
    let mut ibi_cons = i3c_ibi_workq_consumer(bus).ok_or("IBI consumer unavailable")?;

    let dyn_addr = ctrl
        .config
        .addrbook
        .alloc_from(8)
        .ok_or("no dynamic address available")?;
    ctrl.attach_i3c_dev(KNOWN_PID, dyn_addr, 0)
        .map_err(|_| "attach_i3c_dev failed")?;
    ctrl.hw.set_ibi_mdb(0);
    ctrl.hw
        .ibi_enable(&mut ctrl.config, dyn_addr)
        .map_err(|_| "ibi_enable failed")?;
    pw_log::info!("pre-attached dev at slot 0, dyn addr {}", dyn_addr as u32);

    let mut received = 0u32;
    loop {
        let Some(work) = ibi_cons.dequeue() else {
            core::hint::spin_loop();
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

                // Private read: MASTER <== TARGET
                let mut rx_buf = [0u8; 128];
                let mut rd_msgs = [I3cMsg {
                    buf: Some(&mut rx_buf[..]),
                    actual_len: 128,
                    num_xfer: 0,
                    flags: I3C_MSG_READ | I3C_MSG_STOP,
                    hdr_mode: 0,
                    hdr_cmd_mode: 0,
                }];
                let _ = ctrl.hw.priv_xfer(&mut ctrl.config, KNOWN_PID, &mut rd_msgs);
                pw_log::info!(
                    "[MASTER <== TARGET] read {} bytes",
                    rd_msgs[0].actual_len as u32
                );

                received += 1;
                if received > MAX_EXCHANGES {
                    pw_log::info!("I3C master test done");
                    return Ok(());
                }

                // Private write: MASTER ==> TARGET
                let mut tx_buf: [u8; 16] = [
                    0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0x11, 0x22, 0x33, 0x44, 0x55,
                    0x66, 0x77, 0x88,
                ];
                let mut wr_msgs = [I3cMsg {
                    buf: Some(&mut tx_buf[..]),
                    actual_len: 16,
                    num_xfer: 0,
                    flags: I3C_MSG_WRITE | I3C_MSG_STOP,
                    hdr_mode: 0,
                    hdr_cmd_mode: 0,
                }];
                let _ = ctrl.hw.priv_xfer(&mut ctrl.config, KNOWN_PID, &mut wr_msgs);
                pw_log::info!("[MASTER ==> TARGET] wrote 16 bytes");
            }
            IbiWork::TargetDaAssignment => pw_log::info!("[IBI] TargetDaAssignment"),
        }
    }
}

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
