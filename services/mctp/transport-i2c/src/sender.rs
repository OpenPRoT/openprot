// Licensed under the Apache-2.0 license

//! I2C MCTP sender — outbound transport binding.
//!
//! Direct port of Hubris `mctp-server/src/i2c.rs` `I2cSender`.
//! Only the I2C driver API is replaced: `drv_i2c_api::I2cDevice` →
//! `i2c_api::I2cClientBlocking`.

use mctp::Result;
use mctp_lib::i2c::{MctpI2cEncap, MCTP_I2C_MAXMTU};
use i2c_api::{BusIndex, I2cAddress, I2cClientBlocking};

/// I2C MCTP sender.
///
/// Implements `mctp_lib::Sender` to fragment and send MCTP packets
/// over I2C using the OpenPRoT I2C client API.
///
/// This is a direct port of the Hubris `I2cSender`. The fragmentation
/// loop, I2C encoding via `MctpI2cEncap`, and error mapping are preserved
/// as-is. Only the I2C write call is changed from `drv_i2c_api::I2cDevice::write`
/// to `I2cClientBlocking::write`.
pub struct I2cSender<C: I2cClientBlocking> {
    i2c: C,
    bus: BusIndex,
    own_addr: u8,
    // TODO: neighbor table mapping EID → I2C address
    //       see https://github.com/OpenPRoT/mctp-lib/issues/4
    //       For now, destination address is hardcoded (same as Hubris).
}

impl<C: I2cClientBlocking> I2cSender<C> {
    /// Create a new I2C sender.
    ///
    /// * `i2c` - I2C client for bus writes
    /// * `bus` - I2C bus index to use
    /// * `own_addr` - Own I2C address (7-bit, used in MCTP-I2C header)
    pub fn new(i2c: C, bus: BusIndex, own_addr: u8) -> Self {
        Self {
            i2c,
            bus,
            own_addr,
        }
    }
}

impl<C: I2cClientBlocking> mctp_lib::Sender for I2cSender<C> {
    fn send_vectored(
        &mut self,
        mut fragmenter: mctp_lib::fragment::Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        // TODO The stack needs to provide the destination EID to the sender
        // let addr = self
        //     .neighbor_table
        //     .get(eid)
        //     .ok_or(mctp::Error::Unreachable)?;
        let addr = 0x42;
        let dest_address = I2cAddress::new_unchecked(addr);
        let encoder = MctpI2cEncap::new(self.own_addr);

        loop {
            let mut pkt = [0u8; mctp_lib::serial::MTU_MAX];
            let r = fragmenter.fragment_vectored(payload, &mut pkt);

            match r {
                mctp_lib::fragment::SendOutput::Packet(p) => {
                    let mut out = [0; MCTP_I2C_MAXMTU + 8]; // max MTU + I2C header size
                    let _packet = encoder.encode(addr, p, &mut out, true)?;
                    self.i2c
                        .write(self.bus, dest_address, &out)
                        .map_err(|_| mctp::Error::TxFailure)?;
                }
                mctp_lib::fragment::SendOutput::Complete { tag, .. } => {
                    break Ok(tag);
                }
                mctp_lib::fragment::SendOutput::Error { err, .. } => {
                    break Err(err);
                }
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_I2C_MAXMTU
    }
}
