// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI monitor register and routing smoke test.

#![no_std]
#![no_main]

use ast10x0_board::apply_spim_pinctrl;
use ast10x0_peripherals::scu::{ScuRegisters, SpiMonitorInstance};
use ast10x0_peripherals::spimonitor::{
    command_table_value, ExtMuxSel, MonitorPolicy, MonitorState, PassthroughMode, SpiMonitor,
    SpiMonitorController, SpiMonitorError, Uninitialized,
};
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

const CTRL_MONITOR_ENABLE: u32 = 1 << 2;
const CTRL_SINGLE_BIT_PASSTHROUGH: u32 = 1 << 0;
const TEST_COMMANDS: [u8; 3] = [0x9f, 0x05, 0x06];
const FIRST_GENERAL_COMMAND_SLOT: usize = 2;

fn check_register(actual: u32, expected: u32) -> Result<(), SpiMonitorError> {
    if actual != expected {
        pw_log::info!(
            "register mismatch: expected=0x{:08x}, actual=0x{:08x}",
            expected as u32,
            actual as u32
        );
        return Err(SpiMonitorError::InvalidTransition);
    }
    Ok(())
}

fn run_spimonitor_test() -> Result<(), SpiMonitorError> {
    pw_log::info!("=== AST10x0 SPI monitor smoke test ===");

    // Match the device-tree pinctrl-0 setup for spim1/SPIPF1.
    let scu = unsafe { ScuRegisters::new_global_unlocked() };
    apply_spim_pinctrl(&scu, SpiMonitorInstance::Spim0);

    // This target owns SPIM0/SPIPF1 for its complete lifetime.
    let monitor = unsafe {
        SpiMonitor::<Uninitialized>::new(SpiMonitorController::Spim0)
    };
    if monitor.state() != MonitorState::Uninitialized {
        return Err(SpiMonitorError::InvalidTransition);
    }

    let original_ctrl = monitor.regs().read_ctrl();
    let original_slots = [
        monitor.regs().read_allow_cmd_slot(2),
        monitor.regs().read_allow_cmd_slot(3),
        monitor.regs().read_allow_cmd_slot(4),
    ];

    let mut policy = MonitorPolicy::empty();
    policy.allow_commands[..TEST_COMMANDS.len()].copy_from_slice(&TEST_COMMANDS);
    policy.allow_command_count = TEST_COMMANDS.len();

    let configured = monitor.apply_policy(&policy)?;
    if configured.state() != MonitorState::Configured {
        return Err(SpiMonitorError::InvalidTransition);
    }

    let original_mux = configured.get_ext_mux();
    let result = (|| {
        for (index, command) in TEST_COMMANDS.iter().copied().enumerate() {
            let expected =
                command_table_value(command, false).ok_or(SpiMonitorError::UnsupportedCommand(
                    command,
                ))?;
            check_register(
                configured
                    .regs()
                    .read_allow_cmd_slot(FIRST_GENERAL_COMMAND_SLOT + index),
                expected,
            )?;
        }
        pw_log::info!("command table readback passed");

        configured.enable();
        check_register(
            configured.regs().read_ctrl() & CTRL_MONITOR_ENABLE,
            CTRL_MONITOR_ENABLE,
        )?;

        configured.set_passthrough(PassthroughMode::Enabled);
        check_register(
            configured.regs().read_ctrl() & CTRL_SINGLE_BIT_PASSTHROUGH,
            CTRL_SINGLE_BIT_PASSTHROUGH,
        )?;

        configured.set_passthrough(PassthroughMode::Disabled);
        check_register(
            configured.regs().read_ctrl() & CTRL_SINGLE_BIT_PASSTHROUGH,
            0,
        )?;

        let test_mux = match original_mux {
            ExtMuxSel::Sel0 => ExtMuxSel::Sel1,
            ExtMuxSel::Sel1 => ExtMuxSel::Sel0,
        };
        configured.set_ext_mux(test_mux);
        if configured.get_ext_mux() != test_mux {
            pw_log::info!("external mux readback failed");
            return Err(SpiMonitorError::InvalidTransition);
        }
        pw_log::info!("control and external mux readback passed");
        Ok(())
    })();

    // Restore every register changed by this smoke test.
    configured.set_ext_mux(original_mux);
    configured.regs().write_ctrl(original_ctrl);
    for (index, value) in original_slots.iter().copied().enumerate() {
        configured
            .regs()
            .write_allow_cmd_slot(FIRST_GENERAL_COMMAND_SLOT + index, value);
    }

    result
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 SPI Monitor Smoke Test";

    fn main() -> ! {
        let sentinel = if run_spimonitor_test().is_ok() {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);
