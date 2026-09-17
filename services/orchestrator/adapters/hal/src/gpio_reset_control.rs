// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HAL-backed [`ResetControl`]: drive one device's reset line off a GPIO output.

use core::time::Duration;
use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::system_control::{Error as HalError, ErrorKind, ErrorType, ResetControl};
use openprot_hal_blocking::{DelayNs, OutputPin};

/// Why driving a reset line failed.
///
/// The HAL `Error`/`kind()` pattern over a GPIO-backed line: the pin's own
/// error is carried verbatim so implementation detail survives, while
/// [`kind`](HalError::kind) gives generic code the category it acts on.
#[derive(Debug)]
pub enum ResetLineError<E> {
    /// The reset id named a line this controller does not drive.
    UnknownLine,
    /// The GPIO line could not be driven.
    Pin(E),
}

impl<E: core::fmt::Debug> HalError for ResetLineError<E> {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::UnknownLine => ErrorKind::InvalidResetId,
            Self::Pin(_) => ErrorKind::HardwareFailure,
        }
    }
}

/// Binds one GPIO output line to one managed device's reset signal.
///
/// The actuation-side counterpart to [`GpioBootMonitor`]: the `(pin, line,
/// active)` binding is made once, in platform configuration, and the
/// orchestrator only ever names the device.
///
/// Unlike the monitor, this *owns* its pin rather than borrowing a port. A
/// reset is one dedicated line, so ownership is the exclusivity guarantee —
/// nothing else can drive the device into reset behind the orchestrator's
/// back. The monitor borrows because a bank packs many unrelated ready
/// signals; that does not apply here.
///
/// `active` is the level that *holds the device in reset*, so an active-low
/// line like `RoT_BMC_RESET_L` is configuration rather than an inverted
/// `set_high` the reader has to decode.
///
/// [`GpioBootMonitor`]: crate::GpioBootMonitor
pub struct GpioResetControl<P, D, I> {
    pin: P,
    delay: D,
    line: I,
    active: ActivePolarity,
    asserted: bool,
}

impl<P: OutputPin, D: DelayNs, I: Clone + PartialEq> GpioResetControl<P, D, I> {
    /// Binds `pin`, asserted per `active`, as the reset line named `line`, and
    /// drives the device into reset.
    ///
    /// Binding actuates rather than merely recording, because
    /// [`reset_is_asserted`](ResetControl::reset_is_asserted) answers from the
    /// state this type tracks, and a tracked state inherited from an unknown
    /// pin would be a guess. Driving the line once at bind time makes it a
    /// fact. Reset is the safe end to start from: the orchestrator's own boot
    /// sequence begins by holding a device and then releasing it.
    ///
    /// # Errors
    ///
    /// Returns an error if the line cannot be driven.
    pub fn new(
        pin: P,
        delay: D,
        line: I,
        active: ActivePolarity,
    ) -> Result<Self, ResetLineError<P::Error>> {
        let mut this = Self {
            pin,
            delay,
            line,
            active,
            asserted: false,
        };
        this.drive(true)?;
        Ok(this)
    }

    /// The bound pin, for platform code that must inspect the line directly.
    pub fn pin(&self) -> &P {
        &self.pin
    }

    fn check(&self, reset_id: &I) -> Result<(), ResetLineError<P::Error>> {
        if reset_id == &self.line {
            Ok(())
        } else {
            Err(ResetLineError::UnknownLine)
        }
    }

    fn drive(&mut self, asserted: bool) -> Result<(), ResetLineError<P::Error>> {
        let high = matches!(
            (self.active, asserted),
            (ActivePolarity::ActiveHigh, true) | (ActivePolarity::ActiveLow, false)
        );
        if high {
            self.pin.set_high()
        } else {
            self.pin.set_low()
        }
        .map_err(ResetLineError::Pin)?;
        self.asserted = asserted;
        Ok(())
    }
}

impl<P: OutputPin, D: DelayNs, I: Clone + PartialEq> ErrorType for GpioResetControl<P, D, I> {
    type Error = ResetLineError<P::Error>;
}

impl<P: OutputPin, D: DelayNs, I: Clone + PartialEq> ResetControl for GpioResetControl<P, D, I> {
    type ResetId = I;

    fn reset_assert(&mut self, reset_id: &I) -> Result<(), Self::Error> {
        self.check(reset_id)?;
        self.drive(true)
    }

    fn reset_deassert(&mut self, reset_id: &I) -> Result<(), Self::Error> {
        self.check(reset_id)?;
        self.drive(false)
    }

    fn reset_pulse(&mut self, reset_id: &I, duration: Duration) -> Result<(), Self::Error> {
        self.check(reset_id)?;
        self.drive(true)?;
        // Saturating: a pulse longer than a u32 of microseconds (~71 min) is a
        // misconfiguration, and clamping holds reset longer rather than shorter.
        self.delay
            .delay_us(u32::try_from(duration.as_micros()).unwrap_or(u32::MAX));
        self.drive(false)
    }

    fn reset_is_asserted(&self, reset_id: &I) -> Result<bool, Self::Error> {
        self.check(reset_id)?;
        // Sound because the pin is owned: no one else can drive this line, so
        // the last level we wrote is still the level on it.
        Ok(self.asserted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Everything that is config is normally declared in the board device
    // table (`target/<board>/devices.rs`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Line {
        Bmc,
        Cpld,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct PinFault;

    impl embedded_hal::digital::Error for PinFault {
        fn kind(&self) -> embedded_hal::digital::ErrorKind {
            embedded_hal::digital::ErrorKind::Other
        }
    }

    /// Mock output pin: records every level it is driven to.
    struct MockPin {
        levels: Vec<bool>,
        high: bool,
        fail: bool,
    }

    impl MockPin {
        fn new() -> Self {
            Self {
                levels: Vec::new(),
                high: false,
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                levels: Vec::new(),
                high: false,
                fail: true,
            }
        }

        fn levels(&self) -> &[bool] {
            &self.levels
        }
    }

    impl embedded_hal::digital::ErrorType for MockPin {
        type Error = PinFault;
    }

    impl embedded_hal::digital::OutputPin for MockPin {
        fn set_low(&mut self) -> Result<(), PinFault> {
            if self.fail {
                return Err(PinFault);
            }
            self.high = false;
            self.levels.push(false);
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), PinFault> {
            if self.fail {
                return Err(PinFault);
            }
            self.high = true;
            self.levels.push(true);
            Ok(())
        }
    }

    impl embedded_hal::digital::StatefulOutputPin for MockPin {
        fn is_set_high(&mut self) -> Result<bool, PinFault> {
            if self.fail {
                return Err(PinFault);
            }
            Ok(self.high)
        }

        fn is_set_low(&mut self) -> Result<bool, PinFault> {
            Ok(!self.is_set_high()?)
        }
    }

    /// Mock delay: records every wait it is asked for, sleeps for none of them.
    struct MockDelay {
        waits: Vec<u32>,
    }

    impl MockDelay {
        fn new() -> Self {
            Self { waits: Vec::new() }
        }
    }

    impl DelayNs for MockDelay {
        fn delay_ns(&mut self, ns: u32) {
            self.waits.push(ns);
        }

        fn delay_us(&mut self, us: u32) {
            self.waits.push(us);
        }
    }

    fn active_low() -> GpioResetControl<MockPin, MockDelay, Line> {
        GpioResetControl::new(
            MockPin::new(),
            MockDelay::new(),
            Line::Bmc,
            ActivePolarity::ActiveLow,
        )
        .expect("bind failed")
    }

    fn active_high() -> GpioResetControl<MockPin, MockDelay, Line> {
        GpioResetControl::new(
            MockPin::new(),
            MockDelay::new(),
            Line::Bmc,
            ActivePolarity::ActiveHigh,
        )
        .expect("bind failed")
    }

    /// Levels driven after binding — `new` asserts reset, so every fixture
    /// starts with one recorded level that is not part of what a test exercises.
    fn after_bind(control: &GpioResetControl<MockPin, MockDelay, Line>) -> &[bool] {
        &control.pin().levels()[1..]
    }

    #[test]
    fn binding_drives_the_device_into_reset() {
        let bmc = active_low();

        assert_eq!(bmc.pin().levels(), &[false]);
        assert!(bmc.reset_is_asserted(&Line::Bmc).expect("query failed"));
    }

    // An active-low line like `RoT_BMC_RESET_L` holds the device in reset when
    // driven low, and releases it high.
    #[test]
    fn active_low_asserts_by_driving_the_line_low() {
        let mut bmc = active_low();

        bmc.reset_deassert(&Line::Bmc).expect("deassert failed");
        bmc.reset_assert(&Line::Bmc).expect("assert failed");

        assert_eq!(after_bind(&bmc), &[true, false]);
    }

    #[test]
    fn active_low_deasserts_by_driving_the_line_high() {
        let mut bmc = active_low();

        bmc.reset_deassert(&Line::Bmc).expect("deassert failed");

        assert_eq!(after_bind(&bmc), &[true]);
    }

    #[test]
    fn active_high_inverts_both_levels() {
        let mut bmc = active_high();

        bmc.reset_deassert(&Line::Bmc).expect("deassert failed");
        bmc.reset_assert(&Line::Bmc).expect("assert failed");

        assert_eq!(after_bind(&bmc), &[false, true]);
    }

    #[test]
    fn pulse_asserts_waits_then_deasserts() {
        let mut bmc = active_low();
        bmc.reset_deassert(&Line::Bmc).expect("deassert failed");

        bmc.reset_pulse(&Line::Bmc, Duration::from_millis(10))
            .expect("pulse failed");

        assert_eq!(after_bind(&bmc), &[true, false, true]);
        assert_eq!(bmc.delay.waits, &[10_000]);
    }

    // Clamping must hold reset longer, never skip the wait entirely.
    #[test]
    fn an_absurd_pulse_saturates_instead_of_wrapping() {
        let mut bmc = active_low();

        bmc.reset_pulse(&Line::Bmc, Duration::from_secs(u64::from(u32::MAX)))
            .expect("pulse failed");

        assert_eq!(bmc.delay.waits, &[u32::MAX]);
    }

    #[test]
    fn asserted_state_tracks_every_transition() {
        let mut bmc = active_low();

        assert!(bmc.reset_is_asserted(&Line::Bmc).expect("query failed"));
        bmc.reset_deassert(&Line::Bmc).expect("deassert failed");
        assert!(!bmc.reset_is_asserted(&Line::Bmc).expect("query failed"));
        bmc.reset_assert(&Line::Bmc).expect("assert failed");
        assert!(bmc.reset_is_asserted(&Line::Bmc).expect("query failed"));
    }

    // A pulse leaves the device running, not held.
    #[test]
    fn asserted_state_is_clear_after_a_pulse() {
        let mut bmc = active_low();

        bmc.reset_pulse(&Line::Bmc, Duration::from_millis(1))
            .expect("pulse failed");

        assert!(!bmc.reset_is_asserted(&Line::Bmc).expect("query failed"));
    }

    // A controller drives exactly one line; a misbinding in platform config
    // must be refused rather than silently resetting the wrong device.
    #[test]
    fn a_foreign_reset_id_is_refused_without_touching_the_pin() {
        let mut bmc = active_low();

        let err = bmc
            .reset_assert(&Line::Cpld)
            .expect_err("expected the foreign id to be refused");

        assert_eq!(err.kind(), ErrorKind::InvalidResetId);
        assert!(after_bind(&bmc).is_empty());
    }

    #[test]
    fn a_pin_fault_surfaces_as_a_hardware_failure() {
        let Err(err) = GpioResetControl::new(
            MockPin::failing(),
            MockDelay::new(),
            Line::Bmc,
            ActivePolarity::ActiveLow,
        ) else {
            panic!("expected the pin fault to propagate");
        };

        assert_eq!(err.kind(), ErrorKind::HardwareFailure);
    }

    /// Compile-time fence: the whole point of this adapter is that it satisfies
    /// `HalBootControl`'s bound. Fails to compile if the impl drifts.
    fn _assert_drives_boot_control<C: ResetControl>() {}

    #[test]
    fn the_controller_satisfies_hal_boot_control() {
        _assert_drives_boot_control::<GpioResetControl<MockPin, MockDelay, Line>>();
    }
}
