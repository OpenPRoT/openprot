// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash client, independent of the kernel and of IPC.
//!
//! [`FlashClient`] implements [`Flash`] by sending request frames over a
//! [`FlashTransport`]. The kernel's transport is in `flash_client_ipc`.

#![no_std]
use core::num::NonZero;

use hal_flash::{Flash, FlashAddress};
use openprot_flash_api::*;
use util_error::{self as error, ErrorCode};
use util_types::PowerOf2Usize;
use zerocopy::{FromZeros, IntoBytes};

const STATUS_LEN: usize = core::mem::size_of::<u32>();

/// Carries one request to the flash server and brings back its response.
pub trait FlashTransport {
    /// Sends the pieces of `send`, in order, as one request, and receives the
    /// response into the pieces of `recv`, in order. Returns the length of the
    /// response.
    ///
    /// The pieces are not joined into one buffer, so a transport over the
    /// kernel channel copies nothing.
    fn transact(&mut self, send: &[&[u8]], recv: &mut [&mut [u8]]) -> Result<usize, ErrorCode>;
}

/// A response is a status word, then `payload_len` bytes if the status is
/// zero. A short or long response is an error, not stale buffer contents.
fn check_response(status: u32, received: usize, payload_len: usize) -> Result<(), ErrorCode> {
    if received < STATUS_LEN {
        return Err(error::IPC_ERROR_RSP_BAD_LEN);
    }
    ErrorCode::check_status(status)?;
    if received != STATUS_LEN + payload_len {
        return Err(error::IPC_ERROR_RSP_BAD_LEN);
    }
    Ok(())
}

/// A client for the flash service.
///
/// This struct implements the `Flash` trait by proxying requests to a remote
/// flash server over a [`FlashTransport`].
pub struct FlashClient<T: FlashTransport> {
    transport: T,
    page_size: PowerOf2Usize,
    total_size: NonZero<usize>,
    erasable_sizes_bitmap: u32,
}

impl<T: FlashTransport> FlashClient<T> {
    /// Creates a new `FlashClient` over `transport`.
    ///
    /// This constructor will perform a transaction to retrieve flash
    /// geometry and capabilities from the server.
    pub fn new(mut transport: T) -> Result<Self, ErrorCode> {
        let mut info = FlashInfo::new_zeroed();
        let mut status = 0u32;

        let received = transport.transact(
            &[IPC_OP_FLASH_GET_INFO.as_bytes()],
            &mut [status.as_mut_bytes(), info.as_mut_bytes()],
        )?;
        check_response(status, received, core::mem::size_of::<FlashInfo>())?;

        let Some(page_size) = PowerOf2Usize::new(info.page_size as usize) else {
            return Err(error::FLASH_GENERIC_INVALID_PAGE_SIZE);
        };
        let Some(total_size) = NonZero::new(info.total_size as usize) else {
            return Err(error::FLASH_GENERIC_INVALID_SIZE);
        };
        Ok(Self {
            transport,
            page_size,
            total_size,
            erasable_sizes_bitmap: info.erasable_sizes_bitmap,
        })
    }
}

impl<T: FlashTransport> Flash for FlashClient<T> {
    type Error = ErrorCode;
    fn geometry(&mut self) -> Result<(NonZero<usize>, PowerOf2Usize, u32), ErrorCode> {
        Ok((self.total_size, self.page_size, self.erasable_sizes_bitmap))
    }

    fn erase(&mut self, start_addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        let mut status = 0u32;
        let op = EraseOp {
            address: start_addr,
            size: size.get() as u32,
        };
        let received = self.transport.transact(
            &[IPC_OP_FLASH_ERASE.as_bytes(), op.as_bytes()],
            &mut [status.as_mut_bytes()],
        )?;
        check_response(status, received, 0)
    }

    fn program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        let mut status = 0u32;
        let received = self.transport.transact(
            &[IPC_OP_FLASH_PROGRAM.as_bytes(), start_addr.as_bytes(), data],
            &mut [status.as_mut_bytes()],
        )?;
        check_response(status, received, 0)
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        let mut status = 0u32;
        let length = buf.len();
        let op = ReadOp {
            address: start_addr,
            length: length as u32,
        };
        let received = self.transport.transact(
            &[IPC_OP_FLASH_READ.as_bytes(), op.as_bytes()],
            &mut [status.as_mut_bytes(), buf],
        )?;
        check_response(status, received, length)
    }
}
