// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The transport seam.
//!
//! Bytes in, bytes out, one round-trip, split into a start and a poll so
//! the caller never blocks. The orchestrator's client layer encodes a
//! request, starts it, and polls for the response from its event loop.
//! Swapping the transport is a wiring choice:
//!
//! - `IpcTransport` (in `orchestrator-pldm-client-ipc`): production path
//!   over a kernel channel, backed by `util/ipc`'s `AsyncTransaction`.
//!   That wrapper lends the kernel `'static` buffers, so the impl copies
//!   the request in and the response out of buffers it owns.
//! - `LoopbackTransport` (in `pldm-ipc-server`): calls dispatch directly
//!   in-process and has the response ready on the first poll.
//!   Host-buildable, so the same client encoders/decoders exercise the
//!   real dispatch with no kernel.

/// Why a transport round-trip failed. Small and transport-neutral;
/// PLDM-level status travels inside the response payload, not here.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The underlying channel or loopback call failed.
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
            Self::Failed => f.write_str("pldm ipc transport round-trip failed"),
            Self::WrongState => f.write_str("pldm ipc transport is in the wrong state"),
            Self::TooLarge => f.write_str("pldm ipc message does not fit the buffer"),
        }
    }
}

impl core::error::Error for TransportError {}

/// Bytes-in, bytes-out, exactly one round-trip at a time.
///
/// `start` takes one fully serialized `pldm_ipc_api` request and returns
/// immediately; the transport copies what it needs, so `req` is free
/// afterwards. `poll` returns `Ok(None)` while the response is still
/// outstanding and `Ok(Some(len))` once `resp[..len]` holds one fully
/// serialized reply. No fragmentation, no state between round-trips.
///
/// A transport carries one round-trip at a time: `start` while another
/// is in flight, or `poll`/`cancel` with none, is `WrongState`. Any
/// error from `poll` ends the round-trip, so the next call is `start`.
pub trait Transport {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError>;

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError>;

    /// Abandon the round-trip in flight. The response, if one arrives, is
    /// discarded.
    fn cancel(&mut self) -> Result<(), TransportError>;
}
