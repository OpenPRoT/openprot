// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The packet queue the caller keeps and the sender writes to.

use core::cell::RefCell;

use heapless::Deque;

/// Largest payload this transport carries in one fragment. Nothing here is
/// limited by a bus, so the number only has to be agreed by both ends,
/// which it is by construction.
pub const LOOPBACK_MTU: usize = 255;

/// MCTP transport header, which rides in front of the payload. The
/// fragment buffer has to hold both: a buffer of exactly the MTU makes the
/// router reject the send rather than truncate it.
pub(crate) const MCTP_HEADER_SIZE: usize = 4;

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

    /// Queues one packet, counting a drop instead when the queue is full.
    /// The sender's only way in, which is why `inner` stays private.
    pub(crate) fn push(&self, packet: Packet) {
        let mut inner = self.inner.borrow_mut();
        if inner.packets.push_back(packet).is_err() {
            inner.dropped = inner.dropped.saturating_add(1);
        }
    }
}
