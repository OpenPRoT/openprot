// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use super::{IpcHandle, IpcHandler, IpcInitiator};

pub use userspace::buffer::AsSyscallBuffer;
pub use userspace::time::Instant;

impl IpcHandle {
    /// Set (set=true) or clear (set=false) Signals::USER on the paired peer.
    pub fn set_peer_user_signal(&self, set: bool) -> pw_status::Result<()> {
        userspace::syscall::object_set_peer_user_signal(self.handle, set)
    }
}

impl IpcInitiator for IpcHandle {
    fn transact<BufSend, BufRecv>(
        &self,
        send_data: &BufSend,
        recv_data: &mut BufRecv,
        deadline: Instant,
    ) -> pw_status::Result<usize>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized,
    {
        userspace::syscall::channel_transact(self.handle, send_data, recv_data, deadline)
    }

    unsafe fn async_transact_start<BufSend, BufRecv>(
        &self,
        send_data: &BufSend,
        recv_data: &mut BufRecv,
    ) -> pw_status::Result<()>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized,
    {
        let (send_ptr, send_len) = send_data.as_raw();
        let (recv_ptr, recv_len) = recv_data.as_raw_mut();
        // Safety: caller upholds the buffer-lifetime contract per this fn's doc.
        // nosemgrep
        unsafe {
            userspace::syscall::channel_async_transact(
                self.handle,
                send_ptr,
                send_len,
                recv_ptr,
                recv_len,
            )
        }
    }

    fn async_transact_complete(&self) -> pw_status::Result<usize> {
        userspace::syscall::channel_async_transact_complete(self.handle)
    }

    fn async_cancel(&self) -> pw_status::Result<()> {
        userspace::syscall::channel_async_cancel(self.handle)
    }

    fn as_raw(&self) -> u32 {
        self.handle
    }
}

impl IpcHandler for IpcHandle {
    fn read<Buf>(&self, offset: usize, buffer: &mut Buf) -> pw_status::Result<usize>
    where
        Buf: AsSyscallBuffer + ?Sized,
    {
        userspace::syscall::channel_read(self.handle, offset, buffer)
    }

    fn respond<Buf>(&self, buffer: &Buf) -> pw_status::Result<()>
    where
        Buf: AsSyscallBuffer + ?Sized,
    {
        userspace::syscall::channel_respond(self.handle, buffer)
    }
}
