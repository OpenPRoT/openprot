// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The transport seam.
//!
//! One whole serialized request goes out, one whole response comes back —
//! *bytes in → bytes out, one shot, for every impl*. The i3c protocol is one
//! operation per frame, so this signature is the natural shape: a transport
//! physically cannot express a partial operation or hold state between calls.
//!
//! `I3cClient` (in `i3c-client`) is generic over this trait and contains
//! **all** the wire marshalling. Swapping the transport is a wiring choice,
//! never a code fork:
//!
//! - `IpcTransport` (in `i3c-client-ipc`) — production cross-process path
//!   (Pigweed `channel_transact`); the only IPC-coupled, kernel-tagged piece.
//! - `LoopbackTransport` (in `i3c-server`) — calls the server `dispatch`
//!   directly against an in-process `I3cTarget`. Host-buildable, so the
//!   *same* client encoders/decoders are exercised with no kernel.

/// Why a transport round-trip failed. Deliberately tiny and transport-neutral;
/// i3c-level status travels inside the response payload as an
/// [`I3cStatus`](crate::I3cStatus), not here.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The underlying channel/syscall/loopback call failed.
    Failed,
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Failed => f.write_str("i3c transport round-trip failed"),
        }
    }
}

impl core::error::Error for TransportError {}

/// Bytes-in → bytes-out, exactly one round-trip.
///
/// `transact` writes the response into `resp` and returns its length. The
/// request is one fully serialized `i3c_api` frame; the response is one fully
/// serialized reply. No fragmentation, no state between calls.
pub trait Transport {
    /// Perform one round-trip, returning the response length written to `resp`.
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError>;
}
