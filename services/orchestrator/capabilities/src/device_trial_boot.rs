// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`DeviceTrialBoot`] commit capability contract.
//!
//! This is the gate for the downstream devices, the ones the eRoT watches
//! from outside. [`BootWatch`] checks a device's checkpoints and answers with
//! a [`WalkVerdict`], read through an [`EvidenceReader`] over the board's
//! ready lines. `Complete` means confirm, `Failed` means revert, whether the
//! device reported the failure or a checkpoint's window ran out with no
//! report at all. The eRoT stays alive across the whole flow, so nothing has
//! to persist beyond the device's own slot metadata: an orchestrator that
//! rebooted mid-update re-walks the chain from the start.
//!
//! The eRoT's own image is the same gate under a different lifetime, and it
//! is [`SelfUpdate`] instead. There the judging outlives the judge: the eRoT
//! resets into the candidate, so the verdict is reached by a later boot that
//! has to read what the previous one left behind. That needs durable state
//! this trait deliberately does not carry, and the verdict is the
//! orchestrator's own supervised boot coming up, not a call from the image
//! that armed the trial.
//!
//! A board that wants one gate for both can implement this trait over its
//! [`SelfUpdate`] session; nothing here forbids it. What does not work is one
//! trait for both via a supertrait bound: `confirm` with no trial open
//! succeeds here and is an error on a session, and `is_pending` cannot say
//! "confirmed, but the anti-rollback floor has not moved yet".
//!
//! [`BootWatch`]: crate::BootWatch
//! [`WalkVerdict`]: crate::WalkVerdict
//! [`EvidenceReader`]: crate::EvidenceReader
//! [`SelfUpdate`]: crate::SelfUpdate

/// Commit capability: confirm or revert an image that was activated but not
/// committed, once its boot has been judged.
///
/// Activating an update picks the staged image as the next thing to boot and
/// stops there, whether the eRoT does that itself through
/// [`Updatable::activate`](crate::Updatable::activate) or a PLDM firmware
/// device does it for the eRoT. This trait is the other half: keep that image
/// if its boot was good, or throw it away.
///
/// Implemented for the downstream devices whose slot choice the eRoT drives,
/// behind interposed flash or over PLDM. A device that commits on its own,
/// a PLDM firmware device picking its own slot, has no
/// `DeviceTrialBoot` on the eRoT side, the same split as
/// [`SvnFloor`](crate::SvnFloor). The eRoT's own
/// image has [`SelfUpdate`](crate::SelfUpdate), which holds the same gate
/// plus the state a verdict reached after a reset needs.
///
/// `confirm` and `revert` take no arguments. A device has at most one trial
/// open at a time, the one the last activation armed, so there is nothing to
/// name. Which slot is which stays behind the seam, as it does in
/// `Updatable`.
///
/// The arming counts for the next boot only, but the record of the trial
/// outlives it. An image that hangs, or an eRoT that loses power during the
/// trial, boots the confirmed slot again without anyone calling anything.
/// What makes that happen (a boot-select register the boot ROM clears, or
/// something like it) is the implementation's business. The record stays
/// [`is_pending`](Self::is_pending) until `confirm` or `revert`, so an
/// orchestrator that rebooted in the middle of an update finds the trial
/// again instead of losing it.
///
/// Once `confirm` returns `Ok`, the image stays confirmed across a power
/// loss. A call cut short by power loss never returns, so the next boot still
/// finds the trial open and the caller confirms again. A half-written record
/// must never leave the device booting an image nobody confirmed.
///
/// Both calls are safe to repeat: with no trial open they succeed and do
/// nothing. A caller that needs to tell a repeat from a trial that was never
/// there checks `is_pending` first, the same shape as
/// [`SvnFloor::advance`](crate::SvnFloor::advance). Neither call boots
/// anything; they only move slot metadata. Restarting the device is
/// [`BootControl`](crate::BootControl).
///
/// `is_pending` is a yes or no. It does not say whether the armed image has
/// booted, and this trait cannot tell a `confirm` that came after a watched
/// boot from one that did not. Nothing needs that difference: a half-done
/// update is rerun from the start, not picked up where it left off.
/// Confirming only what was actually watched is up to the caller, the same
/// way [`BootControl`](crate::BootControl)'s caller keeps a device in reset
/// until it has been checked. Which slot the running image booted from is not
/// visible here.
pub trait DeviceTrialBoot {
    /// The error type this device's trial record reports.
    ///
    /// Bounded by [`core::error::Error`] so the caller gets `Display` and a
    /// `source()` cause chain, not just a `Debug` dump. Which errors exist is
    /// up to the implementation.
    type Error: core::error::Error;

    /// Whether an activated image is still waiting for its verdict.
    ///
    /// Answered from stored state, because an orchestrator that rebooted
    /// mid-update has no memory of a device's open trial.
    fn is_pending(&self) -> Result<bool, Self::Error>;

    /// Keeps the activated image as the confirmed one and closes the trial.
    ///
    /// Only moves slot metadata. Raising the anti-rollback floor is
    /// [`SvnFloor::advance`](crate::SvnFloor::advance) and happens later, so
    /// [`revert`](Self::revert) always has an image it can go back to.
    fn confirm(&mut self) -> Result<(), Self::Error>;

    /// Closes the trial without keeping the image; the confirmed slot stays
    /// as it was.
    ///
    /// Also disarms: once this returns, the next boot runs the confirmed slot
    /// even if the trial image never booted at all. An orchestrator that
    /// drops an activation before resetting the device relies on that.
    fn revert(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct MockFault;

    impl core::fmt::Display for MockFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("mock trial record fault")
        }
    }

    impl core::error::Error for MockFault {}

    /// The verdict the caller reached for the boot it judged.
    #[derive(Copy, Clone)]
    enum Verdict {
        Healthy,
        Bad,
    }

    /// The one flow both implementations go through: apply the verdict to
    /// whatever the last activation armed, then check the record came out
    /// clear. A record left open means a later boot finds a trial nobody
    /// owns. Generic over `DeviceTrialBoot`, so every device runs the same code
    /// whether the orchestrator held the instance all along or built it
    /// afresh.
    fn apply_verdict<T: DeviceTrialBoot>(trial: &mut T, verdict: Verdict) -> Result<(), T::Error> {
        match verdict {
            Verdict::Healthy => trial.confirm()?,
            Verdict::Bad => trial.revert()?,
        }
        assert!(
            !trial.is_pending()?,
            "confirm or revert must close the trial"
        );
        Ok(())
    }

    /// A slot-selection record whose arming counts for the next boot only,
    /// the way a device's eRoT-held store behaves.
    struct SlotRecord {
        confirmed_slot: u8,
        trial_slot: Option<u8>,
        next_boot_armed: bool,
    }

    impl SlotRecord {
        fn new(confirmed_slot: u8) -> Self {
            Self {
                confirmed_slot,
                trial_slot: None,
                next_boot_armed: false,
            }
        }

        /// What `Updatable::activate` maps onto for this device.
        fn activate(&mut self, slot: u8) {
            self.trial_slot = Some(slot);
            self.next_boot_armed = true;
        }

        /// Boots the device and returns the slot it ran: the trial slot if
        /// the next boot is still armed, the confirmed slot otherwise.
        fn boot(&mut self) -> u8 {
            match self.trial_slot {
                Some(slot) if self.next_boot_armed => {
                    self.next_boot_armed = false;
                    slot
                }
                _ => self.confirmed_slot,
            }
        }
    }

    /// One `DeviceTrialBoot` over a record the caller owns. The two cases
    /// differ only in how long the instance lives: the orchestrator holds it
    /// for the
    /// whole flow and judges the boot over the device's boot-complete line,
    /// or, having rebooted mid-update, builds a fresh one over the record the
    /// device kept.
    ///
    /// Tied to no HAL: the contract has to work from any stack (mock, IPC
    /// proxy, simulator), and a HAL-bound `Error` type would stop this
    /// compiling.
    struct TrialRecord<'a> {
        record: &'a mut SlotRecord,
        fail: bool,
    }

    impl<'a> From<&'a mut SlotRecord> for TrialRecord<'a> {
        fn from(record: &'a mut SlotRecord) -> Self {
            Self {
                record,
                fail: false,
            }
        }
    }

    impl<'a> TrialRecord<'a> {
        /// Every call fails, for the error test. Not a second `From`, since
        /// only one conversion from `&mut SlotRecord` can exist.
        fn faulty(record: &'a mut SlotRecord) -> Self {
            Self { record, fail: true }
        }
    }

    impl DeviceTrialBoot for TrialRecord<'_> {
        type Error = MockFault;

        fn is_pending(&self) -> Result<bool, MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            Ok(self.record.trial_slot.is_some())
        }

        fn confirm(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            if let Some(slot) = self.record.trial_slot.take() {
                self.record.confirmed_slot = slot;
            }
            Ok(())
        }

        fn revert(&mut self) -> Result<(), MockFault> {
            if self.fail {
                return Err(MockFault);
            }
            self.record.trial_slot = None;
            Ok(())
        }
    }

    #[test]
    fn a_healthy_trial_becomes_the_confirmed_slot_of_a_passive_device() {
        let mut record = SlotRecord::new(0);
        let mut device = TrialRecord::from(&mut record);
        device.record.activate(1);

        assert_eq!(device.record.boot(), 1, "the trial slot runs first");
        apply_verdict(&mut device, Verdict::Healthy).unwrap();
        assert_eq!(device.record.boot(), 1, "and stays the default");
    }

    #[test]
    fn a_device_trial_nobody_confirms_falls_back_but_stays_open() {
        let mut record = SlotRecord::new(0);
        let mut device = TrialRecord::from(&mut record);
        device.record.activate(1);

        assert_eq!(device.record.boot(), 1);
        assert_eq!(
            device.record.boot(),
            0,
            "the arming is one-shot: no confirm, no second trial boot"
        );
        assert_eq!(
            device.is_pending(),
            Ok(true),
            "the record outlives the fallback, so a rebooted caller finds it"
        );
        apply_verdict(&mut device, Verdict::Bad).unwrap();
        assert_eq!(device.record.boot(), 0);
    }

    #[test]
    fn a_caller_that_lost_its_instance_confirms_from_stored_state() {
        let mut record = SlotRecord::new(0);
        record.activate(1);

        // The boot that arming triggered. The armed image is now running,
        // and the orchestrator that armed it has since restarted.
        assert_eq!(record.boot(), 1);

        let mut trial = TrialRecord::from(&mut record);
        assert_eq!(
            trial.is_pending(),
            Ok(true),
            "the fresh instance learns of the trial from stored state alone"
        );
        apply_verdict(&mut trial, Verdict::Healthy).unwrap();

        assert_eq!(record.confirmed_slot, 1);
        assert_eq!(record.boot(), 1);
    }

    #[test]
    fn a_trial_that_never_confirms_falls_back_on_the_next_reset() {
        let mut record = SlotRecord::new(0);
        record.activate(1);

        assert_eq!(record.boot(), 1, "the trial image runs and hangs");
        assert_eq!(
            record.boot(),
            0,
            "the next reset runs the confirmed image, with no call from us"
        );

        let mut trial = TrialRecord::from(&mut record);
        assert_eq!(trial.is_pending(), Ok(true));
        apply_verdict(&mut trial, Verdict::Bad).unwrap();
    }

    #[test]
    fn revert_before_the_trial_boots_disarms_the_next_boot() {
        let mut record = SlotRecord::new(0);
        let mut device = TrialRecord::from(&mut record);
        device.record.activate(1);

        // No boot yet: the orchestrator dropped the activation instead of
        // resetting the device.
        device.revert().unwrap();

        assert_eq!(
            device.record.boot(),
            0,
            "revert disarms, so the dropped image never gets a boot"
        );
    }

    #[test]
    fn a_replayed_resolution_is_a_noop() {
        let mut record = SlotRecord::new(0);
        record.activate(1);
        record.boot();

        let mut trial = TrialRecord::from(&mut record);
        apply_verdict(&mut trial, Verdict::Healthy).unwrap();
        apply_verdict(&mut trial, Verdict::Healthy).expect("confirm with nothing pending succeeds");
        apply_verdict(&mut trial, Verdict::Bad).expect("and so does revert");

        assert_eq!(
            record.confirmed_slot, 1,
            "neither replay moved the slot back"
        );
    }

    #[test]
    fn errors_surface_through_the_generic_seam() {
        let mut record = SlotRecord::new(0);
        let mut device = TrialRecord::faulty(&mut record);

        assert_eq!(apply_verdict(&mut device, Verdict::Healthy), Err(MockFault));
        assert_eq!(apply_verdict(&mut device, Verdict::Bad), Err(MockFault));
    }
}
