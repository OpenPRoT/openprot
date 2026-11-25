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
//!   is read back per fragment rather than fixed at construction.
//! * **One response is staged at a time.** The i3c service answers a second
//!   `Send` with `Busy` until the controller has read the first. A
//!   multi-fragment message therefore waits between fragments: the sender
//!   retries the same frame, calling an injected wait hook between attempts, a
//!   bounded number of times.

use core::cell::Cell;

use i3c_api::Transport;
use i3c_client::{ClientError, I3cClient};
use mctp::Result;
use mctp_lib::fragment::SendOutput;

use crate::encap::{
    mtu_for_limit, MctpI3cEncap, MCTP_I3C_IPC_FRAME, MCTP_I3C_IPC_MAXMTU, MCTP_I3C_IPC_PACKET,
};
use util_pec::{Pec, SoftwarePec};

/// How many times one frame is retried while the i3c service reports `Busy`
/// before the send fails.
///
/// The service discards a response the controller has not read after
/// [`i3c_api::STAGED_RESPONSE_TIMEOUT_MS`] and then accepts `Send` again. So
/// `MAX_BUSY_RETRIES` waits have to add up to more than that, or a stalled
/// controller makes every send fail instead of just losing one message. The
/// caller picks the wait; at 5 ms, 400 retries are 2 seconds.
pub const MAX_BUSY_RETRIES: u16 = 400;

/// How many times one fragment is rebuilt for a new address when the service
/// reports the address moved under it, before the send fails. The controller
/// would have to re-run ENTDAA this often in a row to exhaust it.
pub const MAX_ADDRESS_RETRIES: u8 = 3;

/// What became of one frame handed to the service.
enum Staged {
    /// The service took it.
    Yes,
    /// The address moved after the frame was built for it: nothing was staged.
    AddressMoved,
}

/// I3C MCTP sender.
///
/// Implements [`mctp_lib::Sender`] to fragment and frame MCTP packets for an
/// I3C target, handing each finished frame to the i3c service.
///
/// `W` is the wait hook, called between attempts while the service is `Busy`.
/// The crate is kernel-free, so the caller supplies it: a sleep on target, a
/// no-op or a counter in host tests.
pub struct I3cSender<T, W, P = SoftwarePec> {
    client: I3cClient<T>,
    /// The controller's dynamic address. DSP0233 has no destination or source
    /// field, so it is not carried on the wire (an I3C target always sends to the
    /// controller); it is kept for diagnostics and API stability. A full
    /// EID → address neighbour table is tracked in
    /// <https://github.com/OpenPRoT/mctp-lib/issues/4>.
    remote_addr: u8,
    wait: W,
    /// The MTU `get_mtu` reports: what the target could send when the limits
    /// were last read (see [`refresh_limits`](Self::refresh_limits)).
    mtu: Cell<usize>,
    /// Computes the PEC of every outbound frame.
    pec: P,
}

impl<T: Transport, W: FnMut()> I3cSender<T, W> {
    /// Create a sender over an i3c client, computing the PEC in software.
    ///
    /// * `client` — the i3c target service client
    /// * `remote_addr` — the controller's 7-bit dynamic address
    /// * `wait` — called between retries while the service is `Busy`
    pub const fn new(client: I3cClient<T>, remote_addr: u8, wait: W) -> Self {
        Self::with_pec(client, remote_addr, wait, SoftwarePec)
    }
}

impl<T: Transport, W: FnMut(), P: Pec + Copy> I3cSender<T, W, P> {
    /// Create a sender that computes the PEC with `pec`, as [`new`](Self::new)
    /// otherwise.
    pub const fn with_pec(client: I3cClient<T>, remote_addr: u8, wait: W, pec: P) -> Self {
        Self {
            client,
            remote_addr,
            wait,
            mtu: Cell::new(MCTP_I3C_IPC_MAXMTU),
            pec,
        }
    }

    /// Read the target's send limit and set the MTU from it.
    ///
    /// The MTU is the largest packet the target can stage (one byte less than
    /// its send limit, for the PEC), at most [`MCTP_I3C_IPC_MAXMTU`] and at least
    /// the MCTP baseline, as [`mtu_for_limit`] says. The controller can change
    /// the limit (SETMRL), so this runs at the start of every send. `get_mtu`
    /// reports the result, and the stack asks for it *before* a send, so call
    /// this once at startup too: until then the MTU is the ceiling.
    ///
    /// Returns the new MTU.
    ///
    /// # Errors
    ///
    /// [`mctp::Error::TxFailure`] if the limits cannot be read, or if the target
    /// cannot send even the baseline packet. The MTU is left as it was.
    pub fn refresh_limits(&mut self) -> Result<usize> {
        let info = match self.client.device_info() {
            Ok(info) => info,
            Err(_) => {
                pw_log::error!("i3c device info unavailable; MTU unchanged");
                return Err(mctp::Error::TxFailure);
            }
        };
        match mtu_for_limit(usize::from(info.max_send_len)) {
            Some(mtu) => {
                self.mtu.set(mtu);
                Ok(mtu)
            }
            None => {
                pw_log::error!(
                    "i3c send limit 0x{:04x} is below the MCTP baseline",
                    info.max_send_len as u32
                );
                Err(mctp::Error::TxFailure)
            }
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
    fn send_frame(&mut self, addr: u8, frame: &[u8], first: bool) -> Result<Staged> {
        let mut retries: u16 = 0;
        let mut stale_discard_seen = false;
        loop {
            match self.client.send_for(addr, frame) {
                Ok(()) => return Ok(Staged::Yes),
                Err(ClientError::AddressChanged) => return Ok(Staged::AddressMoved),
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

impl<T: Transport, W: FnMut(), P: Pec + Copy> mctp_lib::Sender for I3cSender<T, W, P> {
    fn send_vectored(
        &mut self,
        // DSP0233 has no destination field and a target always sends to the
        // controller, so the EID is not used to pick an address.
        _eid: mctp::Eid,
        mut fragmenter: mctp_lib::fragment::Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        // The limit is read again for every message: the controller can lower it
        // (SETMRL). `fragmenter` was built with the MTU `get_mtu` reported
        // before this call, so a limit that dropped since then is caught below,
        // packet by packet, instead of being sent over.
        self.refresh_limits()?;

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
                    if p.len() > self.mtu.get() {
                        pw_log::error!(
                            "i3c packet 0x{:04x} is over the MTU 0x{:04x}; message abandoned",
                            p.len() as u32,
                            self.mtu.get() as u32
                        );
                        return Err(mctp::Error::TxFailure);
                    }
                    // The address is read again for every fragment, and every
                    // retry: ENTDAA can run at any time and the PEC covers it. The
                    // service checks it against the live address when it stages
                    // the frame, so a change cannot slip in after this read.
                    let mut staged = false;
                    for _ in 0..=MAX_ADDRESS_RETRIES {
                        let addr = self.own_addr()?;
                        let encoder = MctpI3cEncap::with_pec(addr, self.pec);
                        let mut out = [0u8; MCTP_I3C_IPC_FRAME];
                        let n = encoder.encode(p, &mut out)?;
                        let frame = out.get(..n).ok_or(mctp::Error::TxFailure)?;
                        if let Staged::Yes = self.send_frame(addr, frame, first)? {
                            staged = true;
                            break;
                        }
                    }
                    if !staged {
                        pw_log::error!("i3c address kept changing; message abandoned");
                        return Err(mctp::Error::TxFailure);
                    }
                    first = false;
                }
                SendOutput::Complete { tag, .. } => break Ok(tag),
                SendOutput::Error { err, .. } => break Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        self.mtu.get()
    }
}
