// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! IPC abstraction over Pigweed kernel channels.
//!
//! `IpcInitiator` and `IpcHandler` split the two channel roles the kernel
//! itself distinguishes (`ChannelInitiatorObject` vs `ChannelHandlerObject`),
//! so a type only has to implement the operations its role can actually
//! serve. `AsyncTransaction` is a safe layer on top of `IpcInitiator`'s
//! unsafe async trio, tracking the one-transaction-per-channel invariant
//! locally instead of leaving it to the caller.

#![no_std]

use pw_status::Result;

/// Blocking and async operations available on the initiator side of a
/// channel (a `ChannelInitiatorObject` in the kernel).
pub trait IpcInitiator {
    fn transact<BufSend, BufRecv>(
        &self,
        send_data: &BufSend,
        recv_data: &mut BufRecv,
        deadline: Instant,
    ) -> Result<usize>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized;

    /// Starts a transaction and returns immediately; poll readiness via
    /// `object_wait`/`wait_group_add` on this channel's handle
    /// (`Signals::READABLE`), then call `async_transact_complete` or
    /// `async_cancel`. Fails with `Error::Unavailable` if a transaction
    /// (blocking or async) is already pending on this channel.
    ///
    /// # Safety
    /// The kernel holds raw pointers into `send_data`/`recv_data` (and
    /// writes the response into `recv_data`) until the transaction is
    /// completed or cancelled — callers must not read or write either
    /// buffer, or let them be dropped or moved, until then. `recv_data`
    /// must be large enough to hold the response; a response that
    /// overflows it is a kernel error, not truncated silently.
    unsafe fn async_transact_start<BufSend, BufRecv>(
        &self,
        send_data: &BufSend,
        recv_data: &mut BufRecv,
    ) -> Result<()>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized;

    fn async_transact_complete(&self) -> Result<usize>;

    fn async_cancel(&self) -> Result<()>;

    /// The raw channel handle, e.g. to register with a WaitGroup or pass to
    /// `object_wait`.
    fn as_raw(&self) -> u32;
}

/// Operations available on the handler side of a channel (a
/// `ChannelHandlerObject` in the kernel).
pub trait IpcHandler {
    fn read<Buf>(&self, offset: usize, buffer: &mut Buf) -> Result<usize>
    where
        Buf: AsSyscallBuffer + ?Sized;

    fn respond<Buf>(&self, buffer: &Buf) -> Result<()>
    where
        Buf: AsSyscallBuffer + ?Sized;
}

/// Transparent wrapper around a raw IPC handle.
///
/// `set_peer_user_signal` (defined in the target-only impl) is an inherent
/// method here rather than on `IpcInitiator`/`IpcHandler`, since it's usable
/// from either channel role and duplicating it onto both traits would just
/// mean two copies of the same forwarding call to keep in sync.
///
/// One transaction per channel is a kernel rule, not something this type
/// enforces: wrapping the same handle in two `AsyncTransaction`s just gets
/// you `Unavailable` on the second `start`, with the buffers handed back —
/// not anything unsound.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IpcHandle {
    pub handle: u32,
}

impl IpcHandle {
    /// Wraps a raw channel handle. Trusts the caller to pass one the kernel
    /// actually gave out.
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }
}

mod async_transaction;
mod channel_transport;
mod target;

pub use async_transaction::{AsyncTransaction, Buffers, Completion, RecvError, StartError};
pub use channel_transport::AsyncChannelTransport;
pub use target::{AsSyscallBuffer, Instant};
