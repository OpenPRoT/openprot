// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A request that breaks a driver precondition is rejected before the flash
//! is called.

use core::cell::Cell;
use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use openprot_flash_api::*;
use openprot_flash_server::Server;
use util_error::{self as error, ErrorCode};
use util_types::{Opcode, PowerOf2Usize};
use zerocopy::{FromBytes, IntoBytes};

const TOTAL: usize = 0x1_0000;
const SECTOR: usize = 4096;
const OPCODE_LEN: usize = core::mem::size_of::<Opcode>();

struct CountingFlash<'a> {
    calls: &'a Cell<u32>,
}

impl Flash for CountingFlash<'_> {
    type Error = ErrorCode;

    fn geometry(&mut self) -> Result<(NonZero<usize>, PowerOf2Usize, u32), ErrorCode> {
        let total = NonZero::new(TOTAL).unwrap();
        Ok((total, PowerOf2Usize::new(SECTOR).unwrap(), 1 << 12))
    }

    fn read(&mut self, _: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        self.calls.set(self.calls.get() + 1);
        buf.fill(0xaa);
        Ok(())
    }

    fn erase(&mut self, _: FlashAddress, _: PowerOf2Usize) -> Result<(), ErrorCode> {
        self.calls.set(self.calls.get() + 1);
        Ok(())
    }

    fn program(&mut self, _: FlashAddress, _: &[u8]) -> Result<(), ErrorCode> {
        self.calls.set(self.calls.get() + 1);
        Ok(())
    }
}

/// Decodes a response frame: a status word, then the reply if it is zero.
fn decode(frame: &[u8]) -> Result<Vec<u8>, ErrorCode> {
    let status = u32::from_ne_bytes(frame[..4].try_into().unwrap());
    ErrorCode::check_status(status).map(|()| frame[4..].to_vec())
}

/// Returns the reply, or the error, and how many times the flash was called.
fn serve(op: Opcode, parts: &[&[u8]]) -> (Result<Vec<u8>, ErrorCode>, u32) {
    let calls = Cell::new(0);
    let mut server = Server::new(CountingFlash { calls: &calls });
    let mut buf = [0u8; 8192];
    buf[..OPCODE_LEN].copy_from_slice(op.as_bytes());
    let mut len = OPCODE_LEN;
    for part in parts {
        buf[len..len + part.len()].copy_from_slice(part);
        len += part.len();
    }
    let frame_len = server.handle_frame(&mut buf, len);
    (decode(&buf[..frame_len]), calls.get())
}

fn erase(address: u32, size: u32) -> (Result<Vec<u8>, ErrorCode>, u32) {
    let op = EraseOp {
        address: FlashAddress::new(address),
        size,
    };
    serve(IPC_OP_FLASH_ERASE, &[op.as_bytes()])
}

fn program(address: u32, data: &[u8]) -> (Result<Vec<u8>, ErrorCode>, u32) {
    let address = FlashAddress::new(address);
    serve(IPC_OP_FLASH_PROGRAM, &[address.as_bytes(), data])
}

fn read(address: u32, length: u32) -> (Result<Vec<u8>, ErrorCode>, u32) {
    let op = ReadOp {
        address: FlashAddress::new(address),
        length,
    };
    serve(IPC_OP_FLASH_READ, &[op.as_bytes()])
}

#[test]
fn erase_of_a_supported_aligned_block_reaches_the_flash() {
    let (reply, calls) = erase(0x1000, SECTOR as u32);
    assert!(reply.is_ok());
    assert_eq!(calls, 1);
}

#[test]
fn erase_size_outside_the_bitmap_is_rejected() {
    let (reply, calls) = erase(0, 512);
    assert!(reply == Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE));
    assert_eq!(calls, 0);
}

#[test]
fn erase_size_that_is_not_a_power_of_two_is_rejected() {
    let (reply, calls) = erase(0, 3000);
    assert!(reply == Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE));
    assert_eq!(calls, 0);
}

#[test]
fn erase_address_not_aligned_to_the_size_is_rejected() {
    let (reply, calls) = erase(0x1100, SECTOR as u32);
    assert!(reply == Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR));
    assert_eq!(calls, 0);
}

#[test]
fn erase_past_the_end_is_rejected() {
    let (reply, calls) = erase(TOTAL as u32, SECTOR as u32);
    assert!(reply == Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS));
    assert_eq!(calls, 0);
}

#[test]
fn program_inside_the_device_reaches_the_flash() {
    let (reply, calls) = program(0x10, &[1, 2, 3, 4]);
    assert!(reply.is_ok());
    assert_eq!(calls, 1);
}

#[test]
fn program_running_past_the_end_is_rejected() {
    let (reply, calls) = program(TOTAL as u32 - 4, &[0; 8]);
    assert!(reply == Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS));
    assert_eq!(calls, 0);
}

#[test]
fn program_whose_range_wraps_the_address_space_is_rejected() {
    let (reply, calls) = program(u32::MAX - 1, &[0; 8]);
    assert!(reply == Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS));
    assert_eq!(calls, 0);
}

#[test]
fn read_inside_the_device_returns_the_data() {
    let (reply, calls) = read(0x20, 16);
    assert!(reply == Ok(vec![0xaa; 16]));
    assert_eq!(calls, 1);
}

#[test]
fn read_running_past_the_end_is_rejected() {
    let (reply, calls) = read(TOTAL as u32 - 8, 16);
    assert!(reply == Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS));
    assert_eq!(calls, 0);
}

#[test]
fn read_longer_than_the_reply_buffer_is_rejected() {
    let (reply, calls) = read(0, 100_000);
    assert!(reply == Err(error::FLASH_GENERIC_INVALID_SIZE));
    assert_eq!(calls, 0);
}

#[test]
fn truncated_request_is_rejected() {
    let (reply, calls) = serve(IPC_OP_FLASH_ERASE, &[&[0u8; 1]]);
    assert!(reply == Err(error::IPC_ERROR_BAD_REQ_LEN));
    assert_eq!(calls, 0);
}

#[test]
fn unknown_opcode_is_rejected() {
    let (reply, calls) = serve(Opcode::new(*b"NOPE"), &[]);
    assert!(reply == Err(error::IPC_ERROR_UNKNOWN_OP));
    assert_eq!(calls, 0);
}

#[test]
fn get_info_reports_the_geometry() {
    let (reply, calls) = serve(IPC_OP_FLASH_GET_INFO, &[]);
    let info = FlashInfo::read_from_bytes(&reply.unwrap()).unwrap();
    assert_eq!(info.total_size, TOTAL as u32);
    assert_eq!(info.page_size, SECTOR as u32);
    assert_eq!(info.erasable_sizes_bitmap, 1 << 12);
    assert_eq!(calls, 0);
}

#[test]
fn request_shorter_than_an_opcode_is_not_served_under_the_stale_one() {
    let calls = Cell::new(0);
    let mut server = Server::new(CountingFlash { calls: &calls });
    let mut buf = [0u8; 8192];

    for len in 0..OPCODE_LEN {
        buf[..OPCODE_LEN].copy_from_slice(IPC_OP_FLASH_GET_INFO.as_bytes());
        let frame_len = server.handle_frame(&mut buf, len);
        assert!(decode(&buf[..frame_len]) == Err(error::IPC_ERROR_BAD_REQ_LEN));
    }
}

#[test]
fn a_buffer_too_small_for_a_status_gets_no_frame() {
    let calls = Cell::new(0);
    let mut server = Server::new(CountingFlash { calls: &calls });
    let mut buf = [0u8; 3];
    assert_eq!(server.handle_frame(&mut buf, 3), 0);
}
