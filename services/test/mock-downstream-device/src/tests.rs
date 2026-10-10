// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use super::*;

use core::cell::Cell;
use std::rc::Rc;

use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::{InputPin, OutputPin};

/// A pin pair the test drives and reads directly. Shared through `Rc` so a
/// test can hold the reset line while the model holds its other end.
#[derive(Clone, Default)]
struct Line(Rc<Cell<bool>>);

impl Line {
    fn high() -> Self {
        Self(Rc::new(Cell::new(true)))
    }

    fn low() -> Self {
        Self(Rc::new(Cell::new(false)))
    }

    fn set(&self, high: bool) {
        self.0.set(high);
    }

    fn is_set_high(&self) -> bool {
        self.0.get()
    }
}

impl embedded_hal::digital::ErrorType for Line {
    type Error = core::convert::Infallible;
}

impl InputPin for Line {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.0.get())
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.0.get())
    }
}

impl OutputPin for Line {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set(true);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set(false);
        Ok(())
    }
}

/// Reset active low (the RoT_BMC_RESET_L convention), ready active high.
/// Starts asserted: reset low means held.
fn wire(behaviour: BootBehaviour) -> (Line, Line, MockDownstreamDevice<Line, Line>) {
    let reset = Line::low();
    let ready = Line::low();
    let device = MockDownstreamDevice::new(
        reset.clone(),
        ActivePolarity::ActiveLow,
        ready.clone(),
        ActivePolarity::ActiveHigh,
        behaviour,
    );
    (reset, ready, device)
}

/// Releases reset: drives the active-low line high.
fn release(reset: &Line) {
    reset.set(true);
}

/// Asserts reset: drives the active-low line low.
fn assert_reset(reset: &Line) {
    reset.set(false);
}

#[test]
fn held_in_reset_the_ready_line_stays_low() {
    let (_reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 10 });

    assert_eq!(device.poll(0).unwrap(), Phase::Held);
    assert_eq!(device.poll(1_000).unwrap(), Phase::Held);
    assert!(!ready.is_set_high());
}

#[test]
fn ready_goes_active_once_the_delay_has_passed() {
    let (reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 10 });

    device.poll(0).unwrap();
    release(&reset);

    assert_eq!(
        device.poll(100).unwrap(),
        Phase::Booting {
            released_at_millis: 100
        }
    );
    assert!(!ready.is_set_high(), "ready before the delay elapsed");

    assert_eq!(
        device.poll(109).unwrap(),
        Phase::Booting {
            released_at_millis: 100
        }
    );
    assert!(!ready.is_set_high(), "ready one millisecond early");

    assert_eq!(device.poll(110).unwrap(), Phase::Ready);
    assert!(ready.is_set_high());
}

#[test]
fn the_delay_runs_from_release_not_from_the_poll_that_notices() {
    let (reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 10 });

    device.poll(0).unwrap();
    release(&reset);
    device.poll(100).unwrap();

    // A caller that goes away and comes back late still gets ready on the
    // first poll past the deadline, not ten more milliseconds later.
    assert_eq!(device.poll(5_000).unwrap(), Phase::Ready);
    assert!(ready.is_set_high());
}

#[test]
fn a_zero_delay_reports_ready_on_the_first_poll_after_release() {
    let (reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 0 });

    device.poll(0).unwrap();
    release(&reset);

    // Release is seen first, then the delay is judged, so this takes two
    // polls. An event loop polls on every wake, so that costs nothing.
    assert_eq!(
        device.poll(7).unwrap(),
        Phase::Booting {
            released_at_millis: 7
        }
    );
    assert_eq!(device.poll(7).unwrap(), Phase::Ready);
    assert!(ready.is_set_high());
}

#[test]
fn a_hanging_device_never_reports_ready() {
    let (reset, ready, mut device) = wire(BootBehaviour::Hangs);

    device.poll(0).unwrap();
    release(&reset);

    assert_eq!(device.poll(1).unwrap(), Phase::Hung);
    assert_eq!(device.poll(u64::MAX).unwrap(), Phase::Hung);
    assert!(!ready.is_set_high());
}

#[test]
fn asserting_reset_drops_the_evidence() {
    let (reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 10 });

    device.poll(0).unwrap();
    release(&reset);
    device.poll(100).unwrap();
    device.poll(110).unwrap();
    assert!(ready.is_set_high());

    assert_reset(&reset);
    assert_eq!(device.poll(120).unwrap(), Phase::Held);
    assert!(!ready.is_set_high(), "ready survived a reset");
}

#[test]
fn a_reset_mid_boot_restarts_the_delay() {
    let (reset, ready, mut device) = wire(BootBehaviour::Boots { after_millis: 10 });

    device.poll(0).unwrap();
    release(&reset);
    device.poll(100).unwrap();

    assert_reset(&reset);
    assert_eq!(device.poll(105).unwrap(), Phase::Held);

    release(&reset);
    assert_eq!(
        device.poll(200).unwrap(),
        Phase::Booting {
            released_at_millis: 200
        }
    );
    assert!(!ready.is_set_high(), "the old delay carried over");
    assert_eq!(device.poll(210).unwrap(), Phase::Ready);
}

#[test]
fn a_hung_device_is_reset_like_any_other() {
    let (reset, _ready, mut device) = wire(BootBehaviour::Hangs);

    device.poll(0).unwrap();
    release(&reset);
    assert_eq!(device.poll(1).unwrap(), Phase::Hung);

    assert_reset(&reset);
    assert_eq!(device.poll(2).unwrap(), Phase::Held);
}

#[test]
fn an_active_high_reset_reads_the_other_way_round() {
    let reset = Line::high();
    let ready = Line::low();
    let mut device = MockDownstreamDevice::new(
        reset.clone(),
        ActivePolarity::ActiveHigh,
        ready.clone(),
        ActivePolarity::ActiveHigh,
        BootBehaviour::Boots { after_millis: 0 },
    );

    assert_eq!(device.poll(0).unwrap(), Phase::Held);
    reset.set(false);
    device.poll(1).unwrap();
    assert_eq!(device.poll(1).unwrap(), Phase::Ready);
}

#[test]
fn an_active_low_ready_line_is_driven_low_when_active() {
    let reset = Line::low();
    let ready = Line::high();
    let mut device = MockDownstreamDevice::new(
        reset.clone(),
        ActivePolarity::ActiveLow,
        ready.clone(),
        ActivePolarity::ActiveLow,
        BootBehaviour::Boots { after_millis: 0 },
    );

    device.poll(0).unwrap();
    release(&reset);
    device.poll(1).unwrap();
    assert_eq!(device.poll(1).unwrap(), Phase::Ready);
    assert!(!ready.is_set_high(), "an active-low ready line reads low");
}
