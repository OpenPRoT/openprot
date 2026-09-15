// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use pw_status::Result;

/// Trait wrapping basic IPC operations on a channel.
pub trait IpcChannel {
    fn transact<BufSend, BufRecv>(
        &self,
        send_data: &BufSend,
        recv_data: &mut BufRecv,
        deadline: Instant,
    ) -> Result<usize>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized;

    fn read<Buf>(&self, offset: usize, buffer: &mut Buf) -> Result<usize>
    where
        Buf: AsSyscallBuffer + ?Sized;

    fn respond<Buf>(&self, buffer: &Buf) -> Result<()>
    where
        Buf: AsSyscallBuffer + ?Sized;

    /// Set (set=true) or clear (set=false) Signals::USER on the paired peer.
    fn set_peer_user_signal(&self, set: bool) -> Result<()>;
}

/// Transparent wrapper around a raw IPC handle.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IpcHandle {
    pub handle: u32,
}

impl IpcHandle {
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }
}

#[cfg(target_os = "none")]
mod target;
#[cfg(target_os = "none")]
pub use target::{AsSyscallBuffer, Instant};

#[cfg(not(target_os = "none"))]
mod host;
#[cfg(not(target_os = "none"))]
pub use host::{AsSyscallBuffer, Instant};
