// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Orchestrator side of the pldm-notify wire: decodes one notification from the
//! PLDM service and says what it means in the orchestrator's terms.
//!
//! Nothing here reaches the state machine. The notification arrives before the
//! FD has accepted the UA's request, so there is nothing to authenticate yet;
//! the machine hears about an update later, when the candidate is complete.
//!
//! [`dispatch`] is pure, no IPC and no globals, so it tests on the host. The
//! kernel wait-and-respond loop is elsewhere, and it answers before it acts on
//! the notification, since `channel_respond` has to be prompt.
//!
//! Depends on the wire crate only, never on the PLDM stack and never on
//! orchestrator-sm, the same rule the orchestrator's HAL adapters follow.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use openprot_pldm_notify_api::{Request, Response};

/// What the PLDM service reported, in the orchestrator's words.
///
/// Translating here is the point of this crate: [`NotifyOp`] is PLDM's
/// vocabulary across a process boundary, and letting it reach the run loop is
/// the coupling the adapter exists to prevent.
///
/// [`NotifyOp`]: openprot_pldm_notify_api::NotifyOp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notification {
    /// A UA sent a `RequestUpdate`. The orchestrator's verdict decides whether
    /// the FD accepts; no firmware bytes have moved yet.
    UpdateRequested,
}

/// Decodes one notification, encodes an accept or reject, and returns what
/// was reported.
///
/// `accept` is the caller's verdict: true to accept the request, false to
/// reject it. The Notification is returned either way so the caller knows
/// what was requested (for logging on reject, watchdog arming on accept).
///
/// Returns how many bytes of `response` are the answer. `response` must be at
/// least [`MAX_RESPONSE_SIZE`](openprot_pldm_notify_api::MAX_RESPONSE_SIZE)
/// bytes; a shorter one gets nothing written and a return of 0, which the
/// caller must not put on the wire.
///
/// Never panics. A frame that does not decode gets nothing written, a return
/// of 0, and nothing reported.
pub fn dispatch(
    request: &[u8],
    response: &mut [u8],
    accept: bool,
) -> (usize, Option<Notification>) {
    let notification = match Request::decode(request) {
        Ok(Request::UpdateRequested) => Notification::UpdateRequested,
        // non_exhaustive: a future op this build doesn't know, or a bad frame.
        // No answer, so the UA's transact times out and retries.
        Ok(_) | Err(_) => return (0, None),
    };
    let answer = if accept {
        Response::Accepted
    } else {
        Response::Rejected
    };
    match answer.encode(response) {
        Ok(len) => (len, Some(notification)),
        Err(_) => (0, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_pldm_notify_api::{NotifyOp, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};

    fn round_trip(request: Request, accept: bool) -> (Option<Notification>, Option<Response>) {
        let mut out = [0u8; MAX_REQUEST_SIZE];
        let len = request.encode(&mut out).unwrap();
        let mut back = [0u8; MAX_RESPONSE_SIZE];
        let (answer_len, notification) = dispatch(&out[..len], &mut back, accept);
        (notification, Response::decode(&back[..answer_len]).ok())
    }

    #[test]
    fn an_update_request_that_the_caller_accepts_is_reported_and_confirmed() {
        assert_eq!(
            round_trip(Request::UpdateRequested, true),
            (
                Some(Notification::UpdateRequested),
                Some(Response::Accepted)
            )
        );
    }

    #[test]
    fn a_rejected_request_update_is_still_reported() {
        assert_eq!(
            round_trip(Request::UpdateRequested, false),
            (
                Some(Notification::UpdateRequested),
                Some(Response::Rejected)
            )
        );
    }

    #[test]
    fn a_frame_that_does_not_decode_gets_no_answer_and_reports_nothing() {
        let mut back = [0xFFu8; MAX_RESPONSE_SIZE];
        for bad in [
            &[0u8][..],
            &[0xFF, 0, 0, 0][..],
            &[NotifyOp::UpdateRequested as u8, 0, 1, 0][..],
            &[NotifyOp::UpdateRequested as u8, 1, 0, 0][..],
        ] {
            assert_eq!(dispatch(bad, &mut back, true), (0, None));
            assert!(back.iter().all(|&b| b == 0xFF));
        }
    }

    #[test]
    fn a_response_buffer_below_the_minimum_gets_nothing() {
        let mut out = [0u8; MAX_REQUEST_SIZE];
        let len = Request::UpdateRequested.encode(&mut out).unwrap();
        let mut back = [0xFFu8; MAX_RESPONSE_SIZE - 1];
        assert_eq!(dispatch(&out[..len], &mut back, true), (0, None));
        assert!(back.iter().all(|&b| b == 0xFF));
    }
}
