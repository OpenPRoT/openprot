// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use embedded_io::Write;
use mctp::Result;
use mctp_lib::{Sender, fragment::SendOutput, serial::MctpSerialHandler};

/// MCTP serial sender that wraps any `embedded_io::Write` implementation.
pub struct SerialSender<W: Write> {
    writer: W,
    serial_handler: MctpSerialHandler,
}

impl<W: Write> Sender for SerialSender<W> {
    fn send_vectored(
        &mut self,
        mut fragmenter: mctp_lib::fragment::Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        loop {
            let mut pkt = [0u8; mctp_lib::serial::MTU_MAX];
            let r = fragmenter.fragment_vectored(payload, &mut pkt);

            match r {
                SendOutput::Packet(p) => {
                    self.serial_handler.send_sync(p, &mut self.writer)?;
                    self.writer.flush().map_err(|_| mctp::Error::TxFailure)?;
                }
                SendOutput::Complete { tag, .. } => {
                    break Ok(tag);
                }
                SendOutput::Error { err, .. } => {
                    break Err(err);
                }
            }
        }
    }

    fn get_mtu(&self) -> usize {
        mctp_lib::serial::MTU_MAX
    }
}

impl<W: Write> SerialSender<W> {
    /// Create a new SerialSender instance backed by any embedded-io writer.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            serial_handler: MctpSerialHandler::new(),
        }
    }

    /// Access the wrapped writer for low-level setup or diagnostics.
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}
