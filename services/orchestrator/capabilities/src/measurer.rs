// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Firmware measurement trait.

use crate::Progress;
use util_io::ByteSource;

/// SHA-256 digest of a component's firmware image. Newtype so a bare
/// `[u8; 32]` can't be confused for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measurement(pub [u8; Self::LEN]);

impl Measurement {
    /// Bytes in one measurement (SHA-256 = 32). A crypto service that
    /// reports SHA-384 for SPDM needs this widened or made generic.
    pub const LEN: usize = 32;
}

/// Hashes a firmware image in bounded steps. [`start`] begins a session;
/// the returned [`MeasureSession`] does one chunk per [`poll`] call,
/// reading straight from the image source.
///
/// `start` consumes the measurer. Terminal outcomes
/// ([`Complete`](MeasureOutcome::Complete),
/// [`Fault`](MeasureOutcome::Fault)) and
/// [`abandon`](MeasureSession::abandon) return it, so the caller can
/// start another session.
///
/// [`start`]: Measurer::start
pub trait Measurer: Sized {
    /// A fault in the hash engine or an unreadable source. A completed
    /// hash is [`Complete`](MeasureOutcome::Complete), never an error.
    type Error: core::error::Error;

    /// The session type returned by [`start`](Measurer::start).
    type Session: MeasureSession<Measurer = Self, Error = Self::Error>;

    /// Begins a new measurement session.
    fn start(self) -> Self::Session;
}

/// A live measurement session. Each [`poll`] call does one read and one
/// hash update, then returns. Chunk size is up to the implementation.
///
/// [`poll`]: MeasureSession::poll
pub trait MeasureSession: Sized {
    /// The measurer this session was created from.
    type Measurer;

    /// The error type, matching the capability's.
    type Error: core::error::Error;

    /// Processes one bounded step. Never waits, never sleeps. An empty
    /// source is a fault, not a zero-length hash.
    fn poll(self, payload: &dyn ByteSource) -> MeasureOutcome<Self>;

    /// Discards the session and returns the measurer.
    /// Infallible: a half-finished hash state is simply dropped.
    fn abandon(self) -> Self::Measurer;
}

/// Result of one [`MeasureSession::poll`] call.
#[derive(Debug)]
pub enum MeasureOutcome<S: MeasureSession> {
    /// One chunk hashed. The session is inside, ready for the next poll.
    Processing { session: S, progress: Progress },
    /// The complete image has been hashed.
    Complete(S::Measurer, Measurement),
    /// The hash could not run (unreadable source, engine fault).
    Fault(S::Measurer, S::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use util_io::{ByteReadError, ByteSource};

    struct SlicePayload(&'static [u8]);

    impl ByteSource for SlicePayload {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
            let start = usize::try_from(offset).map_err(|_| ByteReadError::OutOfRange)?;
            let end = start
                .checked_add(buf.len())
                .ok_or(ByteReadError::OutOfRange)?;
            buf.copy_from_slice(self.0.get(start..end).ok_or(ByteReadError::OutOfRange)?);
            Ok(())
        }
    }

    // Hashes 4 bytes per poll, XORs all bytes into the first byte of
    // the digest (enough to prove the trait contract without pulling a
    // real SHA-256).
    struct XorMeasurer;

    struct XorSession {
        offset: u64,
        total: u64,
        accum: u8,
    }

    #[derive(Debug, PartialEq)]
    struct MeasureFault;

    impl core::fmt::Display for MeasureFault {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("measure fault")
        }
    }

    impl core::error::Error for MeasureFault {}

    impl Measurer for XorMeasurer {
        type Error = MeasureFault;
        type Session = XorSession;

        fn start(self) -> XorSession {
            XorSession {
                offset: 0,
                total: 0,
                accum: 0,
            }
        }
    }

    impl MeasureSession for XorSession {
        type Measurer = XorMeasurer;
        type Error = MeasureFault;

        fn poll(mut self, payload: &dyn ByteSource) -> MeasureOutcome<Self> {
            if self.total == 0 {
                let len = payload.len();
                if len == 0 {
                    return MeasureOutcome::Fault(XorMeasurer, MeasureFault);
                }
                self.total = len;
            }
            if self.offset >= self.total {
                let mut digest = Measurement([0u8; Measurement::LEN]);
                digest.0[0] = self.accum;
                return MeasureOutcome::Complete(XorMeasurer, digest);
            }
            let chunk = core::cmp::min(4, (self.total - self.offset) as usize);
            let mut buf = [0u8; 4];
            if payload.read_at(self.offset, &mut buf[..chunk]).is_err() {
                return MeasureOutcome::Fault(XorMeasurer, MeasureFault);
            }
            for &b in &buf[..chunk] {
                self.accum ^= b;
            }
            self.offset += chunk as u64;
            let written = self.offset;
            let total = self.total;
            MeasureOutcome::Processing {
                session: self,
                progress: Progress { written, total },
            }
        }

        fn abandon(self) -> XorMeasurer {
            XorMeasurer
        }
    }

    fn drive(
        mut session: XorSession,
        payload: &dyn ByteSource,
    ) -> (XorMeasurer, Option<Measurement>) {
        loop {
            match session.poll(payload) {
                MeasureOutcome::Processing { session: s, .. } => session = s,
                MeasureOutcome::Complete(m, digest) => return (m, Some(digest)),
                MeasureOutcome::Fault(m, _) => return (m, None),
            }
        }
    }

    #[test]
    fn multi_poll_until_complete() {
        let payload = SlicePayload(&[0xAA; 10]);
        let session = XorMeasurer.start();

        let MeasureOutcome::Processing {
            session,
            progress:
                Progress {
                    written: 4,
                    total: 10,
                },
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        let MeasureOutcome::Processing {
            session,
            progress:
                Progress {
                    written: 8,
                    total: 10,
                },
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        let MeasureOutcome::Processing {
            session,
            progress:
                Progress {
                    written: 10,
                    total: 10,
                },
        } = session.poll(&payload)
        else {
            panic!("expected Processing");
        };
        assert!(matches!(
            session.poll(&payload),
            MeasureOutcome::Complete(..)
        ));
    }

    #[test]
    fn digest_reflects_content() {
        let payload = SlicePayload(&[0xFF; 8]);
        let (_, digest) = drive(XorMeasurer.start(), &payload);
        let d = digest.unwrap();
        // 8 bytes of 0xFF XORed: 0xFF ^ 0xFF = 0x00 per pair, so 0x00.
        assert_eq!(d.0[0], 0x00);
        assert_eq!(d.0[1..], [0u8; Measurement::LEN - 1]);
    }

    #[test]
    fn different_content_different_digest() {
        let a = SlicePayload(&[0x01, 0x02, 0x03, 0x04]);
        let b = SlicePayload(&[0x05, 0x06, 0x07, 0x08]);
        let (_, da) = drive(XorMeasurer.start(), &a);
        let (_, db) = drive(XorMeasurer.start(), &b);
        assert_ne!(da.unwrap().0[0], db.unwrap().0[0]);
    }

    #[test]
    fn empty_payload_is_fault() {
        let payload = SlicePayload(&[]);
        let session = XorMeasurer.start();
        assert!(matches!(session.poll(&payload), MeasureOutcome::Fault(..)));
    }

    #[test]
    fn abandon_before_first_poll() {
        let session = XorMeasurer.start();
        let m = session.abandon();
        let payload = SlicePayload(&[0xFF; 4]);
        let (_, digest) = drive(m.start(), &payload);
        assert!(digest.is_some());
    }

    #[test]
    fn abandon_returns_capability_for_reuse() {
        let payload = SlicePayload(&[0xFF; 12]);
        let session = XorMeasurer.start();

        let MeasureOutcome::Processing { session, .. } = session.poll(&payload) else {
            panic!("expected Processing");
        };

        let m = session.abandon();
        let (_, digest) = drive(m.start(), &payload);
        assert!(digest.is_some());
    }

    #[test]
    fn reusable_after_complete() {
        let payload = SlicePayload(&[0xFF; 4]);
        let (m, _) = drive(XorMeasurer.start(), &payload);
        let (_, digest) = drive(m.start(), &payload);
        assert!(digest.is_some());
    }

    #[test]
    fn reusable_after_fault() {
        let session = XorMeasurer.start();
        let MeasureOutcome::Fault(m, _) = session.poll(&Failing) else {
            panic!("expected Fault");
        };
        let payload = SlicePayload(&[0xFF; 4]);
        let (_, digest) = drive(m.start(), &payload);
        assert!(digest.is_some());
    }

    #[test]
    fn fault_after_partial_progress() {
        struct FailsAfterFirstChunk;

        impl ByteSource for FailsAfterFirstChunk {
            fn len(&self) -> u64 {
                12
            }

            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
                if offset >= 4 {
                    return Err(ByteReadError::Storage);
                }
                buf.fill(0xFF);
                Ok(())
            }
        }

        let session = XorMeasurer.start();
        let MeasureOutcome::Processing {
            session,
            progress:
                Progress {
                    written: 4,
                    total: 12,
                },
        } = session.poll(&FailsAfterFirstChunk)
        else {
            panic!("expected Processing");
        };
        let MeasureOutcome::Fault(m, _) = session.poll(&FailsAfterFirstChunk) else {
            panic!("expected Fault");
        };

        let payload = SlicePayload(&[0xFF; 4]);
        let (_, digest) = drive(m.start(), &payload);
        assert!(digest.is_some());
    }

    struct Failing;

    impl ByteSource for Failing {
        fn len(&self) -> u64 {
            64
        }

        fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Result<(), ByteReadError> {
            Err(ByteReadError::Storage)
        }
    }
}
