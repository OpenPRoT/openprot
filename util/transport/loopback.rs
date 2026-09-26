// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! In-process loopback transport for host tests.
//!
//! Calls the server's `Dispatch` directly, no kernel. The response is
//! ready on the first `poll`, so this cannot test not-ready handling.

use crate::{AsyncTransport, Dispatch, Transport, TransportError};

/// In-process transport over an owned server.
///
/// `N` sizes the async response buffer held between `start` and `poll`.
/// The blocking path writes into the caller's buffer directly.
pub struct Loopback<D, const N: usize> {
    server: D,
    pending: Option<usize>,
    held: [u8; N],
}

impl<D, const N: usize> Loopback<D, N> {
    pub const fn new(server: D) -> Self {
        Self {
            server,
            pending: None,
            held: [0u8; N],
        }
    }

    pub fn server(&self) -> &D {
        &self.server
    }
}

impl<D: Dispatch, const N: usize> Transport for Loopback<D, N> {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        if self.pending.is_some() {
            return Err(TransportError::WrongState);
        }
        Ok(self.server.dispatch(req, resp)?)
    }
}

impl<D: Dispatch, const N: usize> AsyncTransport for Loopback<D, N> {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError> {
        if self.pending.is_some() {
            return Err(TransportError::WrongState);
        }
        let len = self.server.dispatch(req, &mut self.held)?;
        self.pending = Some(len);
        Ok(())
    }

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
        let Some(len) = self.pending else {
            return Err(TransportError::WrongState);
        };
        if len > resp.len() {
            // The held response is dropped, not kept for a retry: the
            // round-trip is over and the next call is start.
            self.pending = None;
            return Err(TransportError::TooLarge);
        }
        self.pending = None;
        resp[..len].copy_from_slice(&self.held[..len]);
        Ok(Some(len))
    }

    fn cancel(&mut self) -> Result<(), TransportError> {
        if self.pending.take().is_none() {
            return Err(TransportError::WrongState);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DispatchError;

    /// Increments the first byte of the request.
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
                return Err(DispatchError::ResponseTooSmall);
            }
            response[0] = request[0].wrapping_add(1);
            Ok(1)
        }
    }

    fn loopback() -> Loopback<Increment, 8> {
        Loopback::new(Increment { seen: 0 })
    }

    #[test]
    fn blocking_transact_answers_from_the_server() {
        let mut lb = loopback();
        let mut resp = [0u8; 4];

        assert_eq!(lb.transact(&[0x10], &mut resp), Ok(1));
        assert_eq!(resp[0], 0x11);
        assert_eq!(lb.server().seen, 1);
    }

    #[test]
    fn blocking_transact_reports_a_response_buffer_that_does_not_fit() {
        let mut lb = loopback();
        let mut resp = [0u8; 0];

        assert_eq!(
            lb.transact(&[0x10], &mut resp),
            Err(TransportError::TooLarge)
        );
    }

    #[test]
    fn async_round_trip_is_ready_on_the_first_poll() {
        let mut lb = loopback();
        let mut resp = [0u8; 4];

        assert_eq!(lb.start(&[0x20]), Ok(()));
        assert_eq!(lb.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x21);
    }

    #[test]
    fn a_second_start_while_one_is_in_flight_is_refused() {
        let mut lb = loopback();

        assert_eq!(lb.start(&[0x20]), Ok(()));
        assert_eq!(lb.start(&[0x20]), Err(TransportError::WrongState));
        assert_eq!(lb.server().seen, 1);
    }

    #[test]
    fn poll_with_nothing_in_flight_is_refused() {
        let mut lb = loopback();
        let mut resp = [0u8; 4];

        assert_eq!(lb.poll(&mut resp), Err(TransportError::WrongState));
    }

    #[test]
    fn a_response_that_does_not_fit_ends_the_round_trip() {
        let mut lb = loopback();
        let mut small = [0u8; 0];
        let mut resp = [0u8; 4];

        assert_eq!(lb.start(&[0x20]), Ok(()));
        assert_eq!(lb.poll(&mut small), Err(TransportError::TooLarge));

        assert_eq!(lb.start(&[0x30]), Ok(()));
        assert_eq!(lb.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x31);
    }

    #[test]
    fn cancel_frees_the_transport_for_the_next_request() {
        let mut lb = loopback();
        let mut resp = [0u8; 4];

        assert_eq!(lb.start(&[0x20]), Ok(()));
        assert_eq!(lb.cancel(), Ok(()));
        assert_eq!(lb.poll(&mut resp), Err(TransportError::WrongState));

        assert_eq!(lb.start(&[0x40]), Ok(()));
        assert_eq!(lb.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x41);
    }

    #[test]
    fn cancel_with_nothing_in_flight_is_refused() {
        let mut lb = loopback();

        assert_eq!(lb.cancel(), Err(TransportError::WrongState));
    }

    #[test]
    fn transact_during_a_started_round_trip_is_refused() {
        let mut lb = loopback();
        let mut resp = [0u8; 4];

        assert_eq!(lb.start(&[0x20]), Ok(()));
        assert_eq!(
            lb.transact(&[0x30], &mut resp),
            Err(TransportError::WrongState)
        );
        assert_eq!(lb.poll(&mut resp), Ok(Some(1)));
        assert_eq!(resp[0], 0x21);
    }

    #[test]
    fn a_dispatch_that_cannot_answer_is_reported_at_start() {
        let mut lb = Loopback::<Increment, 0>::new(Increment { seen: 0 });

        assert_eq!(lb.start(&[0x20]), Err(TransportError::TooLarge));
        // Failed start leaves nothing in flight.
        assert_eq!(lb.cancel(), Err(TransportError::WrongState));
    }
}
