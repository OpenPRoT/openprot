// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Production IPC transport for `I3cClient`.
//!
//! The **only** IPC-coupled, kernel-tagged piece of the client path. It
//! implements `i3c_api::Transport` over a Pigweed channel; all wire
//! marshalling stays in the host-buildable `i3c_client`. Wiring:
//!
//! ```rust,ignore
//! use i3c_client::I3cClient;
//! use i3c_client_ipc::IpcTransport;
//! let mut i3c = I3cClient::new(IpcTransport::new(handle::CHANNEL));
//! ```
//!
//! Swapping this for `i3c_server::LoopbackTransport` (host) exercises the same
//! `I3cClient` code with no kernel — that is the point of the seam.
//!
//! The server raises `Signals::USER` from its interrupt path when a frame is
//! latched, so a client that wants to block until one arrives parks in
//! `object_wait(handle, USER)` and then calls `recv`. That wait is deliberately
//! left to the caller: it is a scheduling choice, not part of the transport.

#![no_std]
#![deny(missing_docs)]

use i3c_api::{Transport, TransportError};
use userspace::syscall;
use userspace::time::Instant;

/// Cross-process transport: one `channel_transact` per whole request.
pub struct IpcTransport {
    handle: u32,
}

impl IpcTransport {
    /// Bind to the IPC channel for one i3c target (handle from the app's
    /// generated `handle` module).
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }

    /// The channel handle, for callers that need to `object_wait` on it.
    pub const fn handle(&self) -> u32 {
        self.handle
    }
}

impl Transport for IpcTransport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        syscall::channel_transact(self.handle, req, resp, Instant::MAX)
            .map_err(|_| TransportError::Failed)
    }
}
