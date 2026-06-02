// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Production IPC transport for `I2cClient`.
//!
//! The **only** IPC-coupled, kernel-tagged piece of the client path. It
//! implements `i2c_api::Transport` over a Pigweed channel; all wire
//! marshalling stays in the host-buildable `i2c_client`. Wiring:
//!
//! ```rust,ignore
//! use i2c_client::I2cClient;
//! use i2c_client_ipc::IpcTransport;
//! let mut bus = I2cClient::new(IpcTransport::new(handle::I2C_0));
//! ```
//!
//! Swapping this for `i2c_server::LoopbackTransport` (host) exercises the same
//! `I2cClient` code with no kernel — that is the point of the seam.

#![no_std]

use i2c_api::{Transport, TransportError};
use userspace::syscall;
use userspace::time::Instant;

/// Cross-process transport: one `channel_transact` per whole transaction.
pub struct IpcTransport {
    handle: u32,
}

impl IpcTransport {
    /// Bind to the IPC channel for one bus (handle from the app's generated
    /// `handle` module — one channel per bus).
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }
}

impl Transport for IpcTransport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        syscall::channel_transact(self.handle, req, resp, Instant::MAX)
            .map_err(|_| TransportError::Failed)
    }
}
