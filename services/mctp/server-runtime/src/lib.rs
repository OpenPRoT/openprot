// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP server IPC dispatch loop.
//!
//! Wraps the platform-independent `openprot_mctp_server::{Server, dispatch}`
//! in a `util_service::ServiceLoop`. Topology-agnostic: the caller supplies
//! a list of client [`Channel`]s (identified purely by their runtime IPC
//! handle, never a named `handle::*` constant) plus a single transport wake
//! source, and [`run`] multiplexes all of them. This lets one `mctp_server`
//! process answer any number of client channels (an SPDM requester, an SPDM
//! responder, a PLDM task) without duplicating the loop.
//!
//! The only kernel-tagged MCTP server crate. The WaitGroup and the
//! mechanics of deferring a reply belong to `util_service`; the meaning of
//! a request and the time a deferred `Recv` is due belong to the `Server`.
//! This crate is the glue: it turns each loop event into a dispatch call and
//! the server's millisecond deadline into a kernel one.

#![no_std]

use mctp_lib::config;
use openprot_mctp_api::wire::{self, MctpRequestHeader, MctpResponseHeader};
use openprot_mctp_api::ResponseCode;
use openprot_mctp_server::dispatch::{self, DispatchOutcome};
use openprot_mctp_server::{Sender, Server};
use pw_status::Result;
use userspace::syscall::Signals;
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_service::{ChannelId, Event, ServiceLoop};

pub use util_service::Channel;

const STACK_MAX_PAYLOAD_SIZE: usize = config::MAX_PAYLOAD;
const MAX_REQUEST_SIZE: usize = MctpRequestHeader::SIZE + STACK_MAX_PAYLOAD_SIZE;
const MAX_RESPONSE_SIZE: usize = MctpResponseHeader::SIZE + STACK_MAX_PAYLOAD_SIZE;

/// Run the MCTP IPC dispatch loop until a fatal error.
///
/// Serves every channel in `channels`, which must be non-empty with
/// distinct handles, and wakes on `transport_signal` of `transport_handle`
/// (an inbound-frame notification).
///
/// A client request is decoded and dispatched. A `Recv` that cannot
/// complete yet is deferred: that client stays blocked, the others keep
/// being served, and the reply goes out when a message for it arrives or
/// its timeout expires.
///
/// # The transport callback
///
/// `on_transport` runs on every transport wake. It must do two things:
///
/// - feed every waiting frame to the server with `server.inbound(pkt)`;
/// - leave the wake source quiet. A `USER` signal is level-triggered and
///   stays set until its owner clears it, and this loop cannot clear it on
///   the callback's behalf. If the callback returns with the signal still
///   set, the loop wakes again at once and spins. A transport whose peer
///   clears the signal when the data is fetched (the i2c server does, on
///   `SlaveReceive`) satisfies this by draining; one that raises the signal
///   itself must clear it after draining.
///
/// # Errors
///
/// A per-channel IPC error (a misbehaving client) is logged and that
/// channel is skipped; it never stops service to the others. Only setup and
/// WaitGroup failures return, and those are the only way this returns.
pub fn run<S: Sender, const N: usize>(
    wg: u32,
    channels: &mut [Channel],
    transport_handle: u32,
    transport_signal: Signals,
    server: &mut Server<S, N>,
    mut on_transport: impl FnMut(&mut Server<S, N>),
) -> Result<()> {
    if channels.is_empty() {
        return Err(pw_status::Error::InvalidArgument);
    }
    let mut service = ServiceLoop::new(wg, channels)?;
    service.add_source(transport_handle, transport_signal)?;

    let epoch = Epoch::now();
    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut recv_buf = [0u8; STACK_MAX_PAYLOAD_SIZE];

    loop {
        // The server is the single owner of when a deferred Recv is due.
        let deadline = server
            .next_recv_deadline()
            .map_or(Instant::MAX, |millis| epoch.instant_at(millis));

        let event = service.next(deadline, &mut request_buf)?;

        // Time first. The router dates what an event creates by its own
        // clock, so the clock has to be current before the event is
        // handled, or the first packet after a quiet period looks old.
        server.advance(epoch.millis());

        match event {
            Event::Deadline => {}
            Event::Source(_) => on_transport(server),
            Event::Request { channel, len } => serve(
                &mut service,
                server,
                channel,
                request_buf.get(..len).unwrap_or(&[]),
                &mut response_buf,
                &mut recv_buf,
                epoch.millis(),
            ),
        }

        // Then send whatever deferred replies that event, or the passing of
        // time, resolved.
        dispatch::drive_pending(
            server,
            epoch.millis(),
            &mut recv_buf,
            &mut response_buf,
            |handle, bytes| {
                if service.complete(handle.0, bytes).is_err() {
                    pw_log::error!("mctp: deferred reply failed");
                }
            },
        );
    }
}

/// Dispatch one client request and either answer it or defer it.
fn serve<S: Sender, const N: usize>(
    service: &mut ServiceLoop<'_>,
    server: &mut Server<S, N>,
    channel: ChannelId,
    request: &[u8],
    response_buf: &mut [u8],
    recv_buf: &mut [u8],
    now_millis: u64,
) {
    let len = match dispatch::dispatch_mctp_op(request, response_buf, server, recv_buf, now_millis)
    {
        DispatchOutcome::Reply(n) => n,
        DispatchOutcome::Pending { handle } => {
            if service.defer(channel, handle.0).is_ok() {
                return;
            }
            // The reply path could not be held, so the server must not keep
            // waiting on this client's behalf.
            pw_log::error!("mctp: defer failed");
            server.cancel_recv(handle);
            wire::encode_error_response(response_buf, ResponseCode::InternalError).unwrap_or(0)
        }
    };
    if service
        .reply(channel, response_buf.get(..len).unwrap_or(&[]))
        .is_err()
    {
        pw_log::error!("mctp: reply failed");
    }
}

/// A fixed origin tying the kernel's tick clock to the `Server`'s portable
/// `u64` millisecond clock.
#[derive(Clone, Copy)]
struct Epoch(Instant);

impl Epoch {
    fn now() -> Self {
        Self(SystemClock::now())
    }

    /// Whole milliseconds elapsed since the epoch.
    fn millis(&self) -> u64 {
        (SystemClock::now() - self.0).as_millis() as u64
    }

    /// A kernel instant by which `millis()` is guaranteed to have reached
    /// `millis`, or `Instant::MAX` if that is unrepresentable.
    ///
    /// One millisecond late on purpose. `millis()` rounds down and the
    /// conversion to ticks may too, so waking exactly on time could read
    /// the clock as one short, find nothing due, and wait again on a
    /// deadline already in the past.
    fn instant_at(&self, millis: u64) -> Instant {
        self.0
            .checked_add_duration(Duration::from_millis(millis.saturating_add(1)))
            .unwrap_or(Instant::MAX)
    }
}
