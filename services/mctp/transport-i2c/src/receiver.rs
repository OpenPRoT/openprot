// Licensed under the Apache-2.0 license

//! I2C MCTP receiver — inbound transport binding.
//!
//! Decodes incoming I2C target-mode messages into raw MCTP packets
//! that can be fed to `Server::inbound()`.
//!
//! This corresponds to the `handle_i2c_transport` function in Hubris
//! `mctp-server/src/main.rs`, using `mctp_lib::i2c::MctpI2cEncap`
//! for decoding (same as Hubris).

use mctp_lib::i2c::MctpI2cEncap;
use i2c_api::TargetMessage;

/// Decodes I2C target messages into raw MCTP packets.
///
/// Wraps the `mctp_lib::i2c::MctpI2cEncap` decoder. One instance
/// should exist per I2C bus carrying MCTP traffic.
pub struct MctpI2cReceiver {
    encap: MctpI2cEncap,
}

impl MctpI2cReceiver {
    /// Create a new receiver for the given own I2C address.
    pub fn new(own_addr: u8) -> Self {
        Self {
            encap: MctpI2cEncap::new(own_addr),
        }
    }

    /// Decode an I2C target message into a raw MCTP packet.
    ///
    /// Strips the MCTP-I2C transport header and validates PEC.
    /// Returns the raw MCTP packet bytes (suitable for `Server::inbound()`)
    /// and the I2C source address, or an error if decoding fails.
    ///
    /// This is the same decode path as Hubris `handle_i2c_transport`:
    /// `i2c_reader.recv(data)` → `server.stack.inbound(pkt)`.
    pub fn decode<'a>(
        &self,
        msg: &'a TargetMessage,
    ) -> Result<(&'a [u8], u8), mctp::Error> {
        let data = msg.data();
        // MctpI2cEncap::decode strips the I2C header, validates PEC,
        // and returns the raw MCTP packet + source I2C address.
        self.encap.decode(data, true)
    }
}
