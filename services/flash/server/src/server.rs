// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash request handling, independent of the kernel and of IPC.
//!
//! [`Server::handle_frame`] takes one request frame and leaves one response frame in the same
//! buffer, or reports [`Outcome::Pending`] for an erase not yet done; [`Server::poll_pending_erase`]
//! finishes that one once [`Flash::poll_erase`] stops reporting busy.

use hal_flash::{Flash, FlashAddress, OpPoll};
use openprot_flash_api::*;
use util_error::{self as error, ErrorCode};
use util_types::{Opcode, PowerOf2Usize};
use zerocopy::{FromBytes, IntoBytes};

const OPCODE_LEN: usize = core::mem::size_of::<Opcode>();
const STATUS_LEN: usize = core::mem::size_of::<u32>();

// A reply is built right after the request's opcode, so the status word that
// precedes it in the response has to fit exactly where the opcode was.
const _: () = assert!(OPCODE_LEN == STATUS_LEN);

/// Outcome of [`Server::handle_frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The response frame is ready at the front of the caller's buffer, `usize` bytes long.
    Done(usize),
    /// An erase was started and is not done yet; poll via [`Server::poll_pending_erase`].
    Pending,
}

/// Serves flash requests against a [`Flash`].
///
/// A request is untrusted. Each handler establishes the preconditions the
/// driver documents (erase size in the erasable-size bitmap, erase address
/// aligned to that size, range inside the device) before calling [`Flash`].
/// Read and program may be unaligned; `BlockingFlash` splits those, but does
/// not check that the range stays on the device.
pub struct Server<TFlash: Flash> {
    flash: TFlash,
    /// Set when a deferred erase is abandoned without confirming it stopped; rejects every
    /// request until [`recover_abandoned_erase`](Self::recover_abandoned_erase) clears it.
    erase_unsafe: bool,
}

impl<TFlash: Flash<Error = ErrorCode>> Server<TFlash> {
    /// Creates a new `Server` wrapping the given flash implementation.
    pub fn new(flash: TFlash) -> Self {
        Self {
            flash,
            erase_unsafe: false,
        }
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
    /// Validates size/alignment/bounds, then starts erasing. Returns `Pending` if the erase
    /// is not done by the first poll; the caller defers via [`Server::poll_pending_erase`].
    fn handle_erase(&mut self, data: &mut [u8], reqsz: usize) -> Result<Outcome, ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let op = EraseOp::read_from_bytes(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let Some(size) = PowerOf2Usize::new(op.size as usize) else {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        };
        let (total, bitmap) = self.device_limits()?;
        erase_size_supported(size, bitmap)?;
        erase_address_aligned(op.address, size)?;
        region_in_bounds(op.address, size.get(), total)?;
        self.flash.start_erase(op.address, size)?;
        match self.flash.poll_erase()? {
            OpPoll::Done => Ok(Outcome::Done(0)),
            OpPoll::Busy => Ok(Outcome::Pending),
        }
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

    /// Handles one request: `data` holds `reqsz` request bytes and receives the reply at its
    /// front. While `erase_unsafe` is set, every opcode but `IPC_OP_FLASH_GET_INFO` is rejected.
    fn handle_op(
        &mut self,
        opcode: Opcode,
        data: &mut [u8],
        reqsz: usize,
    ) -> Result<Outcome, ErrorCode> {
        if self.erase_unsafe && opcode != IPC_OP_FLASH_GET_INFO {
            return Err(error::FLASH_GENERIC_BUSY);
        }
        match opcode {
            IPC_OP_FLASH_GET_INFO => self.handle_geometry(data, reqsz).map(Outcome::Done),
            IPC_OP_FLASH_ERASE => self.handle_erase(data, reqsz),
            IPC_OP_FLASH_PROGRAM => self.handle_program(data, reqsz).map(Outcome::Done),
            IPC_OP_FLASH_READ => self.handle_read(data, reqsz).map(Outcome::Done),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    /// Decodes the request in `buf[..len]`, an opcode then its body, and runs
    /// it. The reply is left in `buf` right after the opcode.
    fn handle_request(&mut self, buf: &mut [u8], len: usize) -> Result<Outcome, ErrorCode> {
        let body_len = len
            .checked_sub(OPCODE_LEN)
            .ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let (head, body) = buf
            .split_at_mut_checked(OPCODE_LEN)
            .ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let opcode = Opcode::read_from_bytes(head).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.handle_op(opcode, body, body_len)
    }

    /// A four-byte status (zero for success, else an `ErrorCode`) then
    /// `reply_len` bytes already sitting right after it in `buf`. Returns
    /// the frame length, or 0 if `buf` cannot hold a status.
    fn write_status_frame(buf: &mut [u8], status: u32, reply_len: usize) -> usize {
        let Some(head) = buf.get_mut(..STATUS_LEN) else {
            return 0;
        };
        head.copy_from_slice(status.as_bytes());
        STATUS_LEN.saturating_add(reply_len)
    }

    /// Serves the request frame in `buf[..len]`. `Outcome::Done` leaves the response frame at
    /// the start of `buf`; `Outcome::Pending` leaves `buf` untouched and the caller defers via
    /// [`poll_pending_erase`](Self::poll_pending_erase). `buf` must hold the larger of the
    /// request and response.
    pub fn handle_frame(&mut self, buf: &mut [u8], len: usize) -> Outcome {
        match self.handle_request(buf, len) {
            Ok(Outcome::Pending) => Outcome::Pending,
            Ok(Outcome::Done(reply_len)) => {
                Outcome::Done(Self::write_status_frame(buf, 0, reply_len))
            }
            Err(e) => Outcome::Done(Self::write_status_frame(buf, e.0.get(), 0)),
        }
    }

    /// Polls an erase that `handle_frame` reported `Pending` for. `None` while still busy;
    /// once done, builds the response frame into `buf` and returns its length.
    pub fn poll_pending_erase(&mut self, buf: &mut [u8]) -> Option<usize> {
        let status = match self.flash.poll_erase() {
            Ok(OpPoll::Busy) => return None,
            Ok(OpPoll::Done) => 0,
            Err(e) => e.0.get(),
        };
        Some(Self::write_status_frame(buf, status, 0))
    }

    /// Gives up on a previously-deferred erase and builds an error response frame for it.
    /// Marks the channel unsafe until [`recover_abandoned_erase`](Self::recover_abandoned_erase)
    /// observes the driver idle.
    pub fn abandon_pending_erase(&mut self, buf: &mut [u8], status: ErrorCode) -> usize {
        self.erase_unsafe = true;
        Self::write_status_frame(buf, status.0.get(), 0)
    }

    /// Background check for a channel `abandon_pending_erase` marked unsafe. Returns `true`
    /// once the driver reports idle, clearing the unsafe mark.
    pub fn recover_abandoned_erase(&mut self) -> bool {
        if !self.erase_unsafe {
            return true;
        }
        match self.flash.poll_erase() {
            Ok(OpPoll::Busy) => false,
            Ok(OpPoll::Done) | Err(_) => {
                self.erase_unsafe = false;
                true
            }
        }
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
