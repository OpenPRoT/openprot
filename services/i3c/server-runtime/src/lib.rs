// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pigweed server-runtime for an I3C target.
//!
//! Wraps the host-buildable [`i3c_server`] (target-owning `Server` + request
//! dispatch) in a `util_service::ServiceLoop`. A single IPC channel carries the
//! transport protocol ([`i3c_api`]); the I3C hardware IRQ is added to the loop
//! as a wake source, so a frame arriving on the bus and a client request are
//! both just events. The WaitGroup, and the mechanics of leaving a request
//! unanswered, belong to `util_service`; what a request means, and when a held
//! one is released, belong to the `Server` and to this crate.
//!
//! ## Hot path
//!
//! - **IRQ** — the loop returns the I3C interrupt source; `on_interrupt` decodes
//!   it. On `InboundReady` the runtime drains every available frame into the
//!   server's ring, `interrupt_ack`s, and raises `USER` on the client channel so
//!   a parked client wakes. On `ResponseRead` the staged transmit was consumed,
//!   so the runtime releases outbound back-pressure and answers any held `Send`.
//!
//! ## Outbound back-pressure
//!
//! A `Send` stages a response for the controller's next private read. A second
//! `Send` arriving before that read is *held* — its IPC reply is deferred
//! (`ServiceLoop::defer`, which also takes the channel out of the wait) — so
//! fragments are never staged faster than the controller drains them. The client, being synchronous, simply blocks in
//! `channel_transact` meanwhile.
//!
//! Back-pressure is released on `ResponseRead` and, as a fallback, on the next
//! `InboundReady`: the i3c target derives `ResponseRead` from the hardware
//! `TxDescComplete` interrupt (the controller consumed the staged response). It
//! is not returned when it arrives in the same interrupt as an inbound frame, and
//! a fresh inbound request proves (by request/response ordering) the prior
//! response was read, so `InboundReady` releases too. See [`release_backpressure`].
//! - **IPC** — the loop returns a request, already read; the runtime dispatches it. A `Recv` that consumes the latch also clears `USER`; the IRQ
//!   path re-raises it when the next frame lands.
//!
//! The **only** kernel-tagged crate in the i3c server path.

#![no_std]

use i3c_api::{decode_request, encode_response, I3cOp, I3cStatus, MAX_FRAME};
use i3c_server::{dispatch, Inbound, Server};
use openprot_hal_blocking::i3c_hardware::{I3cTarget, TargetEvent};
use pw_status::Result;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_service::{ChannelId, Event, ServiceLoop};

pub use util_service::Channel;

/// The key a held `Send` is deferred under. At most one is held, because the
/// client is synchronous, so a single fixed key is enough.
const HELD_SEND: u32 = 0;

/// Release outbound back-pressure: clear `tx_pending` and answer any held
/// `Send`, which also puts the channel back in the wait.
fn release_backpressure<T: I3cTarget>(
    service: &mut ServiceLoop<'_>,
    srv: &mut Server<T>,
    held: &[u8; MAX_FRAME],
    held_len: &mut Option<usize>,
    response_buf: &mut [u8; MAX_FRAME],
) {
    srv.notify_response_read();
    let Some(len) = held_len.take() else {
        return;
    };
    let resp_len = dispatch(srv, &held[..len], &mut response_buf[..]);
    if service
        .complete(HELD_SEND, &response_buf[..resp_len])
        .is_err()
    {
        pw_log::error!("held send reply failed");
    }
}

/// Run the I3C server loop until a fatal error.
///
/// Serves the one IPC channel `srv.channel` and wakes on `irq_signals` of
/// `srv.irq`.
///
/// # Errors
///
/// Only setup and WaitGroup failures return; anything that goes wrong with one
/// request or one interrupt is logged and the loop carries on.
pub fn run<T: I3cTarget>(wg: u32, irq_signals: Signals, srv: &mut Server<T>) -> Result<()> {
    if srv.target.enable().is_err() {
        pw_log::error!("i3c target enable failed");
    }
    // This runtime holds a second `Send` until the first has been read, so it
    // wants the `ResponseRead` event (see the module docs for the fallback).
    if srv.target.enable_response_read_events().is_err() {
        pw_log::error!("i3c response-read events enable failed");
    }

    let mut channels = [Channel::new(srv.channel)];
    let mut service = ServiceLoop::new(wg, &mut channels)?;
    service.add_source(srv.irq, irq_signals)?;

    let mut request_buf = [0u8; MAX_FRAME];
    let mut response_buf = [0u8; MAX_FRAME];

    // A `Send` that arrived while a prior staged response was still unread: its
    // reply is held (the client blocks in `channel_transact`) and the channel is
    // out of the wait, so no further request is read, until the controller
    // reads the prior response and the IRQ path releases it. At most one is held
    // -- the client is synchronous, so it cannot issue another request meanwhile.
    let mut held = [0u8; MAX_FRAME];
    let mut held_len: Option<usize> = None;

    loop {
        match service.next(Instant::MAX, &mut request_buf)? {
            // No deadline is ever passed, so this does not happen.
            Event::Deadline => {}

            // ---- I3C hardware IRQ: decode, latch an inbound frame, wake client ----
            Event::Source(_) => {
                match srv.target.on_interrupt() {
                    Ok(TargetEvent::InboundReady) => {
                        // Drain every frame the IRQ covers into the ring; a single
                        // RX-descriptor interrupt can stand for more than one
                        // hardware frame. Stop on Empty.
                        let mut dropped = false;
                        loop {
                            match srv.latch_inbound() {
                                Ok(Inbound::Latched) => continue,
                                Ok(Inbound::DroppedFull) => {
                                    dropped = true;
                                    continue;
                                }
                                Ok(Inbound::Empty) => break,
                                Err(_) => {
                                    pw_log::error!("i3c read_frame failed");
                                    break;
                                }
                            }
                        }
                        if dropped {
                            pw_log::error!("i3c inbound ring full; frame(s) dropped");
                        }
                        // A fresh inbound request proves the controller read the
                        // prior response (request/response ordering), so release
                        // back-pressure here too -- the fallback for a missed
                        // `IbiDone`/`ResponseRead`. A no-op when nothing is pending.
                        release_backpressure(
                            &mut service,
                            srv,
                            &held,
                            &mut held_len,
                            &mut response_buf,
                        );
                    }
                    Ok(TargetEvent::ResponseRead) => {
                        // The controller read the staged response: release outbound
                        // back-pressure and answer any held `Send`.
                        release_backpressure(
                            &mut service,
                            srv,
                            &held,
                            &mut held_len,
                            &mut response_buf,
                        );
                    }
                    Ok(_) => {}
                    Err(_) => pw_log::error!("i3c on_interrupt failed"),
                }
                // The loop does not acknowledge a source, and the IRQ is level
                // triggered, so ack it or `next` returns it again at once.
                if syscall::interrupt_ack(srv.irq, irq_signals).is_err() {
                    pw_log::error!("interrupt_ack failed");
                }
                if srv.has_frame() {
                    // ORs USER onto the client channel without disturbing READABLE.
                    if syscall::object_set_peer_user_signal(srv.channel, true).is_err() {
                        pw_log::error!("object_set_peer_user_signal failed");
                    }
                }
            }

            // ---- client IPC request ----
            Event::Request { channel, len } => {
                let request = request_buf.get(..len).unwrap_or(&[]);
                serve(
                    &mut service,
                    srv,
                    channel,
                    request,
                    &mut held,
                    &mut held_len,
                    &mut response_buf,
                );
            }
        }
    }
}

/// Dispatch one client request and answer it, or hold it.
fn serve<T: I3cTarget>(
    service: &mut ServiceLoop<'_>,
    srv: &mut Server<T>,
    channel: ChannelId,
    request: &[u8],
    held: &mut [u8; MAX_FRAME],
    held_len: &mut Option<usize>,
    response_buf: &mut [u8; MAX_FRAME],
) {
    let op = decode_request(request).map(|(op, _)| op);

    // Outbound back-pressure: a `Send` while a prior staged response is still
    // unread is held -- copy it aside and defer the reply (the loop also drops
    // the channel from the wait, so no further request is read), then release it
    // from the ResponseRead path. Only reached over IPC; the loopback path calls
    // `dispatch` directly and is never gated.
    if op == Some(I3cOp::Send) && srv.tx_pending() {
        if let Some(dst) = held.get_mut(..request.len()) {
            dst.copy_from_slice(request);
            if service.defer(channel, HELD_SEND).is_ok() {
                *held_len = Some(request.len());
                pw_log::info!("i3c: send held");
                return;
            }
        }
        // Not held: nothing was deferred and the channel is still served, so
        // this client must be answered now or it stays blocked.
        pw_log::error!("i3c: could not hold send");
        let n = encode_response(I3cStatus::Internal, &[], &mut response_buf[..]).unwrap_or(0);
        if service.reply(channel, &response_buf[..n]).is_err() {
            pw_log::error!("channel_respond failed");
        }
        return;
    }

    let is_recv = op == Some(I3cOp::Recv);
    let resp_len = dispatch(srv, request, &mut response_buf[..]);
    // A Recv pops one frame from the ring. Keep USER asserted while frames
    // remain so the client drains the rest; clear it only when the ring is
    // empty (the IRQ path re-raises it when the next frame arrives). A blind
    // clear here would park the client with frames still queued.
    if is_recv && syscall::object_set_peer_user_signal(srv.channel, srv.has_frame()).is_err() {
        pw_log::error!("object_set_peer_user_signal update failed");
    }
    if service.reply(channel, &response_buf[..resp_len]).is_err() {
        pw_log::error!("channel_respond failed");
    }
}
