// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash request handling, independent of the kernel and of IPC.

use hal_flash::{Flash, FlashAddress};
use openprot_flash_api::*;
use util_error::{self as error, ErrorCode};
use util_types::{Opcode, PowerOf2Usize};
use zerocopy::{FromBytes, IntoBytes};

/// Serves flash requests against a [`Flash`].
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
    /// Writes the flash geometry into the provided buffer and returns it.
    fn handle_geometry<'a>(
        &mut self,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        if reqsz != 0 {
            return Err(error::IPC_ERROR_BAD_REQ_LEN);
        }
        let (info, _rest) =
            FlashInfo::mut_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (total_size, page_size, erasable_sizes_bitmap) = self.flash.geometry()?;
        info.page_size = page_size.get() as u32;
        info.total_size = total_size.get() as u32;
        info.erasable_sizes_bitmap = erasable_sizes_bitmap;
        Ok(info.as_bytes())
    }

    /// Handles the `IPC_OP_FLASH_ERASE` request.
    ///
    /// Parses the `EraseOp` from the input data and erases the specified block.
    fn handle_erase<'a>(
        &mut self,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let op = EraseOp::read_from_bytes(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let Some(size) = PowerOf2Usize::new(op.size as usize) else {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        };
        self.flash.erase(op.address, size)?;
        Ok(&data[0..0])
    }

    /// Handles the `IPC_OP_FLASH_PROGRAM` request.
    ///
    /// Parses the start address and data from the input, then programs it.
    fn handle_program<'a>(
        &mut self,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let (addr, program_data) =
            FlashAddress::read_from_prefix(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.flash.program(addr, program_data)?;
        Ok(&data[0..0])
    }

    /// Handles the `IPC_OP_FLASH_READ` request.
    ///
    /// Parses the `ReadOp` from the input, reads the data from flash into the
    /// buffer, and returns the read slice.
    fn handle_read<'a>(&mut self, data: &'a mut [u8], reqsz: usize) -> Result<&'a [u8], ErrorCode> {
        let req_data = data.get(..reqsz).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
        let op = ReadOp::read_from_bytes(req_data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let length = op.length as usize;
        if length > data.len() {
            return Err(error::FLASH_GENERIC_INVALID_SIZE);
        }
        self.flash.read(op.address, &mut data[..length])?;
        Ok(&data[..length])
    }

    /// Handles one request: `data` holds `reqsz` request bytes and is the
    /// scratch for the reply, which is returned as a slice of it.
    pub fn handle<'a>(
        &mut self,
        opcode: Opcode,
        data: &'a mut [u8],
        reqsz: usize,
    ) -> Result<&'a [u8], ErrorCode> {
        match opcode {
            IPC_OP_FLASH_GET_INFO => self.handle_geometry(data, reqsz),
            IPC_OP_FLASH_ERASE => self.handle_erase(data, reqsz),
            IPC_OP_FLASH_PROGRAM => self.handle_program(data, reqsz),
            IPC_OP_FLASH_READ => self.handle_read(data, reqsz),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }
}
