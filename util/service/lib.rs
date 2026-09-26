// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Transport and dispatch traits for IPC services.
//!
//! Three traits: `Transport` (blocking), `AsyncTransport` (split-phase),
//! and `Dispatch` (server). A service defines its wire format in its own
//! crate and plugs into these.

#![no_std]

mod delayed;
mod loopback;

pub use delayed::Delayed;
pub use loopback::Loopback;

/// Why a transport round-trip failed.
///
/// Service-level errors travel inside the response payload, not here.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The channel, syscall, or loopback call failed.
    Failed,
    /// `start` while in flight, or `poll`/`cancel` with nothing in flight.
    WrongState,
    /// Request or response does not fit its buffer.
    TooLarge,
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Failed => f.write_str("transport round-trip failed"),
            Self::WrongState => f.write_str("transport is in the wrong state"),
            Self::TooLarge => f.write_str("message does not fit the buffer"),
        }
    }
}

impl core::error::Error for TransportError {}

/// One round-trip, caller waits for the response.
pub trait Transport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError>;
}

/// One round-trip at a time, split so the caller never blocks.
///
/// `start` copies the request and returns immediately. `poll` returns
/// `Ok(None)` while the response is outstanding, `Ok(Some(len))` when
/// `resp[..len]` holds the reply. `poll` never waits.
///
/// One round-trip at a time: `start` while in flight or `poll`/`cancel`
/// with nothing in flight is `WrongState`. Any error from `poll` ends
/// the round-trip and discards the response: a later `poll` cannot
/// retrieve it, and the next call is `start`.
///
/// A request that is too large for the transport is caught at different
/// points depending on the implementation. A channel transport checks at
/// `start` and returns `TooLarge` before anything goes out. A loopback
/// has no send buffer to overflow, so it hands the request straight to
/// the server, which answers with a protocol error frame at `poll`.
/// Callers need to handle both cases. A response too large for the
/// caller's buffer is always `TooLarge` from `poll`.
///
/// Registering for wake-up signals needs the concrete transport (a
/// kernel channel has a handle, a loopback does not), so that happens
/// at wiring time, not through this trait.
pub trait AsyncTransport {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError>;

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError>;

    /// Abandon the in-flight round-trip.
    fn cancel(&mut self) -> Result<(), TransportError>;
}

/// Why a dispatch produced no response frame at all.
///
/// Service errors go in the response frame (still `Ok`). This covers
/// the case where even an error frame does not fit.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The response does not fit the buffer the caller gave, not even
    /// as an error frame.
    ResponseTooLarge,
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ResponseTooLarge => f.write_str("response does not fit the buffer"),
        }
    }
}

impl core::error::Error for DispatchError {}

impl From<DispatchError> for TransportError {
    fn from(e: DispatchError) -> Self {
        match e {
            DispatchError::ResponseTooLarge => Self::TooLarge,
        }
    }
}

/// Server end: one request frame in, one response frame out.
///
/// Returns bytes written to `response`. No state between calls beyond
/// what the service itself owns.
pub trait Dispatch {
    fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, DispatchError>;
}
