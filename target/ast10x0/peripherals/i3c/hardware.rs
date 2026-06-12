// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C Hardware Interface
//!
//! Defines the hardware abstraction traits and IRQ handling infrastructure.
//!
//! # Trait Hierarchy
//!
//! The hardware interface is split into focused sub-traits:
//!
//! ```text
//! HardwareInterface (supertrait)
//! ├── HardwareCore      - Init, IRQ, enable/disable
//! ├── HardwareClock     - Clock configuration
//! ├── HardwareFifo      - FIFO operations
//! ├── HardwareTransfer  - Transfers, CCC, device management
//! ├── HardwareRecovery  - SW mode, bus recovery
//! └── HardwareTarget    - Target mode operations
//! ```
//!
//! # Platform Initialization
//!
//! SCU operations (clock enable, reset control) are **not** part of these traits.
//! They should be performed by the platform/board layer before creating the
//! I3C controller.

use core::cell::UnsafeCell;
use critical_section::Mutex;

use super::ccc::{ccc_events_set, CccPayload};
use super::config::{I3cConfig, I3C_MIN_CORE_CLK_SDR};
use super::constants::{
    bit, field_get, field_prep, CM_TFR_STS_MASTER_HALT, CM_TFR_STS_TARGET_HALT,
    COMMAND_ATTR_ADDR_ASSGN_CMD, COMMAND_ATTR_SLAVE_DATA_CMD, COMMAND_ATTR_XFER_ARG,
    COMMAND_ATTR_XFER_CMD, COMMAND_PORT_ARG_DATA_LEN, COMMAND_PORT_ARG_DB, COMMAND_PORT_ATTR,
    COMMAND_PORT_CMD, COMMAND_PORT_CP, COMMAND_PORT_DBP, COMMAND_PORT_DEV_COUNT,
    COMMAND_PORT_DEV_INDEX, COMMAND_PORT_READ_TRANSFER, COMMAND_PORT_ROC, COMMAND_PORT_SPEED,
    COMMAND_PORT_TID, COMMAND_PORT_TOC, DEV_ADDR_TABLE_IBI_MDB, DEV_ADDR_TABLE_IBI_PEC,
    DEV_ADDR_TABLE_SIR_REJECT, I3CG_REG1_SCL_IN_SW_MODE_EN, I3CG_REG1_SCL_IN_SW_MODE_VAL,
    I3CG_REG1_SDA_IN_SW_MODE_EN, I3CG_REG1_SDA_IN_SW_MODE_VAL, I3C_AST10X0_MIPI_MANUF_ID,
    I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE, I3C_BUS_FREE_TIMING_RESET, I3C_BUS_I2C_FMP_TF_MAX_NS,
    I3C_BUS_I2C_FMP_THIGH_MIN_NS, I3C_BUS_I2C_FMP_TLOW_MIN_NS, I3C_BUS_I2C_FMP_TR_MAX_NS,
    I3C_BUS_I2C_FM_TF_MAX_NS, I3C_BUS_I2C_FM_THIGH_MIN_NS, I3C_BUS_I2C_FM_TLOW_MIN_NS,
    I3C_BUS_I2C_FM_TR_MAX_NS, I3C_BUS_I2C_STD_TF_MAX_NS, I3C_BUS_I2C_STD_THIGH_MIN_NS,
    I3C_BUS_I2C_STD_TLOW_MIN_NS, I3C_BUS_I2C_STD_TR_MAX_NS, I3C_BUS_THIGH_MAX_NS, I3C_CCC_DEVCTRL,
    I3C_CCC_ENTDAA, I3C_CCC_EVT_INTR, I3C_CCC_SETHID, I3C_CTRL_POLL_DELAY_NS,
    I3C_DEFAULT_STATIC_ADDR, I3C_GLOBAL_RESET_DEASSERT_MASK, I3C_IBI_DATA_THRESHOLD_MAX,
    I3C_INIT_POLL_DELAY_NS, I3C_INTR_STATUS_ALL_BITS, I3C_MSG_READ, I3C_OP_TIMEOUT_US,
    I3C_POLL_MAX_ITERS, IBIQ_STATUS_IBI_DATA_LEN, IBIQ_STATUS_IBI_DATA_LEN_SHIFT,
    IBIQ_STATUS_IBI_ID, IBIQ_STATUS_IBI_ID_SHIFT, INTR_CCC_UPDATED_STAT, INTR_DYN_ADDR_ASSGN_STAT,
    INTR_IBI_THLD_STAT, INTR_RESP_READY_STAT, INTR_TRANSFER_ABORT_STAT, INTR_TRANSFER_ERR_STAT,
    MAX_CMDS, MAX_PRIV_XFER_CMDS, MAX_XFER_DATA_LEN, NSEC_PER_SEC, RESET_CTRL_ALL,
    RESET_CTRL_QUEUES, RESET_CTRL_XFER_QUEUES, RESPONSE_ERROR_IBA_NACK,
    RESPONSE_PORT_DATA_LEN_MASK, RESPONSE_PORT_DATA_LEN_SHIFT, RESPONSE_PORT_ERR_STATUS_MASK,
    RESPONSE_PORT_ERR_STATUS_SHIFT, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT,
    SDA_TX_HOLD_MASK, SDA_TX_HOLD_MAX, SDA_TX_HOLD_MIN, SLV_DCR_MASK, SLV_EVENT_CTRL_SIR_EN,
};
use super::error::I3cError as I3cDrvError;
use super::error::I3cError;
use super::ibi as ibi_workq;
use super::types::{Completion, I3cCmd, I3cIbi, I3cMsg, I3cXfer, SpeedI3c, Tid};

use super::registers::I3cRegisters;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// =============================================================================
// IRQ Handler Infrastructure
// =============================================================================
//
// The ISR fabricates NO `&mut` over any thread-owned object. It works only
// with:
//   - its own `I3cRegisters` handle (all methods take `&self`), stored in the
//     per-bus registry below;
//   - the per-bus `ISR_EVENTS` atomics;
//   - the global IBI work rings (`ibi.rs`).
// Master transfer completion is flag-and-defer (the ISR masks the sources and
// latches the status; the polling thread drains the response queue itself),
// so there is no ISR/thread `&mut` aliasing and no transfer-pointer handoff.

/// Everything the ISR needs for one bus. Built by
/// [`Ast1060I3c::isr_ctx`] and parked in the per-bus registry at
/// [`I3cController::start`](super::controller::I3cController::start).
pub struct IsrCtx {
    /// ISR-side register handle. A second `I3cRegisters` for the same bus as
    /// the driver's own — sound for MMIO (no Rust memory is aliased), and
    /// device-access serialization holds because the single-core ISR runs
    /// atomically with respect to the thread.
    regs: I3cRegisters,
    /// Role selected at `start()` (the ISR must not read the thread-owned
    /// config).
    is_secondary: bool,
}

// SAFETY: `IsrCtx` holds raw MMIO pointers (valid from any execution context
// on this single-address-space target) and a bool; parking it in the
// critical-section-guarded registry below is sound.
unsafe impl Send for IsrCtx {}

// INTENTIONAL EXCEPTION to borrow-arbitrated exclusivity: this per-bus
// dispatch table is process-global mutable state, because an ISR cannot
// borrow a stack-owned controller — the global registry is the structural
// price of IRQ dispatch (same rationale as `IBI_RINGS` in `ibi.rs`, ADR-3).
// It is bounded (one slot per bus, claimed once via the single-shot
// `register_i3c_irq_handler`) and serialized by the critical section.
//
// `UnsafeCell` (not `RefCell`): mutual exclusion comes from the critical
// section, and the access helpers below are leaf functions (no caller code
// runs while the reference is live), so the `RefCell` runtime borrow flag
// would only add a reachable panic path that the `no_panics` analysis must
// reject.
static BUS_ISR: [Mutex<UnsafeCell<Option<IsrCtx>>>; 4] = [
    Mutex::new(UnsafeCell::new(None)),
    Mutex::new(UnsafeCell::new(None)),
    Mutex::new(UnsafeCell::new(None)),
    Mutex::new(UnsafeCell::new(None)),
];

/// Per-bus ISR↔thread signal block: plain atomics, written by the ISR and
/// consumed by the polling thread. Part of the same intentional global-state
/// exception as the registry above.
pub(crate) struct IsrEvents {
    /// Latched interrupt-status bits deferred to the thread (master
    /// completion path).
    pending: AtomicU32,
    /// Dynamic address assigned by the bus master; bit 8 = valid.
    dyn_addr: AtomicU32,
    /// A deferred fault: the ISR observed a halted/errored engine and left
    /// recovery (halt/resume sequencing needs the wait policy) to the thread.
    fault: AtomicBool,
    /// Target mode: the SIR (IBI) command completed.
    pub(crate) target_ibi_done: Completion,
    /// Target mode: the pending-read data was fetched by the master.
    pub(crate) target_data_done: Completion,
}

impl IsrEvents {
    const fn new() -> Self {
        Self {
            pending: AtomicU32::new(0),
            dyn_addr: AtomicU32::new(0),
            fault: AtomicBool::new(false),
            target_ibi_done: Completion::new(),
            target_data_done: Completion::new(),
        }
    }

    /// Atomically take (read-and-clear) the latched status bits.
    pub(crate) fn take_pending(&self) -> u32 {
        self.pending.swap(0, Ordering::AcqRel)
    }

    /// Atomically take (read-and-clear) the deferred-fault flag.
    pub(crate) fn take_fault(&self) -> bool {
        self.fault.swap(false, Ordering::AcqRel)
    }

    /// Dynamic address assigned by the master, if any.
    pub(crate) fn dyn_addr(&self) -> Option<u8> {
        let v = self.dyn_addr.load(Ordering::Acquire);
        if v & 0x100 != 0 {
            Some((v & 0x7f) as u8)
        } else {
            None
        }
    }
}

static ISR_EVENTS: [IsrEvents; 4] = [
    IsrEvents::new(),
    IsrEvents::new(),
    IsrEvents::new(),
    IsrEvents::new(),
];

/// Per-bus ISR event block (clamps an out-of-range bus to 0, which a
/// constructed driver can never pass).
#[inline]
pub(crate) fn isr_events(bus: usize) -> &'static IsrEvents {
    ISR_EVENTS.get(bus).unwrap_or(&ISR_EVENTS[0])
}

/// Register the ISR context for an I3C bus.
///
/// Single-shot per bus: the first registration claims the slot, mirroring the
/// one-controller-per-physical-bus contract of [`Ast1060I3c::new`]. Returns
/// `false` (and leaves the existing context in place) if `bus` is out of range
/// or the slot is already claimed.
///
/// Once claimed the slot is normally held for the program's lifetime; the only
/// release is the bring-up failure path via [`unregister_i3c_irq_handler`], so
/// a subsequent registration can succeed after a failed `start()`.
#[must_use]
pub fn register_i3c_irq_handler(bus: usize, ctx: IsrCtx) -> bool {
    let Some(slot) = BUS_ISR.get(bus) else {
        return false;
    };
    critical_section::with(|cs| {
        // SAFETY: the critical section excludes ISR/thread concurrency, and
        // the `&mut` never escapes this leaf function, so this is the only
        // live reference to the slot.
        let parked: &mut Option<IsrCtx> = unsafe { &mut *slot.borrow(cs).get() };
        if parked.is_some() {
            return false;
        }
        *parked = Some(ctx);
        true
    })
}

/// Release a bus's ISR slot.
///
/// Only for the bring-up failure path: `I3cController::start` claims the slot
/// *before* programming the hardware (the claim is the exclusivity gate), so
/// if `init` then fails the claim must be released or every retry would see
/// [`I3cError::Busy`](super::error::I3cError::Busy) forever.
///
/// # Caution
///
/// This clears the slot by bus alone — it does *not* verify which controller
/// owns the parked context. Call it only from the failure path of the same
/// `start()` that claimed the slot; calling it once a controller is live would
/// drop the in-use ISR context. Kept `pub(crate)` for that reason.
pub(crate) fn unregister_i3c_irq_handler(bus: usize) {
    let Some(slot) = BUS_ISR.get(bus) else {
        return;
    };
    critical_section::with(|cs| {
        // SAFETY: same argument as `register_i3c_irq_handler` — the critical
        // section excludes ISR/thread concurrency and the `&mut` never
        // escapes this leaf function.
        let parked: &mut Option<IsrCtx> = unsafe { &mut *slot.borrow(cs).get() };
        *parked = None;
    });
}

// NVIC ownership (Delta D6): the driver does not touch the NVIC and exposes
// no interrupt-line mapping. The kernel/integration layer owns the top-level
// vector *and* the line mask; it selects the bus, so it also knows the line
// (the platform interrupt line) to unmask after `start()` and to mask on
// teardown. This keeps the driver above the register facade entirely free of
// PAC types.

/// Dispatch IRQ for a specific bus.
///
/// Called by the actual IRQ entry points (defined in the kernel integration
/// layer). Runs the service routine inside the critical section: we are
/// already in interrupt context, so no thread can be blocked by it, and the
/// section guarantees the registry slot is not concurrently replaced.
#[inline]
pub fn dispatch_i3c_irq(bus: usize) {
    critical_section::with(|cs| {
        let Some(slot) = BUS_ISR.get(bus) else {
            return;
        };
        // SAFETY: the critical section excludes the writer
        // (`register_i3c_irq_handler`); the shared reference never escapes
        // this leaf closure.
        let parked: &Option<IsrCtx> = unsafe { &*slot.borrow(cs).get() };
        if let Some(ctx) = parked {
            isr_service(ctx);
        }
    });
}

// =============================================================================
// ISR service routines — `&self`/atomics only, no `&mut` anywhere
// =============================================================================

/// Top-level I3C interrupt service. Touches only the ISR-side register
/// handle, the per-bus atomics, and the global IBI rings.
fn isr_service(ctx: &IsrCtx) {
    let regs = &ctx.regs;
    let status = regs.read_intr_status();
    if status == 0 {
        return;
    }
    let bus = regs.bus() as usize;
    let events = isr_events(bus);

    if ctx.is_secondary {
        if status & INTR_DYN_ADDR_ASSGN_STAT != 0 {
            let da = u32::from(regs.dynamic_addr());
            events.dyn_addr.store(0x100 | da, Ordering::Release);
            let _ = ibi_workq::i3c_ibi_work_enqueue_target_da_assignment(bus);
        }

        if (status & INTR_RESP_READY_STAT) != 0 {
            isr_target_responses(regs, events, bus);
        }

        if (status & INTR_CCC_UPDATED_STAT) != 0 {
            // Read-and-clear the event; if the engine halted, defer the
            // resume sequencing (it needs the wait policy) to the thread.
            let event = regs.read_slv_event_ctrl();
            regs.write_slv_event_ctrl(event);
            if regs.xfer_status() == CM_TFR_STS_TARGET_HALT {
                events.fault.store(true, Ordering::Release);
            }
        }
    } else {
        if (status & (INTR_RESP_READY_STAT | INTR_TRANSFER_ERR_STAT | INTR_TRANSFER_ABORT_STAT))
            != 0
        {
            // Flag-and-defer (the SMC model): mask the sources so the level
            // status cannot refire, latch the bits; the polling thread drains
            // the response queue itself and re-enables the sources.
            regs.mask_master_xfer_irqs();
            events.pending.fetch_or(
                status & (INTR_RESP_READY_STAT | INTR_TRANSFER_ERR_STAT | INTR_TRANSFER_ABORT_STAT),
                Ordering::AcqRel,
            );
        }

        if (status & INTR_IBI_THLD_STAT) != 0 {
            isr_master_ibis(regs, bus);
        }
    }

    regs.clear_intr_status(status);
}

/// Target mode: service the response queue from the ISR (master writes and
/// SIR/read completions arrive whether or not a thread is waiting, and the
/// hardware queues are shallow — this is the IBI plane that cannot defer).
fn isr_target_responses(regs: &I3cRegisters, events: &IsrEvents, bus: usize) {
    let nresp = regs.resp_buf_level();

    for _ in 0..nresp {
        let resp = regs.pop_response();

        let tid = field_get(resp, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT) as usize;
        let rx_len = field_get(
            resp,
            RESPONSE_PORT_DATA_LEN_MASK,
            RESPONSE_PORT_DATA_LEN_SHIFT,
        ) as usize;
        let err = field_get(
            resp,
            RESPONSE_PORT_ERR_STATUS_MASK,
            RESPONSE_PORT_ERR_STATUS_SHIFT,
        );

        if err != 0 {
            // Recovery needs halt/resume sequencing (wait policy) — defer.
            // The errored response's data must still be drained here: the
            // deferred recovery may run much later (next SIR attempt), and
            // leftover words would misalign every subsequent RX FIFO read.
            events.fault.store(true, Ordering::Release);
            if rx_len != 0 {
                regs.rx_fifo_drain(rx_len);
            }
            continue;
        }

        if rx_len != 0 {
            // Bounce buffer sized to the work-item payload (NOT 256): this
            // runs on the kernel handler stack, and together with the
            // by-value `IbiWork` copies in the enqueue path a larger buffer
            // HardFaulted the AST1060 ISR stack. Anything beyond the
            // work-item capacity would be truncated at enqueue anyway; the
            // drain below keeps the FIFO aligned for the excess.
            let mut buf = [0u8; ibi_workq::IBI_MWR_DATA_MAX];
            // Bound `rx_len` (a raw hardware field) to the buffer via `get`:
            // an oversized length must not panic in handler mode.
            let n = rx_len.min(buf.len());
            if let Some(dst) = buf.get_mut(..n) {
                regs.rx_fifo_read(dst);
            }
            // An oversized write leaves words beyond the bounce buffer in the
            // RX FIFO; pop them so the next response's data stays aligned
            // (`n` is word-aligned at 256, so the byte count maps 1:1).
            if rx_len > n {
                regs.rx_fifo_drain(rx_len - n);
            }
            let _ = ibi_workq::i3c_ibi_work_enqueue_target_master_write(
                bus,
                buf.get(..n).unwrap_or(&[]),
            );
        }

        if tid == Tid::TargetIbi as usize {
            events.target_ibi_done.complete();
        }

        if tid == Tid::TargetRdData as usize {
            events.target_data_done.complete();
        }
    }
}

/// Master mode: drain the IBI status queue into the global work rings.
///
/// Porting delta: the reference validated the SIR address against the
/// attached-device table here; that table is thread-owned, so the check moves
/// to the consumer (`acknowledge_ibi` already validates before acting).
fn isr_master_ibis(regs: &I3cRegisters, bus: usize) {
    let nibis = regs.ibi_status_count();
    if nibis == 0 {
        return;
    }

    for _ in 0..nibis {
        let reg = regs.ibi_fifo_pop();

        let ibi_id = field_get(reg, IBIQ_STATUS_IBI_ID, IBIQ_STATUS_IBI_ID_SHIFT);
        let ibi_data_len = field_get(
            reg,
            IBIQ_STATUS_IBI_DATA_LEN,
            IBIQ_STATUS_IBI_DATA_LEN_SHIFT,
        ) as usize;
        let ibi_addr = (ibi_id >> 1) & 0x7F;
        let rnw = (ibi_id & 1) != 0;

        if ibi_addr != 2 && rnw {
            // SIR
            let mut ibi_buf: [u8; 2] = [0u8; 2];
            let take = core::cmp::min(ibi_data_len, ibi_buf.len());
            if let Some(dst) = ibi_buf.get_mut(..take) {
                regs.ibi_fifo_read(dst);
            }
            // The read above consumed `take` rounded up to a whole queue word;
            // a payload longer than the bounce buffer leaves further words in
            // the IBI queue, where they would be misparsed as the next entry's
            // status word. Pop the remainder to keep the queue aligned.
            let consumed = take.div_ceil(4) * 4;
            if ibi_data_len > consumed {
                regs.ibi_fifo_drain(ibi_data_len - consumed);
            }
            let _ = ibi_workq::i3c_ibi_work_enqueue_target_irq(
                bus,
                ibi_addr as u8,
                ibi_buf.get(..take).unwrap_or(&[]),
            );
        } else if ibi_addr == 2 && !rnw {
            // hot-join
            let _ = ibi_workq::i3c_ibi_work_enqueue_hotjoin(bus);
        } else {
            // normal ibi
            regs.ibi_fifo_drain(ibi_data_len);
        }
    }
}

// =============================================================================
// Sub-trait: Core Operations
// =============================================================================

/// Core hardware operations: init, IRQ, enable/disable
pub trait HardwareCore {
    /// Initialize the I3C controller hardware.
    /// `Err(I3cError::Timeout)` if the initial queue-reset poll timed out.
    fn init(&mut self, config: &mut I3cConfig) -> Result<(), I3cError>;

    /// Get the bus number for this instance
    fn bus_num(&self) -> u8;

    /// Enable the I3C controller
    fn i3c_enable(&mut self, config: &I3cConfig);

    /// Disable the I3C controller
    fn i3c_disable(&mut self, is_secondary: bool);

    /// Set the controller role (primary/secondary)
    fn set_role(&mut self, is_secondary: bool);

    /// Build the ISR context to park in the per-bus registry
    /// (see [`register_i3c_irq_handler`]).
    fn isr_ctx(&self, is_secondary: bool) -> IsrCtx;
}

// =============================================================================
// Sub-trait: Clock Configuration
// =============================================================================

/// Clock and timing configuration
pub trait HardwareClock {
    /// Initialize clock timing parameters
    ///
    /// Implementations should use `config.core_clk_hz` if set, falling back
    /// to [`get_clock_rate()`](Self::get_clock_rate) for auto-detection.
    fn init_clock(&mut self, config: &mut I3cConfig);

    /// Calculate I2C clock dividers for given SCL frequency
    fn calc_i2c_clk(&mut self, fscl_hz: u32) -> (u32, u32);

    /// Initialize the PID (Provisional ID) for this controller
    fn init_pid(&mut self, config: &mut I3cConfig);
}

// =============================================================================
// Sub-trait: FIFO Operations
// =============================================================================

/// FIFO read/write operations
pub trait HardwareFifo {
    /// Write to TX FIFO
    fn wr_tx_fifo(&mut self, bytes: &[u8]);

    /// Read `out.len()` bytes from the RX FIFO
    fn rd_rx_fifo(&mut self, out: &mut [u8]);

    /// Read `out.len()` bytes from the IBI FIFO
    fn rd_ibi_fifo(&mut self, out: &mut [u8]);
}

// =============================================================================
// Sub-trait: Transfer Operations
// =============================================================================

/// Transfer, CCC, and device management operations
pub trait HardwareTransfer {
    /// Set the IBI Mandatory Data Byte
    fn set_ibi_mdb(&mut self, mdb: u8);

    /// Exit halt state. `Err(I3cError::Timeout)` if the engine did not leave
    /// the halt state within the poll budget.
    fn exit_halt(&mut self, config: &mut I3cConfig) -> Result<(), I3cError>;

    /// Enter halt state. `Err(I3cError::Timeout)` if the engine did not reach
    /// the halt state within the poll budget.
    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig) -> Result<(), I3cError>;

    /// Reset controller components (FIFOs, queues, etc.).
    /// `Err(I3cError::Timeout)` if the reset bits did not self-clear within
    /// the poll budget.
    fn reset_ctrl(&mut self, reset: u32) -> Result<(), I3cError>;

    /// Enable IBI for a device
    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cError>;

    /// Disable IBI for a device (DISEC + reject its SIRs at the controller)
    fn ibi_disable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cError>;

    /// Start a transfer. Overlap is structurally impossible: the transfer is
    /// thread-owned for its whole life (`&mut` exclusivity), and the ISR only
    /// latches completion flags — there is no in-flight pointer to clobber.
    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer);

    /// Wait (bounded, cooperative-yield) for the transfer the ISR flagged,
    /// then drain the response queue into `xfer` on the thread side.
    /// Returns `false` on timeout (after halt/reset recovery).
    fn wait_xfer_complete(
        &mut self,
        config: &mut I3cConfig,
        xfer: &mut I3cXfer,
        timeout_us: u32,
    ) -> bool;

    /// Detach a device by DAT position
    fn detach_i3c_dev(&mut self, pos: usize);

    /// Attach a device to a DAT position
    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cError>;

    /// Execute a CCC
    fn do_ccc(&mut self, config: &mut I3cConfig, ccc: &mut CccPayload) -> Result<(), I3cError>;

    /// Execute ENTDAA (Enter Dynamic Address Assignment)
    fn do_entdaa(&mut self, config: &mut I3cConfig, index: u32) -> Result<(), I3cError>;

    /// Build commands for private transfer.
    ///
    /// **Consumes each message's buffer.** On success every `msgs[i].buf` is
    /// moved into the corresponding command and left `None`; the caller must
    /// re-fill the descriptor (`buf`) before reusing the same `msgs` slice for
    /// another transfer. On error no buffer is taken (all-or-nothing): the
    /// whole `msgs` slice is left untouched. The underlying caller-owned
    /// memory is never modified — only the descriptor's `Option` is cleared.
    fn priv_xfer_build_cmds<'a>(
        &mut self,
        cmds: &mut [I3cCmd<'a>],
        msgs: &mut [I3cMsg<'a>],
        pos: u8,
    ) -> Result<(), I3cError>;

    /// Execute a private transfer.
    ///
    /// **Consumes each message's buffer** (see [`priv_xfer_build_cmds`]): once
    /// the command build succeeds every `msgs[i].buf` is left `None` and stays
    /// `None` for the rest of the call, *including the error paths* below
    /// (timeout / non-zero response status). Only a failure in the build step
    /// itself leaves the slice untouched (the build is all-or-nothing). The
    /// buffers are not restored on a transfer error — the TX side downgrades
    /// the caller's `&mut` to a shared borrow during build and cannot be put
    /// back — so a caller retrying the same descriptors must re-fill `buf`
    /// first regardless of how the previous call returned.
    ///
    /// [`priv_xfer_build_cmds`]: HardwareTransfer::priv_xfer_build_cmds
    fn priv_xfer(
        &mut self,
        config: &mut I3cConfig,
        pid: u64,
        msgs: &mut [I3cMsg],
    ) -> Result<(), I3cError>;
}

// =============================================================================
// Sub-trait: Recovery / Software Mode
// =============================================================================

/// Software mode and bus recovery operations
pub trait HardwareRecovery {
    /// Enter software mode for manual bus control
    fn enter_sw_mode(&mut self);

    /// Exit software mode
    fn exit_sw_mode(&mut self);

    /// Toggle SCL line in software mode
    fn i3c_toggle_scl_in(&mut self, count: u32);

    /// Generate an internal STOP condition
    fn gen_internal_stop(&mut self);

    /// Calculate even parity for a byte
    fn even_parity(byte: u8) -> bool;
}

// =============================================================================
// Sub-trait: Target Mode Operations
// =============================================================================

/// Target (secondary) mode operations
pub trait HardwareTarget {
    /// Write data to target TX buffer
    fn target_tx_write(&mut self, buf: &[u8]);

    /// Raise a Hot-Join IBI (target mode)
    fn target_ibi_raise_hj(&self, config: &mut I3cConfig) -> Result<(), I3cError>;

    /// Notify pending read in target mode
    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cError>;
}

// =============================================================================
// Supertrait: Full Hardware Interface
// =============================================================================

/// Complete hardware abstraction for I3C controllers
///
/// This is a supertrait combining all sub-traits. Implementors must provide
/// all operations.
///
/// # Sub-traits
///
/// - [`HardwareCore`] - Init, IRQ, enable/disable
/// - [`HardwareClock`] - Clock configuration
/// - [`HardwareFifo`] - FIFO operations
/// - [`HardwareTransfer`] - Transfers, CCC, device management
/// - [`HardwareRecovery`] - SW mode, bus recovery
/// - [`HardwareTarget`] - Target mode operations
pub trait HardwareInterface:
    HardwareCore + HardwareClock + HardwareFifo + HardwareTransfer + HardwareRecovery + HardwareTarget
{
}

// Blanket implementation: any type implementing all sub-traits implements HardwareInterface
impl<T> HardwareInterface for T where
    T: HardwareCore
        + HardwareClock
        + HardwareFifo
        + HardwareTransfer
        + HardwareRecovery
        + HardwareTarget
{
}
/// I3C bus 0 interrupt handler - call this from your ISR
#[inline]
pub fn i3c_irq_handler() {
    dispatch_i3c_irq(0);
}

/// I3C bus 1 interrupt handler - call this from your ISR
#[inline]
pub fn i3c1_irq_handler() {
    dispatch_i3c_irq(1);
}

/// I3C bus 2 interrupt handler - call this from your ISR
#[inline]
pub fn i3c2_irq_handler() {
    dispatch_i3c_irq(2);
}

/// I3C bus 3 interrupt handler - call this from your ISR
#[inline]
pub fn i3c3_irq_handler() {
    dispatch_i3c_irq(3);
}

// Delta D6: the reference's `#[cfg(feature = "isr-handlers")] #[no_mangle]
// extern "C" fn i3c{,1,2,3}()` symbol exports are dropped here. openprot is the
// kernel-integration target: the kernel owns the interrupt vector and calls
// `dispatch_i3c_irq(bus)` (via the `i3c*_irq_handler` helpers above), which is
// exactly the case the reference gated those exports OFF for. Carrying a
// never-enabled `isr-handlers` feature would only risk a symbol clash with the
// kernel ISR and an `unexpected_cfgs` lint, with no observable difference in
// the deployed (feature-off) build.

/// Concrete AST1060 I3C hardware implementation: the per-bus
/// [`I3cRegisters`] façade (Delta D3 — all MMIO `unsafe` confined there)
/// plus a Cooperative-Yield wait policy (Delta D2).
///
/// One driver type manages any of the bus instances — the bus is selected at
/// **runtime** in [`new`](Self::new), so several controllers (one per bus)
/// share this single type. `Y` is the caller-injected yield closure invoked
/// between completion polls (see [`super::types::Completion::wait_for_us`]);
/// pass `|_| core::hint::spin_loop()` for a bare-metal busy-wait.
///
/// Not `Copy`/`Clone`: this value owns the (also non-`Copy`) registers
/// wrapper, so bus exclusivity follows from ownership.
pub struct Ast1060I3c<Y: FnMut(u32)> {
    regs: I3cRegisters,
    /// Cooperative yield hook invoked between status polls. Argument is the
    /// suggested wait window in nanoseconds (advisory). Private so external
    /// code cannot swap the wait policy out from under an active driver.
    yield_fn: Y,
}

impl<Y: FnMut(u32)> Ast1060I3c<Y> {
    /// Create the I3C hardware driver for `bus` (0..=3). Returns `None` if
    /// `bus` is out of range.
    ///
    /// # Safety
    ///
    /// Delegates the [`I3cRegisters::new`] contract — the entire MMIO
    /// `unsafe` perimeter:
    /// - the AST1060 PAC singleton pointers are valid for the program's
    ///   lifetime (they are on AST1060 hardware);
    /// - access to the returned instance is serialized by the caller (the
    ///   device is `!Sync`); only one `Ast1060I3c` per physical bus may be
    ///   active at a time.
    pub unsafe fn new(bus: u8, yield_fn: Y) -> Option<Self> {
        // SAFETY: forwarded — see this function's contract above.
        let regs = unsafe { I3cRegisters::new(bus) }?;
        Some(Self { regs, yield_fn })
    }

    /// Bus index this driver was constructed for.
    #[inline]
    fn bus(&self) -> u8 {
        self.regs.bus()
    }
}

/// Debug logging is dropped in the openprot port (Delta D4): the reference's
/// `Logger`/`heapless::String` path is removed. This no-op still evaluates the
/// format arguments (via `format_args!`) so the surrounding `let reg = …`
/// bindings stay "used", but performs no formatting or I/O. The leading
/// `$logger` fragment is captured and ignored (never expanded), so the absent
/// `logger` field is never referenced.
macro_rules! i3c_debug {
    ($logger:expr, $($arg:tt)*) => {{
        let _ = format_args!($($arg)*);
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollError {
    Timeout,
}

/// Bounded poll loop (Cooperative-Yield Bounded-Poll Device, Delta D2).
///
/// The reference took a `&mut D: DelayNs`; here the wait policy is the
/// caller-injected, type-erased `yield_fn`, invoked once per non-completing
/// poll with an advisory wait window (`delay_ns`). Exhausting `max_iters`
/// returns a typed [`PollError::Timeout`] — never an unbounded spin.
pub fn poll_with_timeout<F, C>(
    mut read_reg: F,
    mut condition: C,
    yield_fn: &mut dyn FnMut(u32),
    delay_ns: u32,
    max_iters: u32,
) -> Result<u32, PollError>
where
    F: FnMut() -> u32,
    C: FnMut(u32) -> bool,
{
    for _ in 0..max_iters {
        let val = read_reg();
        if condition(val) {
            return Ok(val);
        }
        yield_fn(delay_ns);
    }
    Err(PollError::Timeout)
}

impl<Y: FnMut(u32)> Ast1060I3c<Y> {
    fn toggle_scl_in(&mut self, count: u32) {
        for _ in 0..count {
            self.regs.i3cg_reg1_clear_bits(I3CG_REG1_SCL_IN_SW_MODE_VAL);
            self.regs.i3cg_reg1_set_bits(I3CG_REG1_SCL_IN_SW_MODE_VAL);
        }
    }

    fn gen_internal_stop(&mut self) {
        self.regs.i3cg_reg1_clear_bits(I3CG_REG1_SCL_IN_SW_MODE_VAL);
        self.regs.i3cg_reg1_clear_bits(I3CG_REG1_SDA_IN_SW_MODE_VAL);
        self.regs.i3cg_reg1_set_bits(I3CG_REG1_SCL_IN_SW_MODE_VAL);
        self.regs.i3cg_reg1_set_bits(I3CG_REG1_SDA_IN_SW_MODE_VAL);
    }

    fn enter_sw_mode(&mut self) {
        i3c_debug!(self.logger, "enter sw mode");
        let mut reg = self.regs.i3cg_read_reg1();
        reg |= I3CG_REG1_SCL_IN_SW_MODE_VAL | I3CG_REG1_SDA_IN_SW_MODE_VAL;
        self.regs.i3cg_reg1_overwrite(reg);
        reg |= I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN;
        self.regs.i3cg_reg1_overwrite(reg);
    }

    fn exit_sw_mode(&mut self) {
        let mut reg = self.regs.i3cg_read_reg1();
        reg &= !(I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN);
        self.regs.i3cg_reg1_overwrite(reg);
    }

    /// Thread-side response drain — the old ISR `end_xfer`, minus the
    /// transfer-pointer handoff: the ISR only latched completion (see
    /// [`isr_service`]), so this runs with the thread's own `&mut xfer` and
    /// no `unsafe`.
    fn process_responses(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer) {
        let nresp = self.regs.resp_buf_level();

        for _ in 0..nresp {
            let resp = self.regs.pop_response();

            let tid = field_get(resp, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT) as usize;
            let rx_len = field_get(
                resp,
                RESPONSE_PORT_DATA_LEN_MASK,
                RESPONSE_PORT_DATA_LEN_SHIFT,
            ) as usize;
            let err = field_get(
                resp,
                RESPONSE_PORT_ERR_STATUS_MASK,
                RESPONSE_PORT_ERR_STATUS_SHIFT,
            );

            i3c_debug!(
                self.logger,
                "process_responses: tid={}, rx_len={}, err={}",
                tid,
                rx_len,
                err
            );
            if tid >= xfer.cmds.len() {
                if rx_len > 0 {
                    self.regs.rx_fifo_drain(rx_len);
                }
                continue;
            }

            // `get_mut` (not `[tid]`) keeps the scatter path panic-free for the
            // `no_panics` analysis; `tid < len` is already guaranteed above.
            let Some(cmd) = xfer.cmds.get_mut(tid) else {
                continue;
            };
            cmd.rx_len = u32::try_from(rx_len).unwrap_or(0);
            cmd.ret = i32::try_from(err).unwrap_or(-1);

            if rx_len == 0 {
                continue;
            }

            if err == 0 {
                // `get_mut(..rx_len)` guards a malformed hardware length that
                // would otherwise panic on `rx_buf[..rx_len]`; on mismatch the
                // bytes are drained instead.
                if let Some(dst) = cmd.rx.as_deref_mut().and_then(|b| b.get_mut(..rx_len)) {
                    self.regs.rx_fifo_read(dst);
                } else {
                    self.regs.rx_fifo_drain(rx_len);
                }
            } else if rx_len > 0 {
                self.regs.rx_fifo_drain(rx_len);
            }
        }

        let mut ret = 0;
        for i in 0..nresp {
            if let Some(c) = xfer.cmds.get(i)
                && c.ret != 0
            {
                ret = c.ret;
            }
        }

        if ret != 0 {
            // Best-effort recovery; the transfer error is already being
            // reported via `xfer.ret`, so a recovery timeout on top of it has
            // no separate observable outcome. `RESET_CTRL_XFER_QUEUES` (not
            // `RESET_CTRL_QUEUES`) follows the vendor C driver
            // (`aspeed_i3c_end_xfer`): this is the master completion path, and
            // resetting the IBI queue here would silently drop IBIs that
            // arrived during the failed transfer.
            let _ = self.enter_halt(false, config);
            let _ = self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            let _ = self.exit_halt(config);
        }

        xfer.ret = ret;
    }
}

impl<Y: FnMut(u32)> HardwareCore for Ast1060I3c<Y> {
    fn init(&mut self, config: &mut I3cConfig) -> Result<(), I3cError> {
        i3c_debug!(self.logger, "i3c init");

        self.regs
            .global_reset_deassert(I3C_GLOBAL_RESET_DEASSERT_MASK);

        self.regs.i3cg_program_reg1(I3C_DEFAULT_STATIC_ADDR);
        let reg = self.regs.i3cg_read_reg1();
        i3c_debug!(self.logger, "i3cg_reg1: {:#x}", reg);

        self.regs.i3cg_write_reg0(0x0);
        let reg = self.regs.i3cg_read_reg0();
        i3c_debug!(self.logger, "i3cg_reg0: {:#x}", reg);

        self.regs.core_reset_assert();
        self.regs.clock_on();
        self.regs.core_reset_deassert();
        self.i3c_disable(config.is_secondary);

        i3c_debug!(
            self.logger,
            "bus num: {}, is_secondary: {}",
            self.bus(),
            config.is_secondary
        );

        self.regs.assert_all_queue_resets();

        let regs = &self.regs;
        poll_with_timeout(
            || regs.read_reset_ctrl(),
            |val| val == 0,
            &mut self.yield_fn,
            I3C_INIT_POLL_DELAY_NS,
            I3C_POLL_MAX_ITERS,
        )
        .map_err(|_| I3cError::Timeout)?;

        self.set_role(config.is_secondary);
        self.init_clock(config);

        self.regs.clear_intr_status(I3C_INTR_STATUS_ALL_BITS);
        if config.is_secondary {
            self.regs.enable_target_irqs();
        } else {
            self.regs.enable_master_irqs();
        }

        config.sir_allowed_by_sw = false;

        self.regs.set_ibi_data_threshold(I3C_IBI_DATA_THRESHOLD_MAX);
        self.regs.set_rx_buf_threshold(0);

        self.init_pid(config);

        config.maxdevs = self.regs.dat_depth();
        config.free_pos = if config.maxdevs == 32 {
            u32::MAX
        } else {
            (1u32 << config.maxdevs) - 1
        };
        config.need_da = 0;

        for i in 0..(config.maxdevs) {
            self.regs.dat_set_reject(i.into());
        }

        self.regs.write_mr_reject(I3C_INTR_STATUS_ALL_BITS);
        self.regs.write_sir_reject(I3C_INTR_STATUS_ALL_BITS);
        self.regs.set_hot_join_nack(true);

        if config.is_secondary {
            self.regs.program_secondary_static_addr(9);
        } else {
            self.regs.program_primary_dynamic_addr(8);
        }

        self.i3c_enable(config);

        i3c_debug!(self.logger, "i3c enabled");
        if !config.is_secondary {
            self.regs.enable_ibi_thld_irq();
        }
        self.regs.set_hot_join_nack(false);
        i3c_debug!(self.logger, "i3c init done");

        // Safety: Ensure memory barrier and init completion before interrupts are enabled by the caller
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        Ok(())
    }

    fn bus_num(&self) -> u8 {
        self.bus()
    }

    fn i3c_disable(&mut self, is_secondary: bool) {
        i3c_debug!(self.logger, "i3c disable");
        if !self.regs.controller_enabled() {
            return;
        }

        if is_secondary {
            self.enter_sw_mode();
        }
        self.regs.disable_controller();

        if is_secondary {
            self.toggle_scl_in(8);
            self.gen_internal_stop();
            self.exit_sw_mode();
        }
    }

    fn i3c_enable(&mut self, config: &I3cConfig) {
        i3c_debug!(self.logger, "i3c enable");
        if config.is_secondary {
            i3c_debug!(self.logger, "i3c enable as secondary");
            self.regs.write_slv_event_ctrl(0);
            self.enter_sw_mode();
            self.regs.enable_controller_secondary();
            let wait_cnt = self.regs.ibi_free_cycles();
            let wait_ns = wait_cnt * config.core_period;
            (self.yield_fn)(wait_ns * 100_u32);
            self.toggle_scl_in(8);
            if self.regs.controller_enabled() {
                self.gen_internal_stop();
            }
            self.exit_sw_mode();
        } else {
            self.regs.enable_controller_primary();
        }
    }

    fn set_role(&mut self, is_secondary: bool) {
        self.regs.set_dev_op_mode(u8::from(is_secondary));
    }

    fn isr_ctx(&self, is_secondary: bool) -> IsrCtx {
        // SAFETY: the ISR runs atomically with respect to the thread on this
        // single-core target, so device access through the alias stays
        // serialized (see `I3cRegisters::isr_alias`).
        let regs = unsafe { self.regs.isr_alias() };
        IsrCtx { regs, is_secondary }
    }
}

impl<Y: FnMut(u32)> HardwareClock for Ast1060I3c<Y> {
    fn init_clock(&mut self, config: &mut I3cConfig) {
        // `unwrap_or` + `.max(1)` (not `.expect()` / raw divides) keep this
        // panic-free for the `no_panics` analysis: a missing/zero core clock
        // cannot trigger an `expect` panic or a divide-by-zero. For a valid
        // config the values are unchanged. `period` is a local clamped `>= 1`
        // so the compiler proves every `div_ceil(period)` divisor non-zero.
        let clk_rate = config.core_clk_hz.unwrap_or(I3C_MIN_CORE_CLK_SDR).max(1);
        i3c_debug!(self.logger, "i3c clock rate: {} Hz", clk_rate);
        config.core_period = (NSEC_PER_SEC).div_ceil(clk_rate);
        let period = config.core_period.max(1);

        let ns_to_cnt_u8 = |ns: u32| -> u8 { u8::try_from(ns.div_ceil(period)).unwrap_or(u8::MAX) };
        let ns_to_cnt_u16 =
            |ns: u32| -> u16 { u16::try_from(ns.div_ceil(period)).unwrap_or(u16::MAX) };

        // I2C FM
        let (fm_hi_ns, fm_lo_ns) = self.calc_i2c_clk(config.i2c_scl_hz);
        self.regs
            .set_i2c_fm_timing(ns_to_cnt_u16(fm_hi_ns), ns_to_cnt_u16(fm_lo_ns));

        // I2C FMP
        let (i2c_fmp_hi_ns, i2c_fmp_lo_ns) = self.calc_i2c_clk(1_000_000);
        self.regs
            .set_i2c_fmp_timing(ns_to_cnt_u8(i2c_fmp_hi_ns), ns_to_cnt_u16(i2c_fmp_lo_ns));

        // I3C OD
        let (od_hi_ns, od_lo_ns) =
            if config.i3c_od_scl_hi_period_ns != 0 && config.i3c_od_scl_lo_period_ns != 0 {
                (
                    config.i3c_od_scl_hi_period_ns,
                    config.i3c_od_scl_lo_period_ns,
                )
            } else {
                (i2c_fmp_hi_ns, i2c_fmp_lo_ns)
            };
        self.regs
            .set_od_timing(ns_to_cnt_u8(od_hi_ns), ns_to_cnt_u8(od_lo_ns));

        // I3C PP
        let (i3c_pp_hi_ns, i3c_pp_lo_ns) =
            if config.i3c_pp_scl_hi_period_ns != 0 && config.i3c_pp_scl_lo_period_ns != 0 {
                (
                    config.i3c_pp_scl_hi_period_ns,
                    config.i3c_pp_scl_lo_period_ns,
                )
            } else {
                let total_ns = NSEC_PER_SEC.div_ceil(config.i3c_scl_hz.max(1));
                let hi_ns = core::cmp::min(I3C_BUS_THIGH_MAX_NS, total_ns.saturating_sub(1));
                let lo_ns = total_ns.saturating_sub(hi_ns).max(1);
                (hi_ns, lo_ns)
            };
        self.regs
            .set_pp_timing(ns_to_cnt_u8(i3c_pp_hi_ns), ns_to_cnt_u8(i3c_pp_lo_ns));

        // SDA TX hold time (`period` is the clamped, provably-non-zero divisor)
        let hold_steps = (config.sda_tx_hold_ns)
            .div_ceil(period)
            .clamp(SDA_TX_HOLD_MIN, SDA_TX_HOLD_MAX);
        let mut reg = self.regs.read_sda_hold();
        reg = (reg & !SDA_TX_HOLD_MASK) | ((hold_steps & 0x7) << 16);
        self.regs.write_sda_hold(reg);

        // BUS_FREE_TIMING
        self.regs.write_bus_free_timing(I3C_BUS_FREE_TIMING_RESET);
    }

    fn calc_i2c_clk(&mut self, fscl_hz: u32) -> (u32, u32) {
        use core::cmp::max;

        // `.max(1)` on both the SCL frequency and the resulting period keeps the
        // downstream `div_ceil(period_ns)` divisors provably non-zero (panic-free
        // for the `no_panics` analysis); a valid `fscl_hz` is unaffected.
        let period_ns: u32 = (1_000_000_000u32).div_ceil(fscl_hz.max(1)).max(1);

        let (lo_min, hi_min): (u32, u32) = if fscl_hz <= 100_000 {
            (
                (I3C_BUS_I2C_STD_TLOW_MIN_NS + I3C_BUS_I2C_STD_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_STD_THIGH_MIN_NS + I3C_BUS_I2C_STD_TR_MAX_NS).div_ceil(period_ns),
            )
        } else if fscl_hz <= 400_000 {
            (
                (I3C_BUS_I2C_FM_TLOW_MIN_NS + I3C_BUS_I2C_FM_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_FM_THIGH_MIN_NS + I3C_BUS_I2C_FM_TR_MAX_NS).div_ceil(period_ns),
            )
        } else {
            (
                (I3C_BUS_I2C_FMP_TLOW_MIN_NS + I3C_BUS_I2C_FMP_TF_MAX_NS).div_ceil(period_ns),
                (I3C_BUS_I2C_FMP_THIGH_MIN_NS + I3C_BUS_I2C_FMP_TR_MAX_NS).div_ceil(period_ns),
            )
        };

        let leftover = period_ns.saturating_sub(lo_min + hi_min);
        let lo = lo_min + leftover / 2;
        let hi = max(period_ns.saturating_sub(lo), hi_min);

        (hi, lo)
    }

    fn init_pid(&mut self, config: &mut I3cConfig) {
        let bus = self.bus();
        self.regs.set_pid_mfg_id(I3C_AST10X0_MIPI_MANUF_ID);

        let rev_id: u32 = self.regs.hw_rev_id();
        let mut reg: u32 = rev_id << 16 | u32::from(bus) << 12;
        reg |= 0xa000_0000;
        self.regs.write_slv_pid_value(reg);
        let mut reg: u32 = self.regs.read_slv_char_ctrl();
        reg &= !SLV_DCR_MASK;
        reg |= (config.dcr << 8) | 0x66;
        self.regs.write_slv_char_ctrl(reg);
    }
}

impl<Y: FnMut(u32)> HardwareFifo for Ast1060I3c<Y> {
    fn wr_tx_fifo(&mut self, bytes: &[u8]) {
        self.regs.tx_fifo_write(bytes);
    }

    fn rd_rx_fifo(&mut self, out: &mut [u8]) {
        self.regs.rx_fifo_read(out);
    }

    fn rd_ibi_fifo(&mut self, out: &mut [u8]) {
        self.regs.ibi_fifo_read(out);
    }
}

impl<Y: FnMut(u32)> HardwareRecovery for Ast1060I3c<Y> {
    fn enter_sw_mode(&mut self) {
        self.enter_sw_mode();
    }

    fn exit_sw_mode(&mut self) {
        self.exit_sw_mode();
    }

    fn i3c_toggle_scl_in(&mut self, count: u32) {
        self.toggle_scl_in(count);
    }

    fn gen_internal_stop(&mut self) {
        self.gen_internal_stop();
    }

    fn even_parity(byte: u8) -> bool {
        let mut parity = false;
        let mut b = byte;

        while b != 0 {
            parity = !parity;
            b &= b - 1;
        }

        !parity
    }
}

impl<Y: FnMut(u32)> HardwareTransfer for Ast1060I3c<Y> {
    fn set_ibi_mdb(&mut self, mdb: u8) {
        self.regs.set_ibi_mdb(mdb);
    }

    fn exit_halt(&mut self, config: &mut I3cConfig) -> Result<(), I3cError> {
        let state = self.regs.xfer_status();
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if state != expected {
            return Ok(());
        }

        self.regs.resume();

        let regs = &self.regs;
        let rc = poll_with_timeout(
            || u32::from(regs.xfer_status()),
            |val| val != u32::from(expected),
            &mut self.yield_fn,
            I3C_CTRL_POLL_DELAY_NS,
            I3C_POLL_MAX_ITERS,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "exit_halt: timeout");
            return Err(I3cError::Timeout);
        }
        Ok(())
    }

    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig) -> Result<(), I3cError> {
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if by_sw {
            self.regs.abort();
        }

        let regs = &self.regs;
        let rc = poll_with_timeout(
            || u32::from(regs.xfer_status()),
            |val| val == u32::from(expected),
            &mut self.yield_fn,
            I3C_CTRL_POLL_DELAY_NS,
            I3C_POLL_MAX_ITERS,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "enter_halt: timeout");
            return Err(I3cError::Timeout);
        }
        Ok(())
    }

    fn reset_ctrl(&mut self, reset: u32) -> Result<(), I3cError> {
        let reg = reset & RESET_CTRL_ALL;

        if reg == 0 {
            return Ok(());
        }

        self.regs.write_reset_ctrl(reg);
        let regs = &self.regs;
        let rc = poll_with_timeout(
            || regs.read_reset_ctrl(),
            |val| val == 0,
            &mut self.yield_fn,
            I3C_CTRL_POLL_DELAY_NS,
            I3C_POLL_MAX_ITERS,
        );

        if rc.is_err() {
            i3c_debug!(self.logger, "reset_ctrl: timeout");
            return Err(I3cError::Timeout);
        }
        Ok(())
    }

    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cDrvError> {
        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(addr)
            .ok_or(I3cDrvError::NoSuchDev)?;
        i3c_debug!(self.logger, "ibi_enable: dev_idx={}", dev_idx);
        // `get(dev_idx)` (not `[dev_idx]`) keeps this path panic-free for the
        // `no_panics` analysis; `find_dev_idx_by_addr` already returns a valid
        // index.
        let pos_opt = config
            .attached
            .pos_of(dev_idx)
            .or_else(|| config.attached.devices.get(dev_idx).and_then(|d| d.pos));

        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;
        i3c_debug!(self.logger, "ibi_enable: pos={}", pos);
        let dev = config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cDrvError::NoSuchDev)?;
        let tgt_bcr: u32 = u32::from(dev.bcr);
        let mut reg = self.regs.dat_read(pos.into());
        reg &= !DEV_ADDR_TABLE_SIR_REJECT;
        if tgt_bcr & I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE != 0 {
            reg |= DEV_ADDR_TABLE_IBI_MDB | DEV_ADDR_TABLE_IBI_PEC;
        }

        self.regs.dat_write_raw(pos.into(), reg);

        let mut sir_reject = self.regs.read_sir_reject();
        sir_reject &= !bit(pos.into());
        self.regs.write_sir_reject(sir_reject);

        self.regs.enable_ibi_thld_irq();

        let events = I3C_CCC_EVT_INTR;
        // ccc_events_set requires HardwareTransfer trait bound on Self.
        // We are inside HardwareTransfer impl for Ast1060I3c.
        // Rust might have trouble inferring if Self: HardwareTransfer is not fully established yet?
        // But Ast1060I3c implements HardwareTransfer (this block).
        // However, ccc_events_set takes `&mut impl HardwareInterface`.
        // Ast1060I3c implements HardwareInterface (blanket impl over all sub-traits).
        // So this call should be valid.
        let _ = ccc_events_set(self, config, dev.dyn_addr, true, events);

        i3c_debug!(self.logger, "i3cd030 (SIR reject) = {:#x}", sir_reject);
        i3c_debug!(
            self.logger,
            "i3cd040 (IBI thld) = {:#x}",
            self.regs.read_intr_status_en()
        );
        i3c_debug!(
            self.logger,
            "i3cd044 (IBI thld sig) = {:#x}",
            self.regs.read_intr_signal_en()
        );
        i3c_debug!(
            self.logger,
            "i3cd280 dat_addr[{}] = {:#x}",
            pos,
            self.regs.dat_read(pos.into())
        );
        i3c_debug!(self.logger, "ibi_enable done");
        Ok(())
    }

    fn ibi_disable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cDrvError> {
        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(addr)
            .ok_or(I3cDrvError::NoSuchDev)?;
        let pos_opt = config
            .attached
            .pos_of(dev_idx)
            .or_else(|| config.attached.devices.get(dev_idx).and_then(|d| d.pos));
        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;
        let dyn_addr = config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cDrvError::NoSuchDev)?
            .dyn_addr;

        // Tell the device to stop raising SIRs first (DISEC), while the
        // controller still ACKs its IBIs; best-effort, mirroring ibi_enable.
        let _ = ccc_events_set(self, config, dyn_addr, false, I3C_CCC_EVT_INTR);

        // Then reject at the controller: DAT slot + SIR-reject mask.
        let mut reg = self.regs.dat_read(pos.into());
        reg |= DEV_ADDR_TABLE_SIR_REJECT;
        reg &= !(DEV_ADDR_TABLE_IBI_MDB | DEV_ADDR_TABLE_IBI_PEC);
        self.regs.dat_write_raw(pos.into(), reg);

        let mut sir_reject = self.regs.read_sir_reject();
        sir_reject |= bit(pos.into());
        self.regs.write_sir_reject(sir_reject);

        Ok(())
    }

    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer) {
        let _ = config;
        xfer.ret = -1;

        // Clear any stale completion flag and drain any stale responses left
        // by an earlier timed-out transfer (the old ISR-side null-pointer
        // drain, now done on the thread before the next submission).
        let _ = isr_events(self.bus() as usize).take_pending();
        let nresp = self.regs.resp_buf_level();
        for _ in 0..nresp {
            let _ = self.regs.pop_response();
        }
        // Re-arm the completion IRQ sources. If a late response (e.g. from a
        // transfer that timed out) arrived with no waiter, the ISR masked the
        // sources and nobody unmasked them — without this, the new transfer's
        // completion would never be latched and would falsely time out.
        self.regs.unmask_master_xfer_irqs();

        for cmd in xfer.cmds.iter() {
            if let Some(tx) = cmd.tx {
                let take = tx.len().min(cmd.tx_len as usize);
                if take > 0 {
                    i3c_debug!(self.logger, "start_xfer: write {} bytes", take);
                    self.wr_tx_fifo(&tx[..take]);
                }
            }
        }
        self.regs
            .set_resp_buf_threshold(u8::try_from(xfer.cmds.len().saturating_sub(1)).unwrap_or(0));

        for cmd in xfer.cmds.iter() {
            i3c_debug!(
                self.logger,
                "start_xfer: cmd: cmd_hi={:#x}, cmd_lo={:#x}",
                cmd.cmd_hi,
                cmd.cmd_lo
            );
            self.regs.push_cmd(cmd.cmd_hi);
            self.regs.push_cmd(cmd.cmd_lo);
        }
    }

    fn wait_xfer_complete(
        &mut self,
        config: &mut I3cConfig,
        xfer: &mut I3cXfer,
        timeout_us: u32,
    ) -> bool {
        let events = isr_events(self.bus() as usize);

        // Cooperative-yield bounded poll (Delta D2) on the ISR's latched
        // status; the ISR masked the sources, so on a hit this thread owns
        // the response queue and drains it into its own `xfer` — no `&mut`
        // ever crosses the ISR boundary.
        let mut left = timeout_us;
        loop {
            let pending = events.take_pending();
            if pending & (INTR_RESP_READY_STAT | INTR_TRANSFER_ERR_STAT | INTR_TRANSFER_ABORT_STAT)
                != 0
            {
                self.process_responses(config, xfer);
                self.regs.unmask_master_xfer_irqs();
                return true;
            }
            if left == 0 {
                break;
            }
            (self.yield_fn)(1_000);
            left -= 1;
        }

        // Timeout: recover the engine and re-arm the IRQ sources. Recovery is
        // best-effort — the `false` return already reports the timeout.
        i3c_debug!(self.logger, "wait_xfer_complete: timeout");
        let _ = self.enter_halt(true, config);
        let _ = self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
        let _ = self.exit_halt(config);
        self.regs.unmask_master_xfer_irqs();
        false
    }

    fn detach_i3c_dev(&mut self, pos: usize) {
        self.regs.dat_set_reject(pos);
    }

    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cDrvError> {
        let mut da_with_parity = addr;
        if Self::even_parity(addr) {
            da_with_parity |= 1 << 7;
        }

        self.regs.dat_program_addr(pos, da_with_parity);

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn do_ccc(
        &mut self,
        config: &mut I3cConfig,
        payload: &mut CccPayload<'_, '_>,
    ) -> Result<(), I3cDrvError> {
        let mut cmds = [I3cCmd {
            cmd_lo: 0,
            cmd_hi: 0,
            tx: None,
            rx: None,
            tx_len: 0,
            rx_len: 0,
            ret: 0,
        }];

        let mut pos = 0;
        let mut rnw: bool = false;
        let mut is_broadcast = false;

        let (id, data_len) = {
            let Some(ccc) = payload.ccc.as_ref() else {
                return Err(I3cDrvError::Invalid);
            };
            (ccc.id, ccc.data.as_deref().map_or(0, <[u8]>::len))
        };

        let dbp_is_direct = id > 0x7F;
        let db: u8 = if dbp_is_direct && data_len > 0 {
            payload
                .ccc
                .as_ref()
                .and_then(|c| c.data.as_deref())
                .map_or(0, |d| d[0])
        } else {
            0
        };

        {
            let cmd = &mut cmds[0];

            if id <= 0x7F {
                is_broadcast = true;

                if data_len > 0
                    && let Some(d) = payload.ccc.as_ref().and_then(|c| c.data.as_deref())
                {
                    cmd.tx = Some(d);
                    cmd.tx_len = u32::try_from(data_len).map_err(|_| I3cDrvError::Invalid)?;
                }
            } else {
                let Some(tgt_addr) = payload
                    .targets
                    .as_ref()
                    .and_then(|ts| ts.first())
                    .map(|t| t.addr)
                else {
                    return Err(I3cDrvError::Invalid);
                };
                let pos_ops = config.attached.pos_of_addr(tgt_addr);
                i3c_debug!(
                    self.logger,
                    "do_ccc: tgt_addr=0x{:02x}, pos_ops={:?}",
                    tgt_addr,
                    pos_ops
                );
                pos = match pos_ops {
                    Some(p) => p,
                    None => return Err(I3cDrvError::Invalid),
                };
                i3c_debug!(
                    self.logger,
                    "do_ccc: tgt_addr=0x{:02x}, pos={}",
                    tgt_addr,
                    pos
                );

                let Some(tp) = payload.targets.as_deref_mut().and_then(|ts| ts.first_mut()) else {
                    return Err(I3cDrvError::Invalid);
                };

                rnw = tp.rnw;

                if rnw {
                    let len = tp.data.as_deref().map_or(0, <[u8]>::len);
                    if len == 0 {
                        return Err(I3cDrvError::Invalid);
                    }
                    cmd.rx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                    cmd.rx = tp.data.as_deref_mut();
                } else {
                    let (d_opt, len) = match tp.data.as_deref() {
                        Some(d) => (Some(d), d.len()),
                        None => (None, 0),
                    };
                    cmd.tx = d_opt;
                    cmd.tx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                    tp.num_xfer = len;
                }
            }
        }

        let cmd = &mut cmds[0];
        cmd.cmd_hi = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG);

        if dbp_is_direct && data_len > 0 {
            cmd.cmd_lo |= COMMAND_PORT_DBP;
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DB, db.into());
        }

        if rnw {
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DATA_LEN, cmd.rx_len);
        } else {
            cmd.cmd_hi |= field_prep(COMMAND_PORT_ARG_DATA_LEN, cmd.tx_len);
        }

        cmd.cmd_lo |= field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_CMD)
            | field_prep(COMMAND_PORT_CMD, id.into())
            | field_prep(COMMAND_PORT_READ_TRANSFER, u32::from(rnw))
            | COMMAND_PORT_CP
            | COMMAND_PORT_ROC
            | COMMAND_PORT_TOC;

        if !is_broadcast {
            cmd.cmd_lo |= field_prep(COMMAND_PORT_DEV_INDEX, u32::from(pos));
        }

        if id == I3C_CCC_SETHID || id == I3C_CCC_DEVCTRL {
            cmd.cmd_lo |= field_prep(COMMAND_PORT_SPEED, SpeedI3c::I2cFmAsI3c as u32);
        }

        let mut xfer = I3cXfer::new(&mut cmds[..]);
        self.start_xfer(config, &mut xfer);

        // On timeout `wait_xfer_complete` already recovered the engine;
        // `xfer.ret` stays -1 and falls through to the error mapping below
        // (same outcome as the reference's timeout path).
        let _ = self.wait_xfer_complete(config, &mut xfer, I3C_OP_TIMEOUT_US);

        let ret = xfer.ret;
        if ret == i32::try_from(RESPONSE_ERROR_IBA_NACK).map_err(|_| I3cDrvError::Invalid)? {
            return Ok(());
        }

        if is_broadcast && let Some(ccc_rw) = payload.ccc.as_mut() {
            let num_xfer = ccc_rw.data.as_deref().map(<[u8]>::len);
            if let Some(n) = num_xfer {
                ccc_rw.num_xfer = n;
            }
        }

        match ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Invalid),
        }
    }

    fn do_entdaa(&mut self, config: &mut I3cConfig, pos: u32) -> Result<(), I3cDrvError> {
        i3c_debug!(self.logger, "do_entdaa: pos={}", pos);
        let cmd = I3cCmd {
            cmd_lo: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_ADDR_ASSGN_CMD)
                | field_prep(COMMAND_PORT_CMD, u32::from(I3C_CCC_ENTDAA))
                | field_prep(COMMAND_PORT_DEV_COUNT, 1)
                | field_prep(COMMAND_PORT_DEV_INDEX, pos)
                | COMMAND_PORT_ROC
                | COMMAND_PORT_TOC,
            cmd_hi: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG),
            tx: None,
            rx: None,
            tx_len: 0,
            rx_len: 0,
            ret: 0,
        };

        i3c_debug!(
            self.logger,
            "do_entdaa: cmd_lo=0x{:08x}, cmd_hi=0x{:08x}",
            cmd.cmd_lo,
            cmd.cmd_hi
        );
        let mut cmds = [cmd];
        let mut xfer = I3cXfer::new(&mut cmds[..]);
        xfer.ret = -1;

        self.start_xfer(config, &mut xfer);

        // Full operation budget — NOT the C driver's 10 ms. The C timeout is
        // wall-clock (`k_sem_take(K_MSEC(10))`); this driver's timeout unit is
        // cooperative-yield ticks, and a fast `yield_fn` makes the nominal
        // value run far shorter in real time. A short budget here aborts a
        // live ENTDAA mid-flight (halt + queue reset), wedging the DAA
        // handshake. An ENTDAA nobody answers still exits early via NACK.
        if !self.wait_xfer_complete(config, &mut xfer, I3C_OP_TIMEOUT_US) {
            return Err(I3cDrvError::Timeout);
        }

        i3c_debug!(self.logger, "do_entdaa: xfer done");
        match xfer.ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Invalid),
        }
    }

    fn priv_xfer_build_cmds<'a>(
        &mut self,
        cmds: &mut [I3cCmd<'a>],
        msgs: &mut [I3cMsg<'a>],
        pos: u8,
    ) -> Result<(), I3cDrvError> {
        let cmds_len = cmds.len();
        if cmds_len != msgs.len() {
            return Err(I3cDrvError::Invalid);
        }

        // The transfer-ID field is 4 bits, so the message index used as the TID
        // must stay below `MAX_PRIV_XFER_CMDS`. A larger batch would see indices
        // >= 16 alias earlier commands once `field_prep` masks the TID, routing
        // their responses onto the wrong message. Reject before consuming any
        // buffer so the build stays all-or-nothing.
        if cmds_len > MAX_PRIV_XFER_CMDS {
            return Err(I3cDrvError::TooManyMsgs);
        }

        // Pre-validate every message before taking any buffer, so the build is
        // all-or-nothing: a bad message late in the batch must not leave the
        // earlier messages' `buf` already moved out to `None`. Non-consuming
        // (`as_deref`) — purely a length/presence check. The upper bound is the
        // 16-bit `COMMAND_PORT_ARG_DATA_LEN` field width; a longer buffer would
        // truncate silently in `field_prep` (the per-command `u32::try_from`
        // below only catches lengths above `u32::MAX`).
        for m in msgs.iter() {
            match m.buf.as_deref() {
                Some(b) if !b.is_empty() && b.len() <= MAX_XFER_DATA_LEN => {}
                _ => return Err(I3cDrvError::Invalid),
            }
        }

        // Zip (not parallel `cmds[i]`/`msgs[i]` indexing) so the build loop is
        // panic-free for the `no_panics` analysis; lengths are equal (checked).
        for (i, (cmd, m)) in cmds.iter_mut().zip(msgs.iter_mut()).enumerate() {
            let is_read = (m.flags & I3C_MSG_READ) != 0;

            // Move (`Option::take`) — never alias — the caller's buffer out of
            // the message and into the command: the one `&'a mut` reborrow is
            // transferred, so the FIFO scatter path in `process_responses`
            // holds the only live reference. No `unsafe`, no second `&mut` to
            // the same memory. The message keeps only the transfer lengths
            // (`num_xfer`/`actual_len`); `m.buf` is `None` after this call and
            // no caller reads it post-transfer (the caller still owns the real
            // buffer the reborrow came from). Pre-validation above guarantees
            // the `Some(non-empty)` arm here, so no message is ever left with a
            // half-built (taken) buffer on the error path.
            let buf = match m.buf.take() {
                Some(b) if !b.is_empty() => b,
                other => {
                    m.buf = other;
                    return Err(I3cDrvError::Invalid);
                }
            };
            let len = buf.len();

            *cmd = I3cCmd {
                cmd_hi: field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_XFER_ARG)
                    | field_prep(
                        COMMAND_PORT_ARG_DATA_LEN,
                        u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?,
                    ),
                cmd_lo: field_prep(
                    COMMAND_PORT_TID,
                    u32::try_from(i).map_err(|_| I3cDrvError::Invalid)?,
                ) | field_prep(COMMAND_PORT_DEV_INDEX, u32::from(pos))
                    | COMMAND_PORT_ROC,
                tx: None,
                rx: None,
                tx_len: 0,
                rx_len: 0,
                ret: 0,
            };

            if is_read {
                cmd.rx = Some(buf);
                cmd.rx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                cmd.cmd_lo |= COMMAND_PORT_READ_TRANSFER;
            } else {
                m.num_xfer = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                // Downgrade the moved `&'a mut [u8]` to `&'a [u8]` for the TX
                // side (a move-coercion, not a reborrow — keeps lifetime `'a`).
                let tx_slice: &'a [u8] = buf;
                cmd.tx = Some(tx_slice);
                cmd.tx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
            }

            let is_last = i + 1 == cmds_len;
            if is_last {
                cmd.cmd_lo |= COMMAND_PORT_TOC;
            }
        }

        Ok(())
    }

    fn priv_xfer(
        &mut self,
        config: &mut I3cConfig,
        pid: u64,
        msgs: &mut [I3cMsg],
    ) -> Result<(), I3cDrvError> {
        let pos_opt = config.attached.pos_of_pid(pid);
        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;

        if msgs.len() == 1 {
            let mut cmd = I3cCmd::new();
            let cmds = core::slice::from_mut(&mut cmd);

            self.priv_xfer_build_cmds(cmds, msgs, pos)?;

            let mut xfer = I3cXfer::new(cmds);
            self.start_xfer(config, &mut xfer);

            if !self.wait_xfer_complete(config, &mut xfer, I3C_OP_TIMEOUT_US) {
                return Err(I3cDrvError::Timeout);
            }

            if let Some(m) = msgs.first_mut()
                && (m.flags & I3C_MSG_READ) != 0
            {
                m.actual_len = xfer.cmds.first().map_or(0, |c| c.rx_len);
            }

            return match xfer.ret {
                0 => Ok(()),
                _ => Err(I3cDrvError::Timeout),
            };
        }

        let mut cmds: heapless::Vec<I3cCmd, MAX_CMDS> = heapless::Vec::new();
        for _ in 0..msgs.len() {
            // `?` (not `.unwrap()`) keeps this panic-free; > MAX_CMDS msgs is a
            // typed error, not a panic.
            cmds.push(I3cCmd {
                cmd_lo: 0,
                cmd_hi: 0,
                tx: None,
                rx: None,
                tx_len: 0,
                rx_len: 0,
                ret: 0,
            })
            .map_err(|_| I3cDrvError::TooManyMsgs)?;
        }

        let ret = self.priv_xfer_build_cmds(cmds.as_mut_slice(), msgs, pos);
        match ret {
            Ok(()) => {}
            Err(e) => return Err(e),
        }

        let mut xfer = I3cXfer::new(cmds.as_mut_slice());
        self.start_xfer(config, &mut xfer);

        if !self.wait_xfer_complete(config, &mut xfer, I3C_OP_TIMEOUT_US) {
            return Err(I3cDrvError::Timeout);
        }

        for (i, m) in msgs.iter_mut().enumerate() {
            if (m.flags & I3C_MSG_READ) != 0
                && let Some(c) = xfer.cmds.get(i)
            {
                m.actual_len = c.rx_len;
            }
        }

        match xfer.ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Timeout),
        }
    }
}

impl<Y: FnMut(u32)> HardwareTarget for Ast1060I3c<Y> {
    fn target_tx_write(&mut self, buf: &[u8]) {
        self.wr_tx_fifo(buf);
        let cmd = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(
                COMMAND_PORT_ARG_DATA_LEN,
                u32::try_from(buf.len()).map_or(0, |v| v),
            )
            | field_prep(COMMAND_PORT_TID, Tid::TargetRdData as u32);

        self.regs.push_cmd(cmd);
    }

    fn target_ibi_raise_hj(&self, config: &mut I3cConfig) -> Result<(), I3cDrvError> {
        if !config.is_secondary {
            return Err(I3cDrvError::Invalid);
        }
        if !self.regs.hj_capable() {
            return Err(I3cDrvError::Invalid);
        }

        if self.regs.dynamic_addr_valid() {
            return Err(I3cDrvError::Access);
        }

        self.regs.raise_hot_join_request();

        Ok(())
    }

    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cDrvError> {
        let events = isr_events(self.bus() as usize);

        // A fault the ISR deferred (errored response / halted engine after a
        // CCC): recover here on the thread, where the wait policy lives.
        if events.take_fault() {
            // Best-effort: a recovery timeout here must not block the SIR
            // attempt below, which has its own timeout/recovery path.
            i3c_debug!(self.logger, "recovering deferred target fault");
            let _ = self.enter_halt(false, config);
            let _ = self.reset_ctrl(RESET_CTRL_QUEUES);
            let _ = self.exit_halt(config);
        }

        let reg = self.regs.read_slv_event_ctrl();
        if !(config.sir_allowed_by_sw && (reg & SLV_EVENT_CTRL_SIR_EN != 0)) {
            return Err(I3cDrvError::Access);
        }

        let Some(mdb) = notifier.first_byte() else {
            return Err(I3cDrvError::Invalid);
        };

        self.set_ibi_mdb(mdb);
        if let Some(p) = notifier.payload
            && !p.is_empty()
        {
            self.wr_tx_fifo(p);
        }

        let payload_len = u32::try_from(notifier.payload.map_or(0, <[u8]>::len))
            .map_err(|_| I3cDrvError::Invalid)?;
        let cmd: u32 = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(COMMAND_PORT_ARG_DATA_LEN, payload_len)
            | field_prep(COMMAND_PORT_TID, Tid::TargetIbi as u32);
        self.regs.push_cmd(cmd);

        events.target_ibi_done.reset();

        self.regs.set_resp_buf_threshold(0);

        self.target_tx_write(buf);
        events.target_data_done.reset();

        self.regs.raise_sir();

        if !events
            .target_ibi_done
            .wait_for_us(I3C_OP_TIMEOUT_US, &mut self.yield_fn)
        {
            // Vendor C driver parity (`target_rst_worker`): an unanswered SIR
            // means the engine may be wedged beyond a queue reset — re-run the
            // full controller init. Side effects match the C driver: the
            // dynamic address is dropped (the bus master must re-run DAA) and
            // `sir_allowed_by_sw` is cleared until the next DA assignment.
            // Best-effort; `IoError` below already reports the failure.
            i3c_debug!(self.logger, "SIR timeout! Reset I3C controller");
            let _ = self.init(config);
            return Err(I3cDrvError::IoError);
        }

        if !events
            .target_data_done
            .wait_for_us(I3C_OP_TIMEOUT_US, &mut self.yield_fn)
        {
            // Best-effort recovery; `Timeout` below already reports the failure.
            i3c_debug!(self.logger, "wait master read timeout! Reset queues");
            self.i3c_disable(config.is_secondary);
            let _ = self.reset_ctrl(RESET_CTRL_QUEUES);
            self.i3c_enable(config);
            return Err(I3cDrvError::Timeout);
        }

        Ok(())
    }
}
