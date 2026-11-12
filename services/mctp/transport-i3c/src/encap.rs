// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP-over-I3C encapsulation (DSP0233).
//!
//! `mctp-estack` ships `i2c`, `serial` and `usb` bindings but no I3C one, so
//! the framing lives here.
//!
//! # Wire format
//!
//! DSP0233 carries one MCTP packet as the data of one I3C private transfer, with
//! a PEC byte after it. Unlike the SMBus/I2C binding (DSP0237) there is **no**
//! command code, byte count or source-address field in front of the packet:
//!
//! ```text
//! [Target Address + R/W][ACK][ MCTP packet ... ][ PEC ]
//!                             ^ Write/Read Data 1
//! ```
//!
//! * The address byte is the address phase of the bus transfer. It is not part
//!   of the data this module sees or produces.
//! * The PEC is an SMBus CRC-8 over the address byte (`addr << 1 | R/W`),
//!   followed by the packet. For a controller write to this target that is
//!   `(own_addr << 1) | 0`; for a controller read of this target's response it
//!   is `(own_addr << 1) | 1`.
//! * DSP0233 requires the PEC, so there is no way to turn it off here.
//!
//! Addresses are I3C dynamic addresses, assigned by the controller during ENTDAA,
//! so `own_addr` is read back from the hardware at runtime, not a build-time
//! constant.
//!
//! The MCTP packet itself is transport-independent, so fragmentation and
//! reassembly stay in `mctp-estack`.

use mctp::{Error, Result};
use openprot_mctp_api::MCTP_HEADER_LEN;

/// R/W# bit of the address byte for a controller write (to this target).
const WRITE: u8 = 0;
/// R/W# bit of the address byte for a controller read (of this target).
const READ: u8 = 1;

/// Bitwise SMBus CRC-8 step: polynomial 0x07, no reflection. Only used to build
/// [`CRC8_TABLE`] at compile time.
const fn crc8_step_bitwise(mut crc: u8, value: u8) -> u8 {
    crc ^= value;
    let mut i = 0;
    while i < 8 {
        crc = if crc & 0x80 != 0 {
            (crc << 1) ^ 0x07
        } else {
            crc << 1
        };
        i += 1;
    }
    crc
}

/// CRC-8 (poly 0x07) of every single byte value, so a step is one lookup. The
/// bitwise loop costs ~60-90 instructions per byte, which dominated frame
/// decode on the slow core.
static CRC8_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = crc8_step_bitwise(0, i as u8);
        i += 1;
    }
    table
};

/// One SMBus CRC-8 step.
#[inline(always)]
fn crc8_step(crc: u8, value: u8) -> u8 {
    CRC8_TABLE[(crc ^ value) as usize]
}

/// SMBus PEC: CRC-8 with polynomial 0x07, zero seed, no reflection.
///
/// Inlined rather than pulled from `smbus-pec`, which is only a transitive
/// dependency here. A known-answer test in this crate pins it.
pub fn smbus_pec(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for &value in data {
        crc = crc8_step(crc, value);
    }
    crc
}

/// PEC of one I3C transfer: CRC-8 over the address byte, then the packet.
///
/// `addr_byte` is the 7-bit target address shifted left once with the transfer's
/// R/W# bit in bit 0 (see [`addr_byte`]).
pub fn transfer_pec(addr_byte: u8, packet: &[u8]) -> u8 {
    let mut crc = crc8_step(0, addr_byte);
    for &value in packet {
        crc = crc8_step(crc, value);
    }
    crc
}

/// The address byte of a transfer: 7-bit address plus the R/W# bit.
const fn addr_byte(addr: u8, rnw: u8) -> u8 {
    (addr << 1) | rnw
}

/// The MTU `I3cSender::get_mtu` reports (251).
///
/// The i3c service carries one framed packet per `I3cOp::Send`, capped at
/// `i3c_api::MAX_PAYLOAD` (the TTI FIFO size). `mctp-estack`'s `Fragmenter`
/// treats its MTU as the largest whole packet, transport header included, so
/// the MTU is what is left of the frame after the PEC:
///
/// ```text
/// packet + pec <= 256   =>   packet <= 255
/// ```
///
/// This reports 4 bytes (the MCTP header) fewer than that, so a frame has room
/// to spare; raising it to `MAX_PAYLOAD - 1` would use them. The derivation is
/// the bench branch's, which ran at 245 when the service limit was 250.
///
/// Derived from `i3c_api::MAX_PAYLOAD` rather than a literal so it tracks the
/// service limit.
pub const MCTP_I3C_IPC_MAXMTU: usize = i3c_api::MAX_PAYLOAD - MCTP_HEADER_LEN - 1;

/// Buffer size for `Fragmenter::fragment_vectored`, which refuses to emit into
/// anything smaller than its MTU.
pub const MCTP_I3C_IPC_PACKET: usize = MCTP_I3C_IPC_MAXMTU + MCTP_HEADER_LEN;

/// Largest whole framed packet the transport ever hands to `I3cOp::Send`.
pub const MCTP_I3C_IPC_FRAME: usize = i3c_api::MAX_PAYLOAD;

/// Strip the PEC from an inbound private write and return the MCTP packet.
///
/// `own_addr` is this target's 7-bit dynamic address: the controller's write
/// address phase was `(own_addr << 1) | 0`, and that byte is covered by the PEC
/// even though it is not in `frame`.
///
/// Fails if the frame is empty, the PEC is wrong, or nothing is left after the
/// PEC is removed.
pub fn decode_write(own_addr: u8, frame: &[u8]) -> Result<&[u8]> {
    if frame.is_empty() {
        return Err(Error::InvalidInput);
    }
    if own_addr > 0x7f {
        return Err(Error::BadArgument);
    }

    let (received, packet) = frame.split_last().ok_or(Error::InvalidInput)?;
    if packet.is_empty() || transfer_pec(addr_byte(own_addr, WRITE), packet) != *received {
        return Err(Error::InvalidInput);
    }
    Ok(packet)
}

/// Frames MCTP packets for transfer over I3C.
///
/// `own_addr` is this target's I3C dynamic address. It is not known until the
/// controller has run ENTDAA, so construct this after the address has been read
/// back from the hardware.
#[derive(Debug, Clone)]
pub struct MctpI3cEncap {
    own_addr: u8,
}

impl MctpI3cEncap {
    /// Bind to a dynamic address.
    pub fn new(own_addr: u8) -> Self {
        Self { own_addr }
    }

    /// This target's dynamic address.
    pub fn own_addr(&self) -> u8 {
        self.own_addr
    }

    /// Strip the PEC from an inbound private write (see [`decode_write`]).
    pub fn decode<'f>(&self, frame: &'f [u8]) -> Result<&'f [u8]> {
        decode_write(self.own_addr, frame)
    }

    /// Frame one MCTP packet for an outbound transfer, which the controller
    /// collects with a private read.
    ///
    /// Writes the packet and the trailing PEC into `out`, returning the framed
    /// length. The PEC covers `(own_addr << 1) | 1`
    /// followed by the packet.
    ///
    /// Fails with [`Error::NoSpace`] if `out` is too small, or
    /// [`Error::BadArgument`] if the address is not 7-bit or the framed packet
    /// would exceed [`MCTP_I3C_IPC_FRAME`].
    pub fn encode(&self, packet: &[u8], out: &mut [u8]) -> Result<usize> {
        if self.own_addr > 0x7f {
            return Err(Error::BadArgument);
        }

        let framed = packet.len() + 1;
        if framed > MCTP_I3C_IPC_FRAME {
            return Err(Error::BadArgument);
        }

        out.get_mut(..packet.len())
            .ok_or(Error::NoSpace)?
            .copy_from_slice(packet);

        let crc = transfer_pec(addr_byte(self.own_addr, READ), packet);
        *out.get_mut(packet.len()).ok_or(Error::NoSpace)? = crc;

        Ok(framed)
    }
}

#[cfg(test)]
mod crc_tests {
    use super::*;

    #[test]
    fn table_matches_bitwise_for_all_inputs() {
        for crc in 0..=255u8 {
            for value in 0..=255u8 {
                assert_eq!(crc8_step(crc, value), crc8_step_bitwise(crc, value));
            }
        }
    }

    #[test]
    fn smbus_pec_check_value() {
        // CRC-8/SMBUS check value for "123456789".
        assert_eq!(smbus_pec(b"123456789"), 0xF4);
    }
}
