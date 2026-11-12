// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C transport binding for the MCTP server.
//!
//! Mirrors `services/mctp/transport-i2c`, but over an I3C target rather than an
//! I2C bus. `mctp-estack` provides no I3C encapsulation, so [`encap`] supplies
//! the DSP0233 framing (an MCTP packet followed by a PEC); fragmentation and
//! reassembly stay in the MCTP stack, which is transport-independent.

#![cfg_attr(not(test), no_std)]

pub mod encap;
mod receiver;
mod sender;

pub use encap::{
    decode_write, transfer_pec, MctpI3cEncap, MCTP_I3C_IPC_FRAME, MCTP_I3C_IPC_MAXMTU,
    MCTP_I3C_IPC_PACKET,
};
pub use receiver::MctpI3cReceiver;
pub use sender::{I3cSender, MAX_BUSY_RETRIES};

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;
    use openprot_mctp_api::MCTP_HEADER_LEN;

    const OWN: u8 = 0x08;
    const OTHER: u8 = 0x1d;

    /// A frame as a conforming controller writes it to `addr`: the packet, then a
    /// PEC over the write address byte followed by the packet.
    fn controller_write(addr: u8, packet: &[u8]) -> Vec<u8> {
        let mut frame = packet.to_vec();
        frame.push(encap::smbus_pec(&[&[addr << 1][..], packet].concat()));
        frame
    }

    #[test]
    fn pec_matches_known_answer() {
        // CRC-8, polynomial 0x07: the SMBus PEC.
        assert_eq!(encap::smbus_pec(&[0x10, 0x01, 0x02, 0x03, 0x04]), 0xd1);
    }

    #[test]
    fn transfer_pec_covers_the_address_byte_then_the_packet() {
        let packet = [0x01, 0x02, 0x03, 0x04];
        assert_eq!(
            transfer_pec(0x10, &packet),
            encap::smbus_pec(&[0x10, 0x01, 0x02, 0x03, 0x04])
        );
        assert_ne!(transfer_pec(0x10, &packet), transfer_pec(0x11, &packet));
    }

    #[test]
    fn encode_has_no_header_and_appends_the_read_pec() {
        let encap = MctpI3cEncap::new(OWN);
        let packet = [0xde, 0xad, 0xbe, 0xef];
        let mut out = [0u8; 64];

        let n = encap.encode(&packet, &mut out).expect("encode");
        assert_eq!(n, packet.len() + 1);
        assert_eq!(&out[..packet.len()], &packet, "packet starts at byte 0");

        // A controller read of this target: the address byte has R/W# = 1.
        let expected = encap::smbus_pec(&[(OWN << 1) | 1, 0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(out[packet.len()], expected);
    }

    #[test]
    fn decode_strips_a_controller_write_pec() {
        let packet = [0x01, 0x0a, 0x08, 0xc8, 0x01, 0x02];
        let frame = controller_write(OWN, &packet);

        let decoded = MctpI3cEncap::new(OWN).decode(&frame).expect("decode");
        assert_eq!(decoded, &packet);
    }

    #[test]
    fn decode_rejects_corrupt_pec() {
        let mut frame = controller_write(OWN, &[0xaa, 0xbb, 0xcc]);
        let last = frame.len() - 1;
        frame[last] ^= 0xff;
        assert!(MctpI3cEncap::new(OWN).decode(&frame).is_err());
    }

    #[test]
    fn decode_rejects_a_pec_for_a_different_address() {
        let frame = controller_write(OTHER, &[0xaa, 0xbb, 0xcc]);
        assert!(MctpI3cEncap::new(OWN).decode(&frame).is_err());
    }

    #[test]
    fn decode_rejects_a_read_direction_pec() {
        // The PEC of a controller *read* covers (addr << 1) | 1; a write must
        // not be accepted with it.
        let packet = [0xaa, 0xbb, 0xcc];
        let mut frame = packet.to_vec();
        frame.push(transfer_pec((OWN << 1) | 1, &packet));
        assert!(MctpI3cEncap::new(OWN).decode(&frame).is_err());
    }

    #[test]
    fn decode_rejects_empty_and_pec_only_frames() {
        assert!(MctpI3cEncap::new(OWN).decode(&[]).is_err());
        assert!(MctpI3cEncap::new(OWN).decode(&[0x00]).is_err());
    }

    #[test]
    fn encode_rejects_undersized_buffer() {
        let encap = MctpI3cEncap::new(OWN);
        let packet = [0u8; 8];
        let mut out = [0u8; 4];
        assert!(encap.encode(&packet, &mut out).is_err());
        // Room for the packet but not the PEC.
        let mut out = [0u8; 8];
        assert!(encap.encode(&packet, &mut out).is_err());
    }

    #[test]
    fn encode_rejects_oversize_packet() {
        let encap = MctpI3cEncap::new(OWN);
        let packet = [0u8; MCTP_I3C_IPC_FRAME + 1];
        let mut out = [0u8; 512];
        assert!(encap.encode(&packet, &mut out).is_err());
        // The packet and its PEC must fit: one byte under the limit does, the
        // limit itself leaves no room for the PEC.
        let packet = [0u8; MCTP_I3C_IPC_FRAME - 1];
        assert!(encap.encode(&packet, &mut out).is_ok());
        let packet = [0u8; MCTP_I3C_IPC_FRAME];
        assert!(encap.encode(&packet, &mut out).is_err());
    }

    #[test]
    fn encode_rejects_a_non_7bit_address() {
        let mut out = [0u8; 16];
        assert!(MctpI3cEncap::new(0x80)
            .encode(&[1, 2, 3], &mut out)
            .is_err());
    }

    #[test]
    fn mtu_accounts_for_pec_and_the_mctp_header() {
        assert_eq!(
            MCTP_I3C_IPC_MAXMTU,
            i3c_api::MAX_PAYLOAD - MCTP_HEADER_LEN - 1
        );
        // A maximum-size packet plus its PEC exactly fills the i3c frame.
        assert_eq!(
            MCTP_I3C_IPC_MAXMTU + MCTP_HEADER_LEN + 1,
            MCTP_I3C_IPC_FRAME
        );
    }

    #[test]
    fn receiver_reads_the_address_once_and_caches_it() {
        let mut rx = MctpI3cReceiver::new();
        let reads = Cell::new(0u32);
        let read_addr = || {
            reads.set(reads.get() + 1);
            Some(OWN)
        };

        for n in 0..3u8 {
            let packet = [0x01, n, 0x03];
            let frame = controller_write(OWN, &packet);
            assert_eq!(rx.decode(&frame, read_addr).expect("decode"), &packet);
        }
        assert_eq!(reads.get(), 1, "address should be read once, then cached");
    }

    #[test]
    fn receiver_re_reads_the_address_after_a_pec_failure() {
        let mut rx = MctpI3cReceiver::new();
        let current = Cell::new(OWN);
        let reads = Cell::new(0u32);
        let read_addr = || {
            reads.set(reads.get() + 1);
            Some(current.get())
        };

        let packet = [0x01, 0x02, 0x03];
        rx.decode(&controller_write(OWN, &packet), read_addr)
            .expect("first decode");
        assert_eq!(reads.get(), 1);

        // The controller re-ran ENTDAA: the target now answers at a new address.
        current.set(OTHER);
        let frame = controller_write(OTHER, &packet);
        assert_eq!(rx.decode(&frame, read_addr).expect("decode"), &packet);
        assert_eq!(reads.get(), 2, "one re-read after the PEC mismatch");

        // The new address is now cached.
        let frame = controller_write(OTHER, &packet);
        rx.decode(&frame, read_addr).expect("decode");
        assert_eq!(reads.get(), 2);
    }

    #[test]
    fn receiver_fails_before_an_address_is_assigned() {
        let mut rx = MctpI3cReceiver::new();
        let frame = controller_write(OWN, &[0x01, 0x02, 0x03]);
        assert!(rx.decode(&frame, || None).is_err());
    }

    #[test]
    fn receiver_rejects_a_corrupt_pec_without_changing_the_cache() {
        let mut rx = MctpI3cReceiver::new();
        let packet = [0x01, 0x02, 0x03];
        rx.decode(&controller_write(OWN, &packet), || Some(OWN))
            .expect("decode");

        let mut bad = controller_write(OWN, &packet);
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert!(rx.decode(&bad, || Some(OWN)).is_err());

        // Still decodes good frames afterwards.
        rx.decode(&controller_write(OWN, &packet), || Some(OWN))
            .expect("decode");
    }
}
