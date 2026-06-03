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
    i3c_ibi_workq_consumer, Ast1060I3c, HardwareCore, HardwareTarget, HardwareTransfer, I3cConfig,
    I3cController, I3cTargetConfig, IbiWork,
};
use ast10x0_peripherals::scu::pinctrl;
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

/// Number of IBIs the target raises once it has a dynamic address.
const MAX_IBIS: u32 = 10;

fn run_target() -> Result<(), &'static str> {
    pw_log::info!("####### I3C target test #######");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[pinctrl::PINCTRL_HVI3C2],
    });
    // SAFETY: single call at boot with exclusive access to the board.
    unsafe { board.init() };

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

    // SAFETY: the test owns I3C bus 2 and uses the matching PAC blocks.
    let hw = unsafe { Ast1060I3c::<ast1060_pac::I3c2, _>::new(|_| core::hint::spin_loop()) };
    let mut ctrl = I3cController::new(hw, config);
    ctrl.init_hardware();

    let bus = ctrl.hw.bus_num() as usize;
    let mut ibi_cons = i3c_ibi_workq_consumer(bus).ok_or("IBI consumer unavailable")?;

    let dyn_addr = 8u8;
    let dev_idx = 0usize;
    let _ = ctrl.hw.attach_i3c_dev(dev_idx, dyn_addr);
    pw_log::info!(
        "target dev at slot {}, dyn addr {}",
        dev_idx as u32,
        dyn_addr as u32
    );

    pw_log::info!("raising hot-join; waiting for dynamic address assignment...");
    let _ = ctrl.hw.target_ibi_raise_hj(&mut ctrl.config);

    // Wait for the controller to assign our dynamic address.
    loop {
        let Some(work) = ibi_cons.dequeue() else {
            core::hint::spin_loop();
            continue;
        };
        match work {
            IbiWork::TargetDaAssignment => {
                let da = ctrl.config.target_config.as_ref().and_then(|t| t.addr);
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
        }
    }

    // Raise IBIs, each presenting a 16-byte incrementing payload for the master.
    let mut ibi_count = 0u32;
    while ibi_count < MAX_IBIS {
        let mut data = [0u8; 16];
        for (i, b) in data.iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap_or(0);
        }
        pw_log::info!(
            "[MASTER <== TARGET] target write, ibi #{}",
            ibi_count as u32
        );
        if ctrl.target_get_ibi_payload(&mut data).is_err() {
            return Err("target_get_ibi_payload failed");
        }
        ibi_count += 1;
    }

    pw_log::info!("I3C target test done");
    Ok(())
}

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
