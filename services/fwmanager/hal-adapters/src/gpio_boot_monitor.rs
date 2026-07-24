// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HAL-backed [`BootMonitor`]: read a device's boot-complete signal off a GPIO
//! input line.

use fwmanager_api::{BootMonitor, BootStatus};
use openprot_hal_blocking::gpio_port::{
    ActivePolarity, GpioError, GpioErrorKind, GpioPort, PinMask,
};

/// Adapts any HAL GPIO error into a [`core::error::Error`].
///
/// GPIO ports keep implementing the HAL `GpioError`/`kind()` pattern
/// unchanged; this wrapper supplies the `Display` and `core::error::Error`
/// machinery [`BootMonitor::Error`] requires, so no per-implementation work is
/// needed. The underlying category stays reachable via [`MonitorError::kind`],
/// and the concrete HAL error through the
/// [`source()`](core::error::Error::source) chain, downcast to
/// [`GpioCause<E>`](GpioCause).
#[derive(Debug)]
pub struct MonitorError<E>(GpioCause<E>);

/// The concrete HAL error underneath a [`MonitorError`], surfaced as its
/// [`source()`](core::error::Error::source).
///
/// `GpioError` only guarantees `Debug`, so the HAL error cannot be the source
/// itself; this wrapper adds the `Display`/`Error` machinery. It renders the
/// full concrete error, not just its kind — the tail of a rendered error
/// chain carries the implementation's details.
#[derive(Debug)]
pub struct GpioCause<E>(pub E);

impl<E: GpioError> core::fmt::Display for MonitorError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "boot monitor error: {:?}", self.kind())
    }
}

impl<E: GpioError + 'static> core::error::Error for MonitorError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl<E: GpioError> core::fmt::Display for GpioCause<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<E: GpioError + 'static> core::error::Error for GpioCause<E> {}

impl<E: GpioError> GpioError for MonitorError<E> {
    fn kind(&self) -> GpioErrorKind {
        let GpioCause(inner) = &self.0;
        inner.kind()
    }
}

impl<E: GpioError> From<E> for MonitorError<E> {
    fn from(err: E) -> Self {
        Self(GpioCause(err))
    }
}

/// Binds one input line of a HAL GPIO port to a managed device's boot-complete
/// signal.
///
/// The input-side counterpart to [`HalBootControl`]: the `(port, ready_pin,
/// active)` binding is made once, in platform configuration, and the
/// orchestrator never learns which line belongs to which device. A signal that
/// latches high is read directly; an edge or pulse (for example a heartbeat) is
/// latched in hardware at bring-up so `read_input` still reflects it.
///
/// Where a hardware latch is used, the platform must clear it whenever the
/// device re-enters reset (typically by wiring the latch's clear to the
/// device's reset line) — [`BootMonitor`] requires that evidence from a
/// previous boot never reads as [`BootStatus::Booted`], and this adapter only
/// reads the line, it cannot re-arm it.
///
/// [`HalBootControl`]: crate::HalBootControl
pub struct GpioBootMonitor<P: GpioPort> {
    port: P,
    ready_pin: P::Mask,
    active: ActivePolarity,
}

impl<P: GpioPort> GpioBootMonitor<P> {
    /// Binds `port`'s line `ready_pin`, asserted per `active`, as a device's
    /// boot-complete signal.
    ///
    /// `ready_pin` must name exactly one line — boot-complete is a single
    /// signal. Masks naming several lines are not supported (the `PinMask`
    /// contract offers no way to reject them here, so they are unspecified
    /// behavior, not a feature).
    ///
    /// # Panics
    ///
    /// Panics if `ready_pin` is empty. An empty mask is vacuously contained
    /// in every sample, so it would report every device `Booted` from the
    /// moment reset is released — a misbinding that must fail at
    /// construction, in platform bring-up, not stay silent in the field.
    pub fn new(port: P, ready_pin: P::Mask, active: ActivePolarity) -> Self {
        assert!(
            !ready_pin.is_empty(),
            "GpioBootMonitor bound to an empty pin mask"
        );
        Self {
            port,
            ready_pin,
            active,
        }
    }
}

// `P::Error: 'static` because `source()` hands out `&(dyn Error + 'static)`
// referencing the wrapped HAL error. Error types are plain data; this costs
// no real implementation anything.
impl<P: GpioPort> BootMonitor for GpioBootMonitor<P>
where
    P::Error: 'static,
{
    type Error = MonitorError<P::Error>;

    /// # Errors
    ///
    /// Propagates any error returned by the port's `read_input`.
    fn boot_status(&self) -> Result<BootStatus, Self::Error> {
        let high = self.port.read_input()?.contains(self.ready_pin);
        let booted = match self.active {
            ActivePolarity::ActiveHigh => high,
            ActivePolarity::ActiveLow => !high,
        };
        Ok(if booted {
            BootStatus::Booted
        } else {
            BootStatus::Booting
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_hal_blocking::gpio_port::GpioErrorType;

    // BMC boot-complete on line 4. Normally set in config.rs.
    const BMC_READY: Mask = Mask(1 << 4);

    /// Bitmask over a single mock GPIO bank.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Mask(u8);

    impl PinMask for Mask {
        fn empty() -> Self {
            Self(0)
        }

        fn all() -> Self {
            Self(u8::MAX)
        }

        fn is_empty(&self) -> bool {
            self.0 == 0
        }

        fn contains(&self, other: Self) -> bool {
            self.0 & other.0 == other.0
        }

        fn union(&self, other: Self) -> Self {
            Self(self.0 | other.0)
        }

        fn intersection(&self, other: Self) -> Self {
            Self(self.0 & other.0)
        }

        fn toggle(&self) -> Self {
            Self(!self.0)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MockError(GpioErrorKind);

    impl GpioError for MockError {
        fn kind(&self) -> GpioErrorKind {
            self.0
        }
    }

    /// Mock HAL GPIO port: returns a fixed input sample, with opt-in read
    /// failure injection. Output-side methods must never be called.
    struct MockGpioPort {
        input: Mask,
        fail: Option<GpioErrorKind>,
    }

    impl MockGpioPort {
        fn with_input(input: Mask) -> Self {
            Self { input, fail: None }
        }

        fn failing(kind: GpioErrorKind) -> Self {
            Self {
                input: Mask::empty(),
                fail: Some(kind),
            }
        }
    }

    impl GpioErrorType for MockGpioPort {
        type Error = MockError;
    }

    impl GpioPort for MockGpioPort {
        type Config = ();
        type Mask = Mask;

        fn read_input(&self) -> Result<Mask, MockError> {
            match self.fail {
                Some(kind) => Err(MockError(kind)),
                None => Ok(self.input),
            }
        }

        fn configure(&mut self, _: Mask, _: ()) -> Result<(), MockError> {
            panic!("BootMonitor must never configure pins");
        }

        fn set_reset(&mut self, _: Mask, _: Mask) -> Result<(), MockError> {
            panic!("BootMonitor must never drive outputs");
        }

        fn toggle(&mut self, _: Mask) -> Result<(), MockError> {
            panic!("BootMonitor must never drive outputs");
        }
    }

    // A ready line asserted high reads as Booted under active-high polarity.
    #[test]
    fn booted_when_ready_line_asserted() {
        let mon = GpioBootMonitor::new(
            MockGpioPort::with_input(BMC_READY),
            BMC_READY,
            ActivePolarity::ActiveHigh,
        );

        assert_eq!(mon.boot_status().expect("read failed"), BootStatus::Booted);
    }

    // A deasserted ready line is not a failure — the device is still Booting
    // until the orchestrator's own timeout fires.
    #[test]
    fn booting_when_ready_line_deasserted() {
        let mon = GpioBootMonitor::new(
            MockGpioPort::with_input(Mask::empty()),
            BMC_READY,
            ActivePolarity::ActiveHigh,
        );

        assert_eq!(mon.boot_status().expect("read failed"), BootStatus::Booting);
    }

    // Active-low inverts the sense: a low line is asserted, hence Booted.
    #[test]
    fn respects_active_low_polarity() {
        let mon = GpioBootMonitor::new(
            MockGpioPort::with_input(Mask::empty()),
            BMC_READY,
            ActivePolarity::ActiveLow,
        );

        assert_eq!(mon.boot_status().expect("read failed"), BootStatus::Booted);
    }

    // Only the configured line matters: an unrelated high line leaves the
    // device Booting.
    #[test]
    fn ignores_other_lines() {
        let mon = GpioBootMonitor::new(
            MockGpioPort::with_input(Mask(1 << 2)),
            BMC_READY,
            ActivePolarity::ActiveHigh,
        );

        assert_eq!(mon.boot_status().expect("read failed"), BootStatus::Booting);
    }

    // An empty mask would read as Booted forever (it is vacuously contained
    // in every sample); binding one must die at construction.
    #[test]
    #[should_panic(expected = "empty pin mask")]
    fn empty_ready_mask_panics_at_construction() {
        GpioBootMonitor::new(
            MockGpioPort::with_input(Mask::empty()),
            Mask::empty(),
            ActivePolarity::ActiveHigh,
        );
    }

    // A controller error surfaces through BootMonitor unchanged.
    #[test]
    fn port_error_propagates_through_boot_monitor() {
        let mon = GpioBootMonitor::new(
            MockGpioPort::failing(GpioErrorKind::HardwareFailure),
            BMC_READY,
            ActivePolarity::ActiveHigh,
        );

        let err = mon
            .boot_status()
            .expect_err("expected the port error to propagate");
        assert_eq!(err.kind(), GpioErrorKind::HardwareFailure);
    }

    /// Compile-time fence: the adapter error must satisfy `core::error::Error`
    /// (Display + source) *and* the HAL `GpioError` (with `kind()`). Fails to
    /// compile if either is dropped.
    fn _assert_error_contract<E: core::error::Error + GpioError>() {}

    #[test]
    fn monitor_error_satisfies_the_full_contract() {
        _assert_error_contract::<MonitorError<MockError>>();
    }

    #[test]
    fn monitor_error_renders_a_human_readable_message() {
        let err = MonitorError::from(MockError(GpioErrorKind::HardwareFailure));

        assert_eq!(err.to_string(), "boot monitor error: HardwareFailure");
    }

    // The cause chain: source() carries the concrete HAL error with its full
    // details (not just the kind), and downcasting recovers the typed error.
    #[test]
    fn source_exposes_the_concrete_hal_error() {
        let err = MonitorError::from(MockError(GpioErrorKind::HardwareFailure));

        let src = core::error::Error::source(&err).expect("source must carry the HAL error");
        assert_eq!(src.to_string(), "MockError(HardwareFailure)");

        let cause = src
            .downcast_ref::<GpioCause<MockError>>()
            .expect("source must downcast to the concrete cause");
        assert_eq!(cause.0, MockError(GpioErrorKind::HardwareFailure));
    }
}
