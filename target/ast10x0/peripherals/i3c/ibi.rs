// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C In-Band Interrupt (IBI) Work Queue
//!
//! Handles IBI events including Hot-Join, SIR (Slave Interrupt Request),
//! and target dynamic address assignment.
//!
//! Ported from `aspeed-rust/src/i3c/ibi.rs` @ ce3b567. Two porting deltas:
//! - **D7 (heapless 0.9)**: `spsc::Producer`/`Consumer` lost their capacity
//!   const-generic in 0.9 — they are now `Producer<'static, T>` /
//!   `Consumer<'static, T>` (the reference used `<'static, T, N>` on 0.8).
//! - **edition 2024**: a direct reference to a `static mut` is denied
//!   (`static_mut_refs`); the queue split goes through `addr_of_mut!` instead.
//!
//! The process-global queue/handler design itself is preserved (goal.md ADR-3):
//! an ISR cannot borrow a stack-owned device, so the IBI plane stays global,
//! arbitrated by `critical_section` + the SPSC discipline rather than by `&mut`.

use core::cell::RefCell;
use core::ptr::addr_of_mut;
use critical_section::Mutex;
use heapless::spsc::Queue;

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
}

// =============================================================================
// Static Queue Storage
// =============================================================================

static mut IBIQ_BUFS: [Queue<IbiWork, IBIQ_DEPTH>; 4] =
    [Queue::new(), Queue::new(), Queue::new(), Queue::new()];

struct IbiBus {
    prod: Option<heapless::spsc::Producer<'static, IbiWork>>,
    cons: Option<heapless::spsc::Consumer<'static, IbiWork>>,
}

static IBI_WORKQS: [Mutex<RefCell<IbiBus>>; 4] = [
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
    Mutex::new(RefCell::new(IbiBus {
        prod: None,
        cons: None,
    })),
];

// =============================================================================
// Queue Management
// =============================================================================

/// Ensure the IBI queue for a bus has been split into producer/consumer.
///
/// Returns `false` if bus index is out of range.
fn ensure_ibiq_split(bus: usize) -> bool {
    let Some(workq) = IBI_WORKQS.get(bus) else {
        return false;
    };

    critical_section::with(|cs| {
        let Ok(mut b) = workq.borrow(cs).try_borrow_mut() else {
            return;
        };
        if b.prod.is_none() || b.cons.is_none() {
            // SAFETY: `bus < 4` (checked by `IBI_WORKQS.get(bus)` above). Each
            // bus's queue is split exactly once, inside this critical section,
            // and `IBIQ_BUFS` is reached only here. Going through
            // `addr_of_mut!` (not a direct `&mut IBIQ_BUFS`) satisfies the
            // edition-2024 `static_mut_refs` rule; the Mutex + critical section
            // serialize access so no aliasing `&mut` to the same element exists.
            // `get_mut` (not `[bus]`) keeps the path panic-free for the
            // `no_panics` analysis even though `bus` is in range.
            let bufs: &'static mut [Queue<IbiWork, IBIQ_DEPTH>; 4] =
                unsafe { &mut *addr_of_mut!(IBIQ_BUFS) };
            if let Some(queue) = bufs.get_mut(bus) {
                let (p, c) = queue.split();
                b.prod = Some(p);
                b.cons = Some(c);
            }
        }
    });
    true
}

/// Get the IBI work queue consumer for a bus
///
/// Returns `None` if bus index is out of range or consumer already taken.
#[must_use]
pub fn i3c_ibi_workq_consumer(bus: usize) -> Option<heapless::spsc::Consumer<'static, IbiWork>> {
    if !ensure_ibiq_split(bus) {
        return None;
    }

    let workq = IBI_WORKQS.get(bus)?;

    // `try_borrow_mut` (not `borrow_mut`) keeps the path panic-free for the
    // `no_panics` analysis. Inside this critical section a conflicting borrow
    // is impossible, so the `Err` arm is unreachable in practice.
    critical_section::with(|cs| {
        workq
            .borrow(cs)
            .try_borrow_mut()
            .ok()
            .and_then(|mut b| b.cons.take())
    })
}

// =============================================================================
// Enqueue Functions
// =============================================================================

/// Enqueue a target dynamic address assignment notification
#[must_use]
pub fn i3c_ibi_work_enqueue_target_da_assignment(bus: usize) -> bool {
    if !ensure_ibiq_split(bus) {
        return false;
    }
    critical_section::with(|cs| {
        if let Some(workq) = IBI_WORKQS.get(bus) {
            let mut ibi_bus = workq.borrow(cs).borrow_mut();
            if let Some(prod) = ibi_bus.prod.as_mut() {
                return prod.enqueue(IbiWork::TargetDaAssignment).is_ok();
            }
        }
        false
    })
}

/// Enqueue a Hot-Join notification
#[must_use]
pub fn i3c_ibi_work_enqueue_hotjoin(bus: usize) -> bool {
    if !ensure_ibiq_split(bus) {
        return false;
    }
    critical_section::with(|cs| {
        if let Some(workq) = IBI_WORKQS.get(bus) {
            let mut ibi_bus = workq.borrow(cs).borrow_mut();
            if let Some(prod) = ibi_bus.prod.as_mut() {
                return prod.enqueue(IbiWork::HotJoin).is_ok();
            }
        }
        false
    })
}

/// Enqueue a target interrupt (SIR) notification
#[must_use]
pub fn i3c_ibi_work_enqueue_target_irq(bus: usize, addr: u8, data: &[u8]) -> bool {
    if !ensure_ibiq_split(bus) {
        return false;
    }
    let mut ibi_buf = [0u8; IBI_DATA_MAX as usize];
    let take = core::cmp::min(IBI_DATA_MAX as usize, data.len());
    ibi_buf[..take].copy_from_slice(&data[..take]);
    critical_section::with(|cs| {
        if let Some(workq) = IBI_WORKQS.get(bus) {
            let mut i3c_bus = workq.borrow(cs).borrow_mut();
            if let Some(prod) = i3c_bus.prod.as_mut() {
                return prod
                    .enqueue(IbiWork::Sirq {
                        addr,
                        len: u8::try_from(take).unwrap_or(IBI_DATA_MAX),
                        data: ibi_buf,
                    })
                    .is_ok();
            }
        }
        false
    })
}
