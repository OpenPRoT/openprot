// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The seams every IPC service is built from.
//!
//! A service splits into wire marshalling (host-buildable, in the service's
//! own `api` crate), a server that turns one request frame into one response
//! frame, and a transport that carries frames between the two. This crate
//! holds the three traits that seam sits on, so a service defines its wire
//! format and nothing else.
//!
//! A transport comes in two shapes and a type implements whichever it can
//! serve. `Transport` blocks until the response arrives, which is what a
//! dedicated server thread or an early-boot in-process path wants.
//! `AsyncTransport` starts a round-trip and returns, so a caller running in
//! an event loop never blocks. Neither is the fallback for the other: a
//! blocking transport has no way to poll, and an event loop cannot wait.
//!
//! `Dispatch` is the server end. One request frame in, one response frame
//! out, no state between calls. The same impl backs the production channel
//! and the in-process loopback, so host tests exercise the real server.

#![no_std]

/// Why a transport round-trip failed. Small and service-neutral;
/// service-level status travels inside the response payload, not here.
///
/// `WrongState` belongs to `AsyncTransport`: a blocking `transact` has no
/// state between calls to get wrong. The other two apply to both.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The underlying channel, syscall, or loopback call failed.
    Failed,
    /// `start` was called while a round-trip was still in flight, or
    /// `poll`/`cancel` was called with nothing in flight.
    WrongState,
    /// The request does not fit the transport's request buffer, or the
    /// response does not fit the caller's.
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

/// Bytes in, bytes out, one round-trip, caller waits for the response.
///
/// `transact` writes the response into `resp` and returns its length. The
/// request is one fully serialized frame and so is the response. No
/// fragmentation, no state between calls.
///
/// Implement this when the caller can afford to wait: a thread dedicated to
/// one service, or an in-process path before IPC exists. A caller inside an
/// event loop wants `AsyncTransport` instead.
pub trait Transport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError>;
}

/// Bytes in, bytes out, one round-trip at a time, split so the caller never
/// blocks.
///
/// `start` takes one fully serialized request and returns immediately; the
/// transport copies what it needs, so `req` is free afterwards. `poll`
/// returns `Ok(None)` while the response is still outstanding and
/// `Ok(Some(len))` once `resp[..len]` holds one fully serialized reply.
/// `len` is never 0: a server that cannot produce a frame at all is a
/// failed round-trip, not an empty reply.
///
/// `poll` never waits: with no response ready it returns `Ok(None)` and
/// returns. The caller is expected to be signal-driven rather than
/// spinning, parking its event loop until the response to this round-trip
/// arrives and polling once when it does. Registering for that needs the
/// concrete transport (a kernel one hands out its channel handle, a
/// loopback has none), so it happens at wiring time, not through this
/// trait.
///
/// A server that raises a signal to say it has news, with no round-trip
/// outstanding, is not this trait's concern. The caller learns what the
/// news is by starting a round-trip and asking.
///
/// A transport carries one round-trip at a time: `start` while another is in
/// flight, or `poll`/`cancel` with none, is `WrongState`. Any error from
/// `poll` ends the round-trip, so the next call is `start`.
pub trait AsyncTransport {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError>;

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError>;

    /// Abandon the round-trip in flight. The response, if one arrives, is
    /// discarded.
    fn cancel(&mut self) -> Result<(), TransportError>;
}

/// Why a dispatch produced no response frame at all.
///
/// A service encodes its own errors into the response frame, so a failed
/// operation is still a frame and still `Ok`. This is the one case where
/// there is nothing to send back.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// `response` is too small to hold even an error frame.
    ResponseTooSmall,
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ResponseTooSmall => f.write_str("response buffer too small for a reply"),
        }
    }
}

impl core::error::Error for DispatchError {}

/// The server end: one request frame in, one response frame out.
///
/// Returns the number of bytes written to `response`, always at least one.
/// A loopback transport writes into the caller's buffer, so it reports a
/// `DispatchError` as `TransportError::TooLarge`.
///
/// Implementations hold no state between calls beyond whatever the service
/// itself owns, so the same impl serves the production channel and the
/// in-process loopback.
pub trait Dispatch {
    fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, DispatchError>;
}
