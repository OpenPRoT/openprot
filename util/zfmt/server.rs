// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::buffer::LogBuffer;
use crate::clock;
use crate::{OPCODE_CLEAR_NOTIFIER, OPCODE_LOG_READ, OPCODE_LOG_WRITE};
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::Opcode;
use zerocopy::{FromBytes, IntoBytes};
use zfmt::events::EventHeader;

/// A generic logging server that processes IPC requests to read/write logs.
///
/// It wraps a `LogBuffer` and manages a monotonically increasing sequence number
/// for events. It injects timestamps and sequence numbers into incoming events.
pub struct LogServer<const N: usize> {
    /// The underlying log buffer.
    pub buffer: LogBuffer<N>,
    /// The current event sequence number.
    pub sequence: u32,
}

/// Injects a 64-bit timestamp and a 24-bit sequence number into a serialized
/// `EventHeader` frame. Does nothing if the frame is not an `EventHeader`.
fn inject_timestamp(event: &mut [u8], timestamp: u64, sequence: u32) {
    if let Ok((&mut EventHeader::ZFMT_TAG, buf)) = u32::mut_from_prefix(event) {
        if let Some(ts) = buf.get_mut(1..9) {
            // 64-bit timestamp.
            ts.copy_from_slice(&timestamp.to_le_bytes());
        }
        if let Some(seq) = buf.get_mut(10..13) {
            // 24-bit sequence number.
            seq.copy_from_slice(&sequence.to_le_bytes()[..3]);
        }
    }
}

impl<const N: usize> Default for LogServer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> LogServer<N> {
    /// Creates a new `LogServer`.
    pub const fn new() -> Self {
        Self {
            buffer: LogBuffer::new(),
            sequence: 0,
        }
    }

    /// Handles a single IPC request from a channel.
    ///
    /// Supported opcodes:
    /// - `OPCODE_LOG_WRITE`: Pushes a log event into the buffer, injecting the current
    ///   timestamp and sequence number. Returns `Ok(true)` to signal that consumers
    ///   should be notified.
    /// - `OPCODE_LOG_READ`: Reads a log event starting at the requested cursor.
    /// - `OPCODE_CLEAR_NOTIFIER`: Responds with success (used to acknowledge notifications).
    ///
    /// Returns `Ok(true)` if a new log was written (consumers should be notified).
    /// Returns `Ok(false)` for other successful operations.
    /// Returns `Err(ErrorCode)` on failure.
    pub fn handle_request(
        &mut self,
        channel: &impl IpcChannel,
        req: &mut [u8],
    ) -> Result<bool, ErrorCode> {
        let (op, payload) =
            Opcode::mut_from_prefix(req).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let status = 0u32;
        match *op {
            OPCODE_CLEAR_NOTIFIER => {
                channel
                    .respond(&[0u8; 0])
                    .map_err(ErrorCode::kernel_error)?;
                Ok(false)
            }
            OPCODE_LOG_WRITE => {
                inject_timestamp(payload, clock::now_ticks(), self.sequence);
                self.sequence += 1;
                self.buffer.push_frame(payload);
                channel
                    .respond(&[0u8; 0])
                    .map_err(ErrorCode::kernel_error)?;
                Ok(true)
            }
            OPCODE_LOG_READ => {
                let (cursor, _) =
                    u64::read_from_prefix(payload).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
                let cursor = if cursor < self.buffer.read {
                    self.buffer.read
                } else {
                    cursor
                };
                if let Some((_tag, s1, s2)) = self.buffer.next_frame_slice(cursor) {
                    channel
                        .respond(&[status.as_bytes(), cursor.as_bytes(), s1, s2])
                        .map_err(ErrorCode::kernel_error)?;
                    Ok(false)
                } else {
                    // TODO: log is corrupted? Choose a better error code.
                    Err(error::IPC_ERROR_BAD_REQ)
                }
            }
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }
}
