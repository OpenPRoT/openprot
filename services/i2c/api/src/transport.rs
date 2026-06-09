// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The transport seam.
//!
//! One whole serialized transaction goes out, one whole response comes back —
//! *bytes in → bytes out, one shot, for every impl*. This signature is what
//! makes the whole-object / run-to-completion invariant structural rather than
//! conventional: a transport physically cannot express a per-operation
//! round-trip or a held bus lock.
//!
//! The `I2cClient` is generic over this trait and contains **all** the wire
//! marshalling. Swapping the transport is a wiring choice, never a code fork:
//!
//! - `IpcTransport` (in `i2c-client-ipc`) — production cross-process path
//!   (Pigweed `channel_transact`); the only IPC-coupled, kernel-tagged piece.
//! - `LoopbackTransport` (in `i2c-server`) — calls the server `dispatch`
//!   directly against an in-process `embedded_hal::i2c::I2c`. Host-buildable,
//!   so the *same* client encoders/decoders are exercised with no kernel; also
//!   the correct early-boot path before IPC exists.

/// Why a transport round-trip failed. Deliberately tiny and transport-neutral;
/// i2c-level status travels inside the response payload, not here.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The underlying channel/syscall/loopback call failed.
    Failed,
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Failed => f.write_str("i2c transport round-trip failed"),
        }
    }
}

impl core::error::Error for TransportError {}

/// Bytes-in → bytes-out, exactly one round-trip.
///
/// `transact` writes the response into `resp` and returns its length. The
/// request is one fully serialized `i2c_api` transaction; the response is one
/// fully serialized reply. No fragmentation, no state between calls.
pub trait Transport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError>;
}
