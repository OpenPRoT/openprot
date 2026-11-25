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
//! A controller that never reads would leave `Busy` set for good. So the runtime
//! gives each staged response [`STAGED_RESPONSE_TIMEOUT_MS`](i3c_api::STAGED_RESPONSE_TIMEOUT_MS):
//! when that passes without a `ResponseRead` it discards the response in the
//! target ([`Server::cancel_staged`]), logs it, and accepts `Send` again. The
//! discarded message is lost, and the next `Send` is answered `Discarded` so the
//! client learns of it.
//!
//! A staged response is also discarded when the dynamic address changes, since
//! the client framed it (its PEC) for the old one: on an `AddressAssigned` or
//! `AddressLost` event here, and in `dispatch` for a target that never reports
//! them ([`Server::discard_if_address_moved`]).
//!
//! The **only** kernel-tagged crate in the i3c server path.

#![no_std]

use i3c_api::{decode_request, I3cOp, MAX_FRAME, STAGED_RESPONSE_TIMEOUT_MS};
use i3c_server::{dispatch, Inbound, Server};
use openprot_hal_blocking::i3c_hardware::{I3cTarget, TargetEvent};
use pw_status::Result;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_service::{Event, ServiceLoop};

pub use util_service::Channel;

/// How long a staged response may go unread. See
/// [`STAGED_RESPONSE_TIMEOUT_MS`].
const STAGED_TIMEOUT: Duration = Duration::from_millis(STAGED_RESPONSE_TIMEOUT_MS as u64);

/// What one entry of the [`Trace`] records.
mod kind {
    /// The interrupt object woke the loop.
    pub const IRQ: u8 = 1;
    /// `on_interrupt` reported an inbound frame.
    pub const INBOUND_READY: u8 = 2;
    /// `on_interrupt` reported that the controller read the staged response.
    pub const RESPONSE_READ: u8 = 3;
    /// `on_interrupt` reported anything else.
    pub const OTHER_EVENT: u8 = 4;
    /// A frame was drained from the hardware into the ring; the argument is its length.
    pub const LATCHED: u8 = 5;
    /// A frame was drained and dropped because the ring was full.
    pub const DROPPED: u8 = 6;
    /// A client `Send` request; the argument is the request length.
    pub const SEND: u8 = 7;
    /// A client `Recv` request.
    pub const RECV: u8 = 8;
    /// Any other client request.
    pub const OTHER_REQUEST: u8 = 9;
    /// The staged-response deadline woke the loop.
    pub const DEADLINE: u8 = 10;
}

/// How many events the [`Trace`] keeps.
const TRACE_LEN: usize = 24;

/// The last few things the server did, with the time of each, kept in memory and
/// printed only when the hardware reports a damaged frame.
///
/// A frame the core could not fit arrives cut short, and the question is when the
/// frame before it was drained. Logging each event as it happens would slow the
/// drain path being watched, so the events wait here until there is something to
/// explain.
struct Trace {
    /// (clock ticks, kind, argument), as a ring.
    events: [(u64, u8, u16); TRACE_LEN],
    /// Where the next event goes.
    next: usize,
    /// How many of `events` are in use.
    count: usize,
}

impl Trace {
    const fn new() -> Self {
        Self {
            events: [(0, 0, 0); TRACE_LEN],
            next: 0,
            count: 0,
        }
    }

    fn record(&mut self, kind: u8, arg: usize) {
        let now = SystemClock::now().ticks();
        if let Some(slot) = self.events.get_mut(self.next) {
            *slot = (now, kind, u16::try_from(arg).unwrap_or(u16::MAX));
        }
        self.next = (self.next + 1) % TRACE_LEN;
        self.count = (self.count + 1).min(TRACE_LEN);
    }

    /// Print the events, oldest first, each as clock ticks before now (20 ticks
    /// are a microsecond on the FPGA), with the damaged frame's descriptor.
    fn dump(&self, desc: u32) {
        let now = SystemClock::now().ticks();
        pw_log::error!(
            "i3c rx error: desc=0x{:08x}; events, ticks before now, kind, arg;",
            desc as u32
        );
        let start = (self.next + TRACE_LEN - self.count) % TRACE_LEN;
        for i in 0..self.count {
            if let Some(&(at, kind, arg)) = self.events.get((start + i) % TRACE_LEN) {
                pw_log::error!(
                    "  -{} k={} a={};",
                    now.wrapping_sub(at) as u32,
                    kind as u32,
                    arg as u32
                );
            }
        }
    }
}

/// Discard the staged response if its time is up. `deadline` is when the staged
/// response runs out of time to be read, and is cleared when it does.
fn discard_if_expired<T: I3cTarget, const RING: usize>(
    srv: &mut Server<T, RING>,
    deadline: &mut Option<Instant>,
) {
    let Some(due) = *deadline else { return };
    if SystemClock::now() < due {
        return;
    }
    *deadline = None;
    match srv.cancel_staged() {
        Ok(true) => pw_log::error!("i3c staged response not read in time; discarded"),
        Ok(false) => {}
        Err(_) => pw_log::error!("i3c cancel_response failed"),
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

    // When the staged response, if any, runs out of time to be read.
    let mut staged_deadline: Option<Instant> = None;
    let mut trace = Trace::new();

    loop {
        // Check the clock here and not only through `next`'s deadline: `next`
        // returns `Deadline` only when nothing else is ready, and a client
        // retrying `Busy` faster than the scheduler tick always has a request
        // ready, so the deadline would never be seen and the staged response
        // never discarded.
        discard_if_expired(srv, &mut staged_deadline);

        match service.next(staged_deadline.unwrap_or(Instant::MAX), &mut request_buf)? {
            // The staged response was not read in time: give up on it so the
            // next `Send` is not refused for good.
            Event::Deadline => {
                trace.record(kind::DEADLINE, 0);
                discard_if_expired(srv, &mut staged_deadline);
            }

            // ---- I3C hardware IRQ: decode, latch an inbound frame, wake client ----
            Event::Source(_) => {
                trace.record(kind::IRQ, 0);
                match srv.target.on_interrupt() {
                    Ok(TargetEvent::InboundReady) => {
                        trace.record(kind::INBOUND_READY, 0);
                        // Drain every frame the IRQ covers into the ring; a single
                        // RX-descriptor interrupt can stand for more than one
                        // hardware frame. Stop on Empty.
                        let mut dropped = false;
                        loop {
                            match srv.latch_inbound() {
                                Ok(Inbound::Latched) => {
                                    trace.record(kind::LATCHED, srv.newest_frame_len());
                                    if let Some(desc) = srv.target.take_rx_error() {
                                        trace.dump(desc);
                                    }
                                    continue;
                                }
                                Ok(Inbound::DroppedFull) => {
                                    trace.record(kind::DROPPED, 0);
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
                        trace.record(kind::RESPONSE_READ, 0);
                        // The controller read the staged response: the next
                        // `Send` can be staged.
                        srv.notify_response_read();
                        staged_deadline = None;
                    }
                    // A response framed for the old address would fail the
                    // controller's PEC check, so drop it. The next `Send` is
                    // told (`Discarded`).
                    Ok(TargetEvent::AddressAssigned(_) | TargetEvent::AddressLost) => {
                        match srv.address_changed() {
                            Ok(true) => {
                                pw_log::error!("i3c address changed; staged response discarded");
                                staged_deadline = None;
                            }
                            Ok(false) => {}
                            Err(_) => pw_log::error!("i3c cancel_response failed"),
                        }
                    }
                    Ok(_) => trace.record(kind::OTHER_EVENT, 0),
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
                let op = decode_request(request).map(|(op, _)| op);
                let is_recv = op == Some(I3cOp::Recv);
                trace.record(
                    match op {
                        Some(I3cOp::Send) => kind::SEND,
                        Some(I3cOp::Recv) => kind::RECV,
                        _ => kind::OTHER_REQUEST,
                    },
                    len,
                );
                let was_pending = srv.tx_pending();
                let resp_len = dispatch(srv, request, &mut response_buf);
                if !was_pending && srv.tx_pending() {
                    // This request staged a response: start its clock.
                    staged_deadline = Some(
                        SystemClock::now()
                            .checked_add_duration(STAGED_TIMEOUT)
                            .unwrap_or(Instant::MAX),
                    );
                }
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
