// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A transport wrapper that withholds the response for a fixed number of
//! polls. Loopback answers on the first poll, so a host test cannot reach
//! a client's not-ready path without this.

use crate::{AsyncTransport, TransportError};

/// Wraps a transport and returns `Ok(None)` for the first `ready_after`
/// polls of each round-trip before forwarding to the inner transport.
pub struct Delayed<T> {
    inner: T,
    ready_after: usize,
    // None while idle, Some(n) after n withheld polls. Keeping idle
    // distinct means a poll with nothing started reaches the inner
    // transport and gets WrongState, not a spurious Ok(None).
    polls: Option<usize>,
}

impl<T> Delayed<T> {
    /// `ready_after` is how many polls return `Ok(None)` before the inner
    /// transport is polled. Zero passes through on the first poll.
    pub const fn new(inner: T, ready_after: usize) -> Self {
        Self {
            inner,
            ready_after,
            polls: None,
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }
}

impl<T: AsyncTransport> AsyncTransport for Delayed<T> {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError> {
        self.inner.start(req)?;
        self.polls = Some(0);
        Ok(())
    }

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
        if let Some(n) = self.polls {
            if n < self.ready_after {
                self.polls = Some(n + 1);
                return Ok(None);
            }
        }
        let result = self.inner.poll(resp);
        // Anything but "not yet" ends the round-trip, so the next call
        // is start and the count begins again.
        if !matches!(result, Ok(None)) {
            self.polls = None;
        }
        result
    }

    fn cancel(&mut self) -> Result<(), TransportError> {
        self.polls = None;
        self.inner.cancel()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dispatch, DispatchError, Loopback};

    struct Increment {
        seen: usize,
    }

    impl Dispatch for Increment {
        fn dispatch(
            &mut self,
            request: &[u8],
            response: &mut [u8],
        ) -> Result<usize, DispatchError> {
            self.seen += 1;
            if response.is_empty() {
                return Err(DispatchError::ResponseTooLarge);
            }
            response[0] = request[0].wrapping_add(1);
            Ok(1)
        }
    }

    fn delayed(ready_after: usize) -> Delayed<Loopback<Increment, 8>> {
        Delayed::new(Loopback::new(Increment { seen: 0 }), ready_after)
    }

    #[test]
    fn poll_returns_none_until_the_delay_elapses() {
        let mut d = delayed(3);
        let mut resp = [0u8; 4];

        assert_eq!(d.start(&[0x10]), Ok(()));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(None));
    }

    #[test]
    fn poll_after_the_delay_yields_the_response() {
        let mut d = delayed(2);
        let mut resp = [0u8; 4];

        assert_eq!(d.start(&[0x10]), Ok(()));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x11);
    }

    #[test]
    fn poll_with_nothing_in_flight_is_wrong_state() {
        let mut d = delayed(2);
        let mut resp = [0u8; 4];

        assert_eq!(d.poll(&mut resp), Err(TransportError::WrongState));
    }

    #[test]
    fn cancel_resets_the_delay_for_the_next_round_trip() {
        let mut d = delayed(2);
        let mut resp = [0u8; 4];

        assert_eq!(d.start(&[0x10]), Ok(()));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.cancel(), Ok(()));

        // Next round-trip starts the delay count from scratch.
        assert_eq!(d.start(&[0x20]), Ok(()));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(None));
        assert_eq!(d.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x21);
    }

    #[test]
    fn a_zero_delay_answers_on_the_first_poll() {
        let mut d = delayed(0);
        let mut resp = [0u8; 4];

        assert_eq!(d.start(&[0x10]), Ok(()));
        assert_eq!(d.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x11);
    }
}
