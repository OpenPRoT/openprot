// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The managed device: reacts to its reset line, reports when it is ready.
//!
//! Stands in for the chip on the other end of two board traces. The reset
//! line arrives as a request on the `reset_cmd` channel and the ready line
//! leaves as a request on `boot_evt`, because QEMU models one chip and both
//! ends of those traces live inside it.
//!
//! The behaviour itself is `openprot_mock_bmc::MockBmc`, which is pin-generic
//! and host-tested. Here the pins are two cells: the reset cell is written by
//! the channel handler, and the ready cell is read after each poll to decide
//! whether the orchestrator needs telling.

#![no_main]
#![no_std]

use core::cell::Cell;

use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_mock_bmc::{BootBehaviour, MockBmc};
use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use app_mock_bmc::handle;

/// How long this device takes to boot once reset is released. Well inside
/// any window the orchestrator waits, so the happy path is the happy path.
#[cfg(not(device_hangs))]
const BOOT_MILLIS: u64 = 50;

/// What the device does when released. The scenario picks it at build
/// time, so the negative case is a target of its own rather than an edit
/// somebody has to remember to undo.
#[cfg(not(device_hangs))]
const BEHAVIOUR: BootBehaviour = BootBehaviour::Boots {
    after_millis: BOOT_MILLIS,
};
#[cfg(device_hangs)]
const BEHAVIOUR: BootBehaviour = BootBehaviour::Hangs;

/// How often the model is polled while it is booting. The delay is measured
/// from release rather than counted in polls, so this only bounds how late
/// the ready report can be, never whether it happens.
const POLL_MILLIS: u64 = 10;

/// Reset is asserted low, the RoT_BMC_RESET_L convention.
const RESET_ASSERTED: u8 = 0;

/// One line, shared between the model and the code that drives or watches
/// it. Single-threaded, so a `Cell` is enough.
#[derive(Clone, Copy)]
struct Line<'a>(&'a Cell<bool>);

impl ErrorType for Line<'_> {
    type Error = core::convert::Infallible;
}

impl InputPin for Line<'_> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.get())
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.0.get())
    }
}

impl OutputPin for Line<'_> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set(true);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set(false);
        Ok(())
    }
}

fn now_millis() -> u64 {
    SystemClock::now().ticks() * 1000 / SystemClock::TICKS_PER_SEC
}

fn millis_to_ticks(millis: u64) -> u64 {
    millis * SystemClock::TICKS_PER_SEC / 1000
}

#[entry]
fn entry() {
    // Reset starts asserted (low) and ready starts inactive, which is where
    // a device sits before the RoT lets it go.
    let reset = Cell::new(false);
    let ready = Cell::new(false);

    let mut bmc = MockBmc::new(
        Line(&reset),
        ActivePolarity::ActiveLow,
        Line(&ready),
        ActivePolarity::ActiveHigh,
        BEHAVIOUR,
    );

    if syscall::wait_group_add(handle::WG, handle::RESET_CMD, Signals::READABLE, 0usize).is_err() {
        loop {}
    }

    let mut reported = false;
    let mut cmd = [0u8; 1];

    loop {
        // Wake on a reset command or on the poll interval, whichever comes
        // first. Nothing here blocks on the orchestrator.
        let deadline =
            Instant::from_ticks(SystemClock::now().ticks() + millis_to_ticks(POLL_MILLIS));
        let _ = syscall::object_wait(handle::WG, Signals::READABLE, deadline);

        if let Ok(len) = syscall::channel_read(handle::RESET_CMD, 0usize, &mut cmd) {
            if len == 1usize {
                let asserted = cmd[0] == RESET_ASSERTED;
                // Active low: asserted means the line is driven low.
                reset.set(!asserted);
                // A reset clears the flag so the next boot reports again,
                // except where the scenario asks for a device that comes
                // up once and never again.
                if asserted && !cfg!(device_stays_down) {
                    reported = false;
                }
            }
            let _ = syscall::channel_respond(handle::RESET_CMD, &[0u8; 0]);
        }

        if bmc.poll(now_millis()).is_err() {
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        }

        // The ready line going active is the whole message. Report it once
        // per boot; a reset clears the flag so the next boot reports again.
        if ready.get() && !reported {
            let mut ack = [0u8; 1];
            if syscall::channel_transact(handle::BOOT_EVT, &[1u8], &mut ack, Instant::MAX).is_ok() {
                reported = true;
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
