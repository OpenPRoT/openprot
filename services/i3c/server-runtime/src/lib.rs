// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pigweed server-runtime for an I3C target.
//!
//! Wraps the host-buildable [`i3c_server`] (target-owning `Server` + request
//! dispatch) in a `util_service::ServiceLoop`. A single IPC channel carries the
//! transport protocol ([`i3c_api`]); the I3C hardware IRQ is added to the loop
//! as a wake source, so a frame arriving on the bus and a client request are
//! both just events. The WaitGroup belongs to `util_service`; what a request
//! means belongs to the `Server`.
//!
//! ## Hot path
//!
//! - **IRQ** — the loop returns the I3C interrupt source; `on_interrupt` decodes
//!   it. On `InboundReady` the runtime drains every available frame into the
//!   server's ring, `interrupt_ack`s, and raises `USER` on the client channel so
//!   a parked client wakes. On `ResponseRead` the staged transmit was consumed
//!   by the controller, so the runtime lets the next `Send` through.
//! - **IPC** — the loop returns a request, already read; the runtime dispatches
//!   it and replies. A `Recv` that consumes a frame also updates `USER`: it stays
//!   asserted while frames remain and clears when the ring is empty; the IRQ path
//!   re-raises it when the next frame lands.
//!
//! ## Outbound back-pressure
//!
//! A `Send` stages a response for the controller's next private read. Only one
//! can be staged at a time: a `Send` while the previous one is still unread is
//! answered `Busy` by `i3c_server::dispatch` and nothing is staged. The client
//! retries after the controller has read it. The only thing that releases the
//! next `Send` is `ResponseRead`, which the i3c target derives from the hardware
//! `TxDescComplete` interrupt. A new inbound frame does **not** release it: a
//! controller may write again before it reads. This is also how caliptra-mcu-sw's
//! I3C driver gates outbound data (`ErrorCode::BUSY`).
//!
//! The **only** kernel-tagged crate in the i3c server path.

#![no_std]

use i3c_api::{decode_request, I3cOp, MAX_FRAME};
use i3c_server::{dispatch, Inbound, Server};
use openprot_hal_blocking::i3c_hardware::{I3cTarget, TargetEvent};
use pw_status::Result;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_service::{Event, ServiceLoop};

pub use util_service::Channel;

/// Run the I3C server loop until a fatal error.
///
/// Serves the one IPC channel `srv.channel` and wakes on `irq_signals` of
/// `srv.irq`.
///
/// # Errors
///
/// Only setup and WaitGroup failures return; anything that goes wrong with one
/// request or one interrupt is logged and the loop carries on.
pub fn run<T: I3cTarget, const RING: usize>(
    wg: u32,
    irq_signals: Signals,
    srv: &mut Server<T, RING>,
) -> Result<()> {
    if srv.target.enable().is_err() {
        pw_log::error!("i3c target enable failed");
    }
    // The only thing that releases the next `Send` is `ResponseRead`, so the
    // target must report it.
    if srv.target.enable_response_read_events().is_err() {
        pw_log::error!("i3c response-read events enable failed");
    }

    let mut channels = [Channel::new(srv.channel)];
    let mut service = ServiceLoop::new(wg, &mut channels)?;
    service.add_source(srv.irq, irq_signals)?;

    let mut request_buf = [0u8; MAX_FRAME];
    let mut response_buf = [0u8; MAX_FRAME];

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
                    }
                    Ok(TargetEvent::ResponseRead) => {
                        // The controller read the staged response: the next
                        // `Send` can be staged.
                        srv.notify_response_read();
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
                let is_recv = decode_request(request).map(|(op, _)| op) == Some(I3cOp::Recv);
                let resp_len = dispatch(srv, request, &mut response_buf);
                // A Recv pops one frame from the ring. Keep USER asserted while
                // frames remain so the client drains the rest; clear it only when
                // the ring is empty (the IRQ path re-raises it when the next frame
                // arrives). A blind clear here would park the client with frames
                // still queued.
                if is_recv
                    && syscall::object_set_peer_user_signal(srv.channel, srv.has_frame()).is_err()
                {
                    pw_log::error!("object_set_peer_user_signal update failed");
                }
                if service
                    .reply(channel, response_buf.get(..resp_len).unwrap_or(&[]))
                    .is_err()
                {
                    pw_log::error!("channel_respond failed");
                }
            }
        }
    }
}
