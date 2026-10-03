// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! An MCTP transport that goes nowhere: packets come straight back in.
//!
//! On a board the server's [`Sender`](mctp_lib::Sender) hands fragments to
//! a wire. In a QEMU test there is no wire and no second chip, so both
//! endpoints live in one image. This sender queues what its router
//! fragments, and the caller feeds that queue to the *other* endpoint's
//! `Server::inbound`. One server's outbox is the other's wire.
//!
//! The queue is owned by the caller rather than by the sender, because the
//! router takes the sender by value and offers no way back to it. The
//! caller keeps [`LoopbackQueue`] and lends it to [`LoopbackSender`]:
//!
//! ```ignore
//! let out_a = LoopbackQueue::<8>::new();
//! let out_b = LoopbackQueue::<8>::new();
//! let mut a = Server::new(Eid(8), now, LoopbackSender::new(&out_a));
//! let mut b = Server::new(Eid(42), now, LoopbackSender::new(&out_b));
//! // a sends; what a queued is what b receives.
//! while let Some(pkt) = out_a.take() {
//!     b.inbound(&pkt)?;
//! }
//! ```
//!
//! Two servers, not one looping to itself: a message addressed to a
//! server's own EID has no route out and back.
//!
//! Draining in the caller's loop rather than re-entering the router keeps
//! the send path non-blocking and the borrow straight, since the router
//! owns the sender and offers no way back to it.
//!
//! What this does not model: a wire that drops, reorders, delays or
//! corrupts. Every packet arrives, in order, as soon as the caller drains.
//! A test that wants retry or timeout behaviour needs something else.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

mod queue;
mod sender;

pub use queue::{LoopbackQueue, Packet, LOOPBACK_MTU};
pub use sender::LoopbackSender;

#[cfg(test)]
mod tests;
