// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP over I3C, end to end on the host.
//!
//! `Server::send()` → `I3cSender` → the real `i3c_server::dispatch` → a mock
//! `I3cTarget` that records every staged frame, then back out through
//! `MctpI3cReceiver`. Every layer but the hardware is the production one.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i3c_api::{
    decode_request, encode_response, I3cOp, I3cStatus, Transport, TransportError,
    MAX_FRAME as I3C_MAX_FRAME, MAX_PAYLOAD as I3C_MAX_PAYLOAD,
};
use i3c_client::I3cClient;
use i3c_server::{LoopbackTransport, Server as I3cServer};
use mctp::Eid;
use openprot_hal_blocking::i3c_hardware::{DynamicAddress, I3cTarget, TargetEvent};
use openprot_mctp_api::MCTP_HEADER_LEN;
use openprot_mctp_server::Server;
use openprot_mctp_transport_i3c::{
    transfer_pec, I3cSender, MctpI3cReceiver, MAX_BUSY_RETRIES, MCTP_I3C_IPC_MAXMTU,
};

const OWN_ADDR: u8 = 0x08;
const REMOTE_ADDR: u8 = 0x1d;
const OWN_EID: u8 = 8;
const REMOTE_EID: u8 = 48;
const MSG_TYPE: u8 = 0x05; // SPDM

/// A target that records every frame staged for transmission.
struct CaptureTarget<'a> {
    addr: Option<u8>,
    sends: &'a RefCell<Vec<Vec<u8>>>,
}

#[derive(Debug, PartialEq, Eq)]
struct CaptureError;

impl I3cTarget for CaptureTarget<'_> {
    type Error = CaptureError;

    fn enable(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
        Ok(TargetEvent::None)
    }

    fn read_frame(&mut self, _buf: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        Ok(None)
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.sends.borrow_mut().push(data.to_vec());
        Ok(())
    }

    fn cancel_response(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn dynamic_address(&self) -> Option<DynamicAddress> {
        self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
    }

    fn static_address(&self) -> Option<DynamicAddress> {
        None
    }

    fn max_send_len(&self) -> usize {
        I3C_MAX_PAYLOAD
    }

    fn max_recv_len(&self) -> usize {
        I3C_MAX_PAYLOAD
    }
}

/// The real i3c service over a controller that is slow to read: a staged
/// response stays pending, so a second `Send` is answered `Busy`, until the
/// sender's wait hook says the controller has read it.
struct GatedTransport<'a> {
    inner: LoopbackTransport<CaptureTarget<'a>>,
    released: Rc<Cell<bool>>,
}

impl Transport for GatedTransport<'_> {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        if self.released.take() {
            self.inner.server().notify_response_read();
        }
        self.inner.transact(req, resp)
    }
}

type Sender<'a> = I3cSender<GatedTransport<'a>, Box<dyn FnMut()>>;

/// A server over the gated transport, and a count of how often the sender
/// waited for the controller.
fn server_counting(
    sends: &RefCell<Vec<Vec<u8>>>,
    addr: Option<u8>,
) -> (Server<Sender<'_>, 16>, Rc<Cell<u32>>) {
    let released = Rc::new(Cell::new(false));
    let waits = Rc::new(Cell::new(0));
    let target = CaptureTarget { addr, sends };
    let client = I3cClient::new(GatedTransport {
        inner: LoopbackTransport::new(I3cServer::new(1, 2, target)),
        released: released.clone(),
    });
    let wait: Box<dyn FnMut()> = {
        let waits = waits.clone();
        Box::new(move || {
            waits.set(waits.get() + 1);
            released.set(true);
        })
    };
    let srv = Server::new(Eid(OWN_EID), 0, I3cSender::new(client, REMOTE_ADDR, wait));
    (srv, waits)
}

fn server(sends: &RefCell<Vec<Vec<u8>>>, addr: Option<u8>) -> Server<Sender<'_>, 16> {
    server_counting(sends, addr).0
}

/// Decode a frame the way the I3C *controller* sees it: the target's response to
/// a private read. DSP0233 frames it as the MCTP packet followed by a PEC over
/// the read address byte `(addr << 1) | 1` and the packet. Nothing precedes the
/// packet.
fn controller_read_decode(addr: u8, frame: &[u8]) -> &[u8] {
    let (received, packet) = frame.split_last().expect("frame carries a PEC");
    assert_eq!(
        *received,
        transfer_pec((addr << 1) | 1, packet),
        "PEC of a controller read"
    );
    packet
}

#[test]
fn sender_receiver_roundtrip() {
    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));

    let payload = b"hello mctp";
    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(Some(req), MSG_TYPE, None, None, false, payload)
        .expect("send");

    let frames = sends.borrow();
    assert_eq!(frames.len(), 1, "short payload should be one frame");

    // The frame is exactly the MCTP packet plus a PEC.
    let pkt = controller_read_decode(OWN_ADDR, &frames[0]);
    assert_eq!(frames[0].len(), pkt.len() + 1);

    // MCTP transport header (4) then the message-type byte, then the payload.
    assert!(pkt.len() >= 5, "packet too short: {}", pkt.len());
    assert_eq!(pkt[4] & 0x7f, MSG_TYPE, "message type");
    assert_eq!(&pkt[5..], payload, "payload");
}

#[test]
fn single_fragment_produces_one_send() {
    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));

    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(Some(req), 1, None, None, false, b"short")
        .expect("send");

    assert_eq!(sends.borrow().len(), 1);
}

/// The case the MTU cap exists for: a payload past one fragment must split,
/// and *every* frame must still fit one i3c IPC frame.
#[test]
fn large_payload_fragments_and_every_frame_fits() {
    let sends = RefCell::new(Vec::new());
    let (mut srv, waits) = server_counting(&sends, Some(OWN_ADDR));

    let payload = vec![0x5au8; MCTP_I3C_IPC_MAXMTU * 3];
    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(Some(req), MSG_TYPE, None, None, false, &payload)
        .expect("send");

    let frames = sends.borrow();
    assert!(
        frames.len() > 1,
        "payload of {} should have fragmented, got {} frame(s)",
        payload.len(),
        frames.len()
    );

    // The service holds one response at a time: every fragment after the first
    // was answered `Busy` once, and went through after one wait.
    assert_eq!(waits.get() as usize, frames.len() - 1);

    for (i, frame) in frames.iter().enumerate() {
        assert!(
            frame.len() <= I3C_MAX_PAYLOAD,
            "frame {} is {} bytes, over the {}-byte i3c limit",
            i,
            frame.len(),
            I3C_MAX_PAYLOAD
        );
        // Each frame must carry a valid PEC on its own.
        controller_read_decode(OWN_ADDR, frame);
    }
}

/// The fragmenter treats the MTU as the largest whole packet, transport header
/// included, so a first fragment holds `MTU - 4 - 1` payload bytes (the header,
/// then the message-type byte). One more byte splits the message, and either way
/// the frame, with its PEC, stays within the i3c frame.
#[test]
fn the_mtu_bounds_a_whole_packet_and_the_frame_fits_the_i3c_limit() {
    let first_fragment_payload = MCTP_I3C_IPC_MAXMTU - MCTP_HEADER_LEN - 1;

    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));
    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(
        Some(req),
        MSG_TYPE,
        None,
        None,
        false,
        &vec![0xa5u8; first_fragment_payload],
    )
    .expect("send");
    {
        let frames = sends.borrow();
        assert_eq!(frames.len(), 1, "should still be a single fragment");
        // The packet is `MTU` bytes; the PEC is one more.
        assert_eq!(frames[0].len(), MCTP_I3C_IPC_MAXMTU + 1);
        assert!(frames[0].len() <= I3C_MAX_PAYLOAD);
    }

    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));
    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(
        Some(req),
        MSG_TYPE,
        None,
        None,
        false,
        &vec![0xa5u8; first_fragment_payload + 1],
    )
    .expect("send");
    assert_eq!(sends.borrow().len(), 2, "one byte more must split");
}

/// A send before the controller has assigned a dynamic address must fail
/// without putting anything on the bus.
///
/// This asserts the observable contract through `Server::send`. The precise
/// `mctp::Error::Unreachable` that `I3cSender` returns (documented on its
/// `own_addr`) is not checked here: the MCTP server maps it, along with several
/// other errors, through a wildcard to `ResponseCode::InternalError`, so the
/// specific variant cannot be recovered at this layer.
#[test]
fn send_before_entdaa_fails_without_touching_the_bus() {
    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, None);

    let req = srv.req(REMOTE_EID).expect("req");
    assert!(
        srv.send(Some(req), MSG_TYPE, None, None, false, b"x")
            .is_err(),
        "send without a dynamic address must fail"
    );
    assert!(
        sends.borrow().is_empty(),
        "nothing should reach the bus before an address is assigned"
    );
}

/// A frame the target stages for a controller *read* must not be accepted as an
/// inbound controller *write*: the two PECs cover different address bytes
/// (`(addr << 1) | 1` versus `addr << 1`).
#[test]
fn receiver_rejects_an_outbound_frame() {
    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));

    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(Some(req), MSG_TYPE, None, None, false, b"loopback")
        .expect("send");

    let frame = sends.borrow()[0].clone();
    let mut rx = MctpI3cReceiver::new();
    assert!(rx.decode(&frame, || Some(OWN_ADDR)).is_err());
}

#[test]
fn receiver_rejects_a_corrupt_pec() {
    let sends = RefCell::new(Vec::new());
    let mut srv = server(&sends, Some(OWN_ADDR));

    let req = srv.req(REMOTE_EID).expect("req");
    srv.send(Some(req), MSG_TYPE, None, None, false, b"tamper")
        .expect("send");

    // Re-frame the packet the way a controller writes it, then corrupt the PEC.
    let frame = sends.borrow()[0].clone();
    let packet = controller_read_decode(OWN_ADDR, &frame).to_vec();
    let mut write = packet.clone();
    write.push(transfer_pec(OWN_ADDR << 1, &packet));

    let mut rx = MctpI3cReceiver::new();
    assert_eq!(
        rx.decode(&write, || Some(OWN_ADDR)).expect("good write"),
        &packet[..]
    );

    let last = write.len() - 1;
    write[last] ^= 0xff;
    assert!(rx.decode(&write, || Some(OWN_ADDR)).is_err());
}

/// A service stand-in that answers `DynamicAddress` and answers `Send` from a
/// script, so the sender's handling of `Busy` and of failures can be driven
/// exactly.
struct ScriptedTransport {
    /// Statuses to answer successive `Send`s with; once exhausted, `repeat`.
    script: Vec<I3cStatus>,
    repeat: I3cStatus,
    sends: Rc<Cell<u32>>,
}

impl Transport for ScriptedTransport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        let (op, _) = decode_request(req).expect("request");
        let (status, payload): (I3cStatus, &[u8]) = match op {
            I3cOp::DynamicAddress => (I3cStatus::Ok, &[OWN_ADDR]),
            I3cOp::Send => {
                self.sends.set(self.sends.get() + 1);
                let status = if self.script.is_empty() {
                    self.repeat
                } else {
                    self.script.remove(0)
                };
                (status, &[])
            }
            _ => (I3cStatus::InvalidOp, &[]),
        };
        assert!(resp.len() >= I3C_MAX_FRAME);
        Ok(encode_response(status, payload, resp).expect("response"))
    }
}

/// Send one short message through a sender over `script`; returns the result,
/// the number of `Send`s the service saw and the number of waits.
fn send_scripted(script: Vec<I3cStatus>, repeat: I3cStatus) -> (Result<(), ()>, u32, u32) {
    send_scripted_payload(script, repeat, b"x")
}

/// [`send_scripted`] with a chosen message payload, to span several fragments.
fn send_scripted_payload(
    script: Vec<I3cStatus>,
    repeat: I3cStatus,
    payload: &[u8],
) -> (Result<(), ()>, u32, u32) {
    let sends = Rc::new(Cell::new(0));
    let waits = Rc::new(Cell::new(0u32));
    let client = I3cClient::new(ScriptedTransport {
        script,
        repeat,
        sends: sends.clone(),
    });
    let wait = {
        let waits = waits.clone();
        move || waits.set(waits.get() + 1)
    };
    let mut srv: Server<_, 16> =
        Server::new(Eid(OWN_EID), 0, I3cSender::new(client, REMOTE_ADDR, wait));
    let req = srv.req(REMOTE_EID).expect("req");
    let result = srv
        .send(Some(req), MSG_TYPE, None, None, false, payload)
        .map(|_| ())
        .map_err(|_| ());
    (result, sends.get(), waits.get())
}

#[test]
fn busy_is_retried_with_a_wait_until_the_frame_is_accepted() {
    let (result, sends, waits) = send_scripted(
        vec![I3cStatus::Busy, I3cStatus::Busy, I3cStatus::Busy],
        I3cStatus::Ok,
    );
    assert!(result.is_ok());
    assert_eq!(sends, 4, "three Busy answers, then accepted");
    assert_eq!(waits, 3, "one wait per Busy");
}

#[test]
fn a_service_that_stays_busy_fails_the_send_after_the_retry_bound() {
    let (result, sends, waits) = send_scripted(Vec::new(), I3cStatus::Busy);
    assert!(result.is_err());
    assert_eq!(waits, u32::from(MAX_BUSY_RETRIES));
    assert_eq!(
        sends,
        u32::from(MAX_BUSY_RETRIES) + 1,
        "the first try plus each retry"
    );
}

#[test]
fn an_error_that_is_not_busy_is_not_retried() {
    for status in [
        I3cStatus::TooLong,
        I3cStatus::Internal,
        I3cStatus::Unassigned,
    ] {
        let (result, sends, waits) = send_scripted(Vec::new(), status);
        assert!(result.is_err(), "{status:?}");
        assert_eq!(sends, 1, "{status:?}");
        assert_eq!(waits, 0, "{status:?}");
    }
}

/// A fragment the service discarded unread leaves the message unreassemblable:
/// the send must fail and the remaining fragments must not go out.
#[test]
fn a_discard_mid_message_fails_the_send_and_stops_the_fragments() {
    let payload = vec![0x5au8; MCTP_I3C_IPC_MAXMTU * 3];
    // Fragment 1 is accepted; the service then drops it unread and answers the
    // next `Send` (fragment 2) `Discarded`. Everything after would be `Ok`.
    let (result, sends, waits) = send_scripted_payload(
        vec![I3cStatus::Ok, I3cStatus::Discarded],
        I3cStatus::Ok,
        &payload,
    );
    assert!(result.is_err(), "a message that lost a fragment is not sent");
    assert_eq!(sends, 2, "no fragment after the discard goes out");
    assert_eq!(waits, 0);
}

/// A discard reported to the *first* fragment concerns an earlier message, not
/// this one: the sender goes on, and resends the fragment once.
#[test]
fn a_discard_reported_to_the_first_fragment_is_not_this_messages_loss() {
    let (result, sends, _) = send_scripted(vec![I3cStatus::Discarded], I3cStatus::Ok);
    assert!(result.is_ok());
    assert_eq!(sends, 2, "the refused fragment is sent again");
}

/// The stale-discard retry is bounded: a service that keeps answering
/// `Discarded` to the first fragment cannot hold the sender in a loop.
#[test]
fn a_service_that_keeps_discarding_fails_the_send() {
    let (result, sends, _) = send_scripted(Vec::new(), I3cStatus::Discarded);
    assert!(result.is_err());
    assert_eq!(sends, 2, "one retry, then the send fails");
}
