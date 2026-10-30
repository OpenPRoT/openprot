// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The event loop of a pw_kernel server that answers several IPC channels
//! and may defer a reply.
//!
//! A pw_kernel channel is a rendezvous: the caller stays blocked until the
//! handler calls `channel_respond`, and the handler stays `READABLE` for as
//! long as that transaction is unanswered, not merely until it is read. A
//! server that cannot answer a request yet (an MCTP `Recv` with no message
//! queued, a slave receive with nothing latched) therefore has to take the
//! channel out of its WaitGroup, or the unanswered transaction wakes it on
//! every iteration, and put it back only once the reply is sent.
//!
//! [`ServiceLoop`] owns that rule so its callers cannot break it. It holds
//! the WaitGroup and the channels; a caller sees only three things happen
//! ([`Event`]) and has three things it can do about a request:
//! [`reply`](ServiceLoop::reply) now, [`defer`](ServiceLoop::defer) it under
//! a key of its own choosing, and later [`complete`](ServiceLoop::complete)
//! the deferred reply by that key. How long to wait is the caller's
//! business: it passes a deadline to [`next`](ServiceLoop::next) and gets
//! [`Event::Deadline`] when it passes.
//!
//! What a request means, and when a deferred one is ready, stay with the
//! protocol crate that uses the loop.

#![no_std]
#![deny(missing_docs)]

mod channels;

use pw_status::{Error, Result};
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

pub use channels::Channel;

/// The channel a request arrived on. Only obtained from
/// [`Event::Request`], and only meaningful to the loop that issued it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelId(usize);

/// Why [`ServiceLoop::next`] returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A client request was read into the first `len` bytes of the buffer
    /// passed to `next`. The caller must [`reply`](ServiceLoop::reply) to
    /// or [`defer`](ServiceLoop::defer) `channel` before calling `next`
    /// again; that client stays blocked until it is answered.
    Request {
        /// Where the request came from, and where its answer goes.
        channel: ChannelId,
        /// Length of the request.
        len: usize,
    },
    /// A wake source added with [`add_source`](ServiceLoop::add_source)
    /// raised its signals. Carries that source's handle.
    Source(u32),
    /// The deadline passed to `next` went by with nothing else to do.
    Deadline,
}

/// A WaitGroup, the channels it serves, and their deferred replies.
pub struct ServiceLoop<'a> {
    wg: u32,
    channels: &'a mut [Channel],
    signals: Signals,
}

impl<'a> ServiceLoop<'a> {
    /// Serve `channels` through the WaitGroup `wg`.
    ///
    /// Fails with `InvalidArgument` if two channels share a handle, since a
    /// wake could not then be attributed to one of them.
    pub fn new(wg: u32, channels: &'a mut [Channel]) -> Result<Self> {
        if !channels::distinct(channels) {
            return Err(Error::InvalidArgument);
        }
        for ch in channels.iter() {
            arm(wg, ch)?;
        }
        Ok(Self {
            wg,
            channels,
            signals: Signals::READABLE,
        })
    }

    /// Also wake on `signals` of `handle`: a transport notification, an
    /// IRQ. Reported as [`Event::Source`].
    ///
    /// The loop does not acknowledge or clear a source. A level-triggered
    /// one (a `USER` signal stays set until its owner clears it) must be
    /// quiesced by whoever handles the event, or `next` returns it again
    /// immediately, forever.
    ///
    /// Fails with `InvalidArgument` if `handle` is one of the channels.
    pub fn add_source(&mut self, handle: u32, signals: Signals) -> Result<()> {
        if channels::position(self.channels, handle).is_some() {
            return Err(Error::InvalidArgument);
        }
        syscall::wait_group_add(self.wg, handle, signals, handle as usize)?;
        self.signals |= signals;
        Ok(())
    }

    /// Block until a request arrives, a source fires, or `deadline` passes.
    ///
    /// A request is read into `request`. A channel whose read fails is
    /// logged and skipped, so one misbehaving client never stops service to
    /// the others; an error from the WaitGroup itself is returned.
    pub fn next(&mut self, deadline: Instant, request: &mut [u8]) -> Result<Event> {
        loop {
            let woken = match syscall::object_wait(self.wg, self.signals, deadline) {
                Ok(ev) => ev.user_data as u32,
                Err(Error::DeadlineExceeded) => return Ok(Event::Deadline),
                Err(e) => return Err(e),
            };
            let Some(index) = channels::position(self.channels, woken) else {
                return Ok(Event::Source(woken));
            };
            // Non-blocking: the WaitGroup only reports the channel once
            // READABLE is set.
            match syscall::channel_read(woken, 0, request) {
                Ok(len) => {
                    return Ok(Event::Request {
                        channel: ChannelId(index),
                        len,
                    });
                }
                Err(_) => pw_log::error!("service: channel_read failed"),
            }
        }
    }

    /// Answer the request on `channel` now.
    pub fn reply(&mut self, channel: ChannelId, response: &[u8]) -> Result<()> {
        let ch = self.channels.get(channel.0).ok_or(Error::InvalidArgument)?;
        syscall::channel_respond(ch.handle(), response)
    }

    /// Leave the request on `channel` unanswered for now, to be answered by
    /// [`complete`](Self::complete) with the same `key`. The channel is
    /// taken out of the WaitGroup until then.
    ///
    /// On any error nothing is deferred and the channel is still being
    /// served, so the caller must `reply` instead. Fails with
    /// `AlreadyExists` if another deferred reply already uses `key`.
    pub fn defer(&mut self, channel: ChannelId, key: u32) -> Result<()> {
        if channels::position_parked(self.channels, key).is_some() {
            return Err(Error::AlreadyExists);
        }
        let ch = self
            .channels
            .get_mut(channel.0)
            .ok_or(Error::InvalidArgument)?;
        ch.park(key).map_err(|_| Error::FailedPrecondition)?;
        if let Err(e) = syscall::wait_group_remove(self.wg, ch.handle()) {
            ch.unpark();
            return Err(e);
        }
        Ok(())
    }

    /// Answer the reply deferred under `key` and serve its channel again.
    ///
    /// Fails with `NotFound` if nothing is deferred under `key`. The
    /// channel is put back in the WaitGroup even if the respond fails: that
    /// only happens when the transaction is already gone (the peer reset),
    /// and the channel is then idle and must be served again. The respond
    /// error, if any, is returned after that.
    pub fn complete(&mut self, key: u32, response: &[u8]) -> Result<()> {
        let ch = channels::position_parked(self.channels, key)
            .and_then(|i| self.channels.get_mut(i))
            .ok_or(Error::NotFound)?;
        ch.unpark();
        let responded = syscall::channel_respond(ch.handle(), response);
        arm(self.wg, ch)?;
        responded
    }
}

/// Put `ch` in `wg`. The handle doubles as `user_data`, which is how
/// `next` maps a wake back to its channel or source.
fn arm(wg: u32, ch: &Channel) -> Result<()> {
    syscall::wait_group_add(wg, ch.handle(), Signals::READABLE, ch.handle() as usize)
}
