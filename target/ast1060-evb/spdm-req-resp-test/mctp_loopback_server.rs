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
//!
//! # Hubris-style deferred Recv
//!
//! Modelled on the Hubris `mctp-server` task's `outstanding` map.  When a
//! `Recv` IPC arrives but the inbox is empty, the server does **not** call
//! `channel_respond` immediately.  Instead it:
//!
//! 1. Records the pending handle in `pending_recv_req/resp`.
//! 2. Calls `wait_group_remove` to stop the WG from re-firing for that
//!    channel (the kernel READABLE signal stays set until `channel_respond`
//!    is called, so without the remove the WG would spin).
//! 3. When the opposite side later delivers a packet via a `Send` IPC the
//!    server calls `try_service_pending`, which answers the deferred `Recv`
//!    via `channel_respond` and then re-adds the channel to the WG.

#![no_main]
#![no_std]

use core::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::wire::{self, MctpOp, MctpRequestHeader, MAX_PAYLOAD_SIZE, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use openprot_mctp_api::{Handle, ResponseCode};
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

/// A parked `Recv` IPC: `channel_read` was called but `channel_respond` was
/// deferred because the inbox was empty.  The WG entry for the channel has
/// been removed to prevent the handler's persistent READABLE signal from
/// causing the WG to spin.
struct PendingRecv {
    handle: Handle,
}

/// Transfer all packets from a buffer into a server's inbound path, then clear.
///
/// This is the core loopback mechanism: packets that one server sent outbound
/// get fed into the other server as inbound packets.
fn transfer_and_clear<S: openprot_mctp_server::Sender, const N: usize>(
    packets: &RefCell<PacketBuffer>,
    dest: &mut openprot_mctp_server::Server<S, N>,
) {
    let pkts = packets.borrow();
    pw_log::debug!("transfer_and_clear: {} packet(s)", pkts.len() as u32);
    for pkt in pkts.iter() {
        match dest.inbound(pkt) {
            Ok(Some(cookie)) => {
                pw_log::debug!("transfer_and_clear: pkt delivered to cookie {}", cookie.0 as u32);
            }
            Ok(None) => {
                pw_log::debug!("transfer_and_clear: pkt discarded (no handler)");
            }
            Err(e) => {
                pw_log::error!("transfer_and_clear: inbound failed: {}", e.code as u32);
            }
        }
    }
    drop(pkts);
    packets.borrow_mut().clear();
}

/// Try to answer a previously parked `Recv` IPC now that new data may have
/// arrived in `server`'s inbox.
///
/// Calls `transfer_and_clear(outbox, server)` first so that packets buffered
/// in the outbox are injected lazily — at the moment we need them — rather
/// than eagerly at Send-dispatch time (when the destination listener may not
/// yet be registered).  This eliminates the startup race where GET_VERSION
/// is silently dropped because it arrived before the responder registered.
///
/// If a message is available after transfer: encodes the response, calls
/// `channel_respond` to wake the client, then re-adds the channel to the WG.
/// If the inbox is still empty: **re-parks** and returns `Ok(())`.  Sending
/// `InternalError` in this case would corrupt the client's state.
fn try_service_pending<S: openprot_mctp_server::Sender, const N: usize>(
    pending: &mut Option<PendingRecv>,
    server: &mut openprot_mctp_server::Server<S, N>,
    outbox: &RefCell<PacketBuffer>,
    response_buf: &mut [u8],
    recv_buf: &mut [u8],
    wg_handle: u32,
    chan_handle: u32,
    wg_user_data: usize,
) -> Result<()> {
    let Some(p) = pending.take() else {
        return Ok(());
    };

    pw_log::debug!(
        "try_service_pending: chan={} wg={} ud={} pending_handle={}",
        chan_handle as u32, wg_handle as u32, wg_user_data as u32, p.handle.0 as u32
    );

    // Lazily inject any buffered outbound packets from the other side before
    // checking the inbox.  If this is the first time we check after a Send,
    // the packets will be queued in the destination server's inbox here.
    transfer_and_clear(outbox, server);

    let Some(meta) = server.try_recv(p.handle, recv_buf) else {
        // Inbox still empty — both sides parked simultaneously (startup race).
        // Re-park and return; the Recv will be answered when data arrives.
        pw_log::debug!(
            "try_service_pending: try_recv(handle={}) miss — re-parking on chan={}",
            p.handle.0 as u32, chan_handle as u32
        );
        *pending = Some(p);
        return Ok(());
    };

    pw_log::debug!(
        "try_service_pending: try_recv(handle={}) hit: type={} size={}",
        p.handle.0 as u32, meta.msg_type as u32, meta.payload_size as u32
    );
    let payload = &recv_buf[..meta.payload_size];
    let resp_len = wire::encode_recv_response(
        response_buf,
        meta.msg_type,
        meta.msg_ic,
        meta.remote_eid,
        meta.msg_tag,
        payload,
    )
    .unwrap_or_else(|_| {
        wire::encode_error_response(response_buf, ResponseCode::InternalError)
            .unwrap_or_default()
    });

    syscall::channel_respond(chan_handle, &response_buf[..resp_len])
        .map_err(|e| {
            pw_log::error!(
                "try_service_pending: channel_respond(chan={}) failed: {}",
                chan_handle as u32, e as u32
            );
            e
        })?;

    // channel_respond cleared READABLE on the handler, so re-adding to the WG
    // will not cause an immediate spurious fire.
    pw_log::debug!("try_service_pending: wait_group_add chan={} ud={}", chan_handle as u32, wg_user_data as u32);
    syscall::wait_group_add(wg_handle, chan_handle, Signals::READABLE, wg_user_data)
        .map_err(|e| {
            pw_log::error!(
                "try_service_pending: wait_group_add(wg={} chan={} ud={}) failed: {}",
                wg_handle as u32, chan_handle as u32, wg_user_data as u32, e as u32
            );
            e
        })?;

    Ok(())
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

    // Parked Recv state — at most one outstanding Recv per side at a time.
    let mut pending_recv_req: Option<PendingRecv> = None;  // parked from requester
    let mut pending_recv_resp: Option<PendingRecv> = None; // parked from responder

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
        let ev = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)
            .map_err(|e| { pw_log::error!("object_wait(WG) failed: {}", e as u32); e })?;

        if ev.user_data == 0 {
            // ── IPC from requester client ─────────────────────────────────
            let len = syscall::channel_read(handle::MCTP_REQ, 0, &mut request_buf)
                .map_err(|e| { pw_log::error!("channel_read(MCTP_REQ) failed: {}", e as u32); e })?;

            if len < MctpRequestHeader::SIZE {
                let resp_len = wire::encode_error_response(&mut response_buf, ResponseCode::BadArgument)
                    .unwrap_or_default();
                syscall::channel_respond(handle::MCTP_REQ, &response_buf[..resp_len])?;
                continue;
            }

            // Intercept Recv ops: park them if the inbox is empty rather than
            // returning TimedOut (which would cause the client to spin).
            if let Some(header) = MctpRequestHeader::from_bytes(&request_buf[..len]) {
                if let Some(MctpOp::Recv) = header.operation() {
                    let h = Handle(header.handle);
                    pw_log::debug!(
                        "Recv intercept: ud={} handle={} has_pending_resp={} has_pending_req={}",
                        ev.user_data as u32,
                        h.0 as u32,
                        pending_recv_resp.is_some() as u32,
                        pending_recv_req.is_some() as u32,
                    );
                    if let Some(meta) = server_req.try_recv(h, &mut recv_buf) {
                        // Data already available — answer immediately.
                        let payload = &recv_buf[..meta.payload_size];
                        let resp_len = wire::encode_recv_response(
                            &mut response_buf,
                            meta.msg_type,
                            meta.msg_ic,
                            meta.remote_eid,
                            meta.msg_tag,
                            payload,
                        )
                        .unwrap_or_else(|_| {
                            wire::encode_error_response(&mut response_buf, ResponseCode::InternalError)
                                .unwrap_or_default()
                        });
                        syscall::channel_respond(handle::MCTP_REQ, &response_buf[..resp_len])
                            .map_err(|e| { pw_log::error!("channel_respond(MCTP_REQ, imm-recv) failed: {}", e as u32); e })?;
                    } else {
                        // Inbox empty: park the Recv and remove from WG to prevent spin.
                        // channel_respond is intentionally deferred.
                        pw_log::debug!("parking recv: MCTP_REQ handle={}", h.0 as u32);
                        pending_recv_req = Some(PendingRecv { handle: h });
                        syscall::wait_group_remove(handle::WG, handle::MCTP_REQ)
                            .map_err(|e| { pw_log::error!("wait_group_remove(MCTP_REQ) failed: {}", e as u32); e })?;
                        // The responder may already have a parked Recv — service it
                        // now.  Pass packets_req so any buffered outbound is lazily
                        // transferred into server_resp before try_recv.
                        try_service_pending(
                            &mut pending_recv_resp,
                            &mut server_resp,
                            &packets_req,
                            &mut response_buf,
                            &mut recv_buf,
                            handle::WG,
                            handle::MCTP_RESP,
                            1usize,
                        )?;
                    }
                    continue;
                }
            }

            // Non-Recv op: dispatch normally.
            let response_len = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server_req,
                &mut recv_buf,
            );

            // Packets stay in packets_req until lazily transferred inside
            // try_service_pending when the responder's parked Recv is serviced.
            syscall::channel_respond(handle::MCTP_REQ, &response_buf[..response_len])
                .map_err(|e| { pw_log::error!("channel_respond(MCTP_REQ, dispatch) failed: {}", e as u32); e })?;

            // If the responder has a parked Recv, transfer and answer it now.
            try_service_pending(
                &mut pending_recv_resp,
                &mut server_resp,
                &packets_req,
                &mut response_buf,
                &mut recv_buf,
                handle::WG,
                handle::MCTP_RESP,
                1usize,
            )?;
            pw_log::debug!(
                "post-dispatch: pending_req={} pending_resp={}",
                pending_recv_req.is_some() as u32,
                pending_recv_resp.is_some() as u32,
            );
        } else {
            // ── IPC from responder client ─────────────────────────────────
            let len = syscall::channel_read(handle::MCTP_RESP, 0, &mut request_buf)
                .map_err(|e| { pw_log::error!("channel_read(MCTP_RESP) failed: {}", e as u32); e })?;

            if len < MctpRequestHeader::SIZE {
                let resp_len = wire::encode_error_response(&mut response_buf, ResponseCode::BadArgument)
                    .unwrap_or_default();
                syscall::channel_respond(handle::MCTP_RESP, &response_buf[..resp_len])?;
                continue;
            }

            // Intercept Recv ops: park them if the inbox is empty.
            if let Some(header) = MctpRequestHeader::from_bytes(&request_buf[..len]) {
                if let Some(MctpOp::Recv) = header.operation() {
                    let h = Handle(header.handle);
                    pw_log::debug!(
                        "Recv intercept: ud={} handle={} has_pending_resp={} has_pending_req={}",
                        ev.user_data as u32,
                        h.0 as u32,
                        pending_recv_resp.is_some() as u32,
                        pending_recv_req.is_some() as u32,
                    );
                    if let Some(meta) = server_resp.try_recv(h, &mut recv_buf) {
                        // Data already available — answer immediately.
                        let payload = &recv_buf[..meta.payload_size];
                        let resp_len = wire::encode_recv_response(
                            &mut response_buf,
                            meta.msg_type,
                            meta.msg_ic,
                            meta.remote_eid,
                            meta.msg_tag,
                            payload,
                        )
                        .unwrap_or_else(|_| {
                            wire::encode_error_response(&mut response_buf, ResponseCode::InternalError)
                                .unwrap_or_default()
                        });
                        syscall::channel_respond(handle::MCTP_RESP, &response_buf[..resp_len])
                            .map_err(|e| { pw_log::error!("channel_respond(MCTP_RESP, imm-recv) failed: {}", e as u32); e })?;
                    } else {
                        // Inbox empty: park and remove from WG.
                        pw_log::debug!("parking recv: MCTP_RESP handle={}", h.0 as u32);
                        pending_recv_resp = Some(PendingRecv { handle: h });
                        syscall::wait_group_remove(handle::WG, handle::MCTP_RESP)
                            .map_err(|e| { pw_log::error!("wait_group_remove(MCTP_RESP) failed: {}", e as u32); e })?;
                        // The requester may already have a parked Recv — service it
                        // now.  Pass packets_resp so any buffered outbound is lazily
                        // transferred into server_req before try_recv.
                        try_service_pending(
                            &mut pending_recv_req,
                            &mut server_req,
                            &packets_resp,
                            &mut response_buf,
                            &mut recv_buf,
                            handle::WG,
                            handle::MCTP_REQ,
                            0usize,
                        )?;
                    }
                    continue;
                }
            }

            // Non-Recv op: dispatch normally.
            let response_len = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server_resp,
                &mut recv_buf,
            );

            // Packets stay in packets_resp until lazily transferred inside
            // try_service_pending when the requester's parked Recv is serviced.
            syscall::channel_respond(handle::MCTP_RESP, &response_buf[..response_len])
                .map_err(|e| { pw_log::error!("channel_respond(MCTP_RESP, dispatch) failed: {}", e as u32); e })?;

            // If the requester has a parked Recv, transfer and answer it now.
            try_service_pending(
                &mut pending_recv_req,
                &mut server_req,
                &packets_resp,
                &mut response_buf,
                &mut recv_buf,
                handle::WG,
                handle::MCTP_REQ,
                0usize,
            )?;
            pw_log::debug!(
                "post-dispatch: pending_req={} pending_resp={}",
                pending_recv_req.is_some() as u32,
                pending_recv_resp.is_some() as u32,
            );
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
