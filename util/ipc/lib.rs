// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use util_error::ErrorCode;

pub struct IpcChannel(u32);

impl IpcChannel {
    pub const fn new(channel: u32) -> Self {
        IpcChannel(channel)
    }

    pub fn handle(&self) -> u32 { self.0 }

    pub fn check_status(code: u32) -> Result<(), ErrorCode> {
        if code == 0 {
            Ok(())
        } else {
            Err(ErrorCode::new(code))
        }
    }

    pub fn transaction<const N: usize>(
        &self,
        request: &[&[u8]],
        response: &mut [&mut [u8]],
        deadline: Instant,
    ) -> Result<usize, ErrorCode> {
        if false {
            //let _n = N;
            //syscall::channel_transact_iovec(self.0, request, response, deadline)
            Err(util_error::KERNEL_ERROR_UNIMPLEMENTED)
        } else {
            let mut buffer = [0u8; N];
            let mut offset = 0usize;

            for item in request.iter() {
                let sz = offset + item.len();
                buffer[offset..sz].copy_from_slice(item);
                offset = sz;
            }
            let req = unsafe {
                // SAFETY: naughty creation of a const ref to the same slice
                // so we can use the same buffer for send and recv.
                core::slice::from_raw_parts(buffer.as_ptr(), offset)
            };
            let rsplen = syscall::channel_transact(self.0, req, &mut buffer, deadline)
                .map_err(ErrorCode::kernel_error)?;

            offset = 0usize;
            let rsp = &buffer[..rsplen];
            for item in response.iter_mut() {
                let sz = offset + item.len();
                // TODO: how to handle an incomplete response?.
                if sz > rsp.len() {
                    break;
                }
                item.copy_from_slice(&rsp[offset..sz]);
                offset = sz;
            }
            Ok(rsplen)
        }
    }

    pub fn wait_readable(&self) -> Result<(), ErrorCode> {
        loop {
            let w = syscall::object_wait(self.0, Signals::READABLE, Instant::MAX)
                .map_err(ErrorCode::kernel_error)?;
            if w.pending_signals.contains(Signals::READABLE) {
                break;
            }
        }
        Ok(())
    }

    pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, ErrorCode> {
        syscall::channel_read(self.0, offset, buffer).map_err(ErrorCode::kernel_error)
    }

    pub fn respond(&self, buffer: &[u8]) -> Result<(), ErrorCode> {
        syscall::channel_respond(self.0, buffer).map_err(ErrorCode::kernel_error)
    }
}
