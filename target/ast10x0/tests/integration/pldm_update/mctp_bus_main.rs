// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The wire between the two endpoints, with an MCTP server at each end.
//!
//! On a board each card runs its own MCTP server over
//! `services/mctp/transport-i2c`, and the cards are joined by I2C. QEMU
//! models one chip and has no I2C, so this app holds both servers and joins
//! them with `services/mctp/transport-loopback`: what one server's router
//! fragments is fed to the other's `inbound`.
//!
//! Only the bottom of the stack changes. The firmware device and the update
//! agent still reach their server as `IpcMctpClient` over a channel, which
//! is the same call path they use on hardware, and EID routing,
//! fragmentation and reassembly are the shipped code.
//!
//! ```text
//!   pldm_fd ──IPC── [ server EID 8 ] ──loopback── [ server EID 42 ] ──IPC── pldm_ua
//! ```
//!
//! A blocking `Recv` is answered when a message arrives, not when it is
//! asked for. The asking channel is dropped from the wait group until then,
//! so the loop does not spin on a transaction it cannot yet complete. This
//! mirrors `target/ast10x0/tests/mctp/server/main.rs`, which defers the
//! same way while it waits for I2C.

#![no_main]
#![no_std]

use openprot_mctp_api::wire::{
    self, MctpOp, MctpRequestHeader, MctpResponseHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::{Handle, ResponseCode};
use openprot_mctp_server::dispatch::{self, DispatchOutcome};
use openprot_mctp_server::Server;
use openprot_mctp_transport_loopback::{LoopbackQueue, LoopbackSender};
use userspace::syscall::Signals;
use userspace::time::{Clock, Duration, Instant, SystemClock};
use userspace::{entry, syscall};

use app_mctp_bus::handle;

/// The firmware device's endpoint id.
const EID_FD: u8 = 8;

/// The update agent's endpoint id.
const EID_UA: u8 = 42;

/// Outstanding requests each server tracks.
const OUTSTANDING: usize = 16;

/// Fragments that may sit on one side's queue between drains. Each costs
/// the MTU plus a header, and both queues are locals on this app's thread
/// stack, so raising this means raising `kernel_stack_size_bytes` with it.
const QUEUE_DEPTH: usize = 8;

/// Largest payload either side reassembles.
const MAX_PAYLOAD_SIZE: usize = 1024;

type BusServer<'q> = Server<LoopbackSender<'q, QUEUE_DEPTH>, OUTSTANDING>;

/// A `Recv` that could not be answered yet: which handle asked, and when
/// to give up on it.
struct PendingRecv {
    handle: Handle,
    deadline: Instant,
}

/// One endpoint: its channel, its server, and any deferred receive.
struct Side<'q> {
    channel: u32,
    user_data: usize,
    server: BusServer<'q>,
    pending: Option<PendingRecv>,
}

/// Answers `pending` from whatever the server now holds, if anything, and
/// puts the channel back in the wait group. Returns true if it answered.
fn satisfy(side: &mut Side<'_>, recv_buf: &mut [u8], response: &mut [u8]) -> bool {
    let Some(p) = side.pending.as_ref() else {
        return false;
    };
    let Some(meta) = side.server.try_recv(p.handle, recv_buf) else {
        return false;
    };

    let len = wire::encode_recv_response(
        response,
        meta.msg_type,
        meta.msg_ic,
        meta.remote_eid,
        meta.msg_tag,
        &recv_buf[..meta.payload_size],
    )
    .unwrap_or_else(|_| {
        wire::encode_error_response(response, ResponseCode::InternalError).unwrap_or(0)
    });
    let _ = syscall::channel_respond(side.channel, &response[..len]);

    side.pending = None;
    let _ = syscall::wait_group_add(handle::WG, side.channel, Signals::READABLE, side.user_data);
    true
}

/// Answers `pending` with a timeout and puts the channel back.
fn time_out(side: &mut Side<'_>, response: &mut [u8]) {
    if side.pending.take().is_none() {
        return;
    }
    let err = MctpResponseHeader::error(ResponseCode::TimedOut);
    response[..MctpResponseHeader::SIZE].copy_from_slice(&err.to_bytes());
    let _ = syscall::channel_respond(side.channel, &response[..MctpResponseHeader::SIZE]);
    let _ = syscall::wait_group_add(handle::WG, side.channel, Signals::READABLE, side.user_data);
}

/// Reads one request and answers it, deferring a `Recv` that has nothing
/// waiting.
fn serve(side: &mut Side<'_>, recv_buf: &mut [u8], response: &mut [u8]) {
    let mut request = [0u8; MAX_REQUEST_SIZE];
    let Ok(len) = syscall::channel_read(side.channel, 0usize, &mut request) else {
        return;
    };

    if len < MctpRequestHeader::SIZE {
        let err = MctpResponseHeader::error(ResponseCode::BadArgument);
        response[..MctpResponseHeader::SIZE].copy_from_slice(&err.to_bytes());
        let _ = syscall::channel_respond(side.channel, &response[..MctpResponseHeader::SIZE]);
        return;
    }

    let header = MctpRequestHeader::from_bytes(&request[..len]);
    let is_recv = header
        .and_then(|h| h.operation())
        .is_some_and(|op| matches!(op, MctpOp::Recv));

    if is_recv {
        let header = header.expect("a Recv op implies a parsed header");
        let payload = wire::get_request_payload(&request[..len]);
        if payload.len() < 4 {
            let err = MctpResponseHeader::error(ResponseCode::BadArgument);
            response[..MctpResponseHeader::SIZE].copy_from_slice(&err.to_bytes());
            let _ = syscall::channel_respond(side.channel, &response[..MctpResponseHeader::SIZE]);
            return;
        }
        let timeout_millis = u32::from_le_bytes(payload[..4].try_into().unwrap());
        let recv_handle = Handle(header.handle);

        // Park the ask. Dropping the channel from the wait group stops the
        // loop re-firing on a transaction that is still open.
        side.pending = Some(PendingRecv {
            handle: recv_handle,
            deadline: if timeout_millis == 0 {
                Instant::MAX
            } else {
                SystemClock::now()
                    .checked_add_duration(Duration::from_millis(timeout_millis as u64))
                    .unwrap_or(Instant::MAX)
            },
        });
        let _ = syscall::wait_group_remove(handle::WG, side.channel);

        // It may already be satisfiable, in which case this answers at once.
        satisfy(side, recv_buf, response);
        return;
    }

    let len = match dispatch::dispatch_mctp_op(
        &request[..len],
        response,
        &mut side.server,
        recv_buf,
        0,
    ) {
        DispatchOutcome::Reply(n) => n,
        DispatchOutcome::Pending { .. } => unreachable!("Recv is handled above"),
    };
    let _ = syscall::channel_respond(side.channel, &response[..len]);
}

#[entry]
fn entry() {
    // Each side's outbox. What one queues, the other receives.
    let out_fd: LoopbackQueue<QUEUE_DEPTH> = LoopbackQueue::new();
    let out_ua: LoopbackQueue<QUEUE_DEPTH> = LoopbackQueue::new();

    let mut fd = Side {
        channel: handle::FD,
        user_data: 0,
        server: Server::new(mctp::Eid(EID_FD), 0, LoopbackSender::new(&out_fd)),
        pending: None,
    };
    let mut ua = Side {
        channel: handle::UA,
        user_data: 1,
        server: Server::new(mctp::Eid(EID_UA), 0, LoopbackSender::new(&out_ua)),
        pending: None,
    };

    if syscall::wait_group_add(handle::WG, fd.channel, Signals::READABLE, fd.user_data).is_err()
        || syscall::wait_group_add(handle::WG, ua.channel, Signals::READABLE, ua.user_data).is_err()
    {
        loop {}
    }

    let mut recv_buf = [0u8; MAX_PAYLOAD_SIZE];
    let mut response = [0u8; MAX_RESPONSE_SIZE];

    loop {
        // Parking removes a channel from the wait group, so both sides
        // parked leaves it empty, and object_wait on an empty group
        // returns InvalidArgument at once rather than blocking.
        //
        // So with both parked there is nothing to wait on, and the loop
        // watches the clock instead, answering each side when its own
        // deadline passes. Answering the nearer one straight away would be
        // simpler and is wrong: a receive that still has seconds to run
        // gets a timeout it did not earn, and the client reports the
        // transfer failed. Nothing can arrive while both sides are parked
        // anyway, because neither can send with its transaction open, so
        // the spin ends at the first real deadline.
        if fd.pending.is_some() && ua.pending.is_some() {
            let now = SystemClock::now();
            if fd.pending.as_ref().is_some_and(|p| p.deadline <= now) {
                time_out(&mut fd, &mut response);
            }
            if ua.pending.as_ref().is_some_and(|p| p.deadline <= now) {
                time_out(&mut ua, &mut response);
            }
            if fd.pending.is_some() && ua.pending.is_some() {
                continue;
            }
        }

        // Wake no later than the nearer of the two parked receives.
        let deadline = match (fd.pending.as_ref(), ua.pending.as_ref()) {
            (Some(a), Some(b)) => a.deadline.min(b.deadline),
            (Some(a), None) => a.deadline,
            (None, Some(b)) => b.deadline,
            (None, None) => Instant::MAX,
        };

        match syscall::object_wait(handle::WG, Signals::READABLE, deadline) {
            Ok(ev) => {
                if ev.user_data == fd.user_data {
                    serve(&mut fd, &mut recv_buf, &mut response);
                } else {
                    serve(&mut ua, &mut recv_buf, &mut response);
                }
            }
            Err(pw_status::Error::DeadlineExceeded) => {
                let now = SystemClock::now();
                if fd.pending.as_ref().is_some_and(|p| p.deadline <= now) {
                    time_out(&mut fd, &mut response);
                }
                if ua.pending.as_ref().is_some_and(|p| p.deadline <= now) {
                    time_out(&mut ua, &mut response);
                }
                continue;
            }
            Err(e) => {
                pw_log::error!("BUS: object_wait error {}", e as u32);
                continue;
            }
        }

        // Carry the wire. One side's fragments are the other's inbound,
        // and draining here rather than inside the router keeps the
        // borrow straight.
        while let Some(pkt) = out_fd.take() {
            let _ = ua.server.inbound(&pkt);
        }
        while let Some(pkt) = out_ua.take() {
            let _ = fd.server.inbound(&pkt);
        }

        // A full queue drops silently, and a missing fragment stalls
        // reassembly forever, which looks like anything but a dropped
        // packet. Say so instead.
        if out_fd.dropped() > 0 || out_ua.dropped() > 0 {
            pw_log::error!(
                "BUS: dropped fragments, fd={} ua={}",
                out_fd.dropped() as u32,
                out_ua.dropped() as u32
            );
        }

        // Whatever just crossed may be what a parked receive was waiting
        // for.
        satisfy(&mut fd, &mut recv_buf, &mut response);
        satisfy(&mut ua, &mut recv_buf, &mut response);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
