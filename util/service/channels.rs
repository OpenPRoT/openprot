// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Which channels a service loop answers, and which of them have a reply
//! parked. Pure state, no syscalls: the loop in `lib.rs` pairs every change
//! here with the matching WaitGroup operation.

/// One IPC channel a [`ServiceLoop`](crate::ServiceLoop) answers on.
///
/// Identified purely by its runtime `channel_handler` handle, so the same
/// loop serves one channel or many.
#[derive(Debug)]
pub struct Channel {
    handle: u32,
    /// Key of the deferred reply parked here, if any.
    parked: Option<u32>,
}

impl Channel {
    /// Wrap a `channel_handler` IPC handle.
    pub const fn new(handle: u32) -> Self {
        Self {
            handle,
            parked: None,
        }
    }

    pub(crate) const fn handle(&self) -> u32 {
        self.handle
    }

    /// Record a deferred reply. Fails with the existing key if one is
    /// already parked: a channel carries one transaction at a time.
    pub(crate) fn park(&mut self, key: u32) -> Result<(), u32> {
        match self.parked {
            Some(existing) => Err(existing),
            None => {
                self.parked = Some(key);
                Ok(())
            }
        }
    }

    /// Clear the parked reply, returning its key.
    pub(crate) fn unpark(&mut self) -> Option<u32> {
        self.parked.take()
    }
}

/// Index of the channel answering on `handle`.
pub(crate) fn position(channels: &[Channel], handle: u32) -> Option<usize> {
    channels.iter().position(|c| c.handle == handle)
}

/// Index of the channel whose parked reply has `key`.
pub(crate) fn position_parked(channels: &[Channel], key: u32) -> Option<usize> {
    channels.iter().position(|c| c.parked == Some(key))
}

/// Whether every channel has a different handle.
pub(crate) fn distinct(channels: &[Channel]) -> bool {
    channels
        .iter()
        .enumerate()
        .all(|(i, c)| position(channels, c.handle) == Some(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> [Channel; 3] {
        [Channel::new(10), Channel::new(11), Channel::new(12)]
    }

    #[test]
    fn park_then_unpark_round_trips() {
        let mut ch = Channel::new(7);
        assert_eq!(ch.handle(), 7);
        assert_eq!(ch.unpark(), None);
        assert_eq!(ch.park(42), Ok(()));
        assert_eq!(ch.unpark(), Some(42));
        assert_eq!(ch.unpark(), None);
    }

    #[test]
    fn double_park_keeps_first_and_reports_it() {
        let mut ch = Channel::new(7);
        assert_eq!(ch.park(1), Ok(()));
        assert_eq!(ch.park(2), Err(1));
        assert_eq!(ch.unpark(), Some(1));
    }

    #[test]
    fn position_finds_by_handle() {
        let chs = three();
        assert_eq!(position(&chs, 11), Some(1));
        assert_eq!(position(&chs, 99), None);
    }

    #[test]
    fn position_parked_finds_by_key_not_handle() {
        let mut chs = three();
        assert_eq!(position_parked(&chs, 5), None);
        chs[2].park(5).unwrap();
        assert_eq!(position_parked(&chs, 5), Some(2));
        // 12 is that channel's handle, not a parked key.
        assert_eq!(position_parked(&chs, 12), None);
        chs[2].unpark();
        assert_eq!(position_parked(&chs, 5), None);
    }

    #[test]
    fn distinct_rejects_a_repeated_handle() {
        assert!(distinct(&three()));
        assert!(distinct(&[]));
        assert!(!distinct(&[
            Channel::new(1),
            Channel::new(2),
            Channel::new(1)
        ]));
    }
}
