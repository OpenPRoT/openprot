// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # IPC Event Queue — in-process follow-up buffer
//!
//! A building block for an IPC-backed, blocking event queue that drives a
//! single-threaded state machine (such as `openprot_lifecycle_sm`):
//! [`PendingQueue`], a fixed-capacity, generic, syscall-free buffer for the
//! follow-up events a state handler emits.
//!
//! When a handler reports its outcome, that event is produced *inside* the
//! task and does not need to round-trip through the kernel. The on-target queue
//! buffers it here and drains it ahead of the next blocking channel read, which
//! keeps the common path syscall-free and preserves ordering between a
//! handler's follow-up and an externally injected event. The syscall half and
//! the byte codec live in the consumer crates (the `pw_kernel` userspace crate
//! only builds on-target; the wire codec lives beside the message types in the
//! API crate). This crate stays generic and host-testable.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use heapless::Deque;

/// A fixed-capacity, in-process buffer for follow-up events.
///
/// When a state handler reports its outcome (e.g. `VerifyDone`), that event is
/// produced *inside* the lifecycle task and does not need to round-trip through
/// the kernel. It is pushed here and drained ahead of the next blocking channel
/// read, which keeps the common path syscall-free and preserves ordering
/// between a handler's follow-up and any externally injected event.
///
/// `N` is the maximum number of un-drained follow-up events. For the lifecycle
/// machine a handler emits at most one follow-up per step, so a small `N`
/// (e.g. 4) is ample headroom.
pub struct PendingQueue<M, const N: usize> {
    inner: Deque<M, N>,
}

impl<M, const N: usize> Default for PendingQueue<M, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, const N: usize> PendingQueue<M, N> {
    /// Create an empty queue.
    pub const fn new() -> Self {
        Self {
            inner: Deque::new(),
        }
    }

    /// Enqueue a follow-up message.
    ///
    /// Returns `Err(message)` if the queue is full, handing the message back so
    /// the caller can decide how to surface the overflow rather than silently
    /// dropping it.
    pub fn push(&mut self, message: M) -> Result<(), M> {
        self.inner.push_back(message)
    }

    /// Remove and return the oldest buffered message, if any.
    pub fn pop(&mut self) -> Option<M> {
        self.inner.pop_front()
    }

    /// Whether there are no buffered messages.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Whether the queue is at capacity.
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_drains_in_fifo_order() {
        let mut q: PendingQueue<u8, 4> = PendingQueue::new();
        assert!(q.is_empty());
        q.push(1).unwrap();
        q.push(2).unwrap();
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    #[test]
    fn pending_reports_and_rejects_overflow() {
        let mut q: PendingQueue<u8, 2> = PendingQueue::new();
        q.push(1).unwrap();
        q.push(2).unwrap();
        assert!(q.is_full());
        // Full: the message is handed back rather than dropped.
        assert_eq!(q.push(3), Err(3));
    }
}
