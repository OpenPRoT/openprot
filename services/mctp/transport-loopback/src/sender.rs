// Licensed under the Apache-2.0 license

//! Loopback MCTP sender — in-memory transport binding.
//!
//! Provides a shared buffer mechanism for direct MCTP server-to-server
//! communication without physical transport encoding.

use core::cell::RefCell;
use heapless::Vec as HVec;
use mctp::Result;
use mctp_lib::fragment::{Fragmenter, SendOutput};

/// Maximum packet size (MCTP baseline MTU).
const MAX_PACKET_SIZE: usize = 255;

/// Maximum number of packets in flight per direction.
const MAX_QUEUE_DEPTH: usize = 16;

/// A packet queue for one direction of loopback communication.
type PacketQueue = HVec<HVec<u8, MAX_PACKET_SIZE>, MAX_QUEUE_DEPTH>;

/// Loopback MCTP sender.
///
/// Implements `mctp_lib::Sender` to fragment and send MCTP packets
/// into a shared in-memory buffer. Each sender writes to a queue
/// that the peer endpoint reads from.
///
/// Unlike I2C transport, this works with raw MCTP packets without
/// any transport-layer encoding.
///
/// This struct uses interior mutability (RefCell) so it can be used
/// with the `Sender` trait which requires `&mut self`.
pub struct LoopbackSender<'a> {
    /// Shared packet queue (packets going to the peer).
    queue: &'a RefCell<PacketQueue>,
}

impl<'a> LoopbackSender<'a> {
    /// Create a new loopback sender writing to the given queue.
    pub fn new(queue: &'a RefCell<PacketQueue>) -> Self {
        Self { queue }
    }
}

impl mctp_lib::Sender for LoopbackSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        loop {
            let mut pkt = [0u8; MAX_PACKET_SIZE];
            match fragmenter.fragment_vectored(payload, &mut pkt) {
                SendOutput::Packet(p) => {
                    // Push raw MCTP packet to queue
                    self.queue
                        .borrow_mut()
                        .push(HVec::from_slice(p).map_err(|_| mctp::Error::TxFailure)?)
                        .map_err(|_| mctp::Error::TxFailure)?;
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MAX_PACKET_SIZE
    }
}

/// Bidirectional loopback packet manager.
///
/// Manages two unidirectional packet queues for communication between
/// two MCTP endpoints (A and B). This is the shared state that both
/// senders write to and the application polls from.
///
/// # Example
///
/// ```ignore
/// use std::cell::RefCell;
/// use openprot_mctp_transport_loopback::{LoopbackPair, LoopbackSender};
///
/// let a_to_b = RefCell::new(Vec::new());
/// let b_to_a = RefCell::new(Vec::new());
///
/// let sender_a = LoopbackSender::new(&a_to_b);
/// let sender_b = LoopbackSender::new(&b_to_a);
///
/// let mut server_a = Server::new(Eid(8), 0, sender_a);
/// let mut server_b = Server::new(Eid(42), 0, sender_b);
///
/// // A sends to B
/// server_a.send(...);
/// while !a_to_b.borrow().is_empty() {
///     let pkt = a_to_b.borrow_mut().swap_remove(0);
///     server_b.inbound(&pkt).unwrap();
/// }
/// ```
///
/// For convenience, `LoopbackPair` provides a higher-level API that
/// manages both queues together.
pub struct LoopbackPair {
    /// Packets from A to B.
    pub a_to_b: RefCell<PacketQueue>,
    /// Packets from B to A.
    pub b_to_a: RefCell<PacketQueue>,
}

impl LoopbackPair {
    /// Create a new loopback pair.
    pub fn new() -> Self {
        Self {
            a_to_b: RefCell::new(HVec::new()),
            b_to_a: RefCell::new(HVec::new()),
        }
    }

    /// Pop the next packet from A's send queue (destined for B).
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop_a_to_b(&self) -> Option<HVec<u8, MAX_PACKET_SIZE>> {
        let mut queue = self.a_to_b.borrow_mut();
        if queue.is_empty() {
            None
        } else {
            Some(queue.swap_remove(0))
        }
    }

    /// Pop the next packet from B's send queue (destined for A).
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop_b_to_a(&self) -> Option<HVec<u8, MAX_PACKET_SIZE>> {
        let mut queue = self.b_to_a.borrow_mut();
        if queue.is_empty() {
            None
        } else {
            Some(queue.swap_remove(0))
        }
    }

    /// Clear all packets from A→B queue.
    pub fn clear_a_to_b(&self) {
        self.a_to_b.borrow_mut().clear();
    }

    /// Clear all packets from B→A queue.
    pub fn clear_b_to_a(&self) {
        self.b_to_a.borrow_mut().clear();
    }

    /// Get the number of packets pending A→B.
    pub fn len_a_to_b(&self) -> usize {
        self.a_to_b.borrow().len()
    }

    /// Get the number of packets pending B→A.
    pub fn len_b_to_a(&self) -> usize {
        self.b_to_a.borrow().len()
    }

    /// Check if A→B queue is empty.
    pub fn is_empty_a_to_b(&self) -> bool {
        self.a_to_b.borrow().is_empty()
    }

    /// Check if B→A queue is empty.
    pub fn is_empty_b_to_a(&self) -> bool {
        self.b_to_a.borrow().is_empty()
    }
}

impl Default for LoopbackPair {
    fn default() -> Self {
        Self::new()
    }
}
