// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! In-process transport: `dispatch` with no kernel.
//!
//! Wraps a [`Server`] so the host-buildable `I3cClient` can drive the real
//! request path against a mock [`I3cTarget`]. The same client encoders and the
//! same `dispatch` run here as in production — only the round-trip is a
//! function call instead of a channel.
//!
//! This is also the correct early-boot path, before IPC exists.

use i3c_api::{Transport, TransportError};
use openprot_hal_blocking::i3c_hardware::I3cTarget;

use crate::{dispatch, Server};

/// A [`Transport`] that calls [`dispatch`] directly against an owned [`Server`].
pub struct LoopbackTransport<T> {
    server: Server<T>,
}

impl<T> LoopbackTransport<T> {
    /// Wrap a server, taking ownership of it and its target.
    pub const fn new(server: Server<T>) -> Self {
        Self { server }
    }

    /// Borrow the wrapped server — to latch an inbound frame, or to inspect
    /// what the mock target recorded.
    pub fn server(&mut self) -> &mut Server<T> {
        &mut self.server
    }

    /// Unwrap back to the server.
    pub fn into_server(self) -> Server<T> {
        self.server
    }
}

impl<T: I3cTarget> Transport for LoopbackTransport<T> {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        // `dispatch` always produces a response frame; a failure to encode one
        // surfaces as an `I3cStatus` inside it, never as a transport failure.
        Ok(dispatch(&mut self.server, req, resp))
    }
}
