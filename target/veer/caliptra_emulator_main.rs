// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Entry point for the Caliptra MCU emulator.
//!
//! Replaces the upstream `emulator/app/src/main.rs`, which only pumps I3C
//! socket traffic into the emulated target in selected test modes
//! (for example via `--test-feature ...`). Host-side tests drive firmware
//! over the I3C TCP socket (`--i3c-port`), so this entry point unconditionally
//! starts the I3C controller before entering the run loop.
//!
//! Firmware-requested exits terminate the process from within `step()` (the
//! emulator's exit-control peripheral calls `std::process::exit`), so the
//! loop only ends on fatal errors or breakpoints.

use caliptra_emu_cpu::StepAction;
use clap::Parser;
use emulator::{Emulator, EmulatorArgs};
use mcu_testing_common::MCU_RUNNING;
use std::io;

fn main() -> io::Result<()> {
    let cli = EmulatorArgs::parse();
    let mut emulator = Emulator::from_args(cli, false)?;
    emulator.start_i3c_controller();
    while MCU_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        match emulator.step() {
            StepAction::Break | StepAction::Fatal => break,
            _ => {}
        }
    }
    Ok(())
}
