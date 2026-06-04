// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C In-Band Interrupt (IBI) Work Queue
//!
//! Handles IBI events including Hot-Join, SIR (Slave Interrupt Request),
//! and target dynamic address assignment.
//!
//! Ported from `aspeed-rust/src/i3c/ibi.rs` @ ce3b567.
//!
//! **Porting delta (queue mechanism).** The reference uses `heapless::spsc`
//! `Producer`/`Consumer` handles, split once and parked in a global
//! `Mutex<UnsafeCell<..>>`. On this target (heapless 0.9 + this toolchain) we
//! observed unstable behavior when those handles were stored in a `static` and
//! later re-accessed across separate critical sections: a split that read back
//! `prod=Some, cons=Some` in-place would, after the consumer was taken in a
//! later critical section, read back `prod=None, cons=Some`. The root cause was
//! not fully isolated, so this port uses a simpler fixed-size ring buffer whose
//! aliasing and lifetime rules are easier to audit.
//!
//! So the SPSC split is replaced by a plain fixed-size ring buffer of
//! `Option<IbiWork>` (`IbiWork` is `Copy`, no niche pointers), guarded by the
//! same `critical_section`. The process-global queue/handler design (goal.md
//! ADR-3) is preserved — an ISR still cannot borrow a stack-owned device, so
//! the IBI plane stays global, arbitrated by `critical_section`. The public
//! API (`i3c_ibi_workq_consumer().dequeue()` + the three enqueue functions) is
//! unchanged.

use core::cell::UnsafeCell;
use critical_section::Mutex;

/// IBI queue depth
const IBIQ_DEPTH: usize = 16;
/// Maximum IBI payload data size
const IBI_DATA_MAX: u8 = 16;

// =============================================================================
// IBI Work Item
// =============================================================================

/// IBI work item representing an interrupt event
#[derive(Debug, Clone, Copy)]
pub enum IbiWork {
    /// Hot-Join request from a device
    HotJoin,
    /// Slave Interrupt Request
    Sirq {
        /// Address of requesting device
        addr: u8,
        /// Length of payload data
        len: u8,
        /// Payload data
        data: [u8; IBI_DATA_MAX as usize],
    },
    /// Target dynamic address assignment notification
    TargetDaAssignment,
    /// Private write received by this target from the controller.
    TargetMasterWrite {
        /// Number of received bytes captured in `data`.
        len: u8,
        /// Received data, truncated to `IBI_DATA_MAX`.
        data: [u8; IBI_DATA_MAX as usize],
    },
}

// =============================================================================
// Static Ring-Buffer Storage
// =============================================================================

/// Fixed-size single-producer/single-consumer ring of IBI work items.
///
/// All access is serialized by the per-bus `critical_section::Mutex`, so the
/// indices need no atomics; the producer is the I3C ISR and the consumer is the
/// owning test/driver loop.
struct IbiRing {
    buf: [Option<IbiWork>; IBIQ_DEPTH],
    head: usize,
    len: usize,
}

impl IbiRing {
    const fn new() -> Self {
        Self {
            buf: [None; IBIQ_DEPTH],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, work: IbiWork) -> bool {
        // `get_mut` + modulo keep this panic-free even if the indices were
        // somehow out of range; `head` is normalized first.
        self.head %= IBIQ_DEPTH;
        if self.len >= IBIQ_DEPTH {
            return false;
        }
        let idx = (self.head + self.len) % IBIQ_DEPTH;
        if let Some(slot) = self.buf.get_mut(idx) {
            *slot = Some(work);
            self.len += 1;
            true
        } else {
            false
        }
    }

    fn pop(&mut self) -> Option<IbiWork> {
        self.head %= IBIQ_DEPTH;
        if self.len == 0 || self.len > IBIQ_DEPTH {
            // Empty, or a corrupt length — treat as empty (panic-free).
            return None;
        }
        let work = self.buf.get_mut(self.head).and_then(Option::take);
        self.head = (self.head + 1) % IBIQ_DEPTH;
        self.len -= 1;
        work
    }
}

static IBI_RINGS: [Mutex<UnsafeCell<IbiRing>>; 4] = [
    Mutex::new(UnsafeCell::new(IbiRing::new())),
    Mutex::new(UnsafeCell::new(IbiRing::new())),
    Mutex::new(UnsafeCell::new(IbiRing::new())),
    Mutex::new(UnsafeCell::new(IbiRing::new())),
];

/// Run `f` against the ring for `bus`, serialized by the critical section.
///
/// Returns `None` if `bus` is out of range.
///
/// # Safety Contract
///
/// `f` must not re-enter this module's queue API (`with_ring`, `dequeue`, or
/// any of the enqueue helpers). Nested `critical_section::with(...)` calls are
/// legal on this target, so re-entering while `ring: &mut IbiRing` is live
/// would violate the exclusive-borrow assumption documented below.
fn with_ring<R>(bus: usize, f: impl FnOnce(&mut IbiRing) -> R) -> Option<R> {
    let workq = IBI_RINGS.get(bus)?;
    Some(critical_section::with(|cs| {
        // SAFETY: each ring is reachable only through this helper and wrapped
        // in a `critical_section::Mutex`. While the critical section is held
        // there is no ISR/thread concurrency. The caller-provided closure is
        // also required not to re-enter this module's queue API while the
        // mutable borrow is live, so it is sound to project the
        // `UnsafeCell<IbiRing>` to `&mut IbiRing`.
        let ring: &mut IbiRing = unsafe { &mut *workq.borrow(cs).get() };
        f(ring)
    }))
}

// =============================================================================
// Consumer Handle
// =============================================================================

/// Consumer handle for a bus's IBI work queue.
///
/// Holds no state beyond the bus index; dequeuing reads the shared ring under
/// the critical section. Returned by [`i3c_ibi_workq_consumer`].
pub struct IbiConsumer {
    bus: usize,
}

impl IbiConsumer {
    /// Dequeue the next IBI work item, if any.
    #[must_use]
    pub fn dequeue(&mut self) -> Option<IbiWork> {
        with_ring(self.bus, IbiRing::pop).flatten()
    }
}

/// Get the IBI work queue consumer for a bus.
///
/// Returns `None` if the bus index is out of range.
#[must_use]
pub fn i3c_ibi_workq_consumer(bus: usize) -> Option<IbiConsumer> {
    if bus >= IBI_RINGS.len() {
        return None;
    }
    Some(IbiConsumer { bus })
}

// =============================================================================
// Enqueue Functions
// =============================================================================

/// Enqueue a target dynamic address assignment notification
#[must_use]
pub fn i3c_ibi_work_enqueue_target_da_assignment(bus: usize) -> bool {
    with_ring(bus, |r| r.push(IbiWork::TargetDaAssignment)).unwrap_or(false)
}

/// Enqueue a Hot-Join notification
#[must_use]
pub fn i3c_ibi_work_enqueue_hotjoin(bus: usize) -> bool {
    with_ring(bus, |r| r.push(IbiWork::HotJoin)).unwrap_or(false)
}

/// Enqueue a target interrupt (SIR) notification
#[must_use]
pub fn i3c_ibi_work_enqueue_target_irq(bus: usize, addr: u8, data: &[u8]) -> bool {
    let mut ibi_buf = [0u8; IBI_DATA_MAX as usize];
    let take = core::cmp::min(IBI_DATA_MAX as usize, data.len());
    ibi_buf[..take].copy_from_slice(&data[..take]);
    let work = IbiWork::Sirq {
        addr,
        len: u8::try_from(take).unwrap_or(IBI_DATA_MAX),
        data: ibi_buf,
    };
    with_ring(bus, |r| r.push(work)).unwrap_or(false)
}

/// Enqueue a private write received by this target from the controller.
#[must_use]
pub fn i3c_ibi_work_enqueue_target_master_write(bus: usize, data: &[u8]) -> bool {
    let mut buf = [0u8; IBI_DATA_MAX as usize];
    let take = core::cmp::min(IBI_DATA_MAX as usize, data.len());
    buf[..take].copy_from_slice(&data[..take]);
    let work = IbiWork::TargetMasterWrite {
        len: u8::try_from(take).unwrap_or(IBI_DATA_MAX),
        data: buf,
    };
    with_ring(bus, |r| r.push(work)).unwrap_or(false)
}
