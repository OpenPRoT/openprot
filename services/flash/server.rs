#![no_std]
//use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use services_flash_opcode::*;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::Opcode;
use zerocopy::{FromBytes, IntoBytes};

pub struct FlashIpcServer<TFlash: Flash> {
    flash: TFlash,
}

impl<TFlash: Flash> FlashIpcServer<TFlash> {
    pub fn new(flash: TFlash) -> Self {
        Self { flash }
    }

    fn handle_get_info<'a>(&self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (info, _rest) =
            FlashInfo::mut_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        info.page_size = self.flash.page_size().get();
        info.total_size = self.flash.size().get();
        Ok(info.as_bytes())
    }

    fn handle_erase<'a>(&mut self, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        let (addr, data) =
            FlashAddress::read_from_prefix(data).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.flash.erase_page(addr)?;
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
            FlashAddress::read_from_bytes(&data[0..4]).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        let length =
            usize::read_from_bytes(&data[4..8]).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
        self.flash.read(addr, &mut data[..length])?;
        Ok(&data[..length])
    }

    fn handle_op<'a>(&mut self, opcode: Opcode, data: &'a mut [u8]) -> Result<&'a [u8], ErrorCode> {
        match opcode {
            IPC_OP_FLASH_GET_INFO => self.handle_get_info(data),
            IPC_OP_FLASH_ERASE_PAGE => self.handle_erase(data),
            IPC_OP_FLASH_PROGRAM => self.handle_program(data),
            IPC_OP_FLASH_READ => self.handle_read(data),
            _ => Err(error::IPC_ERROR_UNKNOWN_OP),
        }
    }

    fn handle_one(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
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

    pub fn run(&mut self, ipc: &IpcChannel, data: &mut [u8]) -> Result<(), ErrorCode> {
        loop {
            self.handle_one(ipc, data)?;
        }
    }
}
