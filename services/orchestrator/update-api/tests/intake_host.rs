// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`UpdateIntake`] contract, driven the way an update source drives
//! it: offer, write at whatever offsets the protocol delivers, complete,
//! poll until the phase is terminal.
//!
//! [`MockIntake`] is the orchestrator side, in process. It models a
//! write-through board, the one the crate docs describe: a write goes
//! straight into the target's inactive slot and the eRoT authenticates by
//! reading that slot back, so the phases run `Receiving`,
//! `Authenticating`, `Activated` with no `Staging` in between.
//!
//! The verdict is not part of any answer. `complete` hands the candidate
//! over and the phase arrives later through `poll`, so the mock takes it
//! from [`MockIntake::settle`], which is this test's stand-in for the
//! state machine deciding.

use core::cell::RefCell;

use openprot_orchestrator_update_api::wire::MAX_CHUNK;
use openprot_orchestrator_update_api::{
    FailureCause, IntakeError, IntakeStatus, Progress, Reject, TargetId, UpdateIntake,
};

/// The only device this board declares.
const TARGET: TargetId = TargetId(1);

/// A target the device table does not carry.
const ABSENT: TargetId = TargetId(7);

/// The staging region, which bounds what an offer may declare.
const STAGING: usize = 1024;

/// The candidate every test offers unless it is testing the offer itself.
const CANDIDATE: u64 = 256;

/// The orchestrator's side of the seam, as far as a source can tell.
struct MockIntake {
    state: RefCell<State>,
}

struct State {
    /// The phase `poll` answers with.
    phase: IntakeStatus,
    /// Bytes the accepted offer declared, `None` with no job.
    total: Option<u64>,
    /// Which offered bytes have been written. Exact rather than counted,
    /// so a rewritten range is not credited twice.
    covered: [bool; STAGING],
}

impl MockIntake {
    fn new() -> Self {
        Self {
            state: RefCell::new(State {
                phase: IntakeStatus::Idle,
                total: None,
                covered: [false; STAGING],
            }),
        }
    }

    /// The state machine's verdict on the candidate, which `complete` left
    /// authenticating. Stands in for the orchestrator deciding on its own
    /// clock: nothing the source called returns this.
    fn settle(&self, outcome: IntakeStatus) {
        let mut state = self.state.borrow_mut();
        assert_eq!(
            state.phase,
            IntakeStatus::Authenticating,
            "settling a job that is not authenticating"
        );
        state.phase = outcome;
    }

    /// Bytes of the offered payload that have been written.
    fn written(&self) -> u64 {
        let state = self.state.borrow();
        let total = state.total.unwrap_or(0) as usize;
        state.covered[..total].iter().filter(|seen| **seen).count() as u64
    }

    /// Whether a job is in flight, which is what a second offer collides
    /// with. A terminal phase is not in flight: it is waiting to be
    /// collected.
    fn in_flight(phase: IntakeStatus) -> bool {
        matches!(
            phase,
            IntakeStatus::Receiving(_) | IntakeStatus::Authenticating | IntakeStatus::Staging(_)
        )
    }
}

impl UpdateIntake for MockIntake {
    fn offer(&self, target: TargetId, total: u64) -> Result<(), IntakeError> {
        let mut state = self.state.borrow_mut();
        if target != TARGET {
            return Err(IntakeError::Rejected(Reject::UnknownTarget));
        }
        if total == 0 {
            return Err(IntakeError::Rejected(Reject::Incomplete));
        }
        if total > STAGING as u64 {
            return Err(IntakeError::Rejected(Reject::OutOfRange));
        }
        if Self::in_flight(state.phase) {
            // The job in flight is untouched, which is the whole point of
            // refusing: a second source must not write into a region that
            // is mid-authenticate.
            return Err(IntakeError::Rejected(Reject::Busy));
        }
        // A fresh accepted offer collects whatever the last job ended as.
        state.phase = IntakeStatus::Receiving(Progress { written: 0, total });
        state.total = Some(total);
        state.covered = [false; STAGING];
        Ok(())
    }

    fn write(&self, offset: u64, bytes: &[u8]) -> Result<(), IntakeError> {
        let mut state = self.state.borrow_mut();
        let IntakeStatus::Receiving(_) = state.phase else {
            // No job, or the job has left the phase that takes bytes.
            return Err(IntakeError::Rejected(Reject::NoJob));
        };
        // The chunk cap is the wire decoder's to enforce, as a
        // `WireError`; a caller in this process never encodes anything.
        // Asserted rather than refused, so a test that oversteps says so.
        assert!(bytes.len() <= MAX_CHUNK, "a write over MAX_CHUNK");
        let total = state.total.unwrap_or(0);
        let end = match offset.checked_add(bytes.len() as u64) {
            Some(end) if end <= total => end,
            _ => return Err(IntakeError::Rejected(Reject::OutOfRange)),
        };
        for byte in offset..end {
            state.covered[byte as usize] = true;
        }
        let written = state.covered[..total as usize]
            .iter()
            .filter(|seen| **seen)
            .count() as u64;
        state.phase = IntakeStatus::Receiving(Progress { written, total });
        Ok(())
    }

    fn complete(&self) -> Result<(), IntakeError> {
        let mut state = self.state.borrow_mut();
        let IntakeStatus::Receiving(progress) = state.phase else {
            return Err(IntakeError::Rejected(Reject::NoJob));
        };
        if progress.written != progress.total {
            // Nothing is judged: a candidate with a hole in it is not a
            // candidate.
            return Err(IntakeError::Rejected(Reject::Incomplete));
        }
        state.phase = IntakeStatus::Authenticating;
        Ok(())
    }

    fn abort(&self) -> Result<(), IntakeError> {
        let mut state = self.state.borrow_mut();
        state.phase = IntakeStatus::Idle;
        state.total = None;
        Ok(())
    }

    fn poll(&self) -> Result<IntakeStatus, IntakeError> {
        Ok(self.state.borrow().phase)
    }
}

/// Offers and writes a whole candidate, the way a source with a 64-byte
/// protocol window does.
fn hand_over(intake: &MockIntake, total: u64) {
    intake.offer(TARGET, total).expect("the offer was refused");
    let chunk = [0xa5u8; 64];
    let mut offset = 0;
    while offset < total {
        let take = (total - offset).min(chunk.len() as u64) as usize;
        intake
            .write(offset, &chunk[..take])
            .expect("a write was refused");
        offset += take as u64;
    }
}

#[test]
fn a_candidate_offered_written_and_completed_reaches_activated() {
    let intake = MockIntake::new();
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);

    hand_over(&intake, CANDIDATE);
    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Receiving(Progress {
            written: CANDIDATE,
            total: CANDIDATE
        })
    );

    intake.complete().expect("complete was refused");
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Authenticating);

    intake.settle(IntakeStatus::Activated);
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Activated);
}

#[test]
fn a_second_offer_while_a_job_runs_is_busy_and_leaves_it_untouched() {
    let intake = MockIntake::new();
    hand_over(&intake, CANDIDATE);

    assert_eq!(
        intake.offer(TARGET, 64),
        Err(IntakeError::Rejected(Reject::Busy))
    );
    assert_eq!(intake.written(), CANDIDATE);
    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Receiving(Progress {
            written: CANDIDATE,
            total: CANDIDATE
        })
    );
}

#[test]
fn an_offer_of_zero_bytes_is_incomplete() {
    let intake = MockIntake::new();
    assert_eq!(
        intake.offer(TARGET, 0),
        Err(IntakeError::Rejected(Reject::Incomplete))
    );
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);
}

#[test]
fn an_offer_larger_than_the_staging_region_is_out_of_range() {
    let intake = MockIntake::new();
    assert_eq!(
        intake.offer(TARGET, STAGING as u64 + 1),
        Err(IntakeError::Rejected(Reject::OutOfRange))
    );
}

#[test]
fn an_offer_for_a_target_the_board_does_not_declare_is_unknown() {
    let intake = MockIntake::new();
    assert_eq!(
        intake.offer(ABSENT, CANDIDATE),
        Err(IntakeError::Rejected(Reject::UnknownTarget))
    );
}

#[test]
fn a_write_outside_the_offered_payload_is_out_of_range() {
    let intake = MockIntake::new();
    intake.offer(TARGET, CANDIDATE).unwrap();
    assert_eq!(
        intake.write(CANDIDATE - 4, &[0; 8]),
        Err(IntakeError::Rejected(Reject::OutOfRange))
    );
}

#[test]
fn a_repeated_range_overwrites_so_a_retransmit_needs_no_bookkeeping() {
    let intake = MockIntake::new();
    hand_over(&intake, CANDIDATE);

    intake
        .write(0, &[0x5a; 64])
        .expect("a retransmitted window was refused");
    // Credited once, so the retransmit does not push the count past the
    // offer and the candidate still completes.
    assert_eq!(intake.written(), CANDIDATE);
    intake.complete().expect("complete was refused");
}

#[test]
fn a_write_without_an_accepted_offer_is_no_job() {
    let intake = MockIntake::new();
    assert_eq!(
        intake.write(0, &[0; 8]),
        Err(IntakeError::Rejected(Reject::NoJob))
    );

    // The other half of what `NoJob` covers: a job that has left the phase
    // that takes bytes. Every byte is in, so this one is authenticating.
    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    assert_eq!(
        intake.write(0, &[0; 8]),
        Err(IntakeError::Rejected(Reject::NoJob))
    );
}

#[test]
fn completing_before_every_offered_byte_is_written_is_incomplete() {
    let intake = MockIntake::new();
    intake.offer(TARGET, CANDIDATE).unwrap();
    intake.write(0, &[0; 64]).unwrap();

    assert_eq!(
        intake.complete(),
        Err(IntakeError::Rejected(Reject::Incomplete))
    );
    // Still taking bytes: a refused complete is not a failed job.
    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Receiving(Progress {
            written: 64,
            total: CANDIDATE
        })
    );
}

#[test]
fn aborting_returns_to_idle_from_every_phase() {
    // With no job at all, which is how a source that lost track gets back.
    let intake = MockIntake::new();
    intake.abort().expect("abort was refused with no job");
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);

    // Mid-transfer.
    intake.offer(TARGET, CANDIDATE).unwrap();
    intake.write(0, &[0; 64]).unwrap();
    intake.abort().unwrap();
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);

    // Authenticating, which is after the source has done everything it can.
    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    intake.abort().unwrap();
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);

    // Terminal, where the phase is waiting to be collected. Both
    // terminals: a job that failed and one that activated.
    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    intake.settle(IntakeStatus::Failed(FailureCause::Device));
    intake.abort().unwrap();
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);

    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    intake.settle(IntakeStatus::Activated);
    intake.abort().unwrap();
    assert_eq!(intake.poll().unwrap(), IntakeStatus::Idle);
}

#[test]
fn a_state_machine_that_refuses_the_request_surfaces_as_deferred() {
    let intake = MockIntake::new();
    hand_over(&intake, CANDIDATE);

    // The refusal is not an answer to `complete`: that call succeeded, and
    // the cause arrives on a later poll.
    intake.complete().expect("complete was refused");
    intake.settle(IntakeStatus::Failed(FailureCause::Deferred));

    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Failed(FailureCause::Deferred)
    );
}

#[test]
fn a_fresh_offer_collects_the_previous_terminal_phase() {
    let intake = MockIntake::new();
    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    intake.settle(IntakeStatus::Failed(FailureCause::Authentication));

    intake
        .offer(TARGET, 64)
        .expect("a terminal phase refused a fresh offer");
    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Receiving(Progress {
            written: 0,
            total: 64
        })
    );

    // An activated job is collected the same way, which is the ordinary
    // handoff: one update done, the next offered. The job offered above is
    // still in flight, so it is dropped first.
    intake.abort().unwrap();
    hand_over(&intake, CANDIDATE);
    intake.complete().unwrap();
    intake.settle(IntakeStatus::Activated);
    intake
        .offer(TARGET, 64)
        .expect("an activated job refused a fresh offer");
    assert_eq!(
        intake.poll().unwrap(),
        IntakeStatus::Receiving(Progress {
            written: 0,
            total: 64
        })
    );
}
