// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(test), no_std)]

/// A simple single-threaded ring buffer of type `T` with a backing array of size `N`.
///
/// The effective capacity of this queue is `N - 1`.
///
/// If producer == consumer, the queue is empty.
/// If (producer + 1) % N == consumer, the queue is full.
pub struct RingBuffer<T: Copy + Default, const N: usize> {
    producer: usize,
    consumer: usize,
    data: [T; N],
}

/// Creates a default empty `RingBuffer`.
///
/// # Panics
///
/// Panics at compile time if `N <= 1`.
impl<T: Copy + Default, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        // The queue requires at least 2 slots to store 1 item (N-1 capacity).
        // Using a const block to ensure this is checked at compile time.
        const { assert!(N > 1, "RingBuffer size N must be greater than 1") };
        Self {
            producer: 0,
            consumer: 0,
            data: [T::default(); N],
        }
    }
}


impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    /// Pushes an item into the buffer.
    ///
    /// Returns `Ok(())` if the item was successfully added, or `Err(item)` if the buffer is full.
    pub fn push(&mut self, item: T) -> Result<(), T> {
        let next = (self.producer + 1) % N;
        if next != self.consumer {
            self.data[self.producer] = item;
            self.producer = next;
            Ok(())
        } else {
            Err(item)
        }
    }

    /// Removes and returns the first item from the buffer.
    ///
    /// Returns `Some(item)` if the buffer is not empty, or `None` if it is.
    pub fn pop(&mut self) -> Option<T> {
        if self.consumer != self.producer {
            let item = self.data[self.consumer];
            self.consumer = (self.consumer + 1) % N;
            Some(item)
        } else {
            None
        }
    }

    /// Returns the number of items currently in the buffer.
    pub fn len(&self) -> usize {
        (self.producer + N - self.consumer) % N
    }

    /// Returns `true` if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.producer == self.consumer
    }

    /// Returns `true` if the buffer is full.
    pub fn is_full(&self) -> bool {
        (self.producer + 1) % N == self.consumer
    }

    /// Pushes as many items from the slice into the buffer as possible.
    ///
    /// Returns `Ok(())` if all items were added, or `Err(&[T])` containing the
    /// remaining unbuffered items if the buffer became full.
    pub fn push_slice<'a>(&mut self, s: &'a [T]) -> Result<(), &'a [T]> {
        let free = (self.consumer + N - self.producer - 1) % N;
        let n = core::cmp::min(s.len(), free);

        let mut remaining = n;
        let mut src_idx = 0;

        let chunk1 = core::cmp::min(remaining, N - self.producer);
        if chunk1 > 0 {
            self.data[self.producer..self.producer + chunk1].copy_from_slice(&s[src_idx..src_idx + chunk1]);
            self.producer = (self.producer + chunk1) % N;
            remaining -= chunk1;
            src_idx += chunk1;
        }

        if remaining > 0 {
            self.data[0..remaining].copy_from_slice(&s[src_idx..src_idx + remaining]);
            self.producer = remaining;
        }

        if n < s.len() {
            Err(&s[n..])
        } else {
            Ok(())
        }
    }

    /// Returns a slice containing the contiguous part of the buffered data.
    ///
    /// If the data wraps around the end of the backing array, this only returns
    /// the first part. Callers should use `consume()` and `as_slice()` again to
    /// retrieve the wrapped portion.
    pub fn as_slice(&self) -> &[T] {
        if self.consumer <= self.producer {
            // The available slice is contiguous in the array.
            &self.data[self.consumer..self.producer]
        } else {
            // Slices can't wrap around the end of the array, so give just the chunk we can give.
            &self.data[self.consumer..]
        }
    }

    /// Advances the consumer pointer by `n` items, effectively removing them from the buffer.
    ///
    /// If `n` is greater than the current length, only the available items are consumed.
    pub fn consume(&mut self, n: usize) {
        let n = core::cmp::min(n, self.len());
        self.consumer = (self.consumer + n) % N;
    }
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut q = RingBuffer::<u8, 4>::default();

        assert_eq!(q.push(b'b'), Ok(()));
        assert_eq!(q.push(b'y'), Ok(()));
        assert_eq!(q.push(b'e'), Ok(()));
        assert_eq!(q.push(b'x'), Err(b'x')); // Overflow
        assert_eq!(q.len(), 3);
        assert_eq!(q.is_empty(), false);
        assert_eq!(q.is_full(), true);

        assert_eq!(q.pop(), Some(b'b'));
        assert_eq!(q.pop(), Some(b'y'));
        assert_eq!(q.pop(), Some(b'e'));
        assert_eq!(q.pop(), None); // No more items.
        assert_eq!(q.len(), 0);
        assert_eq!(q.is_empty(), true);
        assert_eq!(q.is_full(), false);
    }

    #[test]
    fn test_as_slice() {
        let mut q = RingBuffer::<u8, 8>::default();
        assert_eq!(q.push_slice(b"Hello"), Ok(()));

        assert_eq!(q.as_slice(), b"Hello");
        q.consume(5);
        assert_eq!(q.is_empty(), true);

        // This part will be wrapped around the end of the array,
        // so we need to perform two `as_slice` ops to get the whole thing.
        assert_eq!(q.push_slice(b"World"), Ok(()));
        assert_eq!(q.as_slice(), b"Wor");
        q.consume(3);
        assert_eq!(q.as_slice(), b"ld");
        q.consume(2);
        assert_eq!(q.is_empty(), true);
    }
}
