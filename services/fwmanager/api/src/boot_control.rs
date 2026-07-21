// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Actuation capability: hold a managed device in reset and release it.

use openprot_hal_blocking::system_control::{Error as HalError, ErrorKind, ResetControl};

/// Actuation capability: hold a managed device in reset and release it.
///
/// Stateless pass-through by design — sequencing discipline (hold before
/// release, release only after verification) belongs to the orchestrator
/// flows, where it is observable behavior.
///
/// # How the orchestrator uses it
///
/// During verified release the orchestrator parks a device in
/// reset, verifies its active slot while nothing is running, then releases
/// it to boot the image it just checked:
///
/// ```ignore
/// // `dev` is this device's BootControl, obtained from the registry.
/// fn verified_release<D: BootControl>(dev: &mut D) -> Result<(), D::Error> {
///     dev.hold_in_reset()?;        // freeze the device; its flash is now safe to inspect
///     verify_active_slot()?;       // re-hash + signature check (a separate capability)
///     dev.release()?;              // run the just-verified image
///     Ok(())
/// }
/// ```
///
/// In a trial boot the same hold/release pair brackets a
/// watchdog-bounded window; the new slot is committed only if a good boot is
/// observed, otherwise the device falls back to the previous slot:
///
/// ```ignore
/// dev.hold_in_reset()?;
/// store.set_trial(new_slot)?;      // tentative boot selection — not yet committed
/// dev.release()?;                  // boot the trial image
/// match monitor.await_boot(window)? {
///     Booted           => store.commit(new_slot)?,   // observed good => make it active
///     Failed | Timeout => { /* nothing committed; previous slot still active */ }
/// }
/// ```
pub trait BootControl {
    /// The error type reported by this device's boot control.
    ///
    /// Requires [`core::error::Error`] (in `core` since Rust 1.81) so the
    /// orchestrator gets `Display` and a `source()` cause chain, not just the
    /// `Debug` dump, *and* the HAL [`Error`](HalError) trait so generic
    /// consumers can categorize failures via `kind()` without knowing the
    /// concrete error type. HAL-backed implementations satisfy both through
    /// [`BootError`].
    type Error: core::error::Error + HalError;

    /// Holds the device in reset.
    fn hold_in_reset(&mut self) -> Result<(), Self::Error>;

    /// Releases the device from reset.
    fn release(&mut self) -> Result<(), Self::Error>;
}

/// Adapts any HAL system-control error into a [`core::error::Error`].
///
/// Reset controllers keep implementing the HAL `Error`/`kind()` pattern
/// unchanged; this wrapper supplies the `Display` and `core::error::Error`
/// machinery [`BootControl::Error`] requires, so no per-implementation work is
/// needed. The underlying category stays reachable via [`BootError::kind`].
#[derive(Debug)]
pub struct BootError<E>(pub E);

impl<E: HalError> core::fmt::Display for BootError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "boot control error: {:?}", self.0.kind())
    }
}

impl<E: HalError> core::error::Error for BootError<E> {}

impl<E: HalError> HalError for BootError<E> {
    /// The wrapped error's category, so a generic `BootControl` consumer can
    /// branch on it (retry `Busy`, escalate `HardwareFailure`, ...) without
    /// knowing the concrete error type.
    fn kind(&self) -> ErrorKind {
        self.0.kind()
    }
}

/// Binds one reset line of a HAL reset controller to one managed device.
pub struct HalBootControl<C: ResetControl> {
    controller: C,
    reset_id: C::ResetId,
}

impl<C: ResetControl> HalBootControl<C> {
    /// Creates the binding of `controller`'s line `reset_id` to a device.
    pub fn new(controller: C, reset_id: C::ResetId) -> Self {
        Self {
            controller,
            reset_id,
        }
    }

    /// Read access to the underlying controller.
    pub fn controller(&self) -> &C {
        &self.controller
    }
}

impl<C: ResetControl> BootControl for HalBootControl<C> {
    type Error = BootError<C::Error>;

    fn hold_in_reset(&mut self) -> Result<(), Self::Error> {
        self.controller
            .reset_assert(&self.reset_id)
            .map_err(BootError)
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        self.controller
            .reset_deassert(&self.reset_id)
            .map_err(BootError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use openprot_hal_blocking::system_control::{Error as HalError, ErrorKind, ErrorType};

    // Normally set in config.rs
    const BMC_LINE: u8 = 7;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Call {
        Assert(u8),
        Deassert(u8),
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct MockError(ErrorKind);

    impl HalError for MockError {
        fn kind(&self) -> ErrorKind {
            self.0
        }
    }

    /// Mock HAL reset controller: records every call it receives.
    struct MockResetController {
        calls: Vec<Call>,
        fail: Option<ErrorKind>,
    }

    impl MockResetController {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                fail: None,
            }
        }

        fn failing(kind: ErrorKind) -> Self {
            Self {
                calls: Vec::new(),
                fail: Some(kind),
            }
        }

        fn calls(&self) -> &[Call] {
            &self.calls
        }
    }

    impl ErrorType for MockResetController {
        type Error = MockError;
    }

    impl ResetControl for MockResetController {
        type ResetId = u8; // Reset line is GPIO here. Real driver should use Enum

        fn reset_assert(&mut self, reset_id: &u8) -> Result<(), MockError> {
            if let Some(kind) = self.fail {
                return Err(MockError(kind));
            }
            self.calls.push(Call::Assert(*reset_id));
            Ok(())
        }

        fn reset_deassert(&mut self, reset_id: &u8) -> Result<(), MockError> {
            if let Some(kind) = self.fail {
                return Err(MockError(kind));
            }
            self.calls.push(Call::Deassert(*reset_id));
            Ok(())
        }

        fn reset_pulse(&mut self, _: &u8, _: Duration) -> Result<(), MockError> {
            panic!(
                "BootControl must never pulse: hold and release are distinct orchestrator steps"
            );
        }

        fn reset_is_asserted(&self, _: &u8) -> Result<bool, MockError> {
            panic!("BootControl does not query line state");
        }
    }

    // `hold_in_reset()` must assert exactly the configured line (BMC = 7)
    // and nothing else.
    #[test]
    fn holding_a_device_in_reset_asserts_its_configured_line() {
        let mut bmc = HalBootControl::new(MockResetController::new(), BMC_LINE);

        bmc.hold_in_reset().expect("hold_in_reset failed");

        assert_eq!(bmc.controller().calls(), &[Call::Assert(BMC_LINE)]);
    }

    #[test]
    fn releasing_a_device_from_reset_deasserts_its_configured_line() {
        let mut bmc = HalBootControl::new(MockResetController::new(), BMC_LINE);

        bmc.hold_in_reset().expect("hold_in_reset failed");
        bmc.release().expect("release failed");

        assert_eq!(
            bmc.controller().calls(),
            &[Call::Assert(BMC_LINE), Call::Deassert(BMC_LINE)]
        );
    }

    #[test]
    fn controller_error_propagates_through_boot_control() {
        let mut bmc = HalBootControl::new(
            MockResetController::failing(ErrorKind::InvalidResetId),
            BMC_LINE,
        );
        let err = bmc
            .hold_in_reset()
            .expect_err("expected the controller error to propagate");
        assert_eq!(err.kind(), ErrorKind::InvalidResetId);
    }

    // ---- Error contract the orchestrator depends on ------------------------
    //
    // `BootControl::Error` requires the modern `core::error::Error` (in `core`
    // since Rust 1.81) instead of the old `Debug`-only bound. `HalBootControl`
    // satisfies it through the `BootError` adapter, which supplies `Display` +
    // the `core::error::Error` marker over any HAL error while passing `kind()`
    // through for categorization.

    /// Compile-time fence: the error must satisfy `core::error::Error`
    /// (Display + source) *and* the HAL `Error` (with `kind()`), so that
    /// generic `BootControl` consumers get both. Fails to compile if either is
    /// dropped.
    fn _assert_error_contract<E: core::error::Error + HalError>() {}

    #[test]
    fn boot_error_satisfies_the_full_contract() {
        _assert_error_contract::<BootError<MockError>>();
    }

    #[test]
    fn boot_error_renders_a_human_readable_message() {
        let err = BootError(MockError(ErrorKind::HardwareFailure));

        // `Display`, not the `Debug` dump — this is what `core::error::Error`
        // buys over the old `Debug`-only bound.
        assert_eq!(err.to_string(), "boot control error: HardwareFailure");
    }

    #[test]
    fn boot_error_is_a_leaf_with_no_source() {
        let err = BootError(MockError(ErrorKind::Timeout));

        // HAL errors carry no nested `core::error::Error` cause.
        assert!(core::error::Error::source(&err).is_none());
    }

    #[test]
    fn dyn_error_downcasts_back_to_the_concrete_type() {
        let err = BootError(MockError(ErrorKind::PermissionDenied));

        let dyn_err: &dyn core::error::Error = &err;
        let recovered = dyn_err
            .downcast_ref::<BootError<MockError>>()
            .expect("expected to recover the concrete error type");
        assert_eq!(recovered.kind(), ErrorKind::PermissionDenied);
    }

    /// Categorize a failure from *any* `BootControl` — the whole point of the
    /// `kind()` bound is that this compiles for a generic `D`, not just a
    /// concrete error type.
    fn categorize_hold_failure<D: BootControl>(dev: &mut D) -> Option<ErrorKind> {
        dev.hold_in_reset().err().map(|e| e.kind())
    }

    #[test]
    fn error_kind_is_reachable_through_generic_boot_control() {
        let mut bmc = HalBootControl::new(
            MockResetController::failing(ErrorKind::HardwareFailure),
            BMC_LINE,
        );

        // No concrete error type in sight — `kind()` comes from the trait bound.
        assert_eq!(
            categorize_hold_failure(&mut bmc),
            Some(ErrorKind::HardwareFailure)
        );
    }
}
