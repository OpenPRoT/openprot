// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use super::{IpcChannel, IpcHandle};

pub use userspace::buffer::AsSyscallBuffer;
pub use userspace::time::Instant;

impl IpcChannel for IpcHandle {
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

    fn set_peer_user_signal(&self, set: bool) -> pw_status::Result<()> {
        userspace::syscall::object_set_peer_user_signal(self.handle, set)
    }
}
