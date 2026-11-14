// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host-buildable core of the i3c server: the target-owning [`Server`] state
//! and the request [`dispatch`] against the [`I3cTarget`] facade.
//!
//! No syscalls live here, so the request handling and the inbound frame ring
//! are verified in host tests. The kernel-tagged `i3c_server_runtime`
//! wraps this in the Pigweed WaitGroup loop and supplies the IRQ/IPC syscalls.

#![no_std]

pub mod loopback;

pub use loopback::LoopbackTransport;

use i3c_api::{
    decode_request, encode_response, DeviceInfo, I3cOp, I3cStatus, DEVICE_INFO_LEN, HEADER,
    MAX_PAYLOAD,
};
use openprot_hal_blocking::i3c_hardware::I3cTarget;

/// Default depth of the inbound frame ring.
///
/// The ring covers the window between the IRQ latching a frame and the client
/// draining it over IPC (a cross-process round-trip, plus a controller that
/// polls and may resend), and it must hold every frame a consumer expects the
/// controller to send back-to-back before it drains any. That last part belongs
/// to the consumer, not to this crate: a [`Server`] takes its depth as a const
/// parameter (`Server<T, RING>`, [`Server::with_ring`]), and `Server::new` uses
/// this default of 6. Overflow is an error path only (see
/// [`Inbound::DroppedFull`]).
pub const RX_RING: usize = 6;

/// Outcome of draining one inbound frame in [`Server::latch_inbound`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inbound {
    /// A frame was read from the target and queued into the ring.
    Latched,
    /// Nothing was available to latch (empty or a zero-length descriptor).
    Empty,
    /// A frame was read but dropped because the ring was full — a lost frame
    /// (for a fragmenting consumer, a lost fragment). [`RX_RING`] is sized so this should not occur in normal flow;
    /// the runtime logs it.
    DroppedFull,
}

/// The I3C target the server owns, plus its IPC channel, IRQ handle, the
/// inbound frame ring, and the outbound "response staged, awaiting read" flag.
///
/// Inbound frames are queued in a [`RX_RING`]-deep FIFO. A consumer is
/// typically request/response, but the client draining a latched frame spans a
/// cross-process round-trip, during which the controller can land the next
/// frame (a resend, or the next fragment); the ring holds those so none is
/// lost. `latch_inbound` pushes the oldest-first; `Recv` pops it. The ring
/// never overwrites an unread frame — on overflow a frame is dropped and logged
/// instead (an error path `RX_RING` is sized to avoid). The bus is never
/// blocked.
///
/// Outbound allows one staged response at a time. A `Send` stages a response
/// for the controller's next private read and sets
/// [`tx_pending`](Server::tx_pending); the flag clears only when the controller
/// has read it, signalled by [`notify_response_read`](Server::notify_response_read)
/// on a [`TargetEvent::ResponseRead`]. While it is set, `dispatch` answers a
/// further `Send` with [`I3cStatus::Busy`] and stages nothing, so a second
/// response can never be queued over an unread one. The client retries once the
/// controller has read the first. (caliptra-mcu-sw's I3C driver does the same:
/// its `transmit_read` returns `ErrorCode::BUSY` while a transmit is
/// outstanding.)
///
/// [`TargetEvent::ResponseRead`]: openprot_hal_blocking::i3c_hardware::TargetEvent::ResponseRead
pub struct Server<T, const RING: usize = RX_RING> {
    /// IPC channel handle carrying the transport protocol.
    pub channel: u32,
    /// IRQ handle for the I3C controller.
    pub irq: u32,
    /// The controller driver implementing the facade.
    pub target: T,
    /// Inbound frame ring buffers, one `MAX_PAYLOAD` slot per ring entry.
    rx: [[u8; MAX_PAYLOAD]; RING],
    /// Payload length latched in each ring slot.
    rx_len: [usize; RING],
    /// Index of the oldest queued frame (the next `Recv` pops this slot).
    rx_head: usize,
    /// Number of frames currently queued in the ring.
    rx_count: usize,
    tx_pending: bool,
    /// A staged response was discarded unread and no `Send` has reported it yet.
    discarded: bool,
}

impl<T> Server<T, RX_RING> {
    /// Bind the server to one channel, one IRQ, and one target driver, with the
    /// default ring depth ([`RX_RING`]). Use [`with_ring`](Self::with_ring) for
    /// another depth.
    pub const fn new(channel: u32, irq: u32, target: T) -> Self {
        Self::with_ring(channel, irq, target)
    }
}

impl<T, const RING: usize> Server<T, RING> {
    /// Bind the server to one channel, one IRQ, and one target driver, with a
    /// ring that holds `RING` inbound frames.
    pub const fn with_ring(channel: u32, irq: u32, target: T) -> Self {
        Self {
            channel,
            irq,
            target,
            rx: [[0u8; MAX_PAYLOAD]; RING],
            rx_len: [0usize; RING],
            rx_head: 0,
            rx_count: 0,
            tx_pending: false,
            discarded: false,
        }
    }

    /// Whether any inbound frame is queued and waiting for a `Recv`.
    pub fn has_frame(&self) -> bool {
        self.rx_count > 0
    }

    /// Whether a staged transmit is still awaiting the controller's private
    /// read. A further `Send` is answered `Busy` while this is set.
    pub fn tx_pending(&self) -> bool {
        self.tx_pending
    }

    /// Clear the outbound flag: the controller has read the staged response, so
    /// the next `Send` can be staged. Called by the runtime on
    /// [`TargetEvent::ResponseRead`], which is the only thing that clears it.
    ///
    /// [`TargetEvent::ResponseRead`]: openprot_hal_blocking::i3c_hardware::TargetEvent::ResponseRead
    pub fn notify_response_read(&mut self) {
        self.tx_pending = false;
    }
}

impl<T: I3cTarget, const RING: usize> Server<T, RING> {
    /// Give up on a staged response the controller has not read.
    ///
    /// Discards it in the target ([`I3cTarget::cancel_response`]) and releases
    /// the outbound flag, so the next `Send` is accepted. Returns whether
    /// anything was cancelled: `false`, with the target untouched, if no
    /// response was staged. The runtime calls this once a response has waited
    /// [`STAGED_RESPONSE_TIMEOUT_MS`](i3c_api::STAGED_RESPONSE_TIMEOUT_MS).
    ///
    /// The flag is released even if the target could not really discard the
    /// response (see [`I3cTarget::cancel_response`]), and also if it reports an
    /// error. Keeping it set would refuse every later `Send` for good, which is
    /// the failure this exists to end.
    ///
    /// # Errors
    ///
    /// The target's error from `cancel_response`, after the flag is released.
    pub fn cancel_staged(&mut self) -> Result<bool, T::Error> {
        if !self.tx_pending {
            return Ok(false);
        }
        self.tx_pending = false;
        // The message is lost whether or not the target could really discard
        // it; the next `Send` reports that, once (`I3cStatus::Discarded`).
        self.discarded = true;
        self.target.cancel_response().map(|()| true)
    }

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
    /// `Some(0)`. A zero-length inbound carries nothing for a consumer, so it is
    /// treated as "nothing arrived" ([`Inbound::Empty`]).
    ///
    /// When the ring is full the frame is still read out of the hardware (to
    /// keep the RX path moving) but dropped — [`Inbound::DroppedFull`]. Dropping
    /// the new frame is the lesser evil: overwriting a queued one would lose an
    /// earlier fragment. The ring is sized so this does not happen in
    /// normal flow.
    pub fn latch_inbound(&mut self) -> Result<Inbound, T::Error> {
        if self.rx_count >= RING {
            let mut scratch = [0u8; MAX_PAYLOAD];
            return Ok(match self.target.read_frame(&mut scratch)? {
                Some(n) if n > 0 => Inbound::DroppedFull,
                _ => Inbound::Empty,
            });
        }
        let tail = (self.rx_head + self.rx_count) % RING;
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
pub fn dispatch<T: I3cTarget, const RING: usize>(
    srv: &mut Server<T, RING>,
    req: &[u8],
    resp: &mut [u8],
) -> usize {
    let Some((op, payload)) = decode_request(req) else {
        return status_only(resp, I3cStatus::InvalidOp);
    };
    match op {
        I3cOp::Send => {
            // Too long for the protocol, or for what this target can stage in one
            // response.
            if payload.len() > MAX_PAYLOAD || payload.len() > srv.target.max_send_len() {
                return status_only(resp, I3cStatus::TooLong);
            }
            // One staged response at a time: staging another over an unread one
            // could overrun the target's TX queue. The client retries.
            if srv.tx_pending {
                return status_only(resp, I3cStatus::Busy);
            }
            // A response was discarded unread since the last `Send`. Say so, once,
            // and stage nothing: a sender part-way through a message has lost a
            // fragment and must not go on with the rest.
            if srv.discarded {
                srv.discarded = false;
                return status_only(resp, I3cStatus::Discarded);
            }
            match srv.target.send(payload) {
                Ok(()) => {
                    // Staged for the controller's next private read; further
                    // sends are `Busy` until it has been read (cleared by the
                    // runtime on ResponseRead).
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
            srv.rx_head = (srv.rx_head + 1) % RING;
            srv.rx_count -= 1;
            out.unwrap_or_else(|| status_only(resp, I3cStatus::Internal))
        }
        I3cOp::DeviceInfo => {
            let info = srv.target.device_info();
            // Report what a client can use: never more than the ring slots and
            // the wire frame hold, whatever the target says.
            let clamp = |n: usize| n.min(MAX_PAYLOAD) as u16;
            let wire = DeviceInfo {
                dynamic_address: info.dynamic_address.map(u8::from),
                static_address: info.static_address.map(u8::from),
                max_send_len: clamp(info.max_send_len),
                max_recv_len: clamp(info.max_recv_len),
            };
            let mut body = [0u8; DEVICE_INFO_LEN];
            match wire.encode(&mut body) {
                Some(n) => encode_response(I3cStatus::Ok, &body[..n], resp)
                    .unwrap_or_else(|| status_only(resp, I3cStatus::Internal)),
                None => status_only(resp, I3cStatus::Internal),
            }
        }
        I3cOp::DynamicAddress => match srv.target.dynamic_address() {
            Some(addr) => encode_response(I3cStatus::Ok, &[u8::from(addr)], resp)
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
        /// How many times `cancel_response` was called.
        cancels: u32,
        /// Pending inbound frames, oldest first; `read_frame` pops the front.
        frames: [[u8; MAX_PAYLOAD]; FAKE_INBOUND],
        frame_lens: [usize; FAKE_INBOUND],
        frame_head: usize,
        frame_count: usize,
        sent: [u8; MAX_PAYLOAD],
        sent_len: usize,
        /// What the fake reports from `max_send_len`.
        send_limit: usize,
        /// What the fake reports from `max_recv_len`.
        recv_limit: usize,
        /// What the fake reports from `static_address`.
        static_addr: Option<u8>,
    }

    impl Default for FakeTarget {
        fn default() -> Self {
            Self {
                addr: None,
                cancels: 0,
                frames: [[0u8; MAX_PAYLOAD]; FAKE_INBOUND],
                frame_lens: [0usize; FAKE_INBOUND],
                frame_head: 0,
                frame_count: 0,
                sent: [0u8; MAX_PAYLOAD],
                sent_len: 0,
                send_limit: MAX_PAYLOAD,
                recv_limit: MAX_PAYLOAD,
                static_addr: None,
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
        fn cancel_response(&mut self) -> Result<(), Self::Error> {
            self.cancels += 1;
            self.sent_len = 0;
            Ok(())
        }
        fn dynamic_address(&self) -> Option<DynamicAddress> {
            self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
        }
        fn max_send_len(&self) -> usize {
            self.send_limit
        }
        fn static_address(&self) -> Option<DynamicAddress> {
            self.static_addr
                .and_then(|a| DynamicAddress::try_from(a).ok())
        }
        fn max_recv_len(&self) -> usize {
            self.recv_limit
        }
        fn disable(&mut self) -> Result<(), Self::Error> {
            Ok(())
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
    fn cancelling_a_staged_response_accepts_the_next_send() {
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        assert_eq!(send(&mut s, b"second"), I3cStatus::Busy, "first is unread");

        assert_eq!(s.cancel_staged(), Ok(true));
        assert_eq!(s.target.cancels, 1, "the target discarded it");
        assert!(!s.tx_pending());

        // The loss is reported once, with nothing staged, then sends go through.
        assert_eq!(send(&mut s, b"second"), I3cStatus::Discarded);
        assert_eq!(s.target.sent_len, 0, "nothing was staged");
        assert!(!s.tx_pending());
        assert_eq!(send(&mut s, b"second"), I3cStatus::Ok);
        assert_eq!(&s.target.sent[..s.target.sent_len], b"second");
        s.notify_response_read();
        assert_eq!(send(&mut s, b"third"), I3cStatus::Ok, "reported once only");
    }

    #[test]
    fn a_response_that_was_read_is_never_reported_discarded() {
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        s.notify_response_read();
        assert_eq!(s.cancel_staged(), Ok(false));
        assert_eq!(send(&mut s, b"second"), I3cStatus::Ok);
    }

    #[test]
    fn cancelling_with_nothing_staged_does_not_touch_the_target() {
        let mut s = srv();
        assert_eq!(s.cancel_staged(), Ok(false));
        assert_eq!(s.target.cancels, 0);

        // A response that was read is not staged any more either.
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        s.notify_response_read();
        assert_eq!(s.cancel_staged(), Ok(false));
        assert_eq!(s.target.cancels, 0);
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

    fn send(s: &mut Server<FakeTarget>, payload: &[u8]) -> I3cStatus {
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Send, payload, &mut req).unwrap();
        let rn = dispatch(s, &req[..n], &mut resp);
        decode_response(&resp[..rn]).unwrap().0
    }

    #[test]
    fn second_send_is_busy_and_stages_nothing() {
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);

        // The first is still unread: the second is refused, and the target is
        // left holding the first, untouched.
        assert_eq!(send(&mut s, b"second"), I3cStatus::Busy);
        assert_eq!(&s.target.sent[..s.target.sent_len], b"first");
        assert!(s.tx_pending(), "the first is still awaiting its read");

        // Busy again on a retry before the read.
        assert_eq!(send(&mut s, b"second"), I3cStatus::Busy);
    }

    #[test]
    fn send_is_accepted_again_after_the_response_is_read() {
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        assert_eq!(send(&mut s, b"second"), I3cStatus::Busy);

        s.notify_response_read();
        assert_eq!(send(&mut s, b"second"), I3cStatus::Ok);
        assert_eq!(&s.target.sent[..s.target.sent_len], b"second");
        assert!(s.tx_pending(), "the second is now the unread one");
    }

    #[test]
    fn an_inbound_frame_does_not_release_a_staged_send() {
        // Only the controller reading the response clears it. A new inbound
        // frame proves nothing: a controller may write again before it reads.
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        s.target.push_inbound(b"hello");
        assert!(!drain(&mut s), "the frame is latched, not dropped");
        assert!(s.has_frame());
        assert!(s.tx_pending());
        assert_eq!(send(&mut s, b"second"), I3cStatus::Busy);
    }

    #[test]
    fn oversize_send_is_too_long_even_while_busy() {
        let mut s = srv();
        assert_eq!(send(&mut s, b"first"), I3cStatus::Ok);
        let mut req = [0u8; MAX_FRAME + 8];
        let mut resp = [0u8; MAX_FRAME];
        req[0] = I3cOp::Send as u8;
        let rn = dispatch(&mut s, &req[..HEADER + MAX_PAYLOAD + 1], &mut resp);
        assert_eq!(decode_response(&resp[..rn]).unwrap().0, I3cStatus::TooLong);
    }

    #[test]
    fn send_larger_than_the_targets_limit_is_too_long() {
        let mut s = srv();
        s.target.send_limit = 8;

        // One byte over what the target can stage: refused before it reaches the
        // driver, and nothing is marked pending.
        assert_eq!(send(&mut s, &[0u8; 9]), I3cStatus::TooLong);
        assert_eq!(s.target.sent_len, 0, "nothing was staged");
        assert!(!s.tx_pending());

        // Exactly the limit is accepted.
        assert_eq!(send(&mut s, &[0u8; 8]), I3cStatus::Ok);
        assert_eq!(s.target.sent_len, 8);
    }

    #[test]
    fn a_frame_of_the_protocol_ceiling_is_accepted_when_the_target_allows_it() {
        let mut s = srv();
        assert_eq!(send(&mut s, &[0xa5u8; MAX_PAYLOAD]), I3cStatus::Ok);
        assert_eq!(s.target.sent_len, MAX_PAYLOAD);
    }

    fn device_info(s: &mut Server<FakeTarget>) -> DeviceInfo {
        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::DeviceInfo, &[], &mut req).unwrap();
        let rn = dispatch(s, &req[..n], &mut resp);
        let (status, body) = decode_response(&resp[..rn]).unwrap();
        assert_eq!(status, I3cStatus::Ok);
        DeviceInfo::decode(body).expect("a device-info payload")
    }

    #[test]
    fn device_info_reports_the_targets_addresses_and_limits() {
        let mut s = srv();
        s.target.addr = Some(0x08);
        s.target.static_addr = Some(0x30);
        s.target.send_limit = 100;
        s.target.recv_limit = 64;
        assert_eq!(
            device_info(&mut s),
            DeviceInfo {
                dynamic_address: Some(0x08),
                static_address: Some(0x30),
                max_send_len: 100,
                max_recv_len: 64,
            }
        );
    }

    #[test]
    fn device_info_for_an_unassigned_target_has_no_addresses() {
        let mut s = srv();
        let info = device_info(&mut s);
        assert_eq!((info.dynamic_address, info.static_address), (None, None));
    }

    #[test]
    fn device_info_limits_are_clamped_to_the_frame_ceiling() {
        // A target claiming more than the ring slots and the wire frame hold is
        // reported at the ceiling, so a client never plans a frame that cannot fit.
        let mut s = srv();
        s.target.send_limit = MAX_PAYLOAD + 100;
        s.target.recv_limit = usize::MAX;
        let info = device_info(&mut s);
        assert_eq!(info.max_send_len as usize, MAX_PAYLOAD);
        assert_eq!(info.max_recv_len as usize, MAX_PAYLOAD);
    }

    #[test]
    fn a_full_size_inbound_frame_arrives_intact() {
        // The controller can write up to the protocol ceiling; none of it may be
        // cut off on the way to the client.
        let mut s = srv();
        let frame: [u8; MAX_PAYLOAD] = core::array::from_fn(|i| i as u8);
        s.target.push_inbound(&frame);
        assert!(!drain(&mut s), "the frame is latched, not dropped");

        let (status, out, n) = recv_once(&mut s);
        assert_eq!(status, I3cStatus::Ok);
        assert_eq!(n, MAX_PAYLOAD);
        assert_eq!(out[..n], frame[..]);
    }

    #[test]
    fn ring_depth_is_a_parameter() {
        // A two-deep ring: the third frame overflows, the first two survive.
        let mut s: Server<FakeTarget, 2> = Server::with_ring(1, 2, FakeTarget::default());
        for i in 0..3u8 {
            s.target.push_inbound(&[i]);
        }
        let mut dropped = 0;
        loop {
            match s.latch_inbound().expect("latch") {
                Inbound::Latched => {}
                Inbound::DroppedFull => dropped += 1,
                Inbound::Empty => break,
            }
        }
        assert_eq!(dropped, 1);
        assert!(s.has_frame());

        let mut req = [0u8; MAX_FRAME];
        let mut resp = [0u8; MAX_FRAME];
        let n = encode_request(I3cOp::Recv, &[], &mut req).unwrap();
        for want in 0..2u8 {
            let rn = dispatch(&mut s, &req[..n], &mut resp);
            assert_eq!(
                decode_response(&resp[..rn]),
                Some((I3cStatus::Ok, &[want][..]))
            );
        }
        assert!(!s.has_frame());
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
