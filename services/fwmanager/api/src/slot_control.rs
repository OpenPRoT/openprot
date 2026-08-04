// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Home of the [`SlotControl`] capability contract.

/// Actuation capability: select which firmware slot a managed device boots.
///
/// Implemented eRoT-side, for devices whose slot mechanism the eRoT actuates
/// directly (interposed flash, the eRoT's own A/B image). PLDM devices keep
/// slot selection internal and do not implement this.
///
/// # Trial semantics
///
/// - The **committed** slot is what the device boots by default.
/// - [`SlotControl::set_trial`] arms a slot for the *next boot only*: the
///   arming is one-shot, consumed by the boot it triggers, so an interrupted
///   trial falls back to the committed slot.
/// - [`SlotControl::commit`] promotes the trial slot to committed;
///   [`SlotControl::rollback`] disarms it. Both error with no trial armed.
pub trait SlotControl {
    /// Identifies one bootable slot of the device (e.g. A/B index).
    type SlotId: Copy + PartialEq + core::fmt::Debug;

    /// The error type reported by this device's slot control.
    type Error: core::error::Error;

    /// The committed slot — what the device boots absent an armed trial.
    fn active_slot(&self) -> Result<Self::SlotId, Self::Error>;

    /// The armed trial slot, if any.
    fn trial_slot(&self) -> Result<Option<Self::SlotId>, Self::Error>;

    /// Arm `slot` to boot on the next release, without committing it.
    fn set_trial(&mut self, slot: Self::SlotId) -> Result<(), Self::Error>;

    /// Promote the armed trial slot to committed and disarm the trial.
    ///
    /// # Errors
    ///
    /// Errors if no trial is armed.
    fn commit(&mut self) -> Result<(), Self::Error>;

    /// Disarm the trial without promoting; the committed slot stays active.
    ///
    /// # Errors
    ///
    /// Errors if no trial is armed.
    fn rollback(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // A SlotControl implemented against no HAL at all — the contract must be
    // satisfiable from any stack (mock, IPC proxy, simulator).
    struct MockSlots {
        committed: u8,
        trial: Option<u8>,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct NoTrialArmed;

    impl core::fmt::Display for NoTrialArmed {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("no trial slot armed")
        }
    }

    impl core::error::Error for NoTrialArmed {}

    impl SlotControl for MockSlots {
        type SlotId = u8;
        type Error = NoTrialArmed;

        fn active_slot(&self) -> Result<u8, NoTrialArmed> {
            Ok(self.committed)
        }

        fn trial_slot(&self) -> Result<Option<u8>, NoTrialArmed> {
            Ok(self.trial)
        }

        fn set_trial(&mut self, slot: u8) -> Result<(), NoTrialArmed> {
            self.trial = Some(slot);
            Ok(())
        }

        fn commit(&mut self) -> Result<(), NoTrialArmed> {
            self.committed = self.trial.take().ok_or(NoTrialArmed)?;
            Ok(())
        }

        fn rollback(&mut self) -> Result<(), NoTrialArmed> {
            self.trial.take().ok_or(NoTrialArmed)?;
            Ok(())
        }
    }

    #[test]
    fn commit_promotes_the_trial_slot() {
        let mut slots = MockSlots {
            committed: 0,
            trial: None,
        };

        slots.set_trial(1).unwrap();
        assert_eq!(slots.trial_slot().unwrap(), Some(1));
        slots.commit().unwrap();

        assert_eq!(slots.active_slot().unwrap(), 1);
        assert_eq!(slots.trial_slot().unwrap(), None);
    }

    #[test]
    fn rollback_disarms_without_promoting() {
        let mut slots = MockSlots {
            committed: 0,
            trial: None,
        };

        slots.set_trial(1).unwrap();
        slots.rollback().unwrap();

        assert_eq!(slots.active_slot().unwrap(), 0);
        assert_eq!(slots.trial_slot().unwrap(), None);
    }

    #[test]
    fn commit_and_rollback_error_with_no_trial_armed() {
        let mut slots = MockSlots {
            committed: 0,
            trial: None,
        };

        assert_eq!(slots.commit().unwrap_err(), NoTrialArmed);
        assert_eq!(slots.rollback().unwrap_err(), NoTrialArmed);
        assert_eq!(slots.active_slot().unwrap(), 0);
    }
}
