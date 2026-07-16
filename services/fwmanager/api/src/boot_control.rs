// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Actuation capability: hold a managed device in reset and release it.

use openprot_hal_blocking::system_control::ResetControl;

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
    type Error: core::fmt::Debug;

    /// Holds the device in reset.
    fn hold_in_reset(&mut self) -> Result<(), Self::Error>;

    /// Releases the device from reset.
    fn release(&mut self) -> Result<(), Self::Error>;
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
    type Error = C::Error;

    fn hold_in_reset(&mut self) -> Result<(), Self::Error> {
        self.controller.reset_assert(&self.reset_id)
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        self.controller.reset_deassert(&self.reset_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use openprot_hal_blocking::system_control::{Error, ErrorKind, ErrorType};

    // Normally set in config.rs
    const BMC_LINE: u8 = 7;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Call {
        Assert(u8),
        Deassert(u8),
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct FakeError(ErrorKind);

    impl Error for FakeError {
        fn kind(&self) -> ErrorKind {
            self.0
        }
    }

    /// Fake HAL reset controller: records every call it receives.
    struct FakeResetController {
        calls: Vec<Call>,
        fail: Option<ErrorKind>,
    }

    impl FakeResetController {
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

    impl ErrorType for FakeResetController {
        type Error = FakeError;
    }

    impl ResetControl for FakeResetController {
        type ResetId = u8; // Reset line is GPIO here. Real driver should use Enum

        fn reset_assert(&mut self, reset_id: &u8) -> Result<(), FakeError> {
            if let Some(kind) = self.fail {
                return Err(FakeError(kind));
            }
            self.calls.push(Call::Assert(*reset_id));
            Ok(())
        }

        fn reset_deassert(&mut self, reset_id: &u8) -> Result<(), FakeError> {
            if let Some(kind) = self.fail {
                return Err(FakeError(kind));
            }
            self.calls.push(Call::Deassert(*reset_id));
            Ok(())
        }

        fn reset_pulse(&mut self, _: &u8, _: Duration) -> Result<(), FakeError> {
            panic!(
                "BootControl must never pulse: hold and release are distinct orchestrator steps"
            );
        }

        fn reset_is_asserted(&self, _: &u8) -> Result<bool, FakeError> {
            panic!("BootControl does not query line state");
        }
    }

    // `hold_in_reset()` must assert exactly the configured line (BMC = 7)
    // and nothing else.
    #[test]
    fn holding_a_device_in_reset_asserts_its_configured_line() {
        let mut bmc = HalBootControl::new(FakeResetController::new(), BMC_LINE);

        bmc.hold_in_reset().expect("hold_in_reset failed");

        assert_eq!(bmc.controller().calls(), &[Call::Assert(BMC_LINE)]);
    }

    #[test]
    fn holding_a_device_in_reset_deasserts_its_configured_line() {
        let mut bmc = HalBootControl::new(FakeResetController::new(), BMC_LINE);

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
            FakeResetController::failing(ErrorKind::InvalidResetId),
            BMC_LINE,
        );
        let err = bmc
            .hold_in_reset()
            .expect_err("expected the controller error to propagate");
        assert_eq!(err.kind(), ErrorKind::InvalidResetId);
    }
}
