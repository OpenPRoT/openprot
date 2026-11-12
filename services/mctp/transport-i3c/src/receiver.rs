// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C MCTP receiver — inbound transport binding.
//!
//! Decodes frames latched by the i3c service into raw MCTP packets suitable
//! for `Server::inbound()`. The counterpart to
//! `services/mctp/transport-i2c`'s `MctpI2cReceiver`.

use crate::encap::decode_write;

/// Decodes inbound I3C private writes into raw MCTP packets.
///
/// One instance per i3c target carrying MCTP traffic.
///
/// The PEC of a private write covers the address byte of the transfer, so
/// decoding needs this target's dynamic address. Reading it can cost an IPC
/// round-trip, so the receiver caches it and asks again only when a PEC check
/// fails -- the one case where the controller may have re-run ENTDAA and moved
/// the address.
pub struct MctpI3cReceiver {
    pec: bool,
    own_addr: Option<u8>,
}

impl MctpI3cReceiver {
    /// Create a receiver.
    ///
    /// `pec` must match what the controller sends. DSP0233 requires a PEC, so
    /// this is `true` for any conforming controller.
    pub const fn new(pec: bool) -> Self {
        Self {
            pec,
            own_addr: None,
        }
    }

    /// Decode one inbound frame into a raw MCTP packet.
    ///
    /// Verifies and strips the trailing PEC (when enabled) and returns the MCTP
    /// packet.
    ///
    /// `read_addr` returns this target's current 7-bit dynamic address, or `None`
    /// if the controller has not assigned one. It is called only when the cache
    /// is empty or the PEC check fails, never on the hot path, and never when PEC
    /// is disabled.
    pub fn decode<'a>(
        &mut self,
        data: &'a [u8],
        mut read_addr: impl FnMut() -> Option<u8>,
    ) -> Result<&'a [u8], mctp::Error> {
        if !self.pec {
            return decode_write(0, data, false);
        }

        let cached = match self.own_addr {
            Some(addr) => addr,
            None => {
                let addr = read_addr().ok_or(mctp::Error::Unreachable)?;
                self.own_addr = Some(addr);
                addr
            }
        };

        match decode_write(cached, data, true) {
            Ok(packet) => Ok(packet),
            Err(err) => {
                // A bad PEC may just mean the address moved. Re-read once and
                // retry only if it really changed.
                match read_addr() {
                    Some(addr) if addr != cached => {
                        self.own_addr = Some(addr);
                        decode_write(addr, data, true)
                    }
                    _ => Err(err),
                }
            }
        }
    }
}
