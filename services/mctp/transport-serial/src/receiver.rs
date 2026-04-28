// Licensed under the Apache-2.0 license

//! Serial MCTP receiver — inbound transport binding.
//!
//! Accumulates raw bytes from a serial port and decodes complete MCTP
//! packets using `mctp-lib` serial framing.

use heapless::Vec;
use mctp_lib::serial::MTU_MAX;

/// Maximum receive buffer size (one MTU-sized serial frame).
const RX_BUF_SIZE: usize = MTU_MAX + 8;

/// Serial MCTP receiver.
///
/// Feed individual bytes from the UART into [`SerialReceiver::feed`].
/// When a complete MCTP packet has been assembled it is returned as a
/// `&[u8]` slice for passing to `Server::inbound`.
pub struct SerialReceiver {
    buf: Vec<u8, RX_BUF_SIZE>,
}

impl SerialReceiver {
    /// Create a new serial receiver with an empty accumulation buffer.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
        }
    }

    /// Feed one byte into the receiver.
    ///
    /// Returns `Some(packet_bytes)` when a complete packet has been
    /// received, or `None` while still accumulating.
    pub fn feed(&mut self, byte: u8) -> Option<&[u8]> {
        // TODO: decode serial framing (SMBUS/MCTP byte-count or HDLC-like
        //       framing depending on mctp-lib serial API).
        if self.buf.push(byte).is_err() {
            // Buffer overflow — reset and start fresh.
            self.buf.clear();
        }
        None
    }

    /// Reset the accumulation buffer (e.g. on framing error).
    pub fn reset(&mut self) {
        self.buf.clear();
    }
}

impl Default for SerialReceiver {
    fn default() -> Self {
        Self::new()
    }
}
