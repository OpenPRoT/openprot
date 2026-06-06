// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use super::{IpcChannel, IpcHandle};

pub trait AsSyscallBuffer {
    fn as_raw(&self) -> (*const u8, usize);
    fn as_raw_mut(&mut self) -> (*mut u8, usize);
    fn total_size(&self) -> usize;
}

// Converts a simple u8 slice.
impl AsSyscallBuffer for [u8] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr(), self.len())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr(), self.len())
    }
    fn total_size(&self) -> usize {
        self.len()
    }
}

// Converts a simple u8 array.
impl<const N: usize> AsSyscallBuffer for [u8; N] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr(), self.len())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr(), self.len())
    }
    fn total_size(&self) -> usize {
        self.len()
    }
}

// Converts a slice of u8 slices.
impl AsSyscallBuffer for [&[u8]] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn total_size(&self) -> usize {
        self.iter().fold(0, |total, item| total + item.len())
    }
}

impl AsSyscallBuffer for [&mut [u8]] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn total_size(&self) -> usize {
        self.iter().fold(0, |total, item| total + item.len())
    }
}

// Converts an array of u8 slices.
impl<const N: usize> AsSyscallBuffer for [&[u8]; N] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn total_size(&self) -> usize {
        self.iter().fold(0, |total, item| total + item.len())
    }
}

impl<const N: usize> AsSyscallBuffer for [&mut [u8]; N] {
    fn as_raw(&self) -> (*const u8, usize) {
        (self.as_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn as_raw_mut(&mut self) -> (*mut u8, usize) {
        (self.as_mut_ptr().cast::<u8>(), self.len().wrapping_neg())
    }
    fn total_size(&self) -> usize {
        self.iter().fold(0, |total, item| total + item.len())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HostClock;

impl time::Clock for HostClock {
    const TICKS_PER_SEC: u64 = 1_000_000;
    fn now() -> time::Instant<Self> {
        time::Instant::from_ticks(0)
    }
}

pub type Instant = time::Instant<HostClock>;

impl IpcChannel for IpcHandle {
    fn transact<BufSend, BufRecv>(
        &self,
        _send_data: &BufSend,
        _recv_data: &mut BufRecv,
        _deadline: Instant,
    ) -> pw_status::Result<usize>
    where
        BufSend: AsSyscallBuffer + ?Sized,
        BufRecv: AsSyscallBuffer + ?Sized,
    {
        panic!("IpcHandle cannot be used on host");
    }

    fn read<Buf>(&self, _offset: usize, _buffer: &mut Buf) -> pw_status::Result<usize>
    where
        Buf: AsSyscallBuffer + ?Sized,
    {
        panic!("IpcHandle cannot be used on host");
    }

    fn respond<Buf>(&self, _buffer: &Buf) -> pw_status::Result<()>
    where
        Buf: AsSyscallBuffer + ?Sized,
    {
        panic!("IpcHandle cannot be used on host");
    }

    fn set_peer_user_signal(&self, _set: bool) -> pw_status::Result<()> {
        panic!("IpcHandle cannot be used on host");
    }
}
