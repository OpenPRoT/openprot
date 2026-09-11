// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`IncrementalVerifier`] update-verification capability contract.

use crate::PayloadSource;

/// Factory for incremental verification sessions. Call [`start`] to
/// begin hashing a candidate image; the returned [`VerifySession`] does
/// the actual work one bounded step at a time.
///
/// `start` consumes the verifier. Terminal outcomes
/// ([`Authenticated`](PollOutcome::Authenticated),
/// [`Rejected`](PollOutcome::Rejected),
/// [`Fault`](PollOutcome::Fault)) and [`abandon`](VerifySession::abandon)
/// return it, so the caller can start another session. Polling after a
/// verdict is unrepresentable: the session is gone.
///
/// The boot-time synchronous `Verifier` (in the driver crate) is
/// unaffected: it stays one-shot for the chain walk, where the image is
/// small and local.
///
/// [`start`]: IncrementalVerifier::start
pub trait IncrementalVerifier: Sized {
    /// The error reported when the check itself cannot run: crypto fault
    /// or unreadable payload. A bad image is
    /// [`Rejected`](PollOutcome::Rejected), not an error.
    type Error: core::error::Error;

    /// The session type returned by [`start`](IncrementalVerifier::start).
    type Session: VerifySession<Verifier = Self, Error = Self::Error>;

    /// Begins a new verification session. The verifier's internal state
    /// (crypto context, scratch buffers) moves into the session.
    fn start(self) -> Self::Session;
}

/// A live verification session. Each [`poll`] call does at most one
/// read from the payload and one hash update, then returns. The chunk
/// size is implementor-chosen, sized so each poll fits the caller's
/// per-poll time budget.
///
/// The caller watches progress via `done` in
/// [`Processing`](PollOutcome::Processing) and abandons a session that
/// stalls, on its own budget; the session never judges liveness.
///
/// `poll` consumes the session and returns a [`PollOutcome`]. On
/// [`Processing`](PollOutcome::Processing) the session comes back
/// inside the variant; on any terminal outcome the verifier comes back
/// instead, ready for a new [`start`](IncrementalVerifier::start).
///
/// [`poll`]: VerifySession::poll
pub trait VerifySession: Sized {
    /// The verifier this session was created from.
    type Verifier;

    /// The error type, matching the verifier's.
    type Error: core::error::Error;

    /// Processes one bounded step. Never waits on device progress, never
    /// sleeps. An empty payload is a fault, never a vacuous
    /// `Authenticated`.
    fn poll(self, payload: &dyn PayloadSource) -> PollOutcome<Self>;

    /// Discards the session and returns the verifier.
    /// Infallible: a half-finished hash state is simply dropped.
    fn abandon(self) -> Self::Verifier;
}

/// Result of one [`VerifySession::poll`] call. Terminal variants return
/// the verifier, ready for a new [`start`](IncrementalVerifier::start).
#[derive(Debug)]
pub enum PollOutcome<S: VerifySession> {
    /// One chunk processed. `done` bytes so far out of `total`.
    /// The session is inside, ready for the next poll.
    Processing { session: S, done: u64, total: u64 },
    /// The complete image authenticated (signature and policy checks
    /// passed).
    Authenticated(S::Verifier),
    /// The complete image was checked and found invalid.
    Rejected(S::Verifier),
    /// The check could not run (unreadable payload, crypto engine
    /// error).
    Fault(S::Verifier, S::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PayloadReadError, PayloadSource};

    // A PayloadSource over a plain byte slice.
    struct SlicePayload(&'static [u8]);

    impl PayloadSource for SlicePayload {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError> {
            let start = usize::try_from(offset).map_err(|_| PayloadReadError::OutOfRange)?;
            let end = start
                .checked_add(buf.len())
                .ok_or(PayloadReadError::OutOfRange)?;
            buf.copy_from_slice(self.0.get(start..end).ok_or(PayloadReadError::OutOfRange)?);
            Ok(())
        }
    }

    // Hashes 4 bytes per poll, accepts images whose first byte is nonzero.
    struct ChunkedVerifier;

    struct ChunkedSession {
        offset: u64,
        total: u64,
    }

    #[derive(Debug, PartialEq)]
    struct VerifierFault;

    impl core::fmt::Display for VerifierFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("verifier fault")
        }
    }

    impl core::error::Error for VerifierFault {}

    impl IncrementalVerifier for ChunkedVerifier {
        type Error = VerifierFault;
        type Session = ChunkedSession;

        fn start(self) -> ChunkedSession {
            ChunkedSession {
                offset: 0,
                total: 0,
            }
        }
    }

    impl VerifySession for ChunkedSession {
        type Verifier = ChunkedVerifier;
        type Error = VerifierFault;

        fn poll(mut self, payload: &dyn PayloadSource) -> PollOutcome<Self> {
            if self.total == 0 {
                self.total = payload.len();
            }
            if self.offset >= self.total {
                let mut first = [0u8; 1];
                if payload.read_at(0, &mut first).is_err() {
                    return PollOutcome::Fault(ChunkedVerifier, VerifierFault);
                }
                return if first[0] != 0 {
                    PollOutcome::Authenticated(ChunkedVerifier)
                } else {
                    PollOutcome::Rejected(ChunkedVerifier)
                };
            }
            let chunk = core::cmp::min(4, (self.total - self.offset) as usize);
            let mut buf = [0u8; 4];
            if payload.read_at(self.offset, &mut buf[..chunk]).is_err() {
                return PollOutcome::Fault(ChunkedVerifier, VerifierFault);
            }
            self.offset += chunk as u64;
            let done = self.offset;
            let total = self.total;
            PollOutcome::Processing {
                session: self,
                done,
                total,
            }
        }

        fn abandon(self) -> ChunkedVerifier {
            ChunkedVerifier
        }
    }

    // Helper: drive a session to completion in a loop, the way the
    // update pump would. Returns the terminal outcome's verifier and
    // whether it authenticated.
    fn drive(
        mut session: ChunkedSession,
        payload: &dyn PayloadSource,
    ) -> (ChunkedVerifier, Option<bool>) {
        loop {
            match session.poll(payload) {
                PollOutcome::Processing { session: s, .. } => session = s,
                PollOutcome::Authenticated(v) => return (v, Some(true)),
                PollOutcome::Rejected(v) => return (v, Some(false)),
                PollOutcome::Fault(v, _) => return (v, None),
            }
        }
    }

    #[test]
    fn multi_poll_until_authenticated() {
        let payload = SlicePayload(&[0xAA; 10]);
        let session = ChunkedVerifier.start();

        // 4 bytes, 4 bytes, 2 bytes = 3 Processing steps, then verdict.
        let PollOutcome::Processing {
            session,
            done: 4,
            total: 10,
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        let PollOutcome::Processing {
            session,
            done: 8,
            total: 10,
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        let PollOutcome::Processing {
            session,
            done: 10,
            total: 10,
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        assert!(matches!(
            session.poll(&payload),
            PollOutcome::Authenticated(_)
        ));
    }

    #[test]
    fn rejected_image() {
        let payload = SlicePayload(&[0x00; 8]);
        let (_, verdict) = drive(ChunkedVerifier.start(), &payload);
        assert_eq!(verdict, Some(false));
    }

    #[test]
    fn abandon_returns_verifier_for_reuse() {
        let payload = SlicePayload(&[0xFF; 12]);
        let session = ChunkedVerifier.start();

        let PollOutcome::Processing { session, .. } = session.poll(&payload) else {
            panic!("expected Processing");
        };

        // Abandon mid-session, get verifier back, start fresh.
        let verifier = session.abandon();
        let (_, verdict) = drive(verifier.start(), &payload);
        assert_eq!(verdict, Some(true));
    }

    #[test]
    fn verifier_reusable_after_fault() {
        let session = ChunkedVerifier.start();
        let PollOutcome::Fault(verifier, _) = session.poll(&Lying) else {
            panic!("expected Fault");
        };

        let payload = SlicePayload(&[0xFF; 4]);
        let (_, verdict) = drive(verifier.start(), &payload);
        assert_eq!(verdict, Some(true));
    }

    #[test]
    fn verifier_reusable_after_verdict() {
        let payload = SlicePayload(&[0xFF; 4]);
        let (verifier, _) = drive(ChunkedVerifier.start(), &payload);

        let (_, verdict) = drive(verifier.start(), &payload);
        assert_eq!(verdict, Some(true));
    }

    #[test]
    fn empty_payload_is_fault() {
        let payload = SlicePayload(&[]);
        let session = ChunkedVerifier.start();
        assert!(
            matches!(session.poll(&payload), PollOutcome::Fault(..)),
            "empty payload must fault, never vacuously authenticate"
        );
    }

    #[test]
    fn fault_after_partial_progress() {
        // Reads succeed for the first chunk, fail after.
        struct FailsAfterFirstChunk;

        impl PayloadSource for FailsAfterFirstChunk {
            fn len(&self) -> u64 {
                12
            }

            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), PayloadReadError> {
                if offset >= 4 {
                    return Err(PayloadReadError::Storage);
                }
                buf.fill(0xFF);
                Ok(())
            }
        }

        let session = ChunkedVerifier.start();
        let PollOutcome::Processing {
            session,
            done: 4,
            total: 12,
        } = session.poll(&FailsAfterFirstChunk)
        else {
            panic!("expected Processing");
        };
        let PollOutcome::Fault(verifier, _) = session.poll(&FailsAfterFirstChunk) else {
            panic!("expected Fault");
        };

        let payload = SlicePayload(&[0xFF; 4]);
        let (_, verdict) = drive(verifier.start(), &payload);
        assert_eq!(verdict, Some(true));
    }

    #[test]
    fn verifier_reusable_after_rejection() {
        let bad = SlicePayload(&[0x00; 4]);
        let (verifier, verdict) = drive(ChunkedVerifier.start(), &bad);
        assert_eq!(verdict, Some(false));

        let good = SlicePayload(&[0xFF; 4]);
        let (_, verdict) = drive(verifier.start(), &good);
        assert_eq!(verdict, Some(true));
    }

    #[test]
    fn abandon_before_first_poll() {
        let verifier = ChunkedVerifier.start().abandon();
        let payload = SlicePayload(&[0xFF; 4]);
        let (_, verdict) = drive(verifier.start(), &payload);
        assert_eq!(verdict, Some(true));
    }

    // A PayloadSource whose read_at always fails.
    struct Lying;

    impl PayloadSource for Lying {
        fn len(&self) -> u64 {
            64
        }

        fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Result<(), PayloadReadError> {
            Err(PayloadReadError::Storage)
        }
    }

    // Pump-shaped test: stores the session state across iterations the
    // way the update pump would, using an enum to hold either the idle
    // verifier or the active session.
    #[test]
    fn pump_loop_with_state_enum() {
        enum State {
            Idle(ChunkedVerifier),
            Verifying(ChunkedSession),
        }

        let payload = SlicePayload(&[0xFF; 10]);
        let mut state = State::Idle(ChunkedVerifier);
        let mut polls = 0;

        loop {
            state = match state {
                State::Idle(v) => State::Verifying(v.start()),
                State::Verifying(s) => match s.poll(&payload) {
                    PollOutcome::Processing { session, .. } => {
                        polls += 1;
                        State::Verifying(session)
                    }
                    PollOutcome::Authenticated(_) => {
                        assert!(polls > 0);
                        return;
                    }
                    PollOutcome::Rejected(_) => panic!("expected authenticated"),
                    PollOutcome::Fault(_, e) => panic!("unexpected fault: {e}"),
                },
            };
        }
    }
}
