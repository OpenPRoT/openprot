// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared in-memory MCTP test plumbing used by the PLDM host integration tests.

use core::cell::RefCell;

use mctp::Tag;
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::Server;

pub const FD_EID: u8 = 42;
pub const UA_EID: u8 = 8;
pub const TIMEOUT_MILLIS: u32 = 0;

/// MTU for MCTP payload (without header)
const MCTP_MTU: usize = 255;
/// MCTP header size (4 bytes)
const MCTP_HEADER_SIZE: usize = 4;

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
            // Fragmenter requires the output buffer to be at least the payload
            // MTU (255) plus the 4-byte MCTP transport header.
            let mut buf = [0u8; MCTP_MTU + MCTP_HEADER_SIZE];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => self.packets.borrow_mut().push(p.to_vec()),
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_MTU
    }
}

pub fn transfer<S: Sender, const N: usize>(
    packets: &RefCell<Vec<Vec<u8>>>,
    dest: &mut Server<S, N>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).expect("inbound should accept packet");
    }
}

pub struct DirectClientWithPump<'a, S: Sender, const N: usize, F: FnMut()> {
    pub server: &'a RefCell<Server<S, N>>,
    pub pre_recv_pump: RefCell<F>,
}

impl<'a, S: Sender, const N: usize, F: FnMut()> DirectClientWithPump<'a, S, N, F> {
    pub fn new(server: &'a RefCell<Server<S, N>>, pre_recv_pump: F) -> Self {
        Self {
            server,
            pre_recv_pump: RefCell::new(pre_recv_pump),
        }
    }
}

impl<S: Sender, const N: usize, F: FnMut()> MctpClient for DirectClientWithPump<'_, S, N, F> {
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
        (self.pre_recv_pump.borrow_mut())();

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
