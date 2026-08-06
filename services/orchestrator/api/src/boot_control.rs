// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Home of the [`BootControl`] capability contract.

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
    /// orchestrator gets `Display` and a `source()` cause chain, not just a
    /// `Debug` dump. Error categories stay implementation-defined — this
    /// crate names no error vocabulary of its own; a consumer that knows the
    /// concrete adapter can recover its details by downcasting the
    /// `&dyn core::error::Error`.
    type Error: core::error::Error;

    /// Holds the device in reset.
    fn hold_in_reset(&mut self) -> Result<(), Self::Error>;

    /// Releases the device from reset.
    fn release(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // A BootControl implemented against no HAL at all — the contract must be
    // satisfiable from any stack (mock, IPC proxy, simulator). Re-adding a
    // HAL-flavored bound on `Error` breaks this compile.
    struct MockDevice {
        fail: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock device fault")
        }
    }

    impl core::error::Error for MockFault {}

    impl BootControl for MockDevice {
        type Error = MockFault;

        fn hold_in_reset(&mut self) -> Result<(), MockFault> {
            if self.fail {
                Err(MockFault)
            } else {
                Ok(())
            }
        }

        fn release(&mut self) -> Result<(), MockFault> {
            if self.fail {
                Err(MockFault)
            } else {
                Ok(())
            }
        }
    }

    /// The doc example's orchestrator shape, generic over any `BootControl`.
    fn hold_then_release<D: BootControl>(dev: &mut D) -> Result<(), D::Error> {
        dev.hold_in_reset()?;
        dev.release()
    }

    #[test]
    fn contract_is_implementable_without_the_hal() {
        let mut dev = MockDevice { fail: false };

        hold_then_release(&mut dev).expect("hold/release failed");
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut dev = MockDevice { fail: true };

        let err = hold_then_release(&mut dev).expect_err("expected the device fault");

        // Display comes from the core::error::Error bound, not a Debug dump.
        assert_eq!(err.to_string(), "mock device fault");
    }
}
