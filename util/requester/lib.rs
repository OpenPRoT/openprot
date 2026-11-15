// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The requester side of a never-blocking client.
//!
//! `util_service` is the loop for a server that answers channels. This crate
//! is its counterpart for a client that asks them: it keeps, per service
//! channel, the one command in flight, the commands waiting behind it, and
//! the deadline the one in flight must meet.
//!
//! A [`Lane`] owns an [`AsyncTransport`] and nothing kernel-specific. It
//! never waits and never reads a clock: the caller passes deadlines in and
//! asks whether one has passed. Registering the transport's handle with a
//! WaitGroup, sleeping until the earliest deadline, and deciding what a
//! response means all stay with the caller, so the lane runs on a host
//! against a fake transport.
//!
//! A channel carries one transaction at a time, so commands for one service
//! queue on its lane and start in the order they were pushed.
//!
//! # Using a lane
//!
//! ```ignore
//! lane.push(cmd)?;                       // queue it
//! if let Some(started) = lane.start_next(|c| Some(now + c.timeout()))? {
//!     /* register lane.transport() with the WaitGroup */
//! }
//! // ...the WaitGroup wakes the loop...
//! if let Some(done) = lane.poll(&mut resp)? { /* decode resp[..done.len] */ }
//! if lane.expired(&now) { lane.cancel()?; /* treat the command as failed */ }
//! ```

#![no_std]

use heapless::Deque;
use util_transport::{AsyncTransport, TransportError};

/// Longest request a [`Command`] may encode.
pub const MAX_REQUEST: usize = 32;

/// A request a lane can send.
pub trait Command: Copy {
    /// Write the request frame into `out`, returning its length, or `None`
    /// if it does not fit.
    fn encode(&self, out: &mut [u8]) -> Option<usize>;
}

/// Why a lane operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneError {
    /// The command did not encode into [`MAX_REQUEST`] bytes.
    Encode,
    /// The transport refused or failed the operation.
    Transport(TransportError),
}

/// A command that did not get through, handed back so the caller can decide
/// what its failure means. The lane is idle afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failed<C> {
    pub error: LaneError,
    pub cmd: C,
}

/// A command whose response has arrived in the buffer given to
/// [`Lane::poll`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done<C> {
    pub cmd: C,
    /// Length of the response in the caller's buffer.
    pub len: usize,
}

struct Inflight<C, I> {
    cmd: C,
    deadline: Option<I>,
}

/// One service channel: a transport, the command in flight on it, and up to
/// `Q` commands waiting. `I` is the caller's instant type.
pub struct Lane<T, C, I, const Q: usize> {
    tr: T,
    inflight: Option<Inflight<C, I>>,
    queue: Deque<C, Q>,
}

impl<T: AsyncTransport, C: Command, I: Copy, const Q: usize> Lane<T, C, I, Q> {
    pub const fn new(tr: T) -> Self {
        Self {
            tr,
            inflight: None,
            queue: Deque::new(),
        }
    }

    /// The transport, for the caller to register its handle with a WaitGroup.
    pub fn transport(&self) -> &T {
        &self.tr
    }

    /// Queue `cmd` behind whatever is already waiting. Gives it back if the
    /// queue is full.
    pub fn push(&mut self, cmd: C) -> Result<(), C> {
        self.queue.push_back(cmd)
    }

    /// Nothing in flight and nothing waiting.
    pub fn is_idle(&self) -> bool {
        self.inflight.is_none() && self.queue.is_empty()
    }

    /// The command in flight, if any.
    pub fn in_flight(&self) -> Option<&C> {
        self.inflight.as_ref().map(|i| &i.cmd)
    }

    /// Commands waiting behind the one in flight.
    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// The deadline of the command in flight, if it has one.
    pub fn deadline(&self) -> Option<I> {
        self.inflight.as_ref().and_then(|i| i.deadline)
    }

    /// Whether the command in flight has a deadline at or before `now`.
    pub fn expired(&self, now: &I) -> bool
    where
        I: PartialOrd,
    {
        self.deadline().is_some_and(|d| d <= *now)
    }

    /// Start the next waiting command if the channel is free. Returns the
    /// command that started, so the caller can register for its wake-up, or
    /// `None` if the channel is busy or nothing is waiting.
    ///
    /// `deadline_of` gives the started command's deadline, if it has one.
    /// If the command cannot be started it is handed back in the error and
    /// is not retried: the caller fails it, or fails closed.
    pub fn start_next(
        &mut self,
        deadline_of: impl FnOnce(&C) -> Option<I>,
    ) -> Result<Option<C>, Failed<C>> {
        if self.inflight.is_some() {
            return Ok(None);
        }
        let Some(cmd) = self.queue.pop_front() else {
            return Ok(None);
        };
        let mut req = [0u8; MAX_REQUEST];
        let Some(len) = cmd.encode(&mut req).filter(|n| *n <= MAX_REQUEST) else {
            return Err(Failed {
                error: LaneError::Encode,
                cmd,
            });
        };
        if let Err(e) = self.tr.start(req.get(..len).unwrap_or(&[])) {
            return Err(Failed {
                error: LaneError::Transport(e),
                cmd,
            });
        }
        self.inflight = Some(Inflight {
            deadline: deadline_of(&cmd),
            cmd,
        });
        Ok(Some(cmd))
    }

    /// Collect the response if it has landed, into `resp`. `Ok(None)` means
    /// it has not, or nothing is in flight.
    ///
    /// A transport error ends the round-trip, as `AsyncTransport` says it
    /// does, so the command comes back in the error and the lane is idle.
    pub fn poll(&mut self, resp: &mut [u8]) -> Result<Option<Done<C>>, Failed<C>> {
        if self.inflight.is_none() {
            return Ok(None);
        }
        match self.tr.poll(resp) {
            Ok(None) => Ok(None),
            Ok(Some(len)) => Ok(self.inflight.take().map(|i| Done { cmd: i.cmd, len })),
            Err(e) => match self.inflight.take() {
                Some(i) => Err(Failed {
                    error: LaneError::Transport(e),
                    cmd: i.cmd,
                }),
                None => Ok(None),
            },
        }
    }

    /// Abandon the command in flight and return it. The lane is idle
    /// afterwards even if the transport reports an error, since nothing more
    /// can be done with that round-trip.
    pub fn cancel(&mut self) -> Result<Option<C>, LaneError> {
        let Some(inflight) = self.inflight.take() else {
            return Ok(None);
        };
        self.tr.cancel().map_err(LaneError::Transport)?;
        Ok(Some(inflight.cmd))
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Cmd {
        A(u8),
        B,
        /// Never fits a request buffer.
        Huge,
    }

    impl Command for Cmd {
        fn encode(&self, out: &mut [u8]) -> Option<usize> {
            match self {
                Cmd::A(n) => {
                    *out.get_mut(0)? = 1;
                    *out.get_mut(1)? = *n;
                    Some(2)
                }
                Cmd::B => {
                    *out.get_mut(0)? = 2;
                    Some(1)
                }
                Cmd::Huge => Some(MAX_REQUEST + 1),
            }
        }
    }

    /// A transport that follows the `AsyncTransport` contract and is driven
    /// by the test: it answers only when told to.
    #[derive(Default)]
    struct Fake {
        in_flight: bool,
        reply: Option<Vec<u8>>,
        sent: Vec<Vec<u8>>,
        cancels: u32,
        fail_start: bool,
        fail_poll: bool,
        fail_cancel: bool,
    }

    impl AsyncTransport for Fake {
        fn start(&mut self, req: &[u8]) -> Result<(), TransportError> {
            if self.in_flight {
                return Err(TransportError::WrongState);
            }
            if self.fail_start {
                return Err(TransportError::Failed);
            }
            self.sent.push(req.to_vec());
            self.in_flight = true;
            Ok(())
        }

        fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
            if !self.in_flight {
                return Err(TransportError::WrongState);
            }
            if self.fail_poll {
                self.in_flight = false;
                return Err(TransportError::Failed);
            }
            let Some(reply) = self.reply.take() else {
                return Ok(None);
            };
            self.in_flight = false;
            resp[..reply.len()].copy_from_slice(&reply);
            Ok(Some(reply.len()))
        }

        fn cancel(&mut self) -> Result<(), TransportError> {
            if !self.in_flight {
                return Err(TransportError::WrongState);
            }
            self.in_flight = false;
            self.cancels += 1;
            if self.fail_cancel {
                return Err(TransportError::Failed);
            }
            Ok(())
        }
    }

    type TestLane = Lane<Fake, Cmd, u32, 2>;

    fn lane() -> TestLane {
        Lane::new(Fake::default())
    }

    fn no_deadline(_: &Cmd) -> Option<u32> {
        None
    }

    #[test]
    fn commands_start_in_push_order_one_at_a_time() {
        let mut l = lane();
        l.push(Cmd::A(7)).unwrap();
        l.push(Cmd::B).unwrap();

        assert_eq!(l.start_next(no_deadline), Ok(Some(Cmd::A(7))));
        // The channel is busy: the second command waits and nothing is sent.
        assert_eq!(l.start_next(no_deadline), Ok(None));
        assert_eq!(l.queued(), 1);
        assert_eq!(l.in_flight(), Some(&Cmd::A(7)));
        assert_eq!(l.transport().sent, std::vec![std::vec![1, 7]]);

        l.tr.reply = Some(std::vec![0]);
        let mut resp = [0u8; 4];
        let done = l.poll(&mut resp).unwrap().unwrap();
        assert_eq!(
            done,
            Done {
                cmd: Cmd::A(7),
                len: 1
            }
        );
        assert!(l.in_flight().is_none());

        assert_eq!(l.start_next(no_deadline), Ok(Some(Cmd::B)));
        assert_eq!(l.transport().sent.len(), 2);
        assert_eq!(l.transport().sent[1], std::vec![2]);
    }

    #[test]
    fn push_gives_the_command_back_when_the_queue_is_full() {
        let mut l = lane();
        l.push(Cmd::A(1)).unwrap();
        l.push(Cmd::A(2)).unwrap();
        assert_eq!(l.push(Cmd::A(3)), Err(Cmd::A(3)));
        assert_eq!(l.queued(), 2);
    }

    #[test]
    fn nothing_waiting_starts_nothing() {
        let mut l = lane();
        assert!(l.is_idle());
        assert_eq!(l.start_next(no_deadline), Ok(None));
        assert!(l.transport().sent.is_empty());
    }

    #[test]
    fn poll_before_the_response_keeps_the_command_in_flight() {
        let mut l = lane();
        l.push(Cmd::B).unwrap();
        l.start_next(no_deadline).unwrap();
        let mut resp = [0u8; 4];

        assert_eq!(l.poll(&mut resp), Ok(None));
        assert_eq!(l.in_flight(), Some(&Cmd::B));
        assert!(!l.is_idle());
    }

    #[test]
    fn poll_with_nothing_in_flight_is_none_and_does_not_touch_the_transport() {
        let mut l = lane();
        let mut resp = [0u8; 4];
        assert_eq!(l.poll(&mut resp), Ok(None));
    }

    #[test]
    fn the_response_is_left_in_the_callers_buffer() {
        let mut l = lane();
        l.push(Cmd::B).unwrap();
        l.start_next(no_deadline).unwrap();
        l.tr.reply = Some(std::vec![0, 9]);
        let mut resp = [0u8; 4];

        let done = l.poll(&mut resp).unwrap().unwrap();
        assert_eq!(&resp[..done.len], &[0, 9]);
        assert!(l.is_idle());
    }

    #[test]
    fn a_failed_start_hands_the_command_back_and_leaves_the_lane_idle() {
        let mut l = lane();
        l.tr.fail_start = true;
        l.push(Cmd::A(5)).unwrap();

        assert_eq!(
            l.start_next(no_deadline),
            Err(Failed {
                error: LaneError::Transport(TransportError::Failed),
                cmd: Cmd::A(5)
            })
        );
        assert!(l.is_idle());
    }

    #[test]
    fn a_command_that_does_not_encode_is_not_sent() {
        let mut l = lane();
        l.push(Cmd::Huge).unwrap();

        assert_eq!(
            l.start_next(no_deadline),
            Err(Failed {
                error: LaneError::Encode,
                cmd: Cmd::Huge
            })
        );
        assert!(l.transport().sent.is_empty());
        assert!(l.is_idle());
    }

    #[test]
    fn a_poll_error_ends_the_round_trip_and_returns_the_command() {
        let mut l = lane();
        l.push(Cmd::A(2)).unwrap();
        l.start_next(no_deadline).unwrap();
        l.tr.fail_poll = true;
        let mut resp = [0u8; 4];

        assert_eq!(
            l.poll(&mut resp),
            Err(Failed {
                error: LaneError::Transport(TransportError::Failed),
                cmd: Cmd::A(2)
            })
        );
        assert!(l.is_idle());

        // The channel is usable again.
        l.tr.fail_poll = false;
        l.push(Cmd::B).unwrap();
        assert_eq!(l.start_next(no_deadline), Ok(Some(Cmd::B)));
    }

    #[test]
    fn cancel_returns_the_command_frees_the_channel_and_keeps_the_queue() {
        let mut l = lane();
        l.push(Cmd::A(1)).unwrap();
        l.push(Cmd::B).unwrap();
        l.start_next(no_deadline).unwrap();

        assert_eq!(l.cancel(), Ok(Some(Cmd::A(1))));
        assert_eq!(l.transport().cancels, 1);
        assert!(l.in_flight().is_none());
        assert_eq!(l.queued(), 1);

        // The waiting command goes next.
        assert_eq!(l.start_next(no_deadline), Ok(Some(Cmd::B)));
    }

    #[test]
    fn cancel_with_nothing_in_flight_is_none() {
        let mut l = lane();
        assert_eq!(l.cancel(), Ok(None));
        assert_eq!(l.transport().cancels, 0);
    }

    #[test]
    fn a_failed_cancel_still_leaves_the_lane_idle() {
        let mut l = lane();
        l.push(Cmd::B).unwrap();
        l.start_next(no_deadline).unwrap();
        l.tr.fail_cancel = true;

        assert_eq!(
            l.cancel(),
            Err(LaneError::Transport(TransportError::Failed))
        );
        assert!(l.in_flight().is_none());
    }

    #[test]
    fn a_deadline_expires_at_and_after_its_instant() {
        let mut l = lane();
        l.push(Cmd::B).unwrap();
        l.start_next(|_| Some(100)).unwrap();

        assert_eq!(l.deadline(), Some(100));
        assert!(!l.expired(&99));
        assert!(l.expired(&100));
        assert!(l.expired(&101));
    }

    #[test]
    fn no_deadline_never_expires() {
        let mut l = lane();
        l.push(Cmd::B).unwrap();
        l.start_next(no_deadline).unwrap();

        assert_eq!(l.deadline(), None);
        assert!(!l.expired(&u32::MAX));
    }

    #[test]
    fn an_idle_lane_has_no_deadline() {
        let l = lane();
        assert_eq!(l.deadline(), None);
        assert!(!l.expired(&u32::MAX));
    }

    #[test]
    fn the_deadline_belongs_to_the_command_in_flight_not_the_next() {
        let mut l = lane();
        l.push(Cmd::A(1)).unwrap();
        l.push(Cmd::B).unwrap();
        l.start_next(|c| if *c == Cmd::A(1) { Some(50) } else { None })
            .unwrap();
        assert_eq!(l.deadline(), Some(50));

        l.tr.reply = Some(std::vec![0]);
        let mut resp = [0u8; 4];
        l.poll(&mut resp).unwrap();
        l.start_next(|c| if *c == Cmd::A(1) { Some(50) } else { None })
            .unwrap();
        assert_eq!(l.deadline(), None);
    }
}
