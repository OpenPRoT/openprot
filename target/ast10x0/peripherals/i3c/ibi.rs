// Licensed under the Apache-2.0 license

//! I3C In-Band Interrupt (IBI) Work Queue
//!
//! Handles IBI events including Hot-Join, SIR (Slave Interrupt Request),
//! and target dynamic address assignment.

use core::cell::RefCell;
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
        let mut b = workq.borrow(cs).borrow_mut();
        if b.prod.is_none() || b.cons.is_none() {
            // SAFETY: Each bus's queue is only split once and we're in a critical section.
            // The static mut is only accessed here, protected by the Mutex + critical section.
            let (p, c) = unsafe { IBIQ_BUFS[bus].split() };
            b.prod = Some(p);
            b.cons = Some(c);
        }
    });
    true
}

/// Get the IBI work queue consumer for a bus
///
/// Returns `None` if bus index is out of range or consumer already taken.
#[must_use]
pub fn i3c_ibi_workq_consumer(
    bus: usize,
) -> Option<heapless::spsc::Consumer<'static, IbiWork>> {
    if !ensure_ibiq_split(bus) {
        return None;
    }

    let workq = IBI_WORKQS.get(bus)?;

    critical_section::with(|cs| {
        let mut b = workq.borrow(cs).borrow_mut();
        b.cons.take()
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
