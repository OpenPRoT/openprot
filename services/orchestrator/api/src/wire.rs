// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Wire protocol for the lifecycle service.
//!
//! Manual, syscall-free byte encodings shared by both ends of the lifecycle IPC
//! channel — the server (`openprot_orchestrator_server`) and the on-target queue
//! (`openprot_orchestrator_ipc`). Centralizing the codec in the API crate, beside
//! the [`Event`] and [`State`] types it encodes, gives a single source of truth
//! and avoids re-deriving the tags on each side; this mirrors
//! `openprot_mctp_api::wire`.
//!
//! Two layers are encoded here:
//!
//! - **events** ([`encode_event`] / [`decode_event`]) — a single lifecycle
//!   [`Event`] crossing the queue channel; and
//! - **requests/responses** ([`Request`] / [`Response`]) — the server protocol
//!   an external producer uses to inject an event or query state and read back
//!   the settled [`State`].
//!
//! All tags are frozen: never renumber a code, only append. The Rust enum
//! discriminant is deliberately *not* used — it is not a stable ABI and the two
//! ends of the channel may be built separately.

use crate::{Event, State, UpdateScope};

/// Bytes in an encoded [`Event`] on the queue channel.
pub const EVENT_WIRE_SIZE: usize = 4;

/// Bytes in an encoded [`Request`]: a 1-byte opcode plus a 1-byte argument.
pub const REQUEST_WIRE_SIZE: usize = 2;

/// Bytes in an encoded [`Response`]: a 1-byte status plus a 1-byte argument.
pub const RESPONSE_WIRE_SIZE: usize = 2;

/// A failure encoding or decoding a lifecycle wire message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireError {
    /// The destination buffer was too small for the encoding.
    BufferTooSmall,
    /// Fewer bytes than the message requires.
    Truncated,
    /// An unrecognized opcode/status, or a tag with no known mapping.
    Malformed,
}

// Request opcodes. Frozen: never renumber.
const OP_INJECT: u8 = 1;
const OP_QUERY_STATE: u8 = 2;

// Response statuses. Frozen: never renumber.
const ST_STATE: u8 = 1;
const ST_REJECTED: u8 = 2;

/// A request from an external producer to the lifecycle service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// Inject a lifecycle [`Event`] (a command, watchdog signal, reset
    /// notification, …).
    Inject(Event),
    /// Ask for the machine's current [`State`] without changing it.
    QueryState,
}

/// A response from the lifecycle service to a producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Response {
    /// The machine settled in this [`State`] after the request.
    State(State),
    /// The request could not be decoded and was ignored.
    Rejected,
}

/// Encode a single [`Event`] into `buf`; returns bytes written
/// ([`EVENT_WIRE_SIZE`]).
///
/// Byte 0 is the frozen tag; byte 1 carries any payload (see
/// [`event_payload`]); the remaining bytes are reserved and written as zero.
pub fn encode_event(buf: &mut [u8], event: Event) -> Result<usize, WireError> {
    if buf.len() < EVENT_WIRE_SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let raw = [event_tag(event), event_payload(event), 0, 0];
    buf[..EVENT_WIRE_SIZE].copy_from_slice(&raw);
    Ok(EVENT_WIRE_SIZE)
}

/// Decode a single [`Event`] from the front of `bytes`.
pub fn decode_event(bytes: &[u8]) -> Result<Event, WireError> {
    let raw: [u8; EVENT_WIRE_SIZE] = bytes
        .get(..EVENT_WIRE_SIZE)
        .ok_or(WireError::Truncated)?
        .try_into()
        .map_err(|_| WireError::Truncated)?;
    let [tag, payload, _, _] = raw;
    event_from_tag(tag, payload).ok_or(WireError::Malformed)
}

/// Encode a [`Request`] into `buf`; returns bytes written
/// ([`REQUEST_WIRE_SIZE`]).
pub fn encode_request(buf: &mut [u8], request: Request) -> Result<usize, WireError> {
    if buf.len() < REQUEST_WIRE_SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let (op, arg) = match request {
        Request::Inject(event) => (OP_INJECT, event_tag(event)),
        Request::QueryState => (OP_QUERY_STATE, 0),
    };
    buf[0] = op;
    buf[1] = arg;
    Ok(REQUEST_WIRE_SIZE)
}

/// Decode a [`Request`] from the front of `bytes`.
pub fn decode_request(bytes: &[u8]) -> Result<Request, WireError> {
    if bytes.len() < REQUEST_WIRE_SIZE {
        return Err(WireError::Truncated);
    }
    match bytes[0] {
        OP_INJECT => event_from_tag(bytes[1], 0)
            .map(Request::Inject)
            .ok_or(WireError::Malformed),
        OP_QUERY_STATE => Ok(Request::QueryState),
        _ => Err(WireError::Malformed),
    }
}

/// Encode a [`Response`] into `buf`; returns bytes written
/// ([`RESPONSE_WIRE_SIZE`]).
pub fn encode_response(buf: &mut [u8], response: Response) -> Result<usize, WireError> {
    if buf.len() < RESPONSE_WIRE_SIZE {
        return Err(WireError::BufferTooSmall);
    }
    let (status, arg) = match response {
        Response::State(state) => (ST_STATE, state_tag(state)),
        Response::Rejected => (ST_REJECTED, 0),
    };
    buf[0] = status;
    buf[1] = arg;
    Ok(RESPONSE_WIRE_SIZE)
}

/// Decode a [`Response`] from the front of `bytes`.
pub fn decode_response(bytes: &[u8]) -> Result<Response, WireError> {
    if bytes.len() < RESPONSE_WIRE_SIZE {
        return Err(WireError::Truncated);
    }
    match bytes[0] {
        ST_STATE => state_from_tag(bytes[1])
            .map(Response::State)
            .ok_or(WireError::Malformed),
        ST_REJECTED => Ok(Response::Rejected),
        _ => Err(WireError::Malformed),
    }
}

/// Frozen wire tag for an [`Event`]. Exhaustive over current variants, so a new
/// one fails to compile until assigned a tag.
fn event_tag(event: Event) -> u8 {
    match event {
        Event::Start => 1,
        Event::InitDone => 2,
        Event::InitRotSecondaryBooted => 3,
        Event::VerifyUnprovisioned => 4,
        Event::VerifyFailed => 5,
        Event::VerifyDone => 6,
        Event::RecoveryFailed => 7,
        Event::RecoveryDone => 8,
        Event::UpdateRequested => 9,
        Event::UpdateDone(_) => 10,
        Event::UpdateFailed => 11,
        Event::ProvisionCmd => 12,
        Event::HandshakeFailed => 13,
        // No wildcard arm: within the defining crate the match is exhaustive, so
        // adding an `Event` variant is a compile error here until it is assigned
        // a frozen tag — the intended guard against an event with no encoding.
    }
}

/// Frozen payload byte for an [`Event`]. Zero for events that carry no data, so
/// the encoding of payloadless events is unchanged.
fn event_payload(event: Event) -> u8 {
    match event {
        Event::UpdateDone(UpdateScope::Platform) => 0,
        Event::UpdateDone(UpdateScope::RotActive) => 1,
        _ => 0,
    }
}

/// Decode the [`UpdateScope`] payload carried by [`Event::UpdateDone`].
fn update_scope_from_payload(payload: u8) -> Option<UpdateScope> {
    match payload {
        0 => Some(UpdateScope::Platform),
        1 => Some(UpdateScope::RotActive),
        _ => None,
    }
}

fn event_from_tag(tag: u8, payload: u8) -> Option<Event> {
    Some(match tag {
        1 => Event::Start,
        2 => Event::InitDone,
        3 => Event::InitRotSecondaryBooted,
        4 => Event::VerifyUnprovisioned,
        5 => Event::VerifyFailed,
        6 => Event::VerifyDone,
        7 => Event::RecoveryFailed,
        8 => Event::RecoveryDone,
        9 => Event::UpdateRequested,
        10 => Event::UpdateDone(update_scope_from_payload(payload)?),
        11 => Event::UpdateFailed,
        12 => Event::ProvisionCmd,
        13 => Event::HandshakeFailed,
        _ => return None,
    })
}

/// Frozen wire tag for a [`State`]. Exhaustive over current variants.
fn state_tag(state: State) -> u8 {
    match state {
        State::Boot => 1,
        State::Init => 2,
        State::RotRecovery => 3,
        State::FirmwareVerify => 4,
        State::FirmwareRecovery => 5,
        State::FirmwareUpdate => 6,
        State::Unprovisioned => 7,
        State::Runtime => 8,
        State::Lockdown => 9,
        State::Reboot => 10,
        // No wildcard arm: exhaustive in-crate, so a new `State` variant is a
        // compile error here until it is assigned a frozen tag.
    }
}

fn state_from_tag(tag: u8) -> Option<State> {
    Some(match tag {
        1 => State::Boot,
        2 => State::Init,
        3 => State::RotRecovery,
        4 => State::FirmwareVerify,
        5 => State::FirmwareRecovery,
        6 => State::FirmwareUpdate,
        7 => State::Unprovisioned,
        8 => State::Runtime,
        9 => State::Lockdown,
        10 => State::Reboot,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_EVENTS: &[Event] = &[
        Event::Start,
        Event::InitDone,
        Event::InitRotSecondaryBooted,
        Event::VerifyUnprovisioned,
        Event::VerifyFailed,
        Event::VerifyDone,
        Event::RecoveryFailed,
        Event::RecoveryDone,
        Event::UpdateRequested,
        Event::UpdateDone(UpdateScope::Platform),
        Event::UpdateFailed,
        Event::ProvisionCmd,
        Event::HandshakeFailed,
    ];

    const ALL_STATES: &[State] = &[
        State::Boot,
        State::Init,
        State::RotRecovery,
        State::FirmwareVerify,
        State::FirmwareRecovery,
        State::FirmwareUpdate,
        State::Unprovisioned,
        State::Runtime,
        State::Lockdown,
        State::Reboot,
    ];

    #[test]
    fn event_round_trips_every_variant() {
        for &event in ALL_EVENTS {
            let mut buf = [0u8; 8];
            let n = encode_event(&mut buf, event).unwrap();
            assert_eq!(n, EVENT_WIRE_SIZE);
            assert_eq!(decode_event(&buf[..n]), Ok(event), "round-trip {event:?}");
        }
    }

    #[test]
    fn update_done_payload_round_trips_on_event_channel() {
        for scope in [UpdateScope::Platform, UpdateScope::RotActive] {
            let mut buf = [0u8; EVENT_WIRE_SIZE];
            encode_event(&mut buf, Event::UpdateDone(scope)).unwrap();
            assert_eq!(decode_event(&buf), Ok(Event::UpdateDone(scope)));
        }
    }

    #[test]
    fn inject_request_round_trips_every_event() {
        for &event in ALL_EVENTS {
            let mut buf = [0u8; REQUEST_WIRE_SIZE];
            encode_request(&mut buf, Request::Inject(event)).unwrap();
            assert_eq!(decode_request(&buf), Ok(Request::Inject(event)));
        }
    }

    #[test]
    fn query_request_round_trips() {
        let mut buf = [0u8; REQUEST_WIRE_SIZE];
        encode_request(&mut buf, Request::QueryState).unwrap();
        assert_eq!(decode_request(&buf), Ok(Request::QueryState));
    }

    #[test]
    fn state_response_round_trips_every_state() {
        for &state in ALL_STATES {
            let mut buf = [0u8; RESPONSE_WIRE_SIZE];
            encode_response(&mut buf, Response::State(state)).unwrap();
            assert_eq!(decode_response(&buf), Ok(Response::State(state)));
        }
    }

    #[test]
    fn rejected_response_round_trips() {
        let mut buf = [0u8; RESPONSE_WIRE_SIZE];
        encode_response(&mut buf, Response::Rejected).unwrap();
        assert_eq!(decode_response(&buf), Ok(Response::Rejected));
    }

    #[test]
    fn tags_are_stable() {
        let mut buf = [0u8; EVENT_WIRE_SIZE];
        encode_event(&mut buf, Event::Start).unwrap();
        assert_eq!(buf, 1u32.to_le_bytes());
        encode_event(&mut buf, Event::HandshakeFailed).unwrap();
        assert_eq!(buf, 13u32.to_le_bytes());

        let mut rb = [0u8; REQUEST_WIRE_SIZE];
        encode_request(&mut rb, Request::Inject(Event::Start)).unwrap();
        assert_eq!(rb, [OP_INJECT, 1]);
    }

    #[test]
    fn decode_rejects_truncated_and_unknown() {
        assert_eq!(decode_event(&[1, 0, 0]), Err(WireError::Truncated));
        assert_eq!(decode_event(&999u32.to_le_bytes()), Err(WireError::Malformed));
        assert_eq!(decode_request(&[OP_INJECT]), Err(WireError::Truncated));
        assert_eq!(decode_request(&[0xFF, 0]), Err(WireError::Malformed));
        assert_eq!(decode_request(&[OP_INJECT, 0xFF]), Err(WireError::Malformed));
    }

    #[test]
    fn encode_rejects_small_buffer() {
        let mut buf = [0u8; EVENT_WIRE_SIZE - 1];
        assert_eq!(encode_event(&mut buf, Event::Start), Err(WireError::BufferTooSmall));
        let mut rb = [0u8; REQUEST_WIRE_SIZE - 1];
        assert_eq!(
            encode_request(&mut rb, Request::QueryState),
            Err(WireError::BufferTooSmall)
        );
    }
}
