// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A bounded view over part of a [`ByteSource`].

use crate::byte_source::{ByteReadError, ByteSource};

/// An offset-and-length view over another [`ByteSource`].
///
/// The window is itself a `ByteSource`, so a reader given one cannot
/// see past it: `len` is the window's length and reads are relative to
/// its offset, so a read at 0 lands on the window's first byte rather
/// than the underlying source's. An update stages through one because
/// the region holding a candidate is board geometry while the candidate
/// is only as long as the offer said.
///
/// Reads outside the window are
/// [`OutOfRange`](ByteReadError::OutOfRange), the same answer the
/// underlying source gives for a read past its end. A window that does
/// not fit its source is refused at construction instead, because a
/// caller that cannot describe its own range has a bug the first read
/// would only hide.
pub struct ByteWindow<'a, S: ByteSource + ?Sized> {
    source: &'a S,
    offset: u64,
    len: u64,
}

impl<'a, S: ByteSource + ?Sized> ByteWindow<'a, S> {
    /// Views `len` bytes of `source` starting at `offset`.
    ///
    /// Returns [`OutOfRange`](ByteReadError::OutOfRange) if the window
    /// runs past the end of `source`.
    pub fn new(source: &'a S, offset: u64, len: u64) -> Result<Self, ByteReadError> {
        let end = offset.checked_add(len).ok_or(ByteReadError::OutOfRange)?;
        if end > source.len() {
            return Err(ByteReadError::OutOfRange);
        }
        Ok(Self {
            source,
            offset,
            len,
        })
    }
}

impl<S: ByteSource + ?Sized> ByteSource for ByteWindow<'_, S> {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
        let end = offset
            .checked_add(buf.len() as u64)
            .ok_or(ByteReadError::OutOfRange)?;
        if end > self.len {
            return Err(ByteReadError::OutOfRange);
        }
        let start = self
            .offset
            .checked_add(offset)
            .ok_or(ByteReadError::OutOfRange)?;
        self.source.read_at(start, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SliceSource(&'static [u8]);

    impl ByteSource for SliceSource {
        fn len(&self) -> u64 {
            self.0.len() as u64
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError> {
            let start = offset as usize;
            let end = start
                .checked_add(buf.len())
                .ok_or(ByteReadError::OutOfRange)?;
            if end > self.0.len() {
                return Err(ByteReadError::OutOfRange);
            }
            buf.copy_from_slice(&self.0[start..end]);
            Ok(())
        }
    }

    static REGION: SliceSource = SliceSource(&[0, 1, 2, 3, 4, 5, 6, 7]);

    #[test]
    fn the_window_reports_its_own_length_not_the_sources() {
        let w = ByteWindow::new(&REGION, 2, 3).unwrap();
        assert_eq!(w.len(), 3);
        assert!(!w.is_empty());
    }

    #[test]
    fn a_read_at_zero_starts_at_the_window_offset() {
        let w = ByteWindow::new(&REGION, 2, 3).unwrap();
        let mut buf = [0u8; 3];
        w.read_at(0, &mut buf).unwrap();
        assert_eq!(buf, [2, 3, 4]);
    }

    #[test]
    fn a_read_past_the_window_end_is_out_of_range() {
        let w = ByteWindow::new(&REGION, 2, 3).unwrap();
        let mut buf = [0u8; 2];
        assert_eq!(w.read_at(2, &mut buf), Err(ByteReadError::OutOfRange));
    }

    #[test]
    fn a_window_past_the_end_of_the_source_is_refused() {
        assert_eq!(
            ByteWindow::new(&REGION, 6, 4).err(),
            Some(ByteReadError::OutOfRange)
        );
    }

    #[test]
    fn a_window_offset_that_overflows_is_refused() {
        assert_eq!(
            ByteWindow::new(&REGION, u64::MAX, 1).err(),
            Some(ByteReadError::OutOfRange)
        );
    }

    #[test]
    fn an_empty_window_is_empty() {
        let w = ByteWindow::new(&REGION, 8, 0).unwrap();
        assert!(w.is_empty());
    }
}
