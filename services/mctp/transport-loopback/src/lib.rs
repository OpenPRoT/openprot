// Licensed under the Apache-2.0 license

//! In-memory loopback transport for MCTP testing (no_std, no-alloc).
//!
//! This module provides a formalized loopback transport that enables two MCTP
//! endpoints to communicate entirely in-memory without any physical transport.
//! It uses fixed-size buffers to mirror the memory behavior of transport-i2c,
//! avoiding any dynamic allocation.
//!
//! # Example
//!
//! ```
//! use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
//! use openprot_mctp_api::MctpClient;
//! use core::cell::RefCell;
//!
//! let packets_a = RefCell::new(PacketBuffer::new());
//! let packets_b = RefCell::new(PacketBuffer::new());
//! let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);
//!
//! // A sends to B
//! let handle_a = pair.client_a().req(42).unwrap();
//! pair.client_a().send(Some(handle_a), 1, None, None, false, b"hello").unwrap();
//! pair.transfer_a_to_b();
//!
//! // B receives
//! let handle_b = pair.client_b().listener(1).unwrap();
//! let mut buf = [0u8; 256];
//! let meta = pair.client_b().recv(handle_b, 0, &mut buf).unwrap();
//! assert_eq!(&buf[..meta.payload_size], b"hello");
//! ```

#![no_std]
#![warn(missing_docs)]

use core::cell::RefCell;

use mctp::Eid;
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata};
use openprot_mctp_server::Server;

/// Maximum packet size (matches MCTP MTU)
pub const MAX_PACKET_SIZE: usize = 255;

/// Maximum number of buffered packets per endpoint
pub const MAX_BUFFERED_PACKETS: usize = 16;

// ---------------------------------------------------------------------------
// PacketBuffer - Fixed-size packet storage
// ---------------------------------------------------------------------------

/// Fixed-size packet buffer for storing outbound MCTP packets.
///
/// This uses fixed-size arrays to avoid dynamic allocation, mirroring the
/// memory behavior of transport-i2c which uses stack-allocated buffers.
/// Packets are stored in a simple array with length tracking.
pub struct PacketBuffer {
    /// Storage for packet data
    packets: [[u8; MAX_PACKET_SIZE]; MAX_BUFFERED_PACKETS],
    /// Length of each packet (0 means slot is empty)
    lengths: [usize; MAX_BUFFERED_PACKETS],
    /// Number of packets currently stored
    count: usize,
}

impl PacketBuffer {
    /// Create a new empty packet buffer.
    pub const fn new() -> Self {
        Self {
            packets: [[0; MAX_PACKET_SIZE]; MAX_BUFFERED_PACKETS],
            lengths: [0; MAX_BUFFERED_PACKETS],
            count: 0,
        }
    }

    /// Add a packet to the buffer.
    ///
    /// Returns an error if the buffer is full or the packet is too large.
    pub fn push(&mut self, data: &[u8]) -> Result<(), ()> {
        if self.count >= MAX_BUFFERED_PACKETS {
            return Err(()); // Buffer full
        }
        if data.len() > MAX_PACKET_SIZE {
            return Err(()); // Packet too large
        }

        // Find first empty slot
        for i in 0..MAX_BUFFERED_PACKETS {
            if self.lengths[i] == 0 {
                self.packets[i][..data.len()].copy_from_slice(data);
                self.lengths[i] = data.len();
                self.count += 1;
                return Ok(());
            }
        }

        Err(()) // Should not happen if count tracking is correct
    }

    /// Get the number of packets currently buffered.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over all packets in the buffer.
    pub fn iter(&self) -> PacketBufferIter<'_> {
        PacketBufferIter {
            buffer: self,
            index: 0,
        }
    }

    /// Clear all packets from the buffer.
    pub fn clear(&mut self) {
        self.lengths.fill(0);
        self.count = 0;
    }
}

impl Default for PacketBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over packets in a PacketBuffer
pub struct PacketBufferIter<'a> {
    buffer: &'a PacketBuffer,
    index: usize,
}

impl<'a> Iterator for PacketBufferIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < MAX_BUFFERED_PACKETS {
            let i = self.index;
            self.index += 1;

            if self.buffer.lengths[i] > 0 {
                return Some(&self.buffer.packets[i][..self.buffer.lengths[i]]);
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// BufferSender - Captures packets into PacketBuffer
// ---------------------------------------------------------------------------

/// A mock [`Sender`] that captures outbound MCTP packets into a fixed-size buffer.
///
/// Each call to `send_vectored` will fragment the payload and append the
/// resulting packets to the buffer. Use with [`transfer()`] to deliver packets
/// to another server's inbound path.
pub struct BufferSender<'a> {
    packets: &'a RefCell<PacketBuffer>,
}

impl<'a> BufferSender<'a> {
    /// Create a new BufferSender that writes to the given packet buffer.
    pub fn new(packets: &'a RefCell<PacketBuffer>) -> Self {
        Self { packets }
    }
}

impl Sender for BufferSender<'_> {
    fn send_vectored(
        &mut self,
        _eid: Eid,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<mctp::Tag> {
        loop {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    self.packets
                        .borrow_mut()
                        .push(p)
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

// ---------------------------------------------------------------------------
// LoopbackClient
// ---------------------------------------------------------------------------

/// An [`MctpClient`] implementation that wraps a [`Server`] for loopback testing.
///
/// This is a formalized version of the `DirectClient` from the test infrastructure.
/// It implements all `MctpClient` methods by calling the underlying `Server` directly,
/// bypassing any IPC layer.
pub struct LoopbackClient<'a, S: Sender, const N: usize> {
    server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: Sender, const N: usize> LoopbackClient<'a, S, N> {
    /// Create a new LoopbackClient wrapping the given server.
    pub fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: Sender, const N: usize> MctpClient for LoopbackClient<'_, S, N> {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().req(eid)
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().listener(msg_type)
    }

    fn get_eid(&self) -> u8 {
        self.server.borrow().get_eid()
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        self.server.borrow_mut().set_eid(eid)
    }

    fn recv(
        &self,
        handle: Handle,
        _timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        use openprot_mctp_api::ResponseCode;
        self.server
            .borrow_mut()
            .try_recv(handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        self.server
            .borrow_mut()
            .send(handle, msg_type, eid, tag, integrity_check, buf)
    }

    fn drop_handle(&self, handle: Handle) {
        let _ = self.server.borrow_mut().unbind(handle);
    }
}

// ---------------------------------------------------------------------------
// Helper function for transferring packets
// ---------------------------------------------------------------------------

/// Transfer packets from a buffer into a server's inbound path.
///
/// This drains `packets` into `dest` as inbound MCTP packets. The buffer is
/// **not** cleared; call `packets.borrow_mut().clear()` manually if needed.
pub fn transfer<S: Sender, const N: usize>(
    packets: &RefCell<PacketBuffer>,
    dest: &RefCell<Server<S, N>>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        let _ = dest.borrow_mut().inbound(pkt);
    }
}

// ---------------------------------------------------------------------------
// LoopbackPair
// ---------------------------------------------------------------------------

/// A pair of MCTP endpoints connected via in-memory loopback.
///
/// This structure manages two MCTP servers and their associated packet buffers,
/// providing methods to transfer packets between them. This is useful for testing
/// MCTP applications without requiring physical transport hardware.
///
/// Uses fixed-size `PacketBuffer` to mirror the memory behavior of `transport-i2c`,
/// avoiding any dynamic allocation.
///
/// # Type Parameters
///
/// - `N`: The maximum number of outstanding handles per server (default: 16)
///
/// # Example
///
/// ```
/// use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
/// use openprot_mctp_api::MctpClient;
/// use core::cell::RefCell;
///
/// let packets_a = RefCell::new(PacketBuffer::new());
/// let packets_b = RefCell::new(PacketBuffer::new());
/// let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);
///
/// // Get clients
/// let client_a = pair.client_a();
/// let client_b = pair.client_b();
///
/// // Send from A to B
/// let handle = client_a.req(42).unwrap();
/// client_a.send(Some(handle), 1, None, None, false, b"test").unwrap();
///
/// // Transfer packets
/// pair.transfer_a_to_b();
///
/// // Receive on B
/// let listener = client_b.listener(1).unwrap();
/// let mut buf = [0u8; 256];
/// let meta = client_b.recv(listener, 0, &mut buf).unwrap();
/// ```
pub struct LoopbackPair<'a, const N: usize = 16> {
    /// Outbound packet buffer for endpoint A
    pub packets_a: &'a RefCell<PacketBuffer>,
    /// Outbound packet buffer for endpoint B
    pub packets_b: &'a RefCell<PacketBuffer>,
    /// MCTP server for endpoint A
    pub server_a: RefCell<Server<BufferSender<'a>, N>>,
    /// MCTP server for endpoint B
    pub server_b: RefCell<Server<BufferSender<'a>, N>>,
}

impl<'a, const N: usize> LoopbackPair<'a, N> {
    /// Create a new loopback pair with the given endpoint IDs and packet buffers.
    ///
    /// # Parameters
    ///
    /// - `eid_a`: Endpoint ID for the first server
    /// - `eid_b`: Endpoint ID for the second server
    /// - `packets_a`: Outbound packet buffer for endpoint A
    /// - `packets_b`: Outbound packet buffer for endpoint B
    ///
    /// # Example
    ///
    /// ```
    /// use core::cell::RefCell;
    /// use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
    ///
    /// let packets_a = RefCell::new(PacketBuffer::new());
    /// let packets_b = RefCell::new(PacketBuffer::new());
    /// let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);
    /// ```
    pub fn new(
        eid_a: u8,
        eid_b: u8,
        packets_a: &'a RefCell<PacketBuffer>,
        packets_b: &'a RefCell<PacketBuffer>,
    ) -> Self {
        let sender_a = BufferSender::new(packets_a);
        let sender_b = BufferSender::new(packets_b);

        let server_a = RefCell::new(Server::new(Eid(eid_a), 0, sender_a));
        let server_b = RefCell::new(Server::new(Eid(eid_b), 0, sender_b));

        Self {
            packets_a,
            packets_b,
            server_a,
            server_b,
        }
    }

    /// Get a client handle for endpoint A.
    pub fn client_a(&self) -> LoopbackClient<'_, BufferSender<'a>, N> {
        LoopbackClient::new(&self.server_a)
    }

    /// Get a client handle for endpoint B.
    pub fn client_b(&self) -> LoopbackClient<'_, BufferSender<'a>, N> {
        LoopbackClient::new(&self.server_b)
    }

    /// Transfer all pending packets from A to B.
    ///
    /// This drains the outbound buffer of endpoint A and feeds the packets
    /// into endpoint B's inbound path. The buffer is **not** automatically
    /// cleared; call [`clear_a()`](Self::clear_a) if needed.
    pub fn transfer_a_to_b(&self) {
        transfer(self.packets_a, &self.server_b)
    }

    /// Transfer all pending packets from B to A.
    ///
    /// This drains the outbound buffer of endpoint B and feeds the packets
    /// into endpoint A's inbound path. The buffer is **not** automatically
    /// cleared; call [`clear_b()`](Self::clear_b) if needed.
    pub fn transfer_b_to_a(&self) {
        transfer(self.packets_b, &self.server_a)
    }

    /// Clear endpoint A's outbound packet buffer.
    pub fn clear_a(&self) {
        self.packets_a.borrow_mut().clear();
    }

    /// Clear endpoint B's outbound packet buffer.
    pub fn clear_b(&self) {
        self.packets_b.borrow_mut().clear();
    }

    /// Perform a full roundtrip: A→B, B→A, then clear both buffers.
    ///
    /// This is a convenience method for bidirectional request/response tests.
    pub fn roundtrip(&self) {
        self.transfer_a_to_b();
        self.transfer_b_to_a();
        self.clear_a();
        self.clear_b();
    }

    /// Get the endpoint ID of server A.
    pub fn eid_a(&self) -> u8 {
        self.server_a.borrow().get_eid()
    }

    /// Get the endpoint ID of server B.
    pub fn eid_b(&self) -> u8 {
        self.server_b.borrow().get_eid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_mctp_api::MctpClient;

    #[test]
    fn test_packet_buffer() {
        let mut buffer = PacketBuffer::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        // Add packets
        buffer.push(b"packet1").unwrap();
        buffer.push(b"packet2").unwrap();
        assert_eq!(buffer.len(), 2);
        assert!(!buffer.is_empty());

        // Iterate
        let mut count = 0;
        let mut iter = buffer.iter();

        let pkt0 = iter.next().unwrap();
        assert_eq!(pkt0, b"packet1");
        count += 1;

        let pkt1 = iter.next().unwrap();
        assert_eq!(pkt1, b"packet2");
        count += 1;

        assert!(iter.next().is_none());
        assert_eq!(count, 2);

        // Clear
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn basic_loopback() {
        let packets_a = RefCell::new(PacketBuffer::new());
        let packets_b = RefCell::new(PacketBuffer::new());
        let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);
        let client_a = pair.client_a();
        let client_b = pair.client_b();

        // B registers listener
        let handle_b = client_b.listener(1).unwrap();

        // A sends to B
        let handle_a = client_a.req(42).unwrap();
        client_a
            .send(Some(handle_a), 1, None, None, false, b"hello from A")
            .unwrap();

        pair.transfer_a_to_b();

        // B receives
        let mut buf = [0u8; 256];
        let meta = client_b.recv(handle_b, 0, &mut buf).unwrap();

        assert_eq!(&buf[..meta.payload_size], b"hello from A");
        assert_eq!(meta.remote_eid, 8);
        assert_eq!(meta.msg_type, 1);
    }

    #[test]
    fn bidirectional_loopback() {
        let packets_a = RefCell::new(PacketBuffer::new());
        let packets_b = RefCell::new(PacketBuffer::new());
        let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);
        let client_a = pair.client_a();
        let client_b = pair.client_b();

        // B registers listener
        let handle_b = client_b.listener(1).unwrap();

        // A sends request
        let handle_a = client_a.req(42).unwrap();
        client_a
            .send(Some(handle_a), 1, None, None, false, b"ping")
            .unwrap();
        pair.transfer_a_to_b();

        // B receives and responds
        let mut buf = [0u8; 256];
        let meta = client_b.recv(handle_b, 0, &mut buf).unwrap();
        assert_eq!(&buf[..meta.payload_size], b"ping");

        client_b
            .send(
                None,
                meta.msg_type,
                Some(meta.remote_eid),
                Some(meta.msg_tag),
                false,
                b"pong",
            )
            .unwrap();
        pair.transfer_b_to_a();

        // A receives response
        let resp_meta = client_a.recv(handle_a, 0, &mut buf).unwrap();
        assert_eq!(&buf[..resp_meta.payload_size], b"pong");
        assert_eq!(resp_meta.remote_eid, 42);
    }

    #[test]
    fn roundtrip_helper() {
        let packets_a = RefCell::new(PacketBuffer::new());
        let packets_b = RefCell::new(PacketBuffer::new());
        let pair = LoopbackPair::<16>::new(10, 20, &packets_a, &packets_b);
        let client_a = pair.client_a();
        let client_b = pair.client_b();

        assert_eq!(pair.eid_a(), 10);
        assert_eq!(pair.eid_b(), 20);

        // Set up request/response
        let handle_a = client_a.req(20).unwrap();
        let handle_b = client_b.listener(99).unwrap();

        client_a
            .send(Some(handle_a), 99, None, None, false, b"data")
            .unwrap();

        let mut buf_b = [0u8; 256];
        pair.transfer_a_to_b();
        let meta_b = client_b.recv(handle_b, 0, &mut buf_b).unwrap();

        client_b
            .send(
                None,
                meta_b.msg_type,
                Some(meta_b.remote_eid),
                Some(meta_b.msg_tag),
                false,
                b"response",
            )
            .unwrap();

        pair.transfer_b_to_a();

        let mut buf_a = [0u8; 256];
        let meta_a = client_a.recv(handle_a, 0, &mut buf_a).unwrap();
        assert_eq!(&buf_a[..meta_a.payload_size], b"response");

        // Clear should work
        pair.clear_a();
        pair.clear_b();
    }
}
