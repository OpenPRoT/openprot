// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP server under test for the multi-protocol QEMU test.
//!
//! One real `mctp_server_runtime::run()` loop is the messaging layer for
//! three client channels: `mctp_spdm`, `mctp_pldm` and `mctp_harness`. The
//! server contains nothing protocol-specific; demultiplexing is purely the
//! MCTP message type each client registered a listener for.
//!
//! The transport is a self-driving loopback. Every outbound packet is
//! queued and the `Sender` raises `USER` on the server's own `transport`
//! handle (through the `loopback` initiator the server also owns), so the
//! packet comes back through `Server::inbound` on the next loop iteration
//! exactly as if a peer on the bus had sent it. No client has to know when
//! to poke the transport, which is what makes the harness and the echo tasks
//! independent processes with no shared timing.

#![no_std]
#![no_main]

use core::cell::RefCell;

use heapless::{Deque, Vec};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_server_runtime::Channel;
use openprot_mctp_server::{Sender, Server};
use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};

use app_mctp_server::handle;

const OWN_EID: u8 = 8;
/// Comfortably larger than any test payload; nothing here ever fragments.
const LOOPBACK_PKT_MAX: usize = 256;
/// Outbound packets parked between a send and the next transport wake. The
/// loop services the transport at least once per client wake, so this only
/// needs to cover the number of client channels.
const LOOPBACK_DEPTH: usize = 4;

type PacketQueue = Deque<Vec<u8, LOOPBACK_PKT_MAX>, LOOPBACK_DEPTH>;

/// Queues outbound MCTP packets and wakes the server's own transport wait.
struct LoopbackSender<'a> {
    queue: &'a RefCell<PacketQueue>,
}

impl Sender for LoopbackSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<mctp::Tag> {
        loop {
            let mut buf = [0u8; LOOPBACK_PKT_MAX];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    let mut pkt = Vec::new();
                    if pkt.extend_from_slice(p).is_err() {
                        return Err(mctp::Error::NoSpace);
                    }
                    if self.queue.borrow_mut().push_back(pkt).is_err() {
                        pw_log::error!("loopback: queue full");
                        return Err(mctp::Error::NoSpace);
                    }
                    // Sticky level signal: the runtime's next object_wait
                    // sees the transport ready whether or not it is waiting
                    // right now.
                    if syscall::object_set_peer_user_signal(handle::LOOPBACK, true).is_err() {
                        pw_log::error!("loopback: wake failed");
                        return Err(mctp::Error::InternalError);
                    }
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        LOOPBACK_PKT_MAX - 4
    }
}

fn server_loop() -> Result<()> {
    pw_log::info!("mctp_server: starting");

    let queue: RefCell<PacketQueue> = RefCell::new(Deque::new());
    let sender = LoopbackSender { queue: &queue };
    let mut server = Server::<_, 16>::new(mctp::Eid(OWN_EID), 0, sender);
    let mut channels = [
        Channel::new(handle::MCTP_SPDM),
        Channel::new(handle::MCTP_PLDM),
        Channel::new(handle::MCTP_HARNESS),
    ];

    mctp_server_runtime::run(
        handle::WG,
        &mut channels,
        handle::TRANSPORT,
        Signals::USER,
        &mut server,
        |server| {
            loop {
                // Borrow ends with the statement, before `inbound` runs.
                let next = queue.borrow_mut().pop_front();
                let Some(pkt) = next else { break };
                if server.inbound(&pkt).is_err() {
                    pw_log::error!("loopback: inbound rejected");
                }
            }
            // Level-triggered: clear it after draining or the loop would
            // spin on the transport forever. Same thread as every send, so
            // nothing can enqueue between the drain and the clear.
            if syscall::object_set_peer_user_signal(handle::LOOPBACK, false).is_err() {
                pw_log::error!("loopback: clear failed");
            }
        },
    )
}

#[entry]
fn entry() {
    if server_loop().is_err() {
        pw_log::error!("mctp_server: exited with error");
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
