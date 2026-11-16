// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `MctpPldmTransport::send_request_serving`: send a request, then wait for its
//! response *or* a command from the peer, answering commands as they arrive.
//!
//! Two in-memory MCTP servers (FD and UA). The FD's requester client pumps
//! packets between them from a script, so each test decides what the UA does
//! while the FD is waiting.

use core::cell::{Cell, RefCell};

use mctp::Eid;
use openprot_mctp_api::Handle;
use openprot_mctp_server::Server;
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};

mod common;
use common::{transfer, BufferSender, DirectClientWithPump, FD_EID, UA_EID};

const PLDM: u8 = 1;

type Packets = RefCell<Vec<Vec<u8>>>;

fn deliver(packets: &Packets, dest: &RefCell<Server<BufferSender<'_>, 16>>) {
    transfer(packets, &mut dest.borrow_mut());
    packets.borrow_mut().clear();
}

/// The UA answers the FD's outstanding request.
fn ua_answers(
    ua: &RefCell<Server<BufferSender<'_>, 16>>,
    ua_listener: Handle,
    ua_to_fd: &Packets,
    fd: &RefCell<Server<BufferSender<'_>, 16>>,
    response: &[u8],
) {
    let mut buf = [0u8; 64];
    let meta = ua
        .borrow_mut()
        .try_recv(ua_listener, &mut buf)
        .expect("the UA should have the FD's request");
    ua.borrow_mut()
        .send(
            None,
            PLDM,
            Some(meta.remote_eid),
            Some(meta.msg_tag),
            false,
            response,
        )
        .unwrap();
    deliver(ua_to_fd, fd);
}

/// The UA sends the FD a command (a request on its own request handle).
fn ua_sends_command(
    ua: &RefCell<Server<BufferSender<'_>, 16>>,
    ua_to_fd: &Packets,
    fd: &RefCell<Server<BufferSender<'_>, 16>>,
    command: &[u8],
) -> Handle {
    let h = ua.borrow_mut().req(FD_EID).unwrap();
    ua.borrow_mut()
        .send(Some(h), PLDM, None, None, false, command)
        .unwrap();
    deliver(ua_to_fd, fd);
    h
}

struct World<'a> {
    fd_to_ua: &'a Packets,
    ua_to_fd: &'a Packets,
    fd: &'a RefCell<Server<BufferSender<'a>, 16>>,
    ua: &'a RefCell<Server<BufferSender<'a>, 16>>,
    ua_listener: Handle,
}

fn world_with<R>(run: impl FnOnce(&World<'_>) -> R) -> R {
    let fd_to_ua: Packets = RefCell::new(Vec::new());
    let ua_to_fd: Packets = RefCell::new(Vec::new());
    let fd = RefCell::new(Server::new(
        Eid(FD_EID),
        0,
        BufferSender { packets: &fd_to_ua },
    ));
    let ua = RefCell::new(Server::new(
        Eid(UA_EID),
        0,
        BufferSender { packets: &ua_to_fd },
    ));
    let ua_listener = ua.borrow_mut().listener(PLDM).unwrap();
    run(&World {
        fd_to_ua: &fd_to_ua,
        ua_to_fd: &ua_to_fd,
        fd: &fd,
        ua: &ua,
        ua_listener,
    })
}

/// The response is the only thing that arrives: it is returned and the command
/// handler is never called.
#[test]
fn response_only_returns_without_calling_the_handler() {
    world_with(|w| {
        let pump = || {
            deliver(w.fd_to_ua, w.ua);
            ua_answers(w.ua, w.ua_listener, w.ua_to_fd, w.fd, b"the-response");
        };
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, pump));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let listener = responder.responder_listener(0).unwrap();

        let calls = Cell::new(0u32);
        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let n = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |_, _, _| {
                    calls.set(calls.get() + 1);
                    Ok(0)
                },
            )
            .unwrap();
        assert_eq!(&buf[1..1 + n], b"the-response");
        assert_eq!(calls.get(), 0);
    });
}

/// A command arrives while the request is outstanding: it is answered before
/// the response shows up, and the request still completes afterwards.
#[test]
fn a_command_while_waiting_is_answered_then_the_response_still_arrives() {
    world_with(|w| {
        let step = Cell::new(0u32);
        let cmd_handle = Cell::new(None::<Handle>);
        let pump = || {
            match step.get() {
                0 => {
                    // First wait: the request goes out and the UA sends a command
                    // instead of answering.
                    deliver(w.fd_to_ua, w.ua);
                    cmd_handle.set(Some(ua_sends_command(
                        w.ua,
                        w.ua_to_fd,
                        w.fd,
                        b"cancel-cmd",
                    )));
                }
                _ => {
                    // Later waits: the UA now answers the request.
                    ua_answers(w.ua, w.ua_listener, w.ua_to_fd, w.fd, b"the-response");
                }
            }
            step.set(step.get() + 1);
        };
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, pump));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let listener = responder.responder_listener(0).unwrap();

        let seen = RefCell::new(Vec::new());
        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let n = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |framed, total, source_eid| {
                    // Framed like `respond_once`: type byte, then the command.
                    assert_eq!(framed[0], PLDM);
                    seen.borrow_mut()
                        .push((framed[1..total].to_vec(), source_eid));
                    framed[1..1 + 5].copy_from_slice(b"ack!!");
                    Ok(1 + 5)
                },
            )
            .unwrap();

        assert_eq!(&buf[1..1 + n], b"the-response");
        assert_eq!(
            seen.borrow().as_slice(),
            &[(b"cancel-cmd".to_vec(), UA_EID)],
            "the command was handed to the handler once, with its source EID"
        );
        // The handler's answer went back to the UA.
        deliver(w.fd_to_ua, w.ua);
        let mut resp = [0u8; 64];
        let h = cmd_handle.get().unwrap();
        let meta =
            w.ua.borrow_mut()
                .try_recv(h, &mut resp)
                .expect("answer to the command");
        assert_eq!(&resp[..meta.payload_size], b"ack!!");
    });
}

/// A handler that returns 0 (a command it does not serve) sends nothing, and the
/// wait resumes for the response.
#[test]
fn a_command_the_handler_declines_gets_no_response() {
    world_with(|w| {
        let step = Cell::new(0u32);
        let cmd_handle = Cell::new(None::<Handle>);
        let pump = || {
            if step.get() == 0 {
                deliver(w.fd_to_ua, w.ua);
                cmd_handle.set(Some(ua_sends_command(w.ua, w.ua_to_fd, w.fd, b"rogue")));
            } else {
                ua_answers(w.ua, w.ua_listener, w.ua_to_fd, w.fd, b"the-response");
            }
            step.set(step.get() + 1);
        };
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, pump));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let listener = responder.responder_listener(0).unwrap();

        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let n = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |_, _, _| Ok(0),
            )
            .unwrap();
        assert_eq!(&buf[1..1 + n], b"the-response");
        assert!(
            w.fd_to_ua.borrow().is_empty(),
            "nothing is sent for a declined command"
        );
    });
}

/// With the response and a command both waiting, the response is returned
/// first and the command stays queued on the listener.
#[test]
fn response_wins_over_a_waiting_command_which_stays_queued() {
    world_with(|w| {
        let pump = || {
            deliver(w.fd_to_ua, w.ua);
            // The command first, then the response: both are queued when the FD looks.
            ua_sends_command(w.ua, w.ua_to_fd, w.fd, b"later-cmd");
            ua_answers(w.ua, w.ua_listener, w.ua_to_fd, w.fd, b"the-response");
        };
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, pump));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let mut listener = responder.responder_listener(0).unwrap();

        let calls = Cell::new(0u32);
        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let n = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |_, _, _| {
                    calls.set(calls.get() + 1);
                    Ok(0)
                },
            )
            .unwrap();
        assert_eq!(&buf[1..1 + n], b"the-response");
        assert_eq!(calls.get(), 0, "the command was not consumed");

        // It is still there for the next receive on the listener.
        let mut next = [0u8; 64];
        responder
            .respond_once(&mut listener, &mut next, |framed, total, _| {
                assert_eq!(&framed[1..total], b"later-cmd");
                calls.set(calls.get() + 1);
                Ok(0)
            })
            .unwrap();
        assert_eq!(calls.get(), 1);
    });
}

/// A handler error stops the wait and is returned.
#[test]
fn a_handler_error_is_returned() {
    world_with(|w| {
        let step = Cell::new(0u32);
        let pump = || {
            if step.get() == 0 {
                deliver(w.fd_to_ua, w.ua);
                ua_sends_command(w.ua, w.ua_to_fd, w.fd, b"bad");
            }
            step.set(step.get() + 1);
        };
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, pump));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let listener = responder.responder_listener(0).unwrap();

        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let err = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |_, _, _| {
                    Err(PldmServiceError::PldmMem(
                        openprot_pldm_service::error::PldmMemError::BufferTooSmall,
                    ))
                },
            )
            .unwrap_err();
        assert!(matches!(err, PldmServiceError::PldmMem(_)));
    });
}

/// No response and no command: the wait times out (here the client reports
/// `TimedOut` for an empty queue).
#[test]
fn nothing_arriving_times_out() {
    world_with(|w| {
        let requester = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let responder = MctpPldmTransport::new(DirectClientWithPump::new(w.fd, || {}));
        let listener = responder.responder_listener(0).unwrap();

        let mut buf = [0u8; 64];
        let mut cmd_buf = [0u8; 64];
        buf[1..5].copy_from_slice(b"ping");
        let err = requester
            .send_request_serving(
                UA_EID,
                4,
                &mut buf,
                0,
                &listener,
                &mut cmd_buf,
                |_, _, _| Ok(0),
            )
            .unwrap_err();
        match err {
            PldmServiceError::Mctp(e) => assert!(e.is_timeout()),
            other => panic!("expected a timeout, got {other:?}"),
        }
    });
}
