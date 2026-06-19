// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! zfmt-based logging client and server utilities.
//!
//! This crate provides `IpcLogger` (the client) and `LogServer`/`LogBuffer` (the server)
//! to enable logging over Pigweed Maize kernel IPC channels.

#![cfg_attr(not(test), no_std)]

use util_error::ErrorCode;
#[cfg(not(test))]
use util_ipc::IpcHandle;
use util_ipc::{Instant, IpcChannel};
use util_types::Opcode;
use zerocopy::IntoBytes;
use zfmt::Logger;

/// IPC opcode to write a log event.
pub const OPCODE_LOG_WRITE: Opcode = Opcode::new(*b"WLOG");
/// IPC opcode to read a log event.
pub const OPCODE_LOG_READ: Opcode = Opcode::new(*b"RLOG");
/// IPC opcode to clear the user notification signal.
pub const OPCODE_CLEAR_NOTIFIER: Opcode = Opcode::new(*b"CLRN");

pub mod buffer;
mod clock;
pub mod messages;
pub mod render;
pub mod server;

pub use buffer::LogBuffer;
pub use server::LogServer;
pub use zfmt::events::StreamStart;
pub use zfmt::FixedBuf;
pub use zfmt::Write;
pub use zfmt::ZfmtU64;

/// Since we're using pigweed's `multi_process_app`s, we can't have any
/// per-process `.data` or `.bss` sections.  It is required that all processes
/// use handle zero as their IPC channel to the logging server so we can
/// have a single const IpcLogger referring to channel zero.
///
/// TODO: investiage other options.
#[cfg(not(test))]
const UTIL_ZFMT_LOGGER: IpcLogger<IpcHandle> = IpcLogger::new(IpcHandle::new(0));

/// A client-side logger that sends events over an `IpcChannel`.
///
/// It implements the `zfmt::Logger` trait, allowing it to be used with
/// `zfmt` logging macros.
pub struct IpcLogger<IPC: IpcChannel> {
    ipc: IPC,
}

impl<IPC: IpcChannel> IpcLogger<IPC> {
    /// Creates a new `IpcLogger` wrapping the given IPC channel.
    pub const fn new(ipc: IPC) -> Self {
        Self { ipc }
    }

    /// Get the event at `cursor` from the logger.  Returns the cursor and event.
    /// Note: the returned cursor is the location from where the event was read.  The client must
    /// adjust their own cursor appropriately after processing the event payload.
    pub fn get_event<'a>(
        &self,
        cursor: u64,
        eventbuf: &'a mut [u8],
    ) -> Result<(u64, &'a [u8]), ErrorCode> {
        let mut status = 0u32;
        let mut log_cursor = 0u64;
        let mut n = self
            .ipc
            .transact(
                &[OPCODE_LOG_READ.as_bytes(), cursor.as_bytes()],
                &mut [status.as_mut_bytes(), log_cursor.as_mut_bytes(), eventbuf],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(status)?;
        n = n.saturating_sub(core::mem::size_of_val(&status) + core::mem::size_of_val(&log_cursor));
        let n = n.min(eventbuf.len());
        Ok((log_cursor, &eventbuf[..n]))
    }

    /// Clear the user notification from the logger.
    pub fn clear_notifier(&self) -> Result<(), ErrorCode> {
        self.ipc
            .transact(
                OPCODE_CLEAR_NOTIFIER.as_bytes(),
                &mut [0u8; 0],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        Ok(())
    }
}

impl<IPC: IpcChannel> Logger for IpcLogger<IPC> {
    fn timestamp(&self) -> ZfmtU64 {
        ZfmtU64::from_u64(0)
    }
    fn send_vectored(&self, bufs: &[&[u8]]) {
        // Note: zfmt will invoke send_vectored with 2 slices.
        // TODO: improve send_vectored to simply have the header and payload so we don't have to
        // reconstruct the iovec slice.
        const EMPTY: [u8; 0] = [];
        let cmd = [
            OPCODE_LOG_WRITE.as_bytes(),
            *bufs.first().unwrap_or(&EMPTY.as_slice()),
            *bufs.get(1).unwrap_or(&EMPTY.as_slice()),
            *bufs.get(2).unwrap_or(&EMPTY.as_slice()),
        ];
        let _ = self.ipc.transact(&cmd, &mut [0u8; 0], Instant::MAX);
    }
}

/// Returns a reference to the global `IpcLogger` instance.
///
/// This logger is configured to use channel handle 0, which is the convention
/// for processes in the multi-process application to communicate with the log server.
#[cfg(not(test))]
#[inline(always)]
pub fn logger() -> &'static IpcLogger<IpcHandle> {
    &UTIL_ZFMT_LOGGER
}

// Global logging macros (firmware only; use zfmt::log_info! directly in tests).
#[cfg(not(test))]
#[macro_export]
macro_rules! trace {
    ($event:expr) => {
        zfmt::log_trace!(*$crate::logger(), $event);
    };
}

#[cfg(not(test))]
#[macro_export]
macro_rules! debug {
    ($event:expr) => {
        zfmt::log_debug!(*$crate::logger(), $event);
    };
}

#[cfg(not(test))]
#[macro_export]
macro_rules! info {
    ($event:expr) => {
        zfmt::log_info!(*$crate::logger(), $event);
    };
}

#[cfg(not(test))]
#[macro_export]
macro_rules! warn {
    ($event:expr) => {
        zfmt::log_warn!(*$crate::logger(), $event);
    };
}

#[cfg(not(test))]
#[macro_export]
macro_rules! error {
    ($event:expr) => {
        zfmt::log_error!(*$crate::logger(), $event);
    };
}
