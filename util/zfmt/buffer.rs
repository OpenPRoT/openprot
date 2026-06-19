// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zfmt::events::{EventHeader, StreamStart};

/// A generic ring buffer for storing serialized `zfmt` log events.
///
/// It supports writing events (with automatic eviction of oldest events when full)
/// and reading events sequentially using a cursor.
pub struct LogBuffer<const N: usize> {
    /// The underlying byte buffer.
    pub buf: [u8; N],
    /// The absolute write cursor position (monotonically increasing).
    pub write: u64,
    /// The absolute read cursor position (monotonically increasing).
    /// Represents the oldest non-evicted data.
    pub read: u64,
}

impl<const N: usize> Default for LogBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> LogBuffer<N> {
    /// Creates a new empty `LogBuffer`.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; N],
            write: 0,
            read: 0,
        }
    }

    /// Peeks at a single byte at the given absolute cursor position.
    ///
    /// Returns `None` if the cursor is at or ahead of the write cursor.
    pub fn peek(&self, at: u64) -> Option<u8> {
        if at < self.write {
            Some(self.buf[at as usize % N])
        } else {
            None
        }
    }

    fn peek_u32(&self, at: u64) -> Option<(u32, usize)> {
        let a = self.peek(at)?;
        let b = self.peek(at + 1)?;
        let c = self.peek(at + 2)?;
        let d = self.peek(at + 3)?;
        Some((u32::from_le_bytes([a, b, c, d]), 4))
    }

    fn peek_leb128(&self, at: u64) -> Option<(usize, usize)> {
        let mut i = at;
        let mut val = 0usize;
        let mut shift = 0;
        loop {
            let n = self.peek(i)? as usize;
            i += 1;
            val |= (n & 0x7f) << shift;
            if n & 0x80 == 0 {
                return Some((val, (i - at) as usize));
            }
            shift += 7;
            if shift >= 32 {
                // Overflow
                return None;
            }
        }
    }

    /// Decodes the size and tag of the frame starting at the absolute cursor `at`.
    ///
    /// Returns `Some((tag, frame_size))` where `frame_size` is the total size of the
    /// frame in bytes (including tag and length headers).
    /// Returns `Some((0, 0))` if `at` is at the write cursor.
    /// Returns `None` if the data is incomplete or corrupted.
    pub fn next_frame_size(&self, at: u64) -> Option<(u32, usize)> {
        let mut i = at;
        if i == self.write {
            return Some((0, 0));
        }

        // Get the next message tag and length and advance i by the
        // consumed bytes and the len.
        let (tag, n) = self.peek_u32(i)?;
        i += n as u64;
        let (len, n) = self.peek_leb128(i)?;
        i += (n + len) as u64;
        match tag {
            StreamStart::ZFMT_TAG => Some((tag, (i - at) as usize)),
            EventHeader::ZFMT_TAG => {
                // The EventHeader always has an event following and we
                // report the EventHeader+Next as a single entity.
                let (_next_tag, n) = self.peek_u32(i)?;
                i += n as u64;
                let (len, n) = self.peek_leb128(i)?;
                i += (n + len) as u64;
                Some((tag, (i - at) as usize))
            }
            _ => Some((tag, (i - at) as usize)),
        }
    }

    /// Gets the next frame at `at` as a pair of slices (handling ring buffer wrap-around).
    ///
    /// Returns `Some((tag, slice1, slice2))` where the frame content is the concatenation
    /// of `slice1` and `slice2`. If there is no wrap-around, `slice2` will be empty.
    #[inline]
    pub fn next_frame_slice(&self, at: u64) -> Option<(u32, &[u8], &[u8])> {
        let (tag, len) = self.next_frame_size(at)?;
        if len > N {
            // TODO: If this ever happens, the buffer is corrupt.
            return None;
        }
        let start = at as usize % N;
        let end = start + len;
        if end > N {
            let end = end - N;
            Some((tag, &self.buf[start..N], &self.buf[0..end]))
        } else {
            const EMPTY: [u8; 0] = [];
            Some((tag, &self.buf[start..end], &EMPTY))
        }
    }

    /// Peeks at and decodes the `EventHeader` starting at absolute cursor `at`.
    pub fn peek_event_header(&self, at: u64) -> Option<EventHeader> {
        let (lo, _) = self.peek_u32(at)?;
        let (hi, _) = self.peek_u32(at + 4)?;
        let (sevseq, _) = self.peek_u32(at + 8)?;
        let [sev, sq0, sq1, sq2] = sevseq.to_le_bytes();
        Some(EventHeader {
            timestamp: zfmt::ZfmtU64::new(lo, hi),
            severity: sev,
            seq: [sq0, sq1, sq2],
        })
    }

    // Return the number of bytes we need to evict to make room for `need` bytes
    // and to advance the cursor to the next record boundary.
    //
    // This only advances one record at a time - keep calling it until it returns false.
    fn evict(&mut self, need: usize) -> Option<bool> {
        // It shouldn't be possible for write to get more than N
        // ahead of read.
        let avail = N.saturating_sub((self.write - self.read) as usize);
        if avail < need {
            let (_tag, len) = self.next_frame_size(self.read)?;
            self.read += len as u64;
            Some(true)
        } else {
            Some(false)
        }
    }

    fn push(&mut self, data: &[u8]) {
        for &byte in data.iter() {
            self.buf[self.write as usize % N] = byte;
            self.write += 1;
        }
    }

    /// Pushes a serialized `zfmt` event frame into the buffer.
    ///
    /// If the buffer is full, oldest frames will be evicted until there is enough space.
    /// The event is assumed to be properly formatted.
    pub fn push_frame(&mut self, event: &[u8]) {
        // We assume that `event` is properly formatted and that the length of the slice is the
        // length of the event to be logged.
        loop {
            match self.evict(event.len()) {
                Some(true) => {
                    continue;
                }
                Some(false) => {
                    break;
                }
                None => {
                    // should never happen
                    break;
                }
            }
        }
        self.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zfmt::events::{DebugMessage, EventHeader, StreamStart};

    fn make_log_event(msg: &str) -> std::vec::Vec<u8> {
        let mut event = std::vec::Vec::new();
        event.extend_from_slice(&EventHeader::ZFMT_TAG.to_le_bytes());
        event.push(12); // len
        event.extend_from_slice(&[0u8; 12]);

        event.extend_from_slice(&DebugMessage::ZFMT_TAG.to_le_bytes());
        assert!(msg.len() < 128);
        event.push(msg.len() as u8);
        event.extend_from_slice(msg.as_bytes());
        event
    }

    #[test]
    fn test_new() {
        let buf = LogBuffer::<64>::new();
        assert_eq!(buf.write, 0);
        assert_eq!(buf.read, 0);
    }

    #[test]
    fn test_push_read() {
        let mut buf = LogBuffer::<64>::new();
        let event = make_log_event("hello");
        buf.push_frame(&event);

        assert_eq!(buf.write, event.len() as u64);
        assert_eq!(buf.read, 0);

        let (tag, s1, s2) = buf.next_frame_slice(0).unwrap();
        assert_eq!(tag, EventHeader::ZFMT_TAG);

        let mut read_event = std::vec::Vec::new();
        read_event.extend_from_slice(s1);
        read_event.extend_from_slice(s2);
        assert_eq!(read_event, event);
    }

    #[test]
    fn test_eviction() {
        let mut buf = LogBuffer::<32>::new();
        let event1 = make_log_event("a");
        let event2 = make_log_event("b");
        assert_eq!(event1.len(), 23);

        buf.push_frame(&event1);
        assert_eq!(buf.write, 23);
        assert_eq!(buf.read, 0);

        buf.push_frame(&event2);
        assert_eq!(buf.write, 46);
        assert_eq!(buf.read, 23);

        let (tag, s1, s2) = buf.next_frame_slice(23).unwrap();
        assert_eq!(tag, EventHeader::ZFMT_TAG);
        let mut read_event = std::vec::Vec::new();
        read_event.extend_from_slice(s1);
        read_event.extend_from_slice(s2);
        assert_eq!(read_event, event2);
    }

    #[test]
    fn test_wrap_around() {
        let mut buf = LogBuffer::<32>::new();

        let mut event1 = std::vec::Vec::new();
        event1.extend_from_slice(&StreamStart::ZFMT_TAG.to_le_bytes());
        event1.push(5);
        event1.extend_from_slice(b"start");
        assert_eq!(event1.len(), 10);

        buf.push_frame(&event1);
        assert_eq!(buf.write, 10);

        buf.push_frame(&event1);
        assert_eq!(buf.write, 20);

        buf.push_frame(&event1);
        assert_eq!(buf.write, 30);
        assert_eq!(buf.read, 0);

        buf.push_frame(&event1);
        assert_eq!(buf.write, 40);
        assert_eq!(buf.read, 10);

        let (tag, s1, s2) = buf.next_frame_slice(30).unwrap();
        assert_eq!(tag, StreamStart::ZFMT_TAG);
        assert_eq!(s1.len(), 2); // 30..32
        assert_eq!(s2.len(), 8); // 0..8

        let mut read_event = std::vec::Vec::new();
        read_event.extend_from_slice(s1);
        read_event.extend_from_slice(s2);
        assert_eq!(read_event, event1);
    }
}
