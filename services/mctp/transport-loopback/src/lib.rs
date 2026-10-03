// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! An MCTP transport that goes nowhere: packets come straight back in.
//!
//! On a board the server's [`Sender`] hands fragments to a wire. In a QEMU
//! test there is no wire and no second chip, so both endpoints live in one
//! image. This sender queues what its router fragments, and the caller
//! feeds that queue to the *other* endpoint's `Server::inbound`. One
//! server's outbox is the other's wire.
//!
//! The queue is owned by the caller rather than by the sender, because the
//! router takes the sender by value and offers no way back to it. The
//! caller keeps [`LoopbackQueue`] and lends it to [`LoopbackSender`]:
//!
//! ```ignore
//! let out_a = LoopbackQueue::<8>::new();
//! let out_b = LoopbackQueue::<8>::new();
//! let mut a = Server::new(Eid(8), now, LoopbackSender::new(&out_a));
//! let mut b = Server::new(Eid(42), now, LoopbackSender::new(&out_b));
//! // a sends; what a queued is what b receives.
//! while let Some(pkt) = out_a.take() {
//!     b.inbound(&pkt)?;
//! }
//! ```
//!
//! Two servers, not one looping to itself: a message addressed to a
//! server's own EID has no route out and back.
//!
//! Draining in the caller's loop rather than re-entering the router keeps
//! the send path non-blocking and the borrow straight, since the router
//! owns the sender and offers no way back to it.
//!
//! What this does not model: a wire that drops, reorders, delays or
//! corrupts. Every packet arrives, in order, as soon as the caller drains.
//! A test that wants retry or timeout behaviour needs something else.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

use core::cell::RefCell;

use heapless::Deque;
use mctp::{Error, Result, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;

/// Largest payload this transport carries in one fragment. Nothing here is
/// limited by a bus, so the number only has to be agreed by both ends,
/// which it is by construction.
pub const LOOPBACK_MTU: usize = 255;

/// MCTP transport header, which rides in front of the payload. The
/// fragment buffer has to hold both: a buffer of exactly the MTU makes the
/// router reject the send rather than truncate it.
const MCTP_HEADER_SIZE: usize = 4;

/// One fragmented packet waiting to be fed back in, header included.
pub type Packet = heapless::Vec<u8, { LOOPBACK_MTU + MCTP_HEADER_SIZE }>;

/// Packets on their way from the router back to it.
///
/// `DEPTH` bounds how many fragments may be in flight between drains. A
/// caller that drains after every send needs one; one that sends a whole
/// message first needs as many fragments as that message takes.
pub struct LoopbackQueue<const DEPTH: usize> {
    inner: RefCell<Inner<DEPTH>>,
}

struct Inner<const DEPTH: usize> {
    packets: Deque<Packet, DEPTH>,
    dropped: u32,
}

impl<const DEPTH: usize> Default for LoopbackQueue<DEPTH> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const DEPTH: usize> LoopbackQueue<DEPTH> {
    /// An empty queue.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(Inner {
                packets: Deque::new(),
                dropped: 0,
            }),
        }
    }

    /// Takes the oldest queued packet, or `None` when none are waiting.
    /// Feed what this returns to `Server::inbound`.
    pub fn take(&self) -> Option<Packet> {
        self.inner.borrow_mut().packets.pop_front()
    }

    /// How many packets are waiting.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.inner.borrow().packets.len()
    }

    /// How many packets were discarded because the queue was full.
    ///
    /// A wire drops packets silently and MCTP is built to cope, so a full
    /// queue is not an error here. It is still worth reporting, because in
    /// this transport it means `DEPTH` is too small rather than that the
    /// medium is lossy, and the test that follows will fail for a reason
    /// that looks unrelated.
    #[must_use]
    pub fn dropped(&self) -> u32 {
        self.inner.borrow().dropped
    }
}

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
                    let mut inner = self.queue.inner.borrow_mut();
                    if inner.packets.push_back(packet).is_err() {
                        inner.dropped = inner.dropped.saturating_add(1);
                    }
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

#[cfg(test)]
mod tests;
