// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C MCTP sender — outbound transport binding.
//!
//! Shaped like `services/mctp/transport-i2c`'s `I2cSender`: the same
//! fragmentation loop, the same error mapping. What differs is dictated by I3C
//! target mode, not by taste:
//!
//! * **This side is a target, not a bus master.** It cannot start a transfer,
//!   so "send" stages a TX and raises an IBI that prompts the controller to
//!   collect it with a private read. That is one `I3cOp::Send`, and the whole
//!   framed packet goes in it — unlike the embedded-hal I2C path, nothing
//!   prepends a destination byte, so nothing is trimmed off the front here.
//! * **The source address is runtime state.** It is assigned by ENTDAA, so it
//!   is read back per message rather than fixed at construction.
//! * **One response is staged at a time.** The i3c service answers a second
//!   `Send` with `Busy` until the controller has read the first. A
//!   multi-fragment message therefore waits between fragments: the sender
//!   retries the same frame, calling an injected wait hook between attempts, a
//!   bounded number of times.

use i3c_api::Transport;
use i3c_client::{ClientError, I3cClient};
use mctp::Result;
use mctp_lib::fragment::SendOutput;

use crate::encap::{MctpI3cEncap, MCTP_I3C_IPC_FRAME, MCTP_I3C_IPC_MAXMTU, MCTP_I3C_IPC_PACKET};

/// How many times one frame is retried while the i3c service reports `Busy`
/// before the send fails.
///
/// The service discards a response the controller has not read after
/// [`i3c_api::STAGED_RESPONSE_TIMEOUT_MS`] and then accepts `Send` again. So
/// `MAX_BUSY_RETRIES` waits have to add up to more than that, or a stalled
/// controller makes every send fail instead of just losing one message. The
/// caller picks the wait; at 5 ms, 400 retries are 2 seconds.
pub const MAX_BUSY_RETRIES: u16 = 400;

/// I3C MCTP sender.
///
/// Implements [`mctp_lib::Sender`] to fragment and frame MCTP packets for an
/// I3C target, handing each finished frame to the i3c service.
///
/// `W` is the wait hook, called between attempts while the service is `Busy`.
/// The crate is kernel-free, so the caller supplies it: a sleep on target, a
/// no-op or a counter in host tests.
pub struct I3cSender<T, W> {
    client: I3cClient<T>,
    /// The controller's dynamic address. DSP0233 has no destination or source
    /// field, so it is not carried on the wire (an I3C target always sends to the
    /// controller); it is kept for diagnostics and API stability. A full
    /// EID → address neighbour table is tracked in
    /// <https://github.com/OpenPRoT/mctp-lib/issues/4>.
    remote_addr: u8,
    wait: W,
}

impl<T: Transport, W: FnMut()> I3cSender<T, W> {
    /// Create a sender over an i3c client.
    ///
    /// * `client` — the i3c target service client
    /// * `remote_addr` — the controller's 7-bit dynamic address
    /// * `wait` — called between retries while the service is `Busy`
    pub const fn new(client: I3cClient<T>, remote_addr: u8, wait: W) -> Self {
        Self {
            client,
            remote_addr,
            wait,
        }
    }

    /// Borrow the underlying client — to `recv` inbound frames on the same
    /// channel, or to read the dynamic address.
    pub fn client(&mut self) -> &mut I3cClient<T> {
        &mut self.client
    }

    /// This target's current dynamic address.
    ///
    /// Read fresh rather than cached: ENTDAA runs whenever the controller
    /// re-enumerates the bus, and a stale value would put the wrong address in
    /// every outbound PEC (which covers the address byte) — a fault that is
    /// invisible locally because the frames still encode and send cleanly.
    fn own_addr(&mut self) -> Result<u8> {
        match self.client.dynamic_address() {
            Ok(Some(addr)) => Ok(addr),
            // On the bus but not yet enumerated: nothing can be addressed from
            // here until the controller assigns an address.
            Ok(None) => Err(mctp::Error::Unreachable),
            Err(_) => Err(mctp::Error::TxFailure),
        }
    }

    /// Hand one finished frame to the service, waiting out `Busy`.
    ///
    /// `first` marks the first fragment of a message. A `Discarded` answer to it
    /// concerns an earlier message, so it is retried once; to any later
    /// fragment it means this message lost one, and the send fails.
    fn send_frame(&mut self, frame: &[u8], first: bool) -> Result<()> {
        let mut retries: u16 = 0;
        let mut stale_discard_seen = false;
        loop {
            match self.client.send(frame) {
                Ok(()) => return Ok(()),
                Err(ClientError::Discarded) if first && !stale_discard_seen => {
                    stale_discard_seen = true;
                }
                Err(ClientError::Discarded) => {
                    pw_log::error!(
                        "i3c fragment discarded unread; message abandoned: len=0x{:04x}",
                        frame.len() as u32
                    );
                    return Err(mctp::Error::TxFailure);
                }
                Err(ClientError::Busy) if retries < MAX_BUSY_RETRIES => {
                    retries += 1;
                    (self.wait)();
                }
                Err(ClientError::Busy) => {
                    pw_log::error!(
                        "i3c send still busy after {} retries: len=0x{:04x}",
                        retries as u32,
                        frame.len() as u32
                    );
                    return Err(mctp::Error::TxFailure);
                }
                Err(_) => {
                    pw_log::error!(
                        "i3c send failed: addr=0x{:02x} len=0x{:04x}",
                        self.remote_addr as u32,
                        frame.len() as u32
                    );
                    return Err(mctp::Error::TxFailure);
                }
            }
        }
    }
}

impl<T: Transport, W: FnMut()> mctp_lib::Sender for I3cSender<T, W> {
    fn send_vectored(
        &mut self,
        // DSP0233 has no destination field and a target always sends to the
        // controller, so the EID is not used to pick an address.
        _eid: mctp::Eid,
        mut fragmenter: mctp_lib::fragment::Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        let encoder = MctpI3cEncap::new(self.own_addr()?);
        // Fragments are staged one at a time with back-pressure: the i3c server
        // answers a second `Send` with `Busy` until the controller has read the
        // first (signalled by TargetEvent::ResponseRead). `send_frame` waits
        // that out, so this loop cannot outrun the controller and overrun the
        // target TX queue.
        let mut first = true;
        loop {
            let mut pkt = [0u8; MCTP_I3C_IPC_PACKET];
            match fragmenter.fragment_vectored(payload, &mut pkt) {
                SendOutput::Packet(p) => {
                    let mut out = [0u8; MCTP_I3C_IPC_FRAME];
                    let n = encoder.encode(p, &mut out)?;
                    let frame = out.get(..n).ok_or(mctp::Error::TxFailure)?;
                    self.send_frame(frame, first)?;
                    first = false;
                }
                SendOutput::Complete { tag, .. } => break Ok(tag),
                SendOutput::Error { err, .. } => break Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_I3C_IPC_MAXMTU
    }
}
