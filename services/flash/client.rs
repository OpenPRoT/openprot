// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use services_flash_opcode::*;
use userspace::time::Instant;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::PowerOf2Usize;
use zerocopy::{FromZeros, IntoBytes};

pub struct FlashIpcClient {
    ipc: IpcChannel,
    page_size: PowerOf2Usize,
    total_size: NonZero<usize>,
}

impl FlashIpcClient {
    pub fn new(ipc: IpcChannel) -> Result<Self, ErrorCode> {
        let mut info = FlashInfo::new_zeroed();
        let mut result = 0u32;

        ipc.transaction::<12>(
            &[IPC_OP_FLASH_GET_INFO.as_bytes()],
            &mut [result.as_mut_bytes(), info.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)?;

        let Some(page_size) = PowerOf2Usize::new(info.page_size) else {
            return Err(error::FLASH_GENERIC_INVALID_PAGE_SIZE);
        };
        let Some(total_size) = NonZero::new(info.total_size) else {
            return Err(error::FLASH_GENERIC_INVALID_SIZE);
        };
        Ok(Self {
            ipc,
            page_size,
            total_size,
        })
    }
}

impl Flash for FlashIpcClient {
    fn page_size(&self) -> PowerOf2Usize {
        self.page_size
    }
    fn size(&self) -> core::num::NonZero<usize> {
        self.total_size
    }
    fn erase_page(&mut self, start_addr: FlashAddress) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc.transaction::<12>(
            &[IPC_OP_FLASH_ERASE_PAGE.as_bytes(), start_addr.as_bytes()],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc.transaction::<2056>(
            &[IPC_OP_FLASH_PROGRAM.as_bytes(), start_addr.as_bytes(), data],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        let length = buf.len();
        self.ipc.transaction::<2056>(
            &[
                IPC_OP_FLASH_READ.as_bytes(),
                start_addr.as_bytes(),
                length.as_bytes(),
            ],
            &mut [result.as_mut_bytes(), buf],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }
}
