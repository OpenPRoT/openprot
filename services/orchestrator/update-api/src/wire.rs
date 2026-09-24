// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The intake wire protocol: fixed-size headers over a kernel channel.
//!
//! ```text
//! Request (16-byte header, then the chunk on Write only):
//! ┌─────┬────────┬──────────┬────────┬────────┬──────────┐
//! │ op  │ target │ reserved │  arg   │  len   │ reserved │
//! │ 1B  │   1B   │   2B     │ 8B LE  │ 2B LE  │   2B     │
//! └─────┴────────┴──────────┴────────┴────────┴──────────┘
//!
//! Response (20 bytes, no payload):
//! ┌──────┬───────┬────────┬──────────┬─────────┬─────────┐
//! │ code │ phase │ detail │ reserved │ written │  total  │
//! │  1B  │  1B   │   1B   │    1B    │  8B LE  │  8B LE  │
//! └──────┴───────┴────────┴──────────┴─────────┴─────────┘
//! ```
//!
//! `arg` is the payload length on [`Offer`](UpdateOp::Offer) and the write
//! offset on [`Write`](UpdateOp::Write); `len` is the chunk length on
//! [`Write`](UpdateOp::Write). Both are zero on every other op. Reserved
//! fields are zero everywhere and the handler rejects a request that sets
//! them, so they can be given a meaning later.
//!
//! Every response carries the phase, not just the answer to
//! [`Poll`](UpdateOp::Poll), so a source acting on each write's outcome sees
//! a job fail without a second round trip. `written`/`total` are zero
//! outside the phases that carry [`Progress`], `detail` zero outside
//! [`IntakeStatus::Failed`].
//!
//! The request side is another process and is treated as untrusted:
//! decoding validates lengths, opcodes, reserved fields and the chunk bound
//! before anything looks at the payload, and never panics.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{FailureCause, IntakeStatus, Progress, Reject, TargetId};

/// Largest chunk one [`Write`](UpdateOp::Write) may carry.
///
/// Bounds the handler's stack read buffer and the work of one request. 512
/// matches `FD_MAX_XFER_SIZE` in `pldm-interface`, the largest transfer the
/// firmware device negotiates, so a `RequestFirmwareData` chunk passes
/// through without splitting.
pub const MAX_CHUNK: usize = 512;

/// Request buffer size a handler must provide.
pub const MAX_REQUEST_SIZE: usize = RequestHeader::SIZE + MAX_CHUNK;

/// Response buffer size an initiator must provide.
pub const MAX_RESPONSE_SIZE: usize = ResponseHeader::SIZE;

/// Why a buffer did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// Shorter than the header, or shorter than the header plus the chunk
    /// length the header declares.
    Truncated,
    /// The output buffer cannot hold the encoding.
    BufferTooSmall,
    /// The op byte names no operation.
    InvalidOpcode(u8),
    /// A field carries a value this op does not define: a reserved field
    /// set, a chunk over [`MAX_CHUNK`], or `arg`/`len` non-zero on an op
    /// that has no use for them.
    InvalidField,
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            WireError::Truncated => "buffer shorter than the message",
            WireError::BufferTooSmall => "buffer too small for the message",
            WireError::InvalidOpcode(_) => "unknown operation code",
            WireError::InvalidField => "field value not defined for this operation",
        })
    }
}

impl core::error::Error for WireError {}

/// The intake operations, one per [`UpdateIntake`](crate::UpdateIntake)
/// method.
///
/// `#[non_exhaustive]`: the two sides are separate processes and may be built
/// from different revisions, so an unknown op is a runtime case
/// ([`WireError::InvalidOpcode`]), not a compile-time one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum UpdateOp {
    /// Offer a candidate and reserve the staging region.
    Offer = 0,
    /// Write one chunk into the staging region.
    Write = 1,
    /// Declare the candidate complete, starting the update.
    Complete = 2,
    /// Drop the job.
    Abort = 3,
    /// Read the phase.
    Poll = 4,
}

impl TryFrom<u8> for UpdateOp {
    type Error = WireError;

    fn try_from(value: u8) -> Result<Self, WireError> {
        match value {
            0 => Ok(UpdateOp::Offer),
            1 => Ok(UpdateOp::Write),
            2 => Ok(UpdateOp::Complete),
            3 => Ok(UpdateOp::Abort),
            4 => Ok(UpdateOp::Poll),
            other => Err(WireError::InvalidOpcode(other)),
        }
    }
}

/// The 16-byte request header. Fields are private because the accessors are
/// what validate them.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct RequestHeader {
    op_code: u8,
    target: u8,
    reserved: u16,
    arg: u64,
    len: u16,
    reserved_tail: u16,
}

impl RequestHeader {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    fn new(op: UpdateOp, target: u8, arg: u64, len: u16) -> Self {
        Self {
            op_code: op as u8,
            target,
            reserved: 0,
            arg: arg.to_le(),
            len: len.to_le(),
            reserved_tail: 0,
        }
    }

    /// The operation, or why the op byte is not one.
    pub fn op(&self) -> Result<UpdateOp, WireError> {
        UpdateOp::try_from(self.op_code)
    }

    /// The target byte, meaningful on [`UpdateOp::Offer`] only.
    pub fn target(&self) -> TargetId {
        TargetId(self.target)
    }

    /// The payload length on [`UpdateOp::Offer`], the write offset on
    /// [`UpdateOp::Write`], zero otherwise.
    pub fn arg(&self) -> u64 {
        u64::from_le(self.arg)
    }

    /// The chunk length on [`UpdateOp::Write`], zero otherwise.
    pub fn len(&self) -> u16 {
        u16::from_le(self.len)
    }

    fn reserved_are_clear(&self) -> bool {
        self.reserved == 0 && self.reserved_tail == 0
    }
}

/// One decoded request. Borrows the chunk out of the handler's read buffer,
/// so a staging write needs no second copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request<'a> {
    /// [`UpdateIntake::offer`](crate::UpdateIntake::offer).
    Offer {
        /// The device the candidate is for.
        target: TargetId,
        /// Payload length in bytes.
        total: u64,
    },
    /// [`UpdateIntake::write`](crate::UpdateIntake::write).
    Write {
        /// Offset into the offered payload.
        offset: u64,
        /// The chunk, at most [`MAX_CHUNK`] bytes.
        bytes: &'a [u8],
    },
    /// [`UpdateIntake::complete`](crate::UpdateIntake::complete).
    Complete,
    /// [`UpdateIntake::abort`](crate::UpdateIntake::abort).
    Abort,
    /// [`UpdateIntake::poll`](crate::UpdateIntake::poll).
    Poll,
}

impl Request<'_> {
    /// Encodes into `buf`, returning the encoded length.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        let (header, chunk) = match *self {
            Request::Offer { target, total } => (
                RequestHeader::new(UpdateOp::Offer, target.0, total, 0),
                None,
            ),
            Request::Write { offset, bytes } => {
                let len = u16::try_from(bytes.len()).map_err(|_| WireError::InvalidField)?;
                if bytes.len() > MAX_CHUNK {
                    return Err(WireError::InvalidField);
                }
                (
                    RequestHeader::new(UpdateOp::Write, 0, offset, len),
                    Some(bytes),
                )
            }
            Request::Complete => (RequestHeader::new(UpdateOp::Complete, 0, 0, 0), None),
            Request::Abort => (RequestHeader::new(UpdateOp::Abort, 0, 0, 0), None),
            Request::Poll => (RequestHeader::new(UpdateOp::Poll, 0, 0, 0), None),
        };
        let chunk = chunk.unwrap_or(&[]);
        let total = RequestHeader::SIZE + chunk.len();
        let out = buf.get_mut(..total).ok_or(WireError::BufferTooSmall)?;
        out[..RequestHeader::SIZE].copy_from_slice(header.as_bytes());
        out[RequestHeader::SIZE..].copy_from_slice(chunk);
        Ok(total)
    }

    /// Decodes one request out of a handler's read buffer, validating header
    /// length, opcode, reserved fields, the chunk bound, and that fields an
    /// op does not define are zero.
    pub fn decode(buf: &[u8]) -> Result<Request<'_>, WireError> {
        let (head, rest) = buf
            .split_at_checked(RequestHeader::SIZE)
            .ok_or(WireError::Truncated)?;
        // Infallible on an exact-size slice; report it as a short buffer.
        let header = RequestHeader::read_from_bytes(head).map_err(|_| WireError::Truncated)?;
        if !header.reserved_are_clear() {
            return Err(WireError::InvalidField);
        }
        let op = header.op()?;
        let arg = header.arg();
        let len = usize::from(header.len());
        // Reject rather than ignore a field the op does not define: a
        // mismatch means the two sides disagree about the protocol.
        if op != UpdateOp::Write && len != 0 {
            return Err(WireError::InvalidField);
        }
        if matches!(op, UpdateOp::Complete | UpdateOp::Abort | UpdateOp::Poll) && arg != 0 {
            return Err(WireError::InvalidField);
        }
        Ok(match op {
            UpdateOp::Offer => Request::Offer {
                target: header.target(),
                total: arg,
            },
            UpdateOp::Write => {
                if len > MAX_CHUNK {
                    return Err(WireError::InvalidField);
                }
                Request::Write {
                    offset: arg,
                    bytes: rest.get(..len).ok_or(WireError::Truncated)?,
                }
            }
            UpdateOp::Complete => Request::Complete,
            UpdateOp::Abort => Request::Abort,
            UpdateOp::Poll => Request::Poll,
        })
    }
}

/// The 20-byte response.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ResponseHeader {
    code: u8,
    phase: u8,
    detail: u8,
    reserved: u8,
    written: u64,
    total: u64,
}

impl ResponseHeader {
    /// Encoded size in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// What the handler answers: the outcome of the request, plus the phase as
/// of that answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Response {
    /// Whether the request took effect. A refusal leaves the phase
    /// unchanged, so `status` still describes the job already there.
    pub outcome: Result<(), Reject>,
    /// The phase as of this answer.
    pub status: IntakeStatus,
}

impl Response {
    /// Encodes into `buf`, returning the encoded length.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        let (phase, detail, progress) = match self.status {
            IntakeStatus::Idle => (PHASE_IDLE, 0, None),
            IntakeStatus::Receiving(p) => (PHASE_RECEIVING, 0, Some(p)),
            IntakeStatus::Authenticating => (PHASE_AUTHENTICATING, 0, None),
            IntakeStatus::Staging(p) => (PHASE_STAGING, 0, Some(p)),
            IntakeStatus::Activated => (PHASE_ACTIVATED, 0, None),
            IntakeStatus::Failed(cause) => (PHASE_FAILED, cause_code(cause), None),
        };
        let progress = progress.unwrap_or(Progress {
            written: 0,
            total: 0,
        });
        let header = ResponseHeader {
            code: match self.outcome {
                Ok(()) => CODE_OK,
                Err(reject) => reject_code(reject),
            },
            phase,
            detail,
            reserved: 0,
            written: progress.written.to_le(),
            total: progress.total.to_le(),
        };
        let out = buf
            .get_mut(..ResponseHeader::SIZE)
            .ok_or(WireError::BufferTooSmall)?;
        out.copy_from_slice(header.as_bytes());
        Ok(ResponseHeader::SIZE)
    }

    /// Decodes the handler's answer.
    pub fn decode(buf: &[u8]) -> Result<Response, WireError> {
        let head = buf
            .get(..ResponseHeader::SIZE)
            .ok_or(WireError::Truncated)?;
        let header = ResponseHeader::read_from_bytes(head).map_err(|_| WireError::Truncated)?;
        if header.reserved != 0 {
            return Err(WireError::InvalidField);
        }
        let progress = Progress {
            written: u64::from_le(header.written),
            total: u64::from_le(header.total),
        };
        let status = match header.phase {
            PHASE_IDLE => IntakeStatus::Idle,
            PHASE_RECEIVING => IntakeStatus::Receiving(progress),
            PHASE_AUTHENTICATING => IntakeStatus::Authenticating,
            PHASE_STAGING => IntakeStatus::Staging(progress),
            PHASE_ACTIVATED => IntakeStatus::Activated,
            PHASE_FAILED => IntakeStatus::Failed(cause_from_code(header.detail)?),
            _ => return Err(WireError::InvalidField),
        };
        let outcome = match header.code {
            CODE_OK => Ok(()),
            code => Err(reject_from_code(code)?),
        };
        Ok(Response { outcome, status })
    }
}

const CODE_OK: u8 = 0;

const PHASE_IDLE: u8 = 0;
const PHASE_RECEIVING: u8 = 1;
const PHASE_AUTHENTICATING: u8 = 2;
const PHASE_STAGING: u8 = 3;
const PHASE_ACTIVATED: u8 = 4;
const PHASE_FAILED: u8 = 5;

fn cause_code(cause: FailureCause) -> u8 {
    match cause {
        FailureCause::Authentication => 1,
        FailureCause::Deferred => 2,
        FailureCause::Superseded => 3,
        FailureCause::Device => 4,
    }
}

fn cause_from_code(code: u8) -> Result<FailureCause, WireError> {
    match code {
        1 => Ok(FailureCause::Authentication),
        2 => Ok(FailureCause::Deferred),
        3 => Ok(FailureCause::Superseded),
        4 => Ok(FailureCause::Device),
        _ => Err(WireError::InvalidField),
    }
}

fn reject_code(reject: Reject) -> u8 {
    match reject {
        Reject::UnknownTarget => 1,
        Reject::Busy => 2,
        Reject::NoJob => 3,
        Reject::OutOfRange => 4,
        Reject::Incomplete => 5,
        Reject::Storage => 6,
        Reject::Malformed => 7,
    }
}

fn reject_from_code(code: u8) -> Result<Reject, WireError> {
    match code {
        1 => Ok(Reject::UnknownTarget),
        2 => Ok(Reject::Busy),
        3 => Ok(Reject::NoJob),
        4 => Ok(Reject::OutOfRange),
        5 => Ok(Reject::Incomplete),
        6 => Ok(Reject::Storage),
        7 => Ok(Reject::Malformed),
        _ => Err(WireError::InvalidField),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip_request(request: Request<'_>) {
        let mut buf = [0u8; MAX_REQUEST_SIZE];
        let len = request.encode(&mut buf).expect("encode failed");
        assert_eq!(Request::decode(&buf[..len]), Ok(request));
    }

    #[test]
    fn every_request_round_trips() {
        round_trip_request(Request::Offer {
            target: TargetId(3),
            total: 12 * 1024 * 1024,
        });
        round_trip_request(Request::Write {
            offset: u64::from(u32::MAX) + 1,
            bytes: &[0xa5; MAX_CHUNK],
        });
        round_trip_request(Request::Write {
            offset: 0,
            bytes: &[],
        });
        round_trip_request(Request::Complete);
        round_trip_request(Request::Abort);
        round_trip_request(Request::Poll);
    }

    #[test]
    fn a_request_header_is_sixteen_bytes() {
        // Part of the protocol, not an artifact of the field list: a handler
        // sizes its read buffer from it.
        assert_eq!(RequestHeader::SIZE, 16);
        assert_eq!(MAX_REQUEST_SIZE, 16 + MAX_CHUNK);
    }

    #[test]
    fn a_chunk_over_the_bound_is_refused_by_both_sides() {
        let oversized = [0u8; MAX_CHUNK + 1];
        let mut buf = [0u8; MAX_REQUEST_SIZE + 1];

        assert_eq!(
            Request::Write {
                offset: 0,
                bytes: &oversized,
            }
            .encode(&mut buf),
            Err(WireError::InvalidField)
        );

        // Hand-built by a source ignoring the bound: the handler must not
        // read past its own buffer on the strength of a declared length.
        let header = RequestHeader::new(UpdateOp::Write, 0, 0, (MAX_CHUNK + 1) as u16);
        buf[..RequestHeader::SIZE].copy_from_slice(header.as_bytes());
        assert_eq!(
            Request::decode(&buf[..RequestHeader::SIZE + MAX_CHUNK + 1]),
            Err(WireError::InvalidField)
        );
    }

    #[test]
    fn a_write_shorter_than_its_declared_chunk_is_truncated() {
        let mut buf = [0u8; MAX_REQUEST_SIZE];
        let len = Request::Write {
            offset: 0,
            bytes: &[1, 2, 3, 4],
        }
        .encode(&mut buf)
        .expect("encode failed");

        assert_eq!(Request::decode(&buf[..len - 1]), Err(WireError::Truncated));
    }

    #[test]
    fn a_short_header_is_truncated_not_a_panic() {
        for len in 0..RequestHeader::SIZE {
            assert_eq!(
                Request::decode(&[0u8; 32][..len]),
                Err(WireError::Truncated)
            );
        }
    }

    #[test]
    fn an_unknown_opcode_is_reported_with_its_value() {
        let mut buf = [0u8; RequestHeader::SIZE];
        buf[0] = 9;

        assert_eq!(Request::decode(&buf), Err(WireError::InvalidOpcode(9)));
    }

    #[test]
    fn fields_an_op_does_not_define_must_be_zero() {
        // Reserved, so they stay free for a later meaning.
        let mut buf = [0u8; RequestHeader::SIZE];
        buf[0] = UpdateOp::Poll as u8;
        buf[2] = 1;
        assert_eq!(Request::decode(&buf), Err(WireError::InvalidField));

        // A chunk length on an op that carries no chunk.
        let header = RequestHeader::new(UpdateOp::Complete, 0, 0, 4);
        assert_eq!(
            Request::decode(header.as_bytes()),
            Err(WireError::InvalidField)
        );

        // An arg on an op that has no use for one.
        let header = RequestHeader::new(UpdateOp::Poll, 0, 64, 0);
        assert_eq!(
            Request::decode(header.as_bytes()),
            Err(WireError::InvalidField)
        );
    }

    fn round_trip_response(response: Response) {
        let mut buf = [0u8; MAX_RESPONSE_SIZE];
        let len = response.encode(&mut buf).expect("encode failed");
        assert_eq!(len, MAX_RESPONSE_SIZE);
        assert_eq!(Response::decode(&buf[..len]), Ok(response));
    }

    #[test]
    fn every_phase_round_trips() {
        let progress = Progress {
            written: 4096,
            total: 12 * 1024 * 1024,
        };
        for status in [
            IntakeStatus::Idle,
            IntakeStatus::Receiving(progress),
            IntakeStatus::Authenticating,
            IntakeStatus::Staging(progress),
            IntakeStatus::Activated,
            IntakeStatus::Failed(FailureCause::Authentication),
            IntakeStatus::Failed(FailureCause::Deferred),
            IntakeStatus::Failed(FailureCause::Superseded),
            IntakeStatus::Failed(FailureCause::Device),
        ] {
            round_trip_response(Response {
                outcome: Ok(()),
                status,
            });
        }
    }

    #[test]
    fn every_reject_round_trips_and_keeps_the_phase() {
        // The phase rides along with a refusal, so a source that only
        // writes still learns that its job died.
        for reject in [
            Reject::UnknownTarget,
            Reject::Busy,
            Reject::NoJob,
            Reject::OutOfRange,
            Reject::Incomplete,
            Reject::Storage,
            Reject::Malformed,
        ] {
            round_trip_response(Response {
                outcome: Err(reject),
                status: IntakeStatus::Failed(FailureCause::Device),
            });
        }
    }

    #[test]
    fn progress_is_zero_outside_the_phases_that_carry_it() {
        let mut buf = [0u8; MAX_RESPONSE_SIZE];
        Response {
            outcome: Ok(()),
            status: IntakeStatus::Activated,
        }
        .encode(&mut buf)
        .expect("encode failed");

        assert_eq!(buf[ResponseHeader::SIZE - 16..], [0u8; 16]);
    }

    #[test]
    fn an_undefined_phase_or_cause_does_not_decode() {
        let mut buf = [0u8; MAX_RESPONSE_SIZE];
        buf[1] = 9;
        assert_eq!(Response::decode(&buf), Err(WireError::InvalidField));

        // Failed with no cause: the two only mean something together.
        buf[1] = PHASE_FAILED;
        buf[2] = 0;
        assert_eq!(Response::decode(&buf), Err(WireError::InvalidField));
    }

    #[test]
    fn a_short_response_is_truncated_not_a_panic() {
        for len in 0..ResponseHeader::SIZE {
            assert_eq!(
                Response::decode(&[0u8; 32][..len]),
                Err(WireError::Truncated)
            );
        }
    }
}
