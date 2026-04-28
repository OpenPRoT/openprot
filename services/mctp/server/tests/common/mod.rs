// Licensed under the Apache-2.0 license

//! Shared test fixtures for MCTP server integration tests.
//!
//! Provides:
//! - [`BufferSender`] — captures outbound packets into a `Vec` (no I2C)
//! - [`DroppingBufferSender`] — discards all outbound packets (for inbound-only tests)
//! - [`transfer`] — drains one server's outbound buffer into another's inbound path
//! - [`DirectClient`] — implements `MctpClient` directly against a `Server` (no IPC)
//! - [`DirectListener`] — implements `MctpListener` via a `DirectClient`
//! - [`DirectRespChannel`] — implements `MctpRespChannel` via a `DirectClient`
//! - [`DirectReqChannel`] — implements `MctpReqChannel` via a `DirectClient`

// Each integration test file is its own crate in Bazel. Not every file uses
// every fixture, so suppress dead-code warnings for the shared module.
#![allow(dead_code)]

use std::cell::RefCell;

use mctp::{Eid, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{
    Handle, MctpClient, MctpError, MctpListener, MctpReqChannel, MctpRespChannel, RecvMetadata,
    ResponseCode,
};
use openprot_mctp_server::Server;

// ---------------------------------------------------------------------------
// BufferSender
// ---------------------------------------------------------------------------

/// A mock [`Sender`] that captures every outbound MCTP packet into a shared buffer.
///
/// Use [`transfer`] to drain the buffer into another server's inbound path.
pub struct BufferSender<'a> {
    pub packets: &'a RefCell<Vec<Vec<u8>>>,
}

impl Sender for BufferSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            let mut buf = [0u8; 255];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    self.packets.borrow_mut().push(p.to_vec());
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        255
    }
}

// ---------------------------------------------------------------------------
// SmallMtuBufferSender
// ---------------------------------------------------------------------------

/// A [`BufferSender`] variant with a configurable (small) MTU for fragment tests.
pub struct SmallMtuBufferSender<'a> {
    pub packets: &'a RefCell<Vec<Vec<u8>>>,
    pub mtu: usize,
}

impl Sender for SmallMtuBufferSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            let mut buf = [0u8; 255];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    self.packets.borrow_mut().push(p.to_vec());
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        self.mtu
    }
}

// ---------------------------------------------------------------------------
// DroppingBufferSender
// ---------------------------------------------------------------------------

/// A mock [`Sender`] that silently discards all outbound packets.
///
/// Use when a test only cares about the inbound path and does not need to
/// inspect what the server would have sent out.
pub struct DroppingBufferSender;

impl Sender for DroppingBufferSender {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            let mut buf = [0u8; 255];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(_) => {}
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        255
    }
}

// ---------------------------------------------------------------------------
// transfer
// ---------------------------------------------------------------------------

/// Drain `packets` into `dest` as inbound MCTP packets.
///
/// Call this after the sender server has processed a send, to deliver the
/// packets to the receiver server. The buffer is **not** cleared; call
/// `packets.borrow_mut().clear()` manually between rounds if needed.
pub fn transfer<S: Sender, const N: usize>(
    packets: &RefCell<Vec<Vec<u8>>>,
    dest: &mut Server<S, N>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).unwrap();
    }
}

// ---------------------------------------------------------------------------
// DirectClient
// ---------------------------------------------------------------------------

/// Implements [`MctpClient`] by calling [`Server`] methods directly (no IPC).
///
/// In production an IPC channel sits between the application and the MCTP
/// server. `DirectClient` replaces that channel, allowing application code
/// written against `MctpClient` to be exercised in pure `std` tests without
/// any transport hardware.
pub struct DirectClient<'a, S: Sender, const N: usize> {
    pub server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: Sender, const N: usize> DirectClient<'a, S, N> {
    pub fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: Sender, const N: usize> MctpClient for DirectClient<'_, S, N> {
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
// DirectRespChannel
// ---------------------------------------------------------------------------

/// Implements [`MctpRespChannel`] — sends a reply back through a [`DirectClient`].
///
/// Captured metadata from the received request carries the EID and tag needed
/// to route the response correctly.
pub struct DirectRespChannel<'a, S: Sender, const N: usize> {
    client: &'a DirectClient<'a, S, N>,
    msg_type: u8,
    remote_eid: u8,
    tag: u8,
}

impl<S: Sender, const N: usize> MctpRespChannel for DirectRespChannel<'_, S, N> {
    fn send(&mut self, buf: &[u8]) -> Result<(), MctpError> {
        self.client
            .send(
                None,
                self.msg_type,
                Some(self.remote_eid),
                Some(self.tag),
                false,
                buf,
            )
            .map(|_| ())
    }

    fn remote_eid(&self) -> u8 {
        self.remote_eid
    }
}

// ---------------------------------------------------------------------------
// DirectListener
// ---------------------------------------------------------------------------

/// Implements [`MctpListener`] by polling a listener handle via [`DirectClient`].
///
/// `recv` will return `TimedOut` if no message is available yet. In tests,
/// call this after feeding inbound packets with [`transfer`].
pub struct DirectListener<'a, S: Sender, const N: usize> {
    pub client: &'a DirectClient<'a, S, N>,
    pub handle: Handle,
}

impl<'a, S: Sender, const N: usize> DirectListener<'a, S, N> {
    pub fn new(client: &'a DirectClient<'a, S, N>, handle: Handle) -> Self {
        Self { client, handle }
    }
}

impl<'a, S: Sender, const N: usize> MctpListener for DirectListener<'a, S, N> {
    type RespChannel<'r> = DirectRespChannel<'a, S, N> where Self: 'r;

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> Result<(RecvMetadata, &'f mut [u8], Self::RespChannel<'_>), MctpError> {
        let meta = self
            .client
            .server
            .borrow_mut()
            .try_recv(self.handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))?;

        let len = meta.payload_size;
        let resp = DirectRespChannel {
            client: self.client,
            msg_type: meta.msg_type,
            remote_eid: meta.remote_eid,
            tag: meta.msg_tag,
        };
        Ok((meta, &mut buf[..len], resp))
    }
}

// ---------------------------------------------------------------------------
// DirectReqChannel
// ---------------------------------------------------------------------------

/// Implements [`MctpReqChannel`] — send a request and receive the response
/// via [`DirectClient`].
pub struct DirectReqChannel<'a, S: Sender, const N: usize> {
    client: &'a DirectClient<'a, S, N>,
    handle: Handle,
    msg_type: u8,
    remote_eid: u8,
}

impl<'a, S: Sender, const N: usize> DirectReqChannel<'a, S, N> {
    pub fn new(
        client: &'a DirectClient<'a, S, N>,
        handle: Handle,
        msg_type: u8,
        remote_eid: u8,
    ) -> Self {
        Self {
            client,
            handle,
            msg_type,
            remote_eid,
        }
    }
}

impl<S: Sender, const N: usize> MctpReqChannel for DirectReqChannel<'_, S, N> {
    fn send(&mut self, msg_type: u8, buf: &[u8]) -> Result<(), MctpError> {
        self.msg_type = msg_type;
        self.client
            .send(Some(self.handle), msg_type, None, None, false, buf)
            .map(|_| ())
    }

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> Result<(RecvMetadata, &'f mut [u8]), MctpError> {
        let meta = self
            .client
            .server
            .borrow_mut()
            .try_recv(self.handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))?;
        let len = meta.payload_size;
        Ok((meta, &mut buf[..len]))
    }

    fn remote_eid(&self) -> u8 {
        self.remote_eid
    }
}

// ---------------------------------------------------------------------------
// make_server helper
// ---------------------------------------------------------------------------

/// Construct a `Server` + its outbound packet buffer, for two-endpoint tests.
pub fn make_server(
    eid: u8,
    packets: &RefCell<Vec<Vec<u8>>>,
) -> Server<BufferSender<'_>, 16> {
    Server::new(Eid(eid), 0, BufferSender { packets })
}
