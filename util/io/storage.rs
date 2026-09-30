// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Byte storage with sessions and split-phase writes.

/// Byte storage with sessions and split-phase writes.
///
/// Works for SPI NOR flash, on-chip SRAM, and RAM test buffers.
/// SPI NOR is the main target (erase-before-write, slow erases, bus
/// ownership). SRAM and RAM skip the erase and finish on the first poll.
///
/// Open a session before I/O, close when done. On SPI flash, open
/// asserts chip-select and claims the bus so no other master can
/// interleave commands mid-operation. Close releases chip-select and
/// frees the bus for other users. Double-open and close-without-open
/// are errors.
///
/// Writes: `start_write`, poll `is_busy`, `complete_write`. Data is
/// copied internally. One write at a time; reads, writes, and close
/// all error while one is in flight. No cancel (an in-progress erase
/// leaves the sector bad if aborted).
///
/// Reads are synchronous. No byte may be written twice per session
/// (lets flash erase on first touch without read-modify-write).
pub trait Storage {
    type Error;

    /// Open a session (claims the bus on SPI flash).
    fn open(&mut self) -> Result<(), Self::Error>;

    /// Read `buf.len()` bytes starting at `offset`.
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Begin writing `data` at `offset`. Data is copied internally.
    fn start_write(&mut self, offset: usize, data: &[u8]) -> Result<(), Self::Error>;

    /// True while a write is still in progress.
    fn is_busy(&mut self) -> bool;

    /// Finalize a completed write. Data is durable when this returns Ok.
    fn complete_write(&mut self) -> Result<(), Self::Error>;

    /// Close the session. Errors if a write is in flight or no session is open.
    fn close(&mut self) -> Result<(), Self::Error>;
}

/// In-memory [`Storage`] mock. Set `polls_per_write` > 0 to verify
/// callers actually poll.
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
    AlreadyOpen,
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
        if self.session_open {
            return Err(MockError::AlreadyOpen);
        }
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
        if end > self.buf.len() {
            return Err(MockError::OutOfBounds);
        }
        buf.copy_from_slice(&self.buf[offset..end]);
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
        if !self.session_open {
            return Err(MockError::NoSession);
        }
        if self.pending.is_some() {
            return Err(MockError::Busy);
        }
        self.session_open = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[test]
    fn storage_complete_without_pending_rejected() {
        let mut s = MockStorage::new(0);
        s.open().unwrap();
        assert_eq!(s.complete_write(), Err(MockError::NotBusy));
        s.close().unwrap();
    }
    #[test]
    fn storage_write_out_of_bounds_rejected() {
        let mut s = MockStorage::new(0);
        s.open().unwrap();
        assert_eq!(s.start_write(250, &[0; 10]), Err(MockError::OutOfBounds));
        s.close().unwrap();
    }
    #[test]
    fn storage_double_open_rejected() {
        let mut s = MockStorage::new(0);
        s.open().unwrap();
        assert_eq!(s.open(), Err(MockError::AlreadyOpen));
        s.close().unwrap();
    }
    #[test]
    fn storage_close_without_session_rejected() {
        let mut s = MockStorage::new(0);
        assert_eq!(s.close(), Err(MockError::NoSession));
    }
    #[test]
    fn storage_reopen_after_close() {
        let mut s = MockStorage::new(1);
        s.open().unwrap();
        s.start_write(0, &[0xAA]).unwrap();
        drain_write(&mut s);
        s.close().unwrap();
        s.open().unwrap();
        s.start_write(4, &[0xBB]).unwrap();
        drain_write(&mut s);
        let mut out = [0u8; 1];
        s.read_at(0, &mut out).unwrap();
        assert_eq!(out, [0xAA]);
        s.read_at(4, &mut out).unwrap();
        assert_eq!(out, [0xBB]);
        s.close().unwrap();
    }
    #[test]
    fn storage_read_out_of_bounds_rejected() {
        let mut s = MockStorage::new(0);
        s.open().unwrap();
        let mut out = [0u8; 10];
        assert_eq!(s.read_at(250, &mut out), Err(MockError::OutOfBounds));
        s.close().unwrap();
    }
}
