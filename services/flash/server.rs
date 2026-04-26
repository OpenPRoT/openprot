//! Flash IPC server implementation.

#![no_std]

use hal_flash::{Flash, FlashAddress};
use services_flash_opcode::*;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::{Opcode, PowerOf2Usize};
use zerocopy::{FromBytes, IntoBytes};

/// A flash server that handles flash IPC requests.
///
/// This struct wraps an object implementing the `Flash` trait and provides
/// an IPC interface to it.
pub struct FlashIpcServer<TFlash: Flash> {
    flash: TFlash,
}

impl<TFlash: Flash> FlashIpcServer<TFlash> {
    /// Creates a new `FlashIpcServer` wrapping the given flash implementation.
    pub fn new(flash: TFlash) -> Self {
        Self { flash }
    }

    fn handle_get_info<'a>(&self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (info, _rest) =
            FlashInfo::mut_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (total_size, page_size, erasable_sizes_bitmap) = self.flash.geometry();
        info.page_size = page_size.get() as u32;
        info.total_size = total_size.get() as u32;
        info.erasable_sizes_bitmap = erasable_sizes_bitmap;
        Ok(info.as_bytes())
    }

    fn handle_erase<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (addr, data) =
            FlashAddress::read_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let (size, data) =
            u32::read_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let Some(size) = PowerOf2Usize::new(size as usize) else {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        };
        self.flash.erase(addr, size)?;
        Ok(&data[0..0])
    }

    fn handle_program<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (addr, data) =
            FlashAddress::mut_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.flash.program(*addr, data)?;
        Ok(&data[0..0])
    }

    fn handle_read<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let addr =
            FlashAddress::read_from_bytes(&data[0..8]).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let length =
            usize::read_from_bytes(&data[8..12]).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.flash.read(addr, &mut data[..length])?;
        Ok(&data[..length])
    }

    fn handle_op<'a>(&mut self, opcode: Opcode, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        match opcode {
            IPC_OP_FLASH_GET_INFO => self.handle_get_info(data),
            IPC_OP_FLASH_ERASE => self.handle_erase(data),
            IPC_OP_FLASH_PROGRAM => self.handle_program(data),
            IPC_OP_FLASH_READ => self.handle_read(data),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    /// Handles a single IPC request.
    ///
    /// This method waits for a request on the given IPC channel, dispatches it
    /// to the appropriate handler, and sends the response.
    pub fn handle_one(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        //pw_log::info!("ipc_wait");
        ipc.wait_readable()?;
        //pw_log::info!("ipc_read");
        let len = ipc.read(0, data)?;
        //pw_log::info!("ipc_exec");
        if len < 4 {
            return Err(error::IPC_ERROR_BAD_REQ_LEN);
        }
        let (op_status, reqrsp) = data.split_at_mut(4);
        let opcode = Opcode::read_from_bytes(op_status).unwrap();
        let len = match self.handle_op(opcode, reqrsp) {
            Ok(result) => {
                op_status.copy_from_slice((0u32).as_bytes());
                result.len()
            }
            Err(e) => {
                op_status.copy_from_slice(e.0.as_bytes());
                0
            }
        };
        //pw_log::info!("ipc_respond: {}", len as usize);
        ipc.respond(&data[..4 + len])?;
        Ok(())
    }

    /// Runs the flash IPC server.
    ///
    /// This method enters an infinite loop, handling IPC requests one by one.
    pub fn run(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        loop {
            self.handle_one(ipc, data)?;
        }
    }
}
