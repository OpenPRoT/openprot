// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-buildable core of the i3c server: the target-owning [`Server`] state
//! and the request [`dispatch`] against the [`I3cTarget`] facade.
//!
//! No syscalls live here, so the request handling and the single-frame inbound
//! latch are verified in host tests. The kernel-tagged `i3c_server_runtime`
//! wraps this in the Pigweed WaitGroup loop and supplies the IRQ/IPC syscalls.

#![no_std]

pub mod loopback;

pub use loopback::LoopbackTransport;

use i3c_api::{decode_request, encode_response, I3cOp, I3cStatus, HEADER, MAX_PAYLOAD};
use openprot_hal_blocking::i3c_hardware::I3cTarget;

/// Depth of the inbound frame ring.
///
/// Sized to cover the window between the IRQ latching a frame and the client
/// draining it over IPC — a cross-process round-trip, plus a host controller
/// that polls and may resend — and, crucially, to hold every fragment of one
/// multi-fragment MCTP message: the controller bursts a message's fragments
/// back-to-back and they are only consumed once the whole message reassembles,
/// so the ring must be at least as deep as the largest message's fragment count
/// (a ~960 B PLDM chunk is ~5 fragments at the 241 B MTU). The ring must not
/// overflow in normal flow: a dropped frame is an unrecoverable hole in MCTP
/// reassembly, so overflow is an error path only (see [`Inbound::DroppedFull`]).
pub const RX_RING: usize = 6;

/// Outcome of draining one inbound frame in [`Server::latch_inbound`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inbound {
    /// A frame was read from the target and queued into the ring.
    Latched,
    /// Nothing was available to latch (empty or a zero-length descriptor).
    Empty,
    /// A frame was read but dropped because the ring was full — a lost MCTP
    /// fragment. [`RX_RING`] is sized so this should not occur in normal flow;
    /// the runtime logs it.
    DroppedFull,
}

/// The I3C target the server owns, plus its IPC channel, IRQ handle, the
/// inbound frame ring, and the outbound "response staged, awaiting read" flag.
///
/// Inbound frames are queued in a [`RX_RING`]-deep FIFO. MCTP over I3C is
/// request/response, but the client draining a latched frame spans a
/// cross-process round-trip, during which the controller can land the next
/// frame (a resend, or the next fragment); the ring holds those so none is
/// lost. `latch_inbound` pushes the oldest-first; `Recv` pops it. The ring
/// never overwrites an unread frame — on overflow a frame is dropped and logged
/// instead (an error path `RX_RING` is sized to avoid). The bus is never
/// blocked.
///
/// Outbound has back-pressure. A `Send` stages a response for the controller's
/// next private read and sets [`tx_pending`](Server::tx_pending); the flag
/// clears only when the controller has read it, signalled by
/// [`notify_response_read`](Server::notify_response_read) on a
/// [`TargetEvent::ResponseRead`]. The runtime uses the flag to hold a second
/// `Send` until the first has been read, so fragments are not staged faster
/// than the controller drains them. `dispatch` itself always stages (the
/// gating is the runtime's), so the in-process loopback path is unaffected.
///
/// [`TargetEvent::ResponseRead`]: openprot_hal_blocking::i3c_hardware::TargetEvent::ResponseRead
pub struct Server<T> {
    /// IPC channel handle carrying the transport protocol.
    pub channel: u32,
    /// IRQ handle for the I3C controller.
    pub irq: u32,
    /// The controller driver implementing the facade.
    pub target: T,
    /// Inbound frame ring buffers, one `MAX_PAYLOAD` slot per ring entry.
    rx: [[u8; MAX_PAYLOAD]; RX_RING],
    /// Payload length latched in each ring slot.
    rx_len: [usize; RX_RING],
    /// Index of the oldest queued frame (the next `Recv` pops this slot).
    rx_head: usize,
    /// Number of frames currently queued in the ring.
    rx_count: usize,
    tx_pending: bool,
}

impl<T> Server<T> {
    /// Bind the server to one channel, one IRQ, and one target driver.
    pub const fn new(channel: u32, irq: u32, target: T) -> Self {
        Self {
            channel,
            irq,
            target,
            rx: [[0u8; MAX_PAYLOAD]; RX_RING],
            rx_len: [0usize; RX_RING],
            rx_head: 0,
            rx_count: 0,
            tx_pending: false,
        }
    }

    /// Whether any inbound frame is queued and waiting for a `Recv`.
    pub fn has_frame(&self) -> bool {
        self.rx_count > 0
    }

    /// Whether a staged transmit is still awaiting the controller's private
    /// read. The runtime holds a further `Send` while this is set.
    pub fn tx_pending(&self) -> bool {
        self.tx_pending
    }

    /// Clear the outbound back-pressure flag: the controller has read the staged
    /// response. Called by the runtime on [`TargetEvent::ResponseRead`].
    ///
    /// [`TargetEvent::ResponseRead`]: openprot_hal_blocking::i3c_hardware::TargetEvent::ResponseRead
    pub fn notify_response_read(&mut self) {
        self.tx_pending = false;
    }
}

impl<T: I3cTarget> Server<T> {
    /// Drain one inbound frame into the ring after a [`TargetEvent::InboundReady`].
    ///
    /// Reads one frame from the target and, if the ring has room, queues it at
    /// the tail; returns [`Inbound`] describing what happened. The runtime calls
    /// this in a loop until it returns [`Inbound::Empty`], so a single IRQ that
    /// covers several hardware frames drains them all.
    ///
    /// A zero-length descriptor is not a real frame: the RX-descriptor interrupt
    /// can fire with no payload (seen on the emulator around the controller's
    /// write/private-read command traffic), and `read_frame` then reports
    /// `Some(0)`. A zero-length inbound is never a valid MCTP frame, so it is
    /// treated as "nothing arrived" ([`Inbound::Empty`]).
    ///
    /// When the ring is full the frame is still read out of the hardware (to
    /// keep the RX path moving) but dropped — [`Inbound::DroppedFull`]. Dropping
    /// the new frame is the lesser evil: overwriting a queued one would lose an
    /// earlier MCTP fragment. `RX_RING` is sized so this does not happen in
    /// normal flow.
    pub fn latch_inbound(&mut self) -> Result<Inbound, T::Error> {
        if self.rx_count >= RX_RING {
            let mut scratch = [0u8; MAX_PAYLOAD];
            return Ok(match self.target.read_frame(&mut scratch)? {
                Some(n) if n > 0 => Inbound::DroppedFull,
                _ => Inbound::Empty,
            });
        }
        let tail = (self.rx_head + self.rx_count) % RX_RING;
        Ok(match self.target.read_frame(&mut self.rx[tail])? {
            Some(n) if n > 0 => {
                self.rx_len[tail] = n.min(MAX_PAYLOAD);
                self.rx_count += 1;
                Inbound::Latched
            }
            _ => Inbound::Empty,
        })
    }
}

fn status_only(resp: &mut [u8], status: I3cStatus) -> usize {
    match encode_response(status, &[], resp) {
        Some(n) => n,
        None => match resp.first_mut() {
            Some(head) => {
                *head = status as u8;
                1
            }
            None => 0,
        },
    }
}

/// Dispatch one client request against the target, writing the response into
/// `resp` and returning its length.
///
/// `Recv` consumes the latch. This function performs no syscalls; the runtime
/// manages the `USER` notification around it.
pub fn dispatch<T: I3cTarget>(srv: &mut Server<T>, req: &[u8], resp: &mut [u8]) -> usize {
    let Some((op, payload)) = decode_request(req) else {
        return status_only(resp, I3cStatus::InvalidOp);
    };
    match op {
        I3cOp::Send => {
            if payload.len() > MAX_PAYLOAD {
                return status_only(resp, I3cStatus::TooLong);
            }
            match srv.target.send(payload) {
                Ok(()) => {
                    // Staged for the controller's next private read; hold further
                    // sends until it has been read (cleared by the runtime on
                    // ResponseRead).
                    srv.tx_pending = true;
                    status_only(resp, I3cStatus::Ok)
                }
                Err(_) => status_only(resp, I3cStatus::Internal),
            }
        }
        I3cOp::Recv => {
            if srv.rx_count == 0 {
                return status_only(resp, I3cStatus::NoData);
            }
            let head = srv.rx_head;
            let cap = resp.len().saturating_sub(HEADER);
            let n = srv.rx_len[head].min(cap);
            let out = encode_response(I3cStatus::Ok, srv.rx[head].get(..n).unwrap_or(&[]), resp);
            // Pop the oldest frame whether or not the encode succeeded: a
            // response buffer too small to hold it would otherwise wedge the
            // ring on the same frame forever.
            srv.rx_head = (srv.rx_head + 1) % RX_RING;
            srv.rx_count -= 1;
            out.unwrap_or_else(|| status_only(resp, I3cStatus::Internal))
        }
        I3cOp::DynamicAddress => match srv.target.dynamic_address() {
            Some(addr) => encode_response(I3cStatus::Ok, &[addr.as_u8()], resp)
                .unwrap_or_else(|| status_only(resp, I3cStatus::Internal)),
            None => status_only(resp, I3cStatus::Unassigned),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use i3c_api::{decode_response, encode_request, MAX_FRAME};
    use openprot_hal_blocking::i3c_hardware::{DynamicAddress, TargetEvent};

    /// Capacity of the test target's own inbound queue; larger than `RX_RING`
    /// so overflow of the server ring can be exercised (tracks `RX_RING`).
    const FAKE_INBOUND: usize = RX_RING + 4;

    struct FakeTarget {
        addr: Option<u8>,
        /// Pending inbound frames, oldest first; `read_frame` pops the front.
        frames: [[u8; MAX_PAYLOAD]; FAKE_INBOUND],
        frame_lens: [usize; FAKE_INBOUND],
        frame_head: usize,
        frame_count: usize,
        sent: [u8; MAX_PAYLOAD],
        sent_len: usize,
    }

    impl Default for FakeTarget {
        fn default() -> Self {
            Self {
                addr: None,
                frames: [[0u8; MAX_PAYLOAD]; FAKE_INBOUND],
                frame_lens: [0usize; FAKE_INBOUND],
                frame_head: 0,
                frame_count: 0,
                sent: [0u8; MAX_PAYLOAD],
                sent_len: 0,
            }
        }
    }

    impl FakeTarget {
        /// Queue one inbound frame for a later `read_frame`.
        fn push_inbound(&mut self, data: &[u8]) {
            assert!(
                self.frame_count < FAKE_INBOUND,
                "test inbound queue overflow"
            );
            let tail = (self.frame_head + self.frame_count) % FAKE_INBOUND;
            let n = data.len().min(MAX_PAYLOAD);
            self.frames[tail][..n].copy_from_slice(&data[..n]);
            self.frame_lens[tail] = n;
            self.frame_count += 1;
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FakeError;

    impl I3cTarget for FakeTarget {
        type Error = FakeError;
        fn enable(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
            if self.frame_count > 0 {
                Ok(TargetEvent::InboundReady)
            } else {
                Ok(TargetEvent::None)
            }
        }
        fn read_frame(&mut self, buf: &mut [u8]) -> Result<Option<usize>, Self::Error> {
            if self.frame_count == 0 {
                return Ok(None);
            }
            let head = self.frame_head;
            let n = self.frame_lens[head];
            if n > buf.len() {
                return Err(FakeError);
            }
            buf[..n].copy_from_slice(&self.frames[head][..n]);
            self.frame_head = (self.frame_head + 1) % FAKE_INBOUND;
            self.frame_count -= 1;
            Ok(Some(n))
        }
        fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            let n = data.len().min(MAX_PAYLOAD);
            self.sent[..n].copy_from_slice(&data[..n]);
            self.sent_len = n;
            Ok(())
        }
        fn dynamic_address(&self) -> Option<DynamicAddress> {
            self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
        }
    }

    fn srv() -> Server<FakeTarget> {
        Server::new(1, 2, FakeTarget::default())
    }

    #[test]
    fn send_dispatches_to_target() {
        let mut s = srv();
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Send, b"pong", &mut req).unwrap();
        let rn = dispatch(&mut s, &req[..n], &mut resp);
        assert_eq!(decode_response(&resp[..rn]), Some((I3cStatus::Ok, &[][..])));
        assert_eq!(&s.target.sent[..s.target.sent_len], b"pong");
    }

    #[test]
    fn send_sets_tx_pending_until_response_read() {
        let mut s = srv();
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        assert!(!s.tx_pending(), "nothing staged yet");

        let n = encode_request(I3cOp::Send, b"pong", &mut req).unwrap();
        let rn = dispatch(&mut s, &req[..n], &mut resp);
        assert_eq!(decode_response(&resp[..rn]), Some((I3cStatus::Ok, &[][..])));
        assert!(s.tx_pending(), "a staged response is awaiting the read");

        // The controller reads it; the runtime clears the flag.
        s.notify_response_read();
        assert!(!s.tx_pending(), "back-pressure released after the read");
    }

    #[test]
    fn recv_without_frame_is_nodata() {
        let mut s = srv();
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Recv, &[], &mut req).unwrap();
        let rn = dispatch(&mut s, &req[..n], &mut resp);
        assert_eq!(
            decode_response(&resp[..rn]),
            Some((I3cStatus::NoData, &[][..]))
        );
    }

    /// Drive the runtime's drain loop against the fake target: latch until
    /// `Empty`, returning whether any overflow drop occurred.
    fn drain(s: &mut Server<FakeTarget>) -> bool {
        let mut dropped = false;
        loop {
            match s.latch_inbound() {
                Ok(Inbound::Latched) => continue,
                Ok(Inbound::DroppedFull) => dropped = true,
                Ok(Inbound::Empty) => break,
                Err(_) => panic!("read_frame failed"),
            }
        }
        dropped
    }

    fn recv_once(s: &mut Server<FakeTarget>) -> (I3cStatus, [u8; MAX_PAYLOAD], usize) {
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Recv, &[], &mut req).unwrap();
        let rn = dispatch(s, &req[..n], &mut resp);
        let (status, body) = decode_response(&resp[..rn]).unwrap();
        let mut out = [0u8; MAX_PAYLOAD];
        out[..body.len()].copy_from_slice(body);
        (status, out, body.len())
    }

    #[test]
    fn latch_then_recv_returns_frame_once() {
        let mut s = srv();
        s.target.push_inbound(b"ping");

        // Simulate the IRQ path draining the frame into the ring.
        assert_eq!(s.on_interrupt_event(), TargetEvent::InboundReady);
        assert_eq!(s.latch_inbound(), Ok(Inbound::Latched));
        assert!(s.has_frame());

        let (status, body, len) = recv_once(&mut s);
        assert_eq!((status, &body[..len]), (I3cStatus::Ok, &b"ping"[..]));
        assert!(!s.has_frame());

        let (status, _, len) = recv_once(&mut s);
        assert_eq!((status, len), (I3cStatus::NoData, 0));
    }

    #[test]
    fn ring_queues_multiple_frames_fifo() {
        let mut s = srv();
        s.target.push_inbound(b"one");
        s.target.push_inbound(b"two");
        s.target.push_inbound(b"three");

        assert!(!drain(&mut s), "three frames fit the ring, no drop");
        assert!(s.has_frame());

        for expect in [&b"one"[..], &b"two"[..], &b"three"[..]] {
            let (status, body, len) = recv_once(&mut s);
            assert_eq!((status, &body[..len]), (I3cStatus::Ok, expect));
        }
        let (status, _, len) = recv_once(&mut s);
        assert_eq!((status, len), (I3cStatus::NoData, 0));
        assert!(!s.has_frame());
    }

    #[test]
    fn ring_full_drops_newest_keeps_queued() {
        let mut s = srv();
        // Push two more than the ring holds; each frame's single-byte payload is
        // its index. The oldest RX_RING survive in order, the last two overflow.
        for i in 0..RX_RING + 2 {
            s.target.push_inbound(&[i as u8]);
        }

        assert!(drain(&mut s), "overflow must be reported as a drop");

        for i in 0..RX_RING {
            let (status, body, len) = recv_once(&mut s);
            assert_eq!((status, &body[..len]), (I3cStatus::Ok, &[i as u8][..]));
        }
        let (status, _, len) = recv_once(&mut s);
        assert_eq!((status, len), (I3cStatus::NoData, 0));
    }

    #[test]
    fn ring_wraps_around() {
        let mut s = srv();
        // Fill most of the ring, drain some to advance head, then refill so the
        // tail wraps past the modulus.
        s.target.push_inbound(b"a");
        s.target.push_inbound(b"b");
        assert!(!drain(&mut s));
        let (_, b, l) = recv_once(&mut s); // pop "a", head -> 1
        assert_eq!(&b[..l], b"a");

        s.target.push_inbound(b"c");
        s.target.push_inbound(b"d");
        s.target.push_inbound(b"e"); // tail wraps
        assert!(!drain(&mut s));

        for expect in [&b"b"[..], &b"c"[..], &b"d"[..], &b"e"[..]] {
            let (status, body, len) = recv_once(&mut s);
            assert_eq!((status, &body[..len]), (I3cStatus::Ok, expect));
        }
        let (status, _, len) = recv_once(&mut s);
        assert_eq!((status, len), (I3cStatus::NoData, 0));
    }

    #[test]
    fn dynamic_address_reports_assignment() {
        let mut s = srv();
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::DynamicAddress, &[], &mut req).unwrap();

        let rn = dispatch(&mut s, &req[..n], &mut resp);
        assert_eq!(
            decode_response(&resp[..rn]),
            Some((I3cStatus::Unassigned, &[][..]))
        );

        s.target.addr = Some(0x42);
        let rn = dispatch(&mut s, &req[..n], &mut resp);
        assert_eq!(
            decode_response(&resp[..rn]),
            Some((I3cStatus::Ok, &[0x42][..]))
        );
    }

    #[test]
    fn unknown_opcode_is_invalid() {
        let mut s = srv();
        let mut resp = [0u8; MAX_FRAME];
        let rn = dispatch(&mut s, &[0xFF], &mut resp);
        assert_eq!(
            decode_response(&resp[..rn]),
            Some((I3cStatus::InvalidOp, &[][..]))
        );
    }

    // Test-only helper mirroring what the runtime reads from `on_interrupt`.
    impl Server<FakeTarget> {
        fn on_interrupt_event(&mut self) -> TargetEvent {
            self.target.on_interrupt().unwrap()
        }
    }
}
