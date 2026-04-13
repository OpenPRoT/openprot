// Licensed under the Apache-2.0 license

//! MCTP Loopback Server
//!
//! A lightweight MCTP server that routes messages between two local IPC clients
//! via in-memory loopback. This enables two separate userspace processes (an SPDM
//! requester and responder) to communicate over MCTP without requiring a physical
//! transport (I2C, SPI, etc.).
//!
//! # Architecture
//!
//! ```text
//! ┌─ spdm_requester ──┐      ┌─ This Server ──────────────────┐      ┌─ spdm_responder ─┐
//! │ IpcMctpClient      │─IPC─▶│ server_req (EID 8)             │      │ IpcMctpClient      │
//! │                    │      │   ↓ BufferSender → packets_req │      │                    │
//! │                    │      │   ↓ transfer → server_resp     │      │                    │
//! │                    │      │                                 │◀─IPC─│                    │
//! │                    │      │ server_resp (EID 42)            │      │                    │
//! │                    │      │   ↓ BufferSender → packets_resp│      │                    │
//! │                    │      │   ↓ transfer → server_req      │      │                    │
//! └────────────────────┘      └─────────────────────────────────┘      └────────────────────┘
//! ```
//!
//! Two `Server<BufferSender>` instances are cross-wired: when one server sends
//! a packet, it is captured in a `PacketBuffer` and then transferred to the
//! other server's inbound path after each IPC dispatch cycle.

#![no_main]
#![no_std]

use core::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::wire::{
    MctpRequestHeader, MAX_PAYLOAD_SIZE, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::ResponseCode;
use openprot_mctp_server::dispatch;
use openprot_mctp_transport_loopback::{BufferSender, PacketBuffer};

use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use app_mctp_loopback_server::handle;

/// EID for the requester's MCTP endpoint
const REQUESTER_EID: u8 = 8;

/// EID for the responder's MCTP endpoint
const RESPONDER_EID: u8 = 42;

/// Transfer all packets from a buffer into a server's inbound path, then clear.
///
/// This is the core loopback mechanism: packets that one server sent outbound
/// get fed into the other server as inbound packets.
fn transfer_and_clear<S: openprot_mctp_server::Sender, const N: usize>(
    packets: &RefCell<PacketBuffer>,
    dest: &mut openprot_mctp_server::Server<S, N>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        let _ = dest.inbound(pkt);
    }
    drop(pkts);
    packets.borrow_mut().clear();
}

fn mctp_loopback_server_loop() -> Result<()> {
    pw_log::info!("MCTP loopback server starting");

    // Create two PacketBuffers — outboxes for each side
    let packets_req = RefCell::new(PacketBuffer::new());
    let packets_resp = RefCell::new(PacketBuffer::new());

    // Create two BufferSenders — each writes to its own outbox
    let sender_req = BufferSender::new(&packets_req);
    let sender_resp = BufferSender::new(&packets_resp);

    // Create two Server instances with different EIDs
    let mut server_req =
        openprot_mctp_server::Server::<_, 16>::new(Eid(REQUESTER_EID), 0, sender_req);
    let mut server_resp =
        openprot_mctp_server::Server::<_, 16>::new(Eid(RESPONDER_EID), 0, sender_resp);

    // Buffers for IPC request/response
    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut recv_buf = [0u8; MAX_PAYLOAD_SIZE];

    // Register both IPC channels in the WaitGroup for multiplexing
    // user_data=0 → requester client IPC
    // user_data=1 → responder client IPC
    syscall::wait_group_add(handle::WG, handle::MCTP_REQ, Signals::READABLE, 0usize)?;
    syscall::wait_group_add(handle::WG, handle::MCTP_RESP, Signals::READABLE, 1usize)?;

    pw_log::info!(
        "MCTP loopback server ready (EID {} <-> EID {})",
        REQUESTER_EID as u32,
        RESPONDER_EID as u32,
    );

    loop {
        let ev = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)?;

        if ev.user_data == 0 {
            // IPC from requester client
            let len = syscall::channel_read(handle::MCTP_REQ, 0, &mut request_buf)?;

            if len < MctpRequestHeader::SIZE {
                let resp =
                    openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::BadArgument);
                response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                    .copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(
                    handle::MCTP_REQ,
                    &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                )?;
                continue;
            }

            // Dispatch the MCTP operation
            let response_len = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server_req,
                &mut recv_buf,
            );

            // Loopback: transfer requester's outbound packets → responder's inbound
            transfer_and_clear(&packets_req, &mut server_resp);

            syscall::channel_respond(handle::MCTP_REQ, &response_buf[..response_len])?;
        } else {
            // IPC from responder client
            let len = syscall::channel_read(handle::MCTP_RESP, 0, &mut request_buf)?;

            if len < MctpRequestHeader::SIZE {
                let resp =
                    openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::BadArgument);
                response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                    .copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(
                    handle::MCTP_RESP,
                    &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                )?;
                continue;
            }

            // Dispatch the MCTP operation
            let response_len = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server_resp,
                &mut recv_buf,
            );

            // Loopback: transfer responder's outbound packets → requester's inbound
            transfer_and_clear(&packets_resp, &mut server_req);

            syscall::channel_respond(handle::MCTP_RESP, &response_buf[..response_len])?;
        }
    }
}

#[entry]
fn entry() -> ! {
    if let Err(e) = mctp_loopback_server_loop() {
        pw_log::error!("MCTP loopback server error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("PANIC in MCTP loopback server");
    loop {}
}
