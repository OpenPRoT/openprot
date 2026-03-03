// Licensed under the Apache-2.0 license

//! MCTP-over-I2C transport adapter
//!
//! Implements [`mctp_lib::Sender`] for the I2C IPC client, enabling it to be
//! used as an MCTP transport binding (DSP0237 SMBus/I2C Transport Binding).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use i2c_api::{BusIndex, I2cAddress};
//! use i2c_client::IpcI2cClient;
//! use i2c_mctp_transport::MctpI2cSender;
//!
//! let client = IpcI2cClient::new(handle::I2C);
//! let dest = I2cAddress::new(0x1d).unwrap();
//! let sender = MctpI2cSender::new(client, BusIndex::BUS_0, 0x1e, dest);
//!
//! // Pass `sender` to mctp_lib::Router::new(own_eid, now_millis, sender)
//! ```

#![no_std]
#![warn(missing_docs)]

use i2c_api::{BusIndex, I2cAddress, I2cClient};
use i2c_client::IpcI2cClient;
use mctp_lib::{
    Sender,
    fragment::{Fragmenter, SendOutput},
    i2c::{MctpI2cEncap, MCTP_I2C_MAXMTU},
};
use mctp::{Eid, Result, Tag};

/// Buffer large enough for one MCTP-I2C packet: 4-byte transport header + payload.
const PKT_BUF_SIZE: usize = 4 + MCTP_I2C_MAXMTU;

/// MCTP-over-I2C transport sender backed by the I2C IPC service.
///
/// Wraps an [`IpcI2cClient`] and implements [`mctp_lib::Sender`] by fragmenting
/// MCTP messages and transmitting each fragment as an SMBus/I2C write.
///
/// This is the analogue of `IoSerialSender` in the `standalone` crate, but for
/// the I2C transport binding defined in DMTF DSP0237.
pub struct MctpI2cSender {
    client: IpcI2cClient,
    encap: MctpI2cEncap,
    bus: BusIndex,
    dest_addr: I2cAddress,
}

impl MctpI2cSender {
    /// Create a new `MctpI2cSender`.
    ///
    /// # Arguments
    /// * `client`    - IPC client connected to the I2C server
    /// * `bus`       - I2C bus to transmit MCTP packets on
    /// * `own_addr`  - This endpoint's 7-bit I2C address (MCTP source)
    /// * `dest_addr` - Remote endpoint's 7-bit I2C address (MCTP destination)
    pub fn new(
        client: IpcI2cClient,
        bus: BusIndex,
        own_addr: u8,
        dest_addr: I2cAddress,
    ) -> Self {
        Self {
            client,
            encap: MctpI2cEncap::new(own_addr),
            bus,
            dest_addr,
        }
    }

    /// Return a shared reference to the underlying [`IpcI2cClient`].
    pub fn i2c_client(&self) -> &IpcI2cClient {
        &self.client
    }

    /// Return a mutable reference to the underlying [`IpcI2cClient`].
    pub fn i2c_client_mut(&mut self) -> &mut IpcI2cClient {
        &mut self.client
    }
}

impl Sender for MctpI2cSender {
    fn send_vectored(
        &mut self,
        _eid: Eid,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> Result<Tag> {
        loop {
            // Obtain the next MCTP packet fragment (no transport header yet).
            let mut inner_buf = [0u8; MCTP_I2C_MAXMTU];
            match fragmenter.fragment_vectored(payload, &mut inner_buf) {
                SendOutput::Packet(mctp_pkt) => {
                    // Prepend the 4-byte MCTP-I2C transport header.
                    let mut out_buf = [0u8; PKT_BUF_SIZE];
                    let packet = self
                        .encap
                        .encode(self.dest_addr.value(), mctp_pkt, &mut out_buf, false)
                        .map_err(|_| mctp::Error::TxFailure)?;

                    // Transmit as an I2C write to the destination address.
                    self.client
                        .write_read(self.bus, self.dest_addr, packet, &mut [])
                        .map_err(|_| mctp::Error::TxFailure)?;
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_I2C_MAXMTU
    }
}
