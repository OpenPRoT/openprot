// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash request handling, independent of the kernel and of IPC.
//!
//! [`Server::handle_frame`] takes one request frame and leaves one response
//! frame in the same buffer. The kernel loop that moves those frames over a
//! channel is `flash_server_runtime`.

use hal_flash::{Flash, FlashAddress};
use openprot_flash_api::*;
use util_error::{self as error, ErrorCode};
use util_types::{Opcode, PowerOf2Usize};
use zerocopy::{FromBytes, IntoBytes};

const OPCODE_LEN: usize = core::mem::size_of::<Opcode>();
const STATUS_LEN: usize = core::mem::size_of::<u32>();

// A reply is built right after the request's opcode, so the status word that
// precedes it in the response has to fit exactly where the opcode was.
const _: () = assert!(OPCODE_LEN == STATUS_LEN);

/// Serves flash requests against a [`Flash`].
///
/// A request is untrusted. Each handler establishes the preconditions the
/// driver documents (erase size in the erasable-size bitmap, erase address
/// aligned to that size, range inside the device) before calling [`Flash`].
/// Read and program may be unaligned; `BlockingFlash` splits those, but does
/// not check that the range stays on the device.
pub struct Server<TFlash: Flash> {
    flash: TFlash,
}

impl<TFlash: Flash<Error = ErrorCode>> Server<TFlash> {
    /// Creates a new `Server` wrapping the given flash implementation.
    pub fn new(flash: TFlash) -> Self {
        Self { flash }
    }

    /// Handles the `IPC_OP_FLASH_GET_INFO` request.
    ///
    /// Writes the flash geometry to the front of `data` and returns its length.
    fn handle_geometry(&mut self, data: &mut [u8], reqsz: usize) -> Result<usize, ErrorCode> {
        if reqsz != 0 {
            return Err(error::IPC_ERROR_BAD_REQ_LEN);
        }
        let (info, _rest) =
            FlashInfo::mut_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (total_size, page_size, erasable_sizes_bitmap) = self.flash.geometry()?;
        info.page_size = page_size.get() as u32;
        info.total_size = total_size.get() as u32;
        info.erasable_sizes_bitmap = erasable_sizes_bitmap;
        Ok(info.as_bytes().len())
    }

    /// Device size and erasable-size bitmap, from the flash the server wraps.
    fn device_limits(&mut self) -> Result<(usize, u32), ErrorCode> {
        let (total, _page, bitmap) = self.flash.geometry()?;
        Ok((total.get(), bitmap))
    }

    /// Handles the `IPC_OP_FLASH_ERASE` request.
    ///
    /// Parses the `EraseOp` from the input data and erases the specified block.
    /// The size must be a supported erase size, the address must be aligned to
    /// it, and the block must lie inside the device.
    fn handle_erase(&mut self, data: &mut [u8], reqsz: usize) -> Result<usize, ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let op = EraseOp::read_from_bytes(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let Some(size) = PowerOf2Usize::new(op.size as usize) else {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        };
        let (total, bitmap) = self.device_limits()?;
        erase_size_supported(size, bitmap)?;
        erase_address_aligned(op.address, size)?;
        region_in_bounds(op.address, size.get(), total)?;
        self.flash.erase(op.address, size)?;
        Ok(0)
    }

    /// Handles the `IPC_OP_FLASH_PROGRAM` request.
    ///
    /// Parses the start address and data from the input, rejects a range
    /// outside the device, then programs it.
    fn handle_program(&mut self, data: &mut [u8], reqsz: usize) -> Result<usize, ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let (addr, program_data) =
            FlashAddress::read_from_prefix(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (total, _bitmap) = self.device_limits()?;
        region_in_bounds(addr, program_data.len(), total)?;
        self.flash.program(addr, program_data)?;
        Ok(0)
    }

    /// Handles the `IPC_OP_FLASH_READ` request.
    ///
    /// Parses the `ReadOp` from the input, rejects a length the reply buffer
    /// cannot hold or a range outside the device, then reads to the front of
    /// `data` and returns the length.
    fn handle_read(&mut self, data: &mut [u8], reqsz: usize) -> Result<usize, ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let op = ReadOp::read_from_bytes(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let length = op.length as usize;
        let Some(dst) = data.get_mut(..length) else {
            return Err(error::FLASH_GENERIC_INVALID_SIZE);
        };
        let (total, _bitmap) = self.device_limits()?;
        region_in_bounds(op.address, length, total)?;
        self.flash.read(op.address, dst)?;
        Ok(length)
    }

    /// Handles one request: `data` holds `reqsz` request bytes and receives
    /// the reply at its front. Returns the reply length.
    fn handle_op(
        &mut self,
        opcode: Opcode,
        data: &mut [u8],
        reqsz: usize,
    ) -> Result<usize, ErrorCode> {
        match opcode {
            IPC_OP_FLASH_GET_INFO => self.handle_geometry(data, reqsz),
            IPC_OP_FLASH_ERASE => self.handle_erase(data, reqsz),
            IPC_OP_FLASH_PROGRAM => self.handle_program(data, reqsz),
            IPC_OP_FLASH_READ => self.handle_read(data, reqsz),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    /// Decodes the request in `buf[..len]`, an opcode then its body, and runs
    /// it. The reply is left in `buf` right after the opcode.
    fn handle_request(&mut self, buf: &mut [u8], len: usize) -> Result<usize, ErrorCode> {
        let body_len = len
            .checked_sub(OPCODE_LEN)
            .ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let (head, body) = buf
            .split_at_mut_checked(OPCODE_LEN)
            .ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let opcode = Opcode::read_from_bytes(head).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.handle_op(opcode, body, body_len)
    }

    /// Serves the request frame in `buf[..len]` and leaves the response frame
    /// at the start of `buf`: a four-byte status, zero for success or an
    /// `ErrorCode`, then the reply. Returns the frame length.
    ///
    /// Only the first `len` bytes of `buf` are the request; the rest is stale.
    /// The frame is built in place, so `buf` has to hold the larger of the
    /// request and the response. Returns 0 if `buf` cannot hold a status.
    pub fn handle_frame(&mut self, buf: &mut [u8], len: usize) -> usize {
        let (status, reply_len) = match self.handle_request(buf, len) {
            Ok(reply_len) => (0, reply_len),
            Err(e) => (e.0.get(), 0),
        };
        let Some(head) = buf.get_mut(..STATUS_LEN) else {
            return 0;
        };
        head.copy_from_slice(status.as_bytes());
        STATUS_LEN + reply_len
    }
}

/// `size` must be a bit in `bitmap`: bit `i` means an erase of `2^i` bytes.
fn erase_size_supported(size: PowerOf2Usize, bitmap: u32) -> Result<(), ErrorCode> {
    match 1u32.checked_shl(size.get().trailing_zeros()) {
        Some(bit) if bitmap & bit != 0 => Ok(()),
        _ => Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE),
    }
}

/// `BlockingFlash` does not align an erase before `FlashDriver::start_erase`.
fn erase_address_aligned(address: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
    let Ok(size) = u32::try_from(size.get()) else {
        return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
    };
    if address.offset() & size.wrapping_sub(1) != 0 {
        return Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR);
    }
    Ok(())
}

/// `[address, address + len)` must lie in `[0, total)`.
fn region_in_bounds(address: FlashAddress, len: usize, total: usize) -> Result<(), ErrorCode> {
    let in_bounds = u64::try_from(len)
        .ok()
        .and_then(|len| u64::from(address.offset()).checked_add(len))
        .is_some_and(|end| end <= total as u64);
    if in_bounds {
        Ok(())
    } else {
        Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
    }
}
