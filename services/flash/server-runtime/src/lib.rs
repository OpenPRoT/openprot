// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash server IPC loop.
//!
//! Serves one flash device per IPC channel, such as one per chip select. The
//! channel a request arrives on picks the device, so the wire protocol carries
//! no device id and a client can only reach the devices it was given a channel
//! to. The server decides what a request means and builds the response frame.
//! This crate only moves the bytes.
//!
//! An erase a `Server` reports [`Outcome::Pending`] for is deferred: its channel leaves the
//! WaitGroup and is polled every `POLL_INTERVAL` until done or `DEFER_TIMEOUT` expires. A
//! timed-out channel stays rejected until [`Server::recover_abandoned_erase`] confirms the
//! driver actually went idle, or `RECOVERY_GIVE_UP` elapses and it's marked dead for good.

#![no_std]

use hal_flash::Flash;
use openprot_flash_server::{Outcome, Server};
use pw_status::{Error, Result};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_error::{self as error, ErrorCode};
use util_service::{ChannelId, Event, ServiceLoop};

pub use util_service::Channel;

/// Highest channel index this loop tracks deferred/recovering state for.
const MAX_CHANNELS: usize = 8;

/// How long between re-checks of a deferred or recovering erase's `is_busy`.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

/// How long this loop waits on a deferred erase before giving up on it.
const DEFER_TIMEOUT: Duration = Duration::from_millis(5000);

/// How long this loop keeps polling a channel for `recover_abandoned_erase` before giving up.
const RECOVERY_GIVE_UP: Duration = Duration::from_millis(60_000);

/// What channel `i` is doing between requests, beyond what `Server` itself tracks.
#[derive(Clone, Copy)]
enum ChannelState {
    Idle,
    /// A client is waiting on this erase; gives up at the deadline.
    Deferred(Instant),
    /// Client gave up; polling in the background for `recover_abandoned_erase`.
    ConfirmingSafe(Instant),
    /// Recovery never confirmed idle. Permanently rejected.
    Dead,
}

/// Serve `servers[i]` on `channels[i]` through the WaitGroup `wg`.
///
/// `channels` must be non-empty, have distinct handles, and match `servers` in length (and be
/// no longer than [`MAX_CHANNELS`]). Only setup and WaitGroup failures return.
pub fn run<TFlash: Flash<Error = ErrorCode>>(
    wg: u32,
    channels: &mut [Channel],
    servers: &mut [Server<TFlash>],
    buf: &mut [u8],
) -> Result<()> {
    if channels.is_empty() || channels.len() != servers.len() || channels.len() > MAX_CHANNELS {
        return Err(Error::InvalidArgument);
    }
    let mut service = ServiceLoop::new(wg, channels)?;
    let mut state = [ChannelState::Idle; MAX_CHANNELS];
    // Fixed once polling starts so other channels' requests can't push it out.
    let mut next_poll: Option<Instant> = None;

    loop {
        let needs_poll = |s: &ChannelState| !matches!(s, ChannelState::Idle | ChannelState::Dead);
        if state[..servers.len()].iter().any(needs_poll) {
            next_poll.get_or_insert_with(|| SystemClock::now() + POLL_INTERVAL);
        } else {
            next_poll = None;
        }
        let deadline = next_poll.unwrap_or(Instant::MAX);

        match service.next(deadline, buf)? {
            Event::Request { channel, len } => {
                serve(&mut service, servers, &mut state, channel, buf, len);
            }
            Event::Deadline => {
                next_poll = None;
                poll_pending(&mut service, servers, &mut state, buf);
            }
            Event::Source(_) => {}
        }
    }
}

fn serve<TFlash: Flash<Error = ErrorCode>>(
    service: &mut ServiceLoop<'_>,
    servers: &mut [Server<TFlash>],
    state: &mut [ChannelState; MAX_CHANNELS],
    channel: ChannelId,
    buf: &mut [u8],
    len: usize,
) {
    let Some(server) = servers.get_mut(channel.index()) else {
        let _ = service.reply(channel, &[]);
        return;
    };
    match server.handle_frame(buf, len) {
        Outcome::Done(frame_len) => {
            if service
                .reply(channel, buf.get(..frame_len).unwrap_or(&[]))
                .is_err()
            {
                pw_log::error!("flash server: reply failed");
            }
        }
        Outcome::Pending => {
            let key = channel.index() as u32;
            match service.defer(channel, key) {
                Ok(()) => {
                    let deadline = SystemClock::now() + DEFER_TIMEOUT;
                    state[channel.index()] = ChannelState::Deferred(deadline);
                }
                Err(_) => {
                    // Defer bookkeeping failed; treat like a timeout and recover.
                    pw_log::error!("flash server: defer failed, answering busy and recovering");
                    let frame_len = server.abandon_pending_erase(buf, error::FLASH_GENERIC_BUSY);
                    let give_up_at = SystemClock::now() + RECOVERY_GIVE_UP;
                    state[channel.index()] = ChannelState::ConfirmingSafe(give_up_at);
                    let _ = service.reply(channel, &buf[..frame_len]);
                }
            }
        }
    }
}

/// Advances every channel that isn't `Idle` or `Dead`.
fn poll_pending<TFlash: Flash<Error = ErrorCode>>(
    service: &mut ServiceLoop<'_>,
    servers: &mut [Server<TFlash>],
    state: &mut [ChannelState; MAX_CHANNELS],
    buf: &mut [u8],
) {
    let now = SystemClock::now();
    for i in 0..servers.len() {
        match state[i] {
            ChannelState::Idle | ChannelState::Dead => {}
            ChannelState::Deferred(deadline) => {
                let Some(server) = servers.get_mut(i) else {
                    continue;
                };
                let frame_len = match server.poll_pending_erase(buf) {
                    Some(frame_len) => {
                        state[i] = ChannelState::Idle;
                        frame_len
                    }
                    None if now < deadline => continue,
                    None => {
                        pw_log::error!("flash server: erase on channel {} timed out", i as u32);
                        let give_up_at = now + RECOVERY_GIVE_UP;
                        state[i] = ChannelState::ConfirmingSafe(give_up_at);
                        server.abandon_pending_erase(buf, error::FLASH_GENERIC_TIMEOUT)
                    }
                };
                if service.complete(i as u32, &buf[..frame_len]).is_err() {
                    pw_log::error!("flash server: complete failed");
                }
            }
            ChannelState::ConfirmingSafe(give_up_at) => {
                let Some(server) = servers.get_mut(i) else {
                    continue;
                };
                if server.recover_abandoned_erase() {
                    pw_log::info!("flash server: channel {} confirmed idle again", i as u32);
                    state[i] = ChannelState::Idle;
                } else if now >= give_up_at {
                    pw_log::error!(
                        "flash server: channel {} never confirmed idle, giving up for good",
                        i as u32
                    );
                    state[i] = ChannelState::Dead;
                }
            }
        }
    }
}
