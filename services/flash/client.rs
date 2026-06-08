//! Flash IPC client implementation.

#![no_std]
use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use services_flash_opcode::*;
use userspace::time::Instant;
use util_error::{self as error, ErrorCode};
use util_ipc::IpcChannel;
use util_types::PowerOf2Usize;
use zerocopy::{FromZeros, IntoBytes};

/// This struct implements the `Flash` trait by sending IPC requests to a remote
/// flash server.
pub struct FlashIpcClient {
    ipc: IpcChannel,
    page_size: PowerOf2Usize,
    total_size: NonZero<usize>,
    erasable_sizes_bitmap: u32,
}

impl FlashIpcClient {
    /// Creates a new `FlashIpcClient` from an existing IPC channel.
    ///
    /// This constructor will perform an IPC transaction to retrieve flash
    /// information (page size and total size) from the server.
    pub fn new(ipc: IpcChannel) -> Result<Self, ErrorCode> {
        let mut info = FlashInfo::new_zeroed();
        let mut result = 0u32;

        ipc.transaction::<16>(
            &[IPC_OP_FLASH_GET_INFO.as_bytes()],
            &mut [result.as_mut_bytes(), info.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)?;

        let Some(page_size) = PowerOf2Usize::new(info.page_size as usize) else {
            return Err(error::FLASH_GENERIC_INVALID_PAGE_SIZE);
        };
        let Some(total_size) = NonZero::new(info.total_size as usize) else {
            return Err(error::FLASH_GENERIC_INVALID_SIZE);
        };
        Ok(Self {
            ipc,
            page_size,
            total_size,
            erasable_sizes_bitmap: info.erasable_sizes_bitmap,
        })
    }
}

impl Flash for FlashIpcClient {
    type Error = ErrorCode;
    fn geometry(&self) -> (NonZero<usize>, PowerOf2Usize, u32) {
        (self.total_size, self.page_size, self.erasable_sizes_bitmap)
    }

    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        let size_val = size.get() as u32;
        self.ipc.transaction::<16>(
            &[
                IPC_OP_FLASH_ERASE.as_bytes(),
                start_addr.as_bytes(),
                size_val.as_bytes(),
            ],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc.transaction::<2064>(
            &[IPC_OP_FLASH_PROGRAM.as_bytes(), start_addr.as_bytes(), data],
            &mut [result.as_mut_bytes()],
            Instant::MAX,
        )?;
        IpcChannel::check_status(result)
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        let length = buf.len() as u32;
        self.ipc.transaction::<2064>(
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
