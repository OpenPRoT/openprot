// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The state machine.

use crate::behaviour::{Behaviour, Phase, Transition};

/// Transitions held for [`DeviceModel::take_transition`]. A caller drains
/// them every wake, so a handful is enough; more are counted, not kept.
const QUEUE: usize = 4;
// The queue indices are taken modulo `QUEUE`, so it must not be zero.
const _: () = assert!(QUEUE > 0);

/// The device on the other end of a reset line.
///
/// Starts with reset asserted, so a device the caller has not released yet is
/// down, the fail-safe state of a managed reset line.
pub struct DeviceModel<'a> {
    script: &'a [Behaviour],
    releases: u32,
    current: Behaviour,
    phase: Phase,
    /// When the next phase change is due, if one is.
    next_at: Option<u64>,
    queue: [Option<Transition>; QUEUE],
    head: usize,
    len: usize,
    dropped: u32,
}

impl<'a> DeviceModel<'a> {
    /// A device that follows `script`, one entry per release of reset. An
    /// empty script is a device that always hangs.
    pub const fn new(script: &'a [Behaviour]) -> Self {
        Self {
            script,
            releases: 0,
            current: Behaviour::Hang,
            phase: Phase::Held,
            next_at: None,
            queue: [None; QUEUE],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    /// Where the device is as of the last call.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The ready level.
    pub fn ready(&self) -> bool {
        self.phase == Phase::Ready
    }

    /// How many times reset has been released.
    pub fn releases(&self) -> u32 {
        self.releases
    }

    /// When the next phase change is due, if one is, so an event loop can
    /// sleep until then.
    pub fn next_event_at(&self) -> Option<u64> {
        self.next_at
    }

    /// Transitions lost because the queue was full when they happened.
    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Drive the reset line. `asserted` holds the device in reset; `false`
    /// releases it and starts the next boot attempt.
    ///
    /// Repeating the level the line already has changes nothing, so a caller
    /// may set it every wake. Releasing while not held is ignored, and
    /// asserting always wins: it drops ready, cancels a boot in progress, and
    /// brings the device back to [`Phase::Held`] from any phase.
    pub fn set_reset(&mut self, asserted: bool, now_millis: u64) {
        // Bring the device up to date first, so a change that fell due before
        // this moment is seen to happen before the reset does.
        self.advance(now_millis);
        if asserted {
            if self.phase == Phase::Ready {
                self.push(Transition {
                    at_millis: now_millis,
                    ready: false,
                });
            }
            self.phase = Phase::Held;
            self.next_at = None;
        } else if self.phase == Phase::Held {
            self.start(now_millis);
        }
    }

    /// Advance to `now_millis` and report where the device is. Never waits.
    pub fn poll(&mut self, now_millis: u64) -> Phase {
        self.advance(now_millis);
        self.phase
    }

    /// The oldest change of the ready level not yet taken, if any.
    pub fn take_transition(&mut self) -> Option<Transition> {
        if self.len == 0 {
            return None;
        }
        let taken = self.queue.get_mut(self.head).and_then(Option::take);
        self.head = self.head.saturating_add(1) % QUEUE;
        self.len = self.len.saturating_sub(1);
        taken
    }

    /// What the `n`th release does: the `n`th script entry, the last one
    /// once the script runs out, or a hang for an empty script.
    fn behaviour_for(&self, n: u32) -> Behaviour {
        // A release count too large for `usize` (not possible on the targets
        // this builds for) falls back to the last entry like any other
        // count past the end of the script.
        usize::try_from(n)
            .ok()
            .and_then(|i| self.script.get(i))
            .or_else(|| self.script.last())
            .copied()
            .unwrap_or(Behaviour::Hang)
    }

    /// Begin a boot attempt at `now_millis`.
    fn start(&mut self, now_millis: u64) {
        let n = self.releases;
        self.releases = self.releases.saturating_add(1);
        self.current = self.behaviour_for(n);
        match self.current {
            Behaviour::Hang => {
                self.phase = Phase::Hung;
                self.next_at = None;
            }
            Behaviour::Boots { after_millis } | Behaviour::BootsThenFails { after_millis, .. } => {
                self.phase = Phase::Booting;
                self.next_at = Some(now_millis.saturating_add(after_millis));
            }
        }
    }

    /// Apply every change due at or before `now_millis`, in order, each
    /// stamped with the moment it was due.
    fn advance(&mut self, now_millis: u64) {
        while let Some(at) = self.next_at {
            if now_millis < at {
                break;
            }
            match self.phase {
                Phase::Booting => {
                    self.phase = Phase::Ready;
                    self.push(Transition {
                        at_millis: at,
                        ready: true,
                    });
                    self.next_at = match self.current {
                        Behaviour::BootsThenFails { up_millis, .. } => {
                            Some(at.saturating_add(up_millis))
                        }
                        _ => None,
                    };
                }
                Phase::Ready => {
                    self.phase = Phase::Down;
                    self.push(Transition {
                        at_millis: at,
                        ready: false,
                    });
                    self.next_at = None;
                }
                // Nothing is due in the other phases.
                _ => self.next_at = None,
            }
        }
    }

    fn push(&mut self, transition: Transition) {
        if self.len == QUEUE {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        let at = self.head.saturating_add(self.len) % QUEUE;
        if let Some(slot) = self.queue.get_mut(at) {
            *slot = Some(transition);
            self.len = self.len.saturating_add(1);
        }
    }
}
