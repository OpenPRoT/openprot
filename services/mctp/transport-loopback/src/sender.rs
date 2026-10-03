// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The [`Sender`] the router sees. Every fragment the router hands it goes
//! to the caller's queue.

use mctp::{Error, Result, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;

use crate::queue::{LoopbackQueue, Packet, LOOPBACK_MTU, MCTP_HEADER_SIZE};

/// The [`Sender`] half. Holds no packets of its own; everything goes to the
/// queue the caller kept.
pub struct LoopbackSender<'q, const DEPTH: usize> {
    queue: &'q LoopbackQueue<DEPTH>,
}

impl<'q, const DEPTH: usize> LoopbackSender<'q, DEPTH> {
    /// Lends `queue` to the router.
    #[must_use]
    pub const fn new(queue: &'q LoopbackQueue<DEPTH>) -> Self {
        Self { queue }
    }
}

impl<const DEPTH: usize> Sender for LoopbackSender<'_, DEPTH> {
    fn send_vectored(&mut self, mut fragmenter: Fragmenter, payload: &[&[u8]]) -> Result<Tag> {
        loop {
            let mut buf = [0u8; LOOPBACK_MTU + MCTP_HEADER_SIZE];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    let mut packet = Packet::new();
                    if packet.extend_from_slice(p).is_err() {
                        // A fragment longer than the MTU we advertised.
                        return Err(Error::NoSpace);
                    }
                    self.queue.push(packet);
                }
                SendOutput::Complete { tag, .. } => break Ok(tag),
                SendOutput::Error { err, .. } => break Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        LOOPBACK_MTU
    }
}
