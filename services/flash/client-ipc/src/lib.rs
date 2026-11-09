// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash client over a kernel IPC channel.
//!
//! The client is `openprot_flash_client::FlashClient`. This crate supplies the
//! transport that carries its requests over a channel.

#![no_std]

use openprot_flash_client::{FlashClient, FlashTransport};
use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::{IpcHandle, IpcInitiator};

/// A [`FlashTransport`] over one kernel channel.
pub struct ChannelTransport(IpcHandle);

impl ChannelTransport {
    pub const fn new(ipc: IpcHandle) -> Self {
        Self(ipc)
    }
}

impl FlashTransport for ChannelTransport {
    fn transact(&mut self, send: &[&[u8]], recv: &mut [&mut [u8]]) -> Result<usize, ErrorCode> {
        self.0
            .transact(send, recv, Instant::MAX)
            .map_err(ErrorCode::kernel_error)
    }
}

/// The flash client that kernel apps use.
pub type FlashIpcClient = FlashClient<ChannelTransport>;
