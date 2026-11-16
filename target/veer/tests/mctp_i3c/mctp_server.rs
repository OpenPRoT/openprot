// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP server process, MCTP-over-I3C transport.
//!
//! The I3C analog of the ast10x0 `tests/mctp/server/main.rs`: it serves MCTP
//! app clients over a Pigweed channel and carries their traffic over I3C via
//! `//services/mctp/transport-i3c`. The loop itself, including a `Recv` that
//! has to wait for a message, is `mctp_server_runtime::run`; this file only
//! supplies the two ends:
//!
//!   the `mctp` channel       a client's requests
//!   `USER` on the i3c channel  an inbound frame the i3c server latched from
//!                              its interrupt path
//!
//! Unlike the I2C server this process does no slave configuration: the i3c
//! server owns and enables the hardware. It only sends (staging a private-read
//! response) and receives (collecting a latched private write).

#![no_main]
#![no_std]

use i3c_client::I3cClient;
use i3c_client_ipc::IpcTransport;
use mctp_server_codegen::handle;
use mctp_server_runtime::Channel;
use openprot_mctp_server::Server;
use openprot_mctp_transport_i3c::{I3cSender, MctpI3cReceiver, MAX_BUSY_RETRIES};
use pw_status::Result;
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::{self, Clock, Duration, Instant, SystemClock};

const OWN_EID: u8 = 8;
/// The controller's 7-bit I3C address. DSP0233 carries no destination, so it is
/// not on the wire; the sender keeps it for diagnostics.
const REMOTE_I3C_ADDR: u8 = 0x0a;
/// How long the sender waits between attempts while the i3c server still holds
/// the previous fragment, which the controller has not read yet.
const BUSY_WAIT_MS: u64 = 5;
const BUSY_WAIT: Duration = Duration::from_millis(BUSY_WAIT_MS);

// The retries must outlast the i3c server's timeout for an unread response, or
// a controller that stops reading would make every send fail, not just one.
const _: () =
    assert!(BUSY_WAIT_MS * MAX_BUSY_RETRIES as u64 > i3c_api::STAGED_RESPONSE_TIMEOUT_MS as u64);
const I3C_RX_MAX: usize = i3c_api::MAX_PAYLOAD;

/// Wait out `Busy` from the i3c server.
fn wait_for_controller() {
    let deadline = SystemClock::now()
        .checked_add_duration(BUSY_WAIT)
        .unwrap_or(Instant::MAX);
    let _ = time::sleep_until(deadline);
}

fn mctp_server_loop() -> Result<()> {
    pw_log::info!("MCTP-over-I3C server starting");

    let mut sender = I3cSender::new(
        I3cClient::new(IpcTransport::new(handle::I3C)),
        REMOTE_I3C_ADDR,
        wait_for_controller,
    );
    // The stack asks for the MTU before each send, so read the target's limits
    // now. Until then the MTU is the ceiling. Not fatal: every send reads them
    // again, and fails if the target cannot carry MCTP.
    if sender.refresh_limits().is_err() {
        pw_log::error!("i3c limits unavailable at startup");
    }
    // A second client on the same i3c channel, used to collect inbound frames.
    let mut i3c_rx_client = I3cClient::new(IpcTransport::new(handle::I3C));
    let mut i3c_receiver = MctpI3cReceiver::new();

    let mut server = Server::<_, 16>::new(mctp::Eid(OWN_EID), 0, sender);
    let mut i3c_rx_buf = [0u8; I3C_RX_MAX];
    let mut channels = [Channel::new(handle::MCTP)];

    mctp_server_runtime::run(
        handle::WG,
        &mut channels,
        handle::I3C,
        Signals::USER,
        &mut server,
        |server| {
            // One frame per wake. The i3c server recomputes USER from what
            // is left in its ring after every `recv`, so the signal is quiet
            // once the ring is empty and brings us straight back if it is
            // not; looping here until `None` would only add an IPC round
            // trip to every wake.
            match i3c_rx_client.recv(&mut i3c_rx_buf) {
                Ok(Some(n)) => {
                    let decoded = i3c_rx_buf.get(..n).map(|frame| {
                        i3c_receiver
                            .decode(frame, || i3c_rx_client.dynamic_address().ok().flatten())
                    });
                    if let Some(Ok(pkt)) = decoded {
                        let _ = server.inbound(pkt);
                    } else {
                        // Say what arrived: a frame cut short by the controller
                        // and one with a bad PEC look alike otherwise.
                        let frame = i3c_rx_buf.get(..n).unwrap_or(&[]);
                        let byte = |i: usize| u32::from(frame.get(i).copied().unwrap_or(0));
                        pw_log::error!(
                            "i3c frame decode failed: len=0x{:04x} head={:02x}{:02x}{:02x}{:02x} tail=0x{:02x}",
                            n as u32,
                            byte(0),
                            byte(1),
                            byte(2),
                            byte(3),
                            byte(n.wrapping_sub(1))
                        );
                    }
                }
                Ok(None) => {}
                Err(_) => pw_log::error!("i3c recv failed"),
            }
        },
    )
}

#[process_entry("mctp_server")]
fn entry() {
    if let Err(e) = mctp_server_loop() {
        pw_log::error!("mctp_server exiting with error");
        let _ = syscall::debug_shutdown(Err(e));
    }
    #[expect(clippy::empty_loop)]
    loop {}
}
