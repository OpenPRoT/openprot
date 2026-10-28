// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Where the pushed image lands.

/// Why the staging region refused bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StagingError {
    /// The write would run past the size the agent announced in
    /// `INDIRECT_FIFO_CTRL`, or past the capacity of the region.
    Overflow,
    /// Bytes arrived before the agent announced an image size.
    NoImageAnnounced,
}

impl core::fmt::Display for StagingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overflow => write!(f, "write past the announced image size"),
            Self::NoImageAnnounced => write!(f, "bytes before an image size was announced"),
        }
    }
}

impl core::error::Error for StagingError {}

/// The sink one `INDIRECT_FIFO_DATA` payload drains into.
///
/// The dispatch calls [`FifoSink::begin_image`] when the agent programs
/// `INDIRECT_FIFO_CTRL` with a reset, and [`FifoSink::append`] for every data
/// payload. The real sink is the staging region the verify path reads back;
/// tests use [`RamStaging`].
pub trait FifoSink {
    /// Starts a fresh image of `size_bytes`, discarding anything staged.
    ///
    /// Refuses a size the region cannot hold, so the refusal reaches the
    /// agent before any bytes move.
    fn begin_image(&mut self, size_bytes: u64) -> Result<(), StagingError>;

    /// Appends one payload.
    fn append(&mut self, bytes: &[u8]) -> Result<(), StagingError>;
}

/// A RAM-backed staging region.
///
/// `len` and `read_at` mirror the `ImageSource` shape the orchestrator verify
/// path consumes (`services/orchestrator/driver/src/board.rs`); the trait
/// impl itself lives in an orchestrator adapter so this crate does not depend
/// on the orchestrator driver.
pub struct RamStaging<const N: usize> {
    buf: [u8; N],
    len: usize,
    announced: Option<u64>,
}

impl<const N: usize> RamStaging<N> {
    /// An empty region with no image announced.
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
            announced: None,
        }
    }

    /// How many bytes the agent has pushed so far.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing is staged.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the staged bytes amount to the size the agent announced.
    pub fn is_complete(&self) -> bool {
        self.announced
            .is_some_and(|announced| announced == self.len as u64)
    }

    /// Reads staged bytes starting at `offset`, the `ImageSource::read_at`
    /// shape. `None` means the range runs past what is staged.
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> Option<()> {
        let end = offset.checked_add(buf.len())?;
        let staged = self.buf.get(offset..end.min(self.len))?;
        if staged.len() != buf.len() {
            return None;
        }
        buf.copy_from_slice(staged);
        Some(())
    }
}

impl<const N: usize> Default for RamStaging<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> FifoSink for RamStaging<N> {
    fn begin_image(&mut self, size_bytes: u64) -> Result<(), StagingError> {
        if size_bytes > N as u64 {
            return Err(StagingError::Overflow);
        }
        self.len = 0;
        self.announced = Some(size_bytes);
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StagingError> {
        let announced = self.announced.ok_or(StagingError::NoImageAnnounced)?;
        let end = self
            .len
            .checked_add(bytes.len())
            .ok_or(StagingError::Overflow)?;
        if end as u64 > announced {
            return Err(StagingError::Overflow);
        }
        // begin_image bounds announced by N, so end fits the buffer.
        let dst = self
            .buf
            .get_mut(self.len..end)
            .ok_or(StagingError::Overflow)?;
        dst.copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_before_an_announced_image_are_refused() {
        let mut staging = RamStaging::<16>::new();
        assert_eq!(staging.append(&[1, 2]), Err(StagingError::NoImageAnnounced));
    }

    #[test]
    fn an_image_larger_than_the_region_is_refused_before_bytes_move() {
        let mut staging = RamStaging::<16>::new();
        assert_eq!(staging.begin_image(17), Err(StagingError::Overflow));
        assert!(staging.begin_image(16).is_ok());
    }

    #[test]
    fn a_write_past_the_announced_size_is_refused_and_the_staged_bytes_stay() {
        let mut staging = RamStaging::<16>::new();
        staging.begin_image(4).unwrap();
        staging.append(&[1, 2, 3]).unwrap();
        assert_eq!(staging.append(&[4, 5]), Err(StagingError::Overflow));
        assert_eq!(staging.len(), 3);
        assert!(!staging.is_complete());
        staging.append(&[4]).unwrap();
        assert!(staging.is_complete());
    }

    #[test]
    fn read_at_serves_the_staged_bytes_and_refuses_past_the_end() {
        let mut staging = RamStaging::<16>::new();
        staging.begin_image(4).unwrap();
        staging.append(&[1, 2, 3, 4]).unwrap();

        let mut out = [0u8; 2];
        staging.read_at(1, &mut out).unwrap();
        assert_eq!(out, [2, 3]);
        assert!(staging.read_at(3, &mut out).is_none());
    }

    #[test]
    fn a_new_image_discards_the_staged_one() {
        let mut staging = RamStaging::<16>::new();
        staging.begin_image(2).unwrap();
        staging.append(&[1, 2]).unwrap();
        staging.begin_image(2).unwrap();
        assert!(staging.is_empty());
    }
}
