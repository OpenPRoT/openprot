// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Input/Output traits and utilities.

#![no_std]

use util_error::{ErrorCode, ErrorModule};

/// The generic IO error module.
pub const IO_GENERIC: ErrorModule = ErrorModule::new(0x494F); // ascii: IO
/// The read operation is out of bounds.
pub const IO_GENERIC_READ_OUT_OF_BOUNDS: ErrorCode =
    IO_GENERIC.from_pw(1, pw_status::Error::OutOfRange);

/// Trait for random access read operations.
pub trait RandomRead {
    type Error;
    /// Reads data from the source into the destination buffer.
    ///
    /// # Arguments
    /// * `start_addr`: The offset from which to start reading.
    /// * `dst`: The buffer to read data into.
    fn read(&mut self, start_addr: usize, dst: &mut [u8]) -> Result<(), Self::Error>;

    /// Returns the total size of the readable data in bytes.
    fn size(&mut self) -> Result<usize, Self::Error>;
}

/// Session-oriented byte storage with split-phase writes.
///
/// Callers open a session before any I/O and close it when done.
/// For SPI flash the open/close pair claims and releases bus
/// mastership; for RAM they perform no hardware action, but the
/// session is still tracked and any I/O outside an open session
/// returns an error.
///
/// Writes follow the start-poll-complete pattern from FlashDriver
/// (hal/blocking/flash/driver.rs): call
/// `start_write`, poll `is_busy` until it returns `false`, then
/// call `complete_write` to finalize and surface any hardware
/// error. `start_write` copies `data` into an internal buffer and
/// returns immediately; `start_write` while a previous write is
/// still in flight is an error. `close` while a write is in
/// flight is an error (poll to completion first). There is no
/// `cancel_write`: a flash erase or program in progress cannot be
/// recalled.
///
/// `read_at` is synchronous: flash reads take microseconds with no
/// erase path, so splitting them would double the API surface for
/// nothing. `read_at` while a write is in flight is an error (SPI
/// NOR cannot service a read during program or erase).
///
/// Erase-on-first-touch happens inside the polled operation:
/// `is_busy` stays true through both erase and program, and
/// `start_write` only stages the data and kicks off the operation.
/// A RAM or test implementation returns `false` from `is_busy` on
/// the first call and completes instantly.
///
/// No byte may be written twice in one session; overlapping ranges
/// that touch the same byte have unspecified results (may corrupt
/// stored data) but are never memory-unsafe. Implementations may
/// but need not detect the violation. This constraint lets
/// flash-backed implementations erase on first touch without
/// read-modify-write.
pub trait Storage {
    /// Error type for storage operations.
    type Error;

    /// Open a storage session. For SPI-backed storage this claims
    /// the bus.
    fn open(&mut self) -> Result<(), Self::Error>;

    /// Read `buf.len()` bytes starting at `offset`.
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Begin writing `data` at `offset`. The implementation copies
    /// `data` into an internal buffer and initiates the flash
    /// operation. Returns an error if a write is already in flight.
    fn start_write(&mut self, offset: usize, data: &[u8]) -> Result<(), Self::Error>;

    /// Returns `true` while a `start_write` operation is still in
    /// progress.
    fn is_busy(&mut self) -> bool;

    /// Finalize a completed write. Returns an error if the operation
    /// is still busy. Surfaces any hardware error that occurred
    /// during the operation. Data is durable when this returns `Ok`.
    fn complete_write(&mut self) -> Result<(), Self::Error>;

    /// Close the storage session, releasing any held resources.
    /// Returns an error if a write is still in flight.
    fn close(&mut self) -> Result<(), Self::Error>;
}

impl RandomRead for &[u8] {
    type Error = ErrorCode;
    fn read(&mut self, start_addr: usize, dst: &mut [u8]) -> Result<(), Self::Error> {
        // Explicit wrapping add. Overflows are expected to
        // be detected in the indexing operation
        let end_addr = start_addr.wrapping_add(dst.len());
        let src = self
            .get(start_addr..end_addr)
            .ok_or(IO_GENERIC_READ_OUT_OF_BOUNDS)?;
        dst.copy_from_slice(src);
        Ok(())
    }
    fn size(&mut self) -> Result<usize, Self::Error> {
        Ok(self.len())
    }
}

/// In-memory [`Storage`] mock that completes writes after a
/// configurable number of `is_busy` polls. Use `polls_per_write` > 0
/// to verify that callers actually poll, not just start and complete.
#[cfg(test)]
pub struct MockStorage {
    buf: [u8; 256],
    session_open: bool,
    pending: Option<PendingWrite>,
    polls_remaining: usize,
    polls_per_write: usize,
}

#[cfg(test)]
struct PendingWrite {
    offset: usize,
    len: usize,
    staging: [u8; 256],
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub enum MockError {
    NoSession,
    Busy,
    NotBusy,
    OutOfBounds,
}

#[cfg(test)]
impl MockStorage {
    fn new(polls_per_write: usize) -> Self {
        Self {
            buf: [0xFF; 256],
            session_open: false,
            pending: None,
            polls_remaining: 0,
            polls_per_write,
        }
    }
}

#[cfg(test)]
impl Storage for MockStorage {
    type Error = MockError;

    fn open(&mut self) -> Result<(), MockError> {
        self.session_open = true;
        Ok(())
    }

    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), MockError> {
        if !self.session_open {
            return Err(MockError::NoSession);
        }
        if self.pending.is_some() {
            return Err(MockError::Busy);
        }
        let end = offset
            .checked_add(buf.len())
            .ok_or(MockError::OutOfBounds)?;
        let src = self.buf.get(offset..end).ok_or(MockError::OutOfBounds)?;
        buf.copy_from_slice(src);
        Ok(())
    }

    fn start_write(&mut self, offset: usize, data: &[u8]) -> Result<(), MockError> {
        if !self.session_open {
            return Err(MockError::NoSession);
        }
        if self.pending.is_some() {
            return Err(MockError::Busy);
        }
        let end = offset
            .checked_add(data.len())
            .ok_or(MockError::OutOfBounds)?;
        if end > self.buf.len() {
            return Err(MockError::OutOfBounds);
        }
        let mut staging = [0u8; 256];
        staging[..data.len()].copy_from_slice(data);
        self.pending = Some(PendingWrite {
            offset,
            len: data.len(),
            staging,
        });
        self.polls_remaining = self.polls_per_write;
        Ok(())
    }

    fn is_busy(&mut self) -> bool {
        if self.pending.is_none() {
            return false;
        }
        if self.polls_remaining > 0 {
            self.polls_remaining -= 1;
            return true;
        }
        false
    }

    fn complete_write(&mut self) -> Result<(), MockError> {
        if self.polls_remaining > 0 {
            return Err(MockError::Busy);
        }
        let pw = self.pending.take().ok_or(MockError::NotBusy)?;
        self.buf[pw.offset..pw.offset + pw.len].copy_from_slice(&pw.staging[..pw.len]);
        Ok(())
    }

    fn close(&mut self) -> Result<(), MockError> {
        if self.pending.is_some() {
            return Err(MockError::Busy);
        }
        self.session_open = false;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_read() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut dst: [u8; 3] = [0; 3];
        assert!(src.read(6, &mut dst).is_ok());
        assert_eq!(&dst, &[7, 8, 9]);
        assert_eq!(RandomRead::size(&mut src).unwrap(), 9);
    }

    #[test]
    fn should_fail() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut dst: [u8; 3] = [0; 3];
        assert!(src.read(7, &mut dst).is_err());
    }

    #[test]
    fn invalid_start_address_should_not_panic() {
        let mut src: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8];
        let mut dst: [u8; 4] = [0; 4];
        // Set `start_addr` so that adding `dst.len()` causes it to
        // wrap around and become smaller than `src.len()`
        let start_addr: usize = usize::MAX - (dst.len() - 1);
        // Should not panic
        let result = src.read(start_addr, &mut dst);
        assert!(result.is_err());
    }

    fn drain_write(s: &mut MockStorage) {
        while s.is_busy() {}
        s.complete_write().unwrap();
    }

    #[test]
    fn storage_write_then_read_back() {
        let mut s = MockStorage::new(3);
        s.open().unwrap();
        s.start_write(4, &[0xAA, 0xBB]).unwrap();
        drain_write(&mut s);
        let mut out = [0u8; 2];
        s.read_at(4, &mut out).unwrap();
        assert_eq!(out, [0xAA, 0xBB]);
        s.close().unwrap();
    }

    #[test]
    fn storage_completes_after_n_polls() {
        let mut s = MockStorage::new(3);
        s.open().unwrap();
        s.start_write(0, &[1]).unwrap();
        assert!(s.is_busy());
        assert!(s.is_busy());
        assert!(s.is_busy());
        assert!(!s.is_busy());
        s.complete_write().unwrap();
        s.close().unwrap();
    }

    #[test]
    fn storage_read_outside_session_returns_error() {
        let mut s = MockStorage::new(0);
        let mut out = [0u8; 1];
        assert_eq!(s.read_at(0, &mut out), Err(MockError::NoSession));
    }

    #[test]
    fn storage_start_write_outside_session_returns_error() {
        let mut s = MockStorage::new(0);
        assert_eq!(s.start_write(0, &[1]), Err(MockError::NoSession));
    }

    #[test]
    fn storage_double_start_rejected() {
        let mut s = MockStorage::new(2);
        s.open().unwrap();
        s.start_write(0, &[1]).unwrap();
        assert_eq!(s.start_write(8, &[2]), Err(MockError::Busy));
        drain_write(&mut s);
        s.close().unwrap();
    }

    #[test]
    fn storage_close_while_in_flight_rejected() {
        let mut s = MockStorage::new(2);
        s.open().unwrap();
        s.start_write(0, &[1]).unwrap();
        assert_eq!(s.close(), Err(MockError::Busy));
        drain_write(&mut s);
        s.close().unwrap();
    }

    #[test]
    fn storage_complete_before_done_rejected() {
        let mut s = MockStorage::new(2);
        s.open().unwrap();
        s.start_write(0, &[1]).unwrap();
        assert_eq!(s.complete_write(), Err(MockError::Busy));
        drain_write(&mut s);
        s.close().unwrap();
    }

    #[test]
    fn storage_read_while_in_flight_rejected() {
        let mut s = MockStorage::new(2);
        s.open().unwrap();
        s.start_write(0, &[1]).unwrap();
        let mut out = [0u8; 1];
        assert_eq!(s.read_at(0, &mut out), Err(MockError::Busy));
        drain_write(&mut s);
        s.close().unwrap();
    }
}
