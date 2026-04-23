// Licensed under the Apache-2.0 license

//! I2C MCTP sender — outbound transport binding.
//!
//! Direct port of Hubris `mctp-server/src/i2c.rs` `I2cSender`.
//! Only the I2C driver API is replaced: `drv_i2c_api::I2cDevice` →
//! `i2c_api::I2cClientBlocking`.

use mctp::{Result, Eid};
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
    // Simple static remote address. Full neighbor table (EID → I2C address mapping)
    // will be implemented later per https://github.com/OpenPRoT/mctp-lib/issues/4.
    // For now, this supports single-peer communication (requester ↔ responder).
    remote_addr: u8,
}

impl<C: I2cClientBlocking> I2cSender<C> {
    /// Create a new I2C sender.
    ///
    /// * `i2c` - I2C client for bus writes
    /// * `bus` - I2C bus index to use
    /// * `own_addr` - Own I2C address (7-bit, used in MCTP-I2C header)
    /// * `remote_addr` - Remote peer's I2C address (7-bit, destination for outbound packets)
    pub fn new(i2c: C, bus: BusIndex, own_addr: u8, remote_addr: u8) -> Self {
        Self {
            i2c,
            bus,
            own_addr,
            remote_addr,
        }
    }
}

impl<C: I2cClientBlocking> mctp_lib::Sender for I2cSender<C> {
    fn send_vectored(
        &mut self,
        _eid: Eid,
        mut fragmenter: mctp_lib::fragment::Fragmenter,
        payload: &[&[u8]],
    ) -> Result<mctp::Tag> {
        // Use the configured remote address. In a full implementation, this would
        // look up the destination EID in a neighbor table to find the corresponding
        // I2C address. For now, we support single-peer communication with a static
        // remote address configured at construction time.
        // TODO: Implement full EID → I2C address neighbor table
        //       (see https://github.com/OpenPRoT/mctp-lib/issues/4)
        let addr = self.remote_addr;
        let dest_address = I2cAddress::new_unchecked(addr);
        let encoder = MctpI2cEncap::new(self.own_addr);
        let mtu = self.get_mtu();
        pw_log::info!("Starting fragmentation: MTU=0x{:04x}, buffer size=0x{:04x}", mtu as u32, mctp_lib::serial::MTU_MAX as u32);

        loop {
            let mut pkt = [0u8; 254]; //mctp_lib::serial::MTU_MAX];
            pw_log::debug!("Calling fragment_vectored with buffer size 0x{:04x}", pkt.len() as u32);
            let r = fragmenter.fragment_vectored(payload, &mut pkt);

            match r {
                mctp_lib::fragment::SendOutput::Packet(p) => {
                    let addr_u8 = u8::from(dest_address);
                    pw_log::info!("packet sending to 0x{:02x}...", addr_u8 as u32);
                    let mut out = [0; MCTP_I2C_MAXMTU + 8]; // max MTU + I2C header size
                    let packet = encoder.encode(addr, p, &mut out, true)?;
                    pw_log::debug!("Encoded packet length: 0x{:04x}", packet.len() as u32);

                    let packet_len = packet.len();
                    if let Err(i2c_err) = self.i2c.write(self.bus, dest_address, packet) {
                        use embedded_hal::i2c::Error as _;
                        let kind = i2c_err.kind();
                        let kind_code = match kind {
                            embedded_hal::i2c::ErrorKind::Bus => 0u32,
                            embedded_hal::i2c::ErrorKind::ArbitrationLoss => 1u32,
                            embedded_hal::i2c::ErrorKind::NoAcknowledge(src) => match src {
                                embedded_hal::i2c::NoAcknowledgeSource::Address => 2u32,
                                embedded_hal::i2c::NoAcknowledgeSource::Data => 3u32,
                                embedded_hal::i2c::NoAcknowledgeSource::Unknown => 4u32,
                            },
                            embedded_hal::i2c::ErrorKind::Overrun => 5u32,
                            embedded_hal::i2c::ErrorKind::Other => 6u32,
                            _ => 0xFFu32,
                        };
                        pw_log::error!("I2C write failed: kind=0x{:02x}", kind_code as u32);
                        pw_log::error!("Bus: 0x{:02x}, Address: 0x{:02x}, Data len: 0x{:04x}",
                                      self.bus.value() as u32, addr_u8 as u32, packet_len as u32);
                        return Err(mctp::Error::TxFailure);
                    }
                    pw_log::info!("packet sent");
                }
                mctp_lib::fragment::SendOutput::Complete { tag, .. } => {
                    pw_log::info!("complete");
                    break Ok(tag);
                }
                mctp_lib::fragment::SendOutput::Error { err, .. } => {
                    let err_code = match err {
                        mctp::Error::TxFailure => 0u32,
                        mctp::Error::RxFailure => 1u32,
                        mctp::Error::TimedOut => 2u32,
                        mctp::Error::BadArgument => 3u32,
                        mctp::Error::InvalidInput => 4u32,
                        mctp::Error::TagUnavailable => 5u32,
                        mctp::Error::Unreachable => 6u32,
                        mctp::Error::AddrInUse => 7u32,
                        mctp::Error::NoSpace => 8u32,
                        mctp::Error::Unsupported => 9u32,
                        _ => 0xFFu32,
                    };
                    pw_log::error!("fragment_vectored failed with error: 0x{:02x}", err_code as u32);
                    pw_log::error!("Buffer size: 0x{:04x}, MTU: 0x{:04x}", mctp_lib::serial::MTU_MAX as u32, mtu as u32);
                    break Err(err);
                }
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_I2C_MAXMTU
    }
}
