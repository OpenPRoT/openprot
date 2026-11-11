// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pigweed server-runtime for an I3C target.
//!
//! Wraps the host-buildable [`i3c_server`] (target-owning `Server` + request
//! dispatch) in the Pigweed WaitGroup reactor. A single IPC channel carries the
//! transport protocol ([`i3c_api`]); the I3C hardware IRQ is multiplexed onto
//! the same wait, so a frame arriving on the bus and a client request are both
//! just wake-ups.
//!
//! ## Hot path
//!
//! - **IRQ** — the wait returns the I3C interrupt signal; `on_interrupt` decodes
//!   it. On `InboundReady` the runtime drains every available frame into the
//!   server's ring, `interrupt_ack`s, and raises `USER` on the client channel so
//!   a parked client wakes. On `ResponseRead` the staged transmit was consumed,
//!   so the runtime releases outbound back-pressure and answers any held `Send`.
//!
//! ## Outbound back-pressure
//!
//! A `Send` stages a response for the controller's next private read. A second
//! `Send` arriving before that read is *held* — its IPC reply is deferred and
//! the channel dropped from the wait — so fragments are never staged faster than
//! the controller drains them. The client, being synchronous, simply blocks in
//! `channel_transact` meanwhile.
//!
//! Back-pressure is released on `ResponseRead` and, as a fallback, on the next
//! `InboundReady`: the i3c target derives `ResponseRead` from the hardware
//! `TxDescComplete` interrupt (the controller consumed the staged response). It
//! is not returned when it arrives in the same interrupt as an inbound frame, and
//! a fresh inbound request proves (by request/response ordering) the prior
//! response was read, so `InboundReady` releases too. See [`release_backpressure`].
//! - **IPC** — the wait returns `READABLE`; the runtime reads one request and
//!   dispatches. A `Recv` that consumes the latch also clears `USER`; the IRQ
//!   path re-raises it when the next frame lands.
//!
//! The **only** kernel-tagged crate in the i3c server path.

#![no_std]

use i3c_api::{decode_request, I3cOp, MAX_FRAME};
use i3c_server::{dispatch, Inbound, Server};
use openprot_hal_blocking::i3c_hardware::{I3cTarget, TargetEvent};
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

/// Release outbound back-pressure: clear `tx_pending` and answer any held
/// `Send` by dispatching it now and re-arming the channel in the wait group.
///
/// Called both from the `ResponseRead` IRQ and — as a fallback — when a fresh
/// inbound request arrives. The i3c target derives `ResponseRead` from the
/// hardware `TxDescComplete` interrupt, which is not returned separately when it
/// shares an interrupt with an inbound frame; a missed `ResponseRead` alone would
/// leave `tx_pending` set and wedge the next `Send` forever. In strict request/response the
/// controller reads a response before issuing its next request, so a new inbound
/// write is itself proof the prior response was read — a sound protocol-level
/// release trigger that keeps the flow alive when `IbiDone` is missed.
fn release_backpressure<T: I3cTarget>(
    wg: u32,
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
    if syscall::channel_respond(srv.channel, &response_buf[..resp_len]).is_err() {
        pw_log::error!("channel_respond (held send) failed");
    }
    if syscall::wait_group_add(wg, srv.channel, Signals::READABLE, srv.channel as usize).is_err() {
        pw_log::error!("wait_group_add (re-arm channel) failed");
    }
}

/// Run the i3c server forever.
///
/// Enables the target, registers its channel (`READABLE`) and IRQ with `wg`,
/// then reacts to bus interrupts and client requests until the process exits.
pub fn run<T: I3cTarget>(wg: u32, irq_signals: Signals, srv: &mut Server<T>) -> ! {
    if srv.target.enable().is_err() {
        pw_log::error!("i3c target enable failed");
    }
    // This runtime holds a second `Send` until the first has been read, so it
    // wants the `ResponseRead` event (see the module docs for the fallback).
    if srv.target.enable_response_read_events().is_err() {
        pw_log::error!("i3c response-read events enable failed");
    }
    if syscall::wait_group_add(wg, srv.channel, Signals::READABLE, srv.channel as usize).is_err() {
        pw_log::error!("wait_group_add channel failed");
    }
    if syscall::wait_group_add(wg, srv.irq, irq_signals, srv.irq as usize).is_err() {
        pw_log::error!("wait_group_add irq failed");
    }

    let wait_mask = Signals::READABLE | irq_signals;
    let mut request_buf = [0u8; MAX_FRAME];
    let mut response_buf = [0u8; MAX_FRAME];

    // A `Send` that arrived while a prior staged response was still unread: its
    // reply is held (the client blocks in `channel_transact`) and the channel is
    // dropped from the wait so no further request is read, until the controller
    // reads the prior response and the IRQ path releases it. At most one is held
    // — the client is synchronous, so it cannot issue another request meanwhile.
    let mut held = [0u8; MAX_FRAME];
    let mut held_len: Option<usize> = None;

    loop {
        let Ok(w) = syscall::object_wait(wg, wait_mask, Instant::MAX) else {
            continue;
        };

        // ---- I3C hardware IRQ: decode, latch an inbound frame, wake client ----
        if w.pending_signals.contains(irq_signals) {
            let acked = w.pending_signals & irq_signals;
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
                    // back-pressure here too — the fallback for a missed
                    // `IbiDone`/`ResponseRead`. A no-op when nothing is pending.
                    release_backpressure(wg, srv, &held, &mut held_len, &mut response_buf);
                }
                Ok(TargetEvent::ResponseRead) => {
                    // The controller read the staged response: release outbound
                    // back-pressure and answer any held `Send`.
                    release_backpressure(wg, srv, &held, &mut held_len, &mut response_buf);
                }
                Ok(_) => {}
                Err(_) => pw_log::error!("i3c on_interrupt failed"),
            }
            if syscall::interrupt_ack(srv.irq, acked).is_err() {
                pw_log::error!("interrupt_ack failed");
            }
            if srv.has_frame() {
                // ORs USER onto the client channel without disturbing READABLE.
                if syscall::object_set_peer_user_signal(srv.channel, true).is_err() {
                    pw_log::error!("object_set_peer_user_signal failed");
                }
            }
            continue;
        }

        // ---- client IPC request ----
        if !w.pending_signals.contains(Signals::READABLE) {
            continue;
        }
        let Ok(req_len) = syscall::channel_read(srv.channel, 0, &mut request_buf) else {
            continue;
        };
        let op = decode_request(&request_buf[..req_len]).map(|(op, _)| op);

        // Outbound back-pressure: a `Send` while a prior staged response is still
        // unread is held — copy it aside, drop the channel from the wait (so its
        // reply stays open and no further request is read), and release it from
        // the ResponseRead path. Only reached over IPC; the loopback path calls
        // `dispatch` directly and is never gated.
        if op == Some(I3cOp::Send) && srv.tx_pending() {
            held[..req_len].copy_from_slice(&request_buf[..req_len]);
            held_len = Some(req_len);
            if syscall::wait_group_remove(wg, srv.channel).is_err() {
                pw_log::error!("wait_group_remove (hold send) failed");
            }
            continue;
        }

        let is_recv = op == Some(I3cOp::Recv);
        let resp_len = dispatch(srv, &request_buf[..req_len], &mut response_buf);
        // A Recv pops one frame from the ring. Keep USER asserted while frames
        // remain so the client drains the rest; clear it only when the ring is
        // empty (the IRQ path re-raises it when the next frame arrives). A blind
        // clear here would park the client with frames still queued.
        if is_recv && syscall::object_set_peer_user_signal(srv.channel, srv.has_frame()).is_err() {
            pw_log::error!("object_set_peer_user_signal update failed");
        }
        if syscall::channel_respond(srv.channel, &response_buf[..resp_len]).is_err() {
            pw_log::error!("channel_respond failed");
        }
    }
}
