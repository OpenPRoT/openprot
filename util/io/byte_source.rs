// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Random-access reads over a fixed-length byte range.

/// A byte range of known length, read at arbitrary offsets.
///
/// Reads take `&self`, so a caller can read through the source while
/// something else is borrowed mutably: a transfer pulls from the source
/// and writes to a device held by `&mut`. The error type is fixed rather
/// than associated, which keeps the trait dyn-compatible for a
/// heterogeneous set of sources behind one `&dyn`.
/// [`RandomRead`](crate::RandomRead) is the `&mut self` counterpart for
/// callers that need neither.
///
/// Where the bytes live (flash, a mapped blob, a test slice) stays
/// behind the source.
pub trait ByteSource {
    /// Total length in bytes, constant for the lifetime of the source:
    /// readers size their buffers from it and treat a transfer as
    /// complete once this many bytes are through.
    fn len(&self) -> u64;

    /// True if the range is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `buf` from `offset`. The read is exact: short fills are a
    /// fault, and `offset + buf.len()` beyond [`len`](Self::len) is out
    /// of range. There are no partial reads: the length is known up
    /// front, so a short read can only mean the source cannot serve
    /// what `len` promised, and a partial-read API would put a retry
    /// loop into every reader.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ByteReadError>;
}

/// Why a read failed, the one distinction retry policy needs.
///
/// No further detail crosses the seam (mirroring `BootWatch`): the source
/// logs the concrete cause while it is still in scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteReadError {
    /// The requested range is outside the source, a caller bug, never
    /// retriable.
    OutOfRange,
    /// The backing storage failed the read, possibly transient; reading
    /// anew may succeed.
    Storage,
}

impl core::fmt::Display for ByteReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            ByteReadError::OutOfRange => "read out of range",
            ByteReadError::Storage => "storage fault",
        })
    }
}

impl core::error::Error for ByteReadError {}
