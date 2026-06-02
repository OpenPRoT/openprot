// Licensed under the Apache-2.0 license

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

use core::cell::RefCell;
use critical_section::Mutex;

use super::ccc::{ccc_events_set, CccPayload};
use super::config::I3cConfig;
use super::constants::{
    bit, field_get, field_prep, CM_TFR_STS_MASTER_HALT, CM_TFR_STS_TARGET_HALT,
    COMMAND_ATTR_ADDR_ASSGN_CMD, COMMAND_ATTR_SLAVE_DATA_CMD, COMMAND_ATTR_XFER_ARG,
    COMMAND_ATTR_XFER_CMD, COMMAND_PORT_ARG_DATA_LEN, COMMAND_PORT_ARG_DB, COMMAND_PORT_ATTR,
    COMMAND_PORT_CMD, COMMAND_PORT_CP, COMMAND_PORT_DBP, COMMAND_PORT_DEV_COUNT,
    COMMAND_PORT_DEV_INDEX, COMMAND_PORT_READ_TRANSFER, COMMAND_PORT_ROC, COMMAND_PORT_SPEED,
    COMMAND_PORT_TID, COMMAND_PORT_TOC, DEV_ADDR_TABLE_IBI_MDB, DEV_ADDR_TABLE_IBI_PEC,
    DEV_ADDR_TABLE_SIR_REJECT, I3CG_REG1_SCL_IN_SW_MODE_EN, I3CG_REG1_SCL_IN_SW_MODE_VAL,
    I3CG_REG1_SDA_IN_SW_MODE_EN, I3CG_REG1_SDA_IN_SW_MODE_VAL, I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE,
    I3C_BUS_I2C_FMP_TF_MAX_NS, I3C_BUS_I2C_FMP_THIGH_MIN_NS, I3C_BUS_I2C_FMP_TLOW_MIN_NS,
    I3C_BUS_I2C_FMP_TR_MAX_NS, I3C_BUS_I2C_FM_TF_MAX_NS, I3C_BUS_I2C_FM_THIGH_MIN_NS,
    I3C_BUS_I2C_FM_TLOW_MIN_NS, I3C_BUS_I2C_FM_TR_MAX_NS, I3C_BUS_I2C_STD_TF_MAX_NS,
    I3C_BUS_I2C_STD_THIGH_MIN_NS, I3C_BUS_I2C_STD_TLOW_MIN_NS, I3C_BUS_I2C_STD_TR_MAX_NS,
    I3C_BUS_THIGH_MAX_NS, I3C_CCC_DEVCTRL, I3C_CCC_ENTDAA, I3C_CCC_EVT_INTR, I3C_CCC_SETHID,
    I3C_MSG_READ, IBIQ_STATUS_IBI_DATA_LEN, IBIQ_STATUS_IBI_DATA_LEN_SHIFT, IBIQ_STATUS_IBI_ID,
    IBIQ_STATUS_IBI_ID_SHIFT, INTR_CCC_UPDATED_STAT, INTR_DYN_ADDR_ASSGN_STAT, INTR_IBI_THLD_STAT,
    INTR_RESP_READY_STAT, INTR_TRANSFER_ABORT_STAT, INTR_TRANSFER_ERR_STAT, MAX_CMDS, NSEC_PER_SEC,
    RESET_CTRL_ALL, RESET_CTRL_QUEUES, RESET_CTRL_XFER_QUEUES, RESPONSE_ERROR_IBA_NACK,
    RESPONSE_PORT_DATA_LEN_MASK, RESPONSE_PORT_DATA_LEN_SHIFT, RESPONSE_PORT_ERR_STATUS_MASK,
    RESPONSE_PORT_ERR_STATUS_SHIFT, RESPONSE_PORT_TID_MASK, RESPONSE_PORT_TID_SHIFT,
    SDA_TX_HOLD_MASK, SDA_TX_HOLD_MAX, SDA_TX_HOLD_MIN, SLV_DCR_MASK, SLV_EVENT_CTRL_SIR_EN,
};
use super::error::I3cError as I3cDrvError;
use super::error::I3cError;
use super::ibi as ibi_workq;
use super::types::{I3cCmd, I3cIbi, I3cMsg, I3cXfer, SpeedI3c, Tid};

use core::marker::PhantomData;
use core::sync::atomic::Ordering;
use cortex_m::peripheral::NVIC;

// =============================================================================
// IRQ Handler Infrastructure
// =============================================================================

#[derive(Clone, Copy)]
struct Handler {
    func: fn(usize),
    ctx: usize,
}

static BUS_HANDLERS: [Mutex<RefCell<Option<Handler>>>; 4] = [
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
    Mutex::new(RefCell::new(None)),
];

/// Register an IRQ handler for an I3C bus
///
/// # Arguments
/// * `bus` - Bus index (0-3)
/// * `func` - Handler function
/// * `ctx` - Context value passed to handler
///
/// # Panics
/// Panics if `bus >= 4`.
pub fn register_i3c_irq_handler(bus: usize, func: fn(usize), ctx: usize) {
    assert!(bus < 4);
    critical_section::with(|cs| {
        *BUS_HANDLERS[bus].borrow(cs).borrow_mut() = Some(Handler { func, ctx });
    });
}

/// Dispatch IRQ for a specific bus
///
/// Called by the actual IRQ entry points (defined elsewhere to avoid symbol conflicts).
#[inline]
pub fn dispatch_i3c_irq(bus: usize) {
    // Copy handler out of critical section to avoid blocking IRQs during handler
    let handler =
        critical_section::with(|cs| BUS_HANDLERS.get(bus).and_then(|m| *m.borrow(cs).borrow()));
    if let Some(h) = handler {
        (h.func)(h.ctx);
    }
}

// IRQ entry points - defined in src/i3c/ module to avoid symbol conflicts.
// Use register_i3c_irq_handler() to register handlers that will be called
// from those entry points.

// =============================================================================
// Sub-trait: Core Operations
// =============================================================================

/// Core hardware operations: init, IRQ, enable/disable
pub trait HardwareCore {
    /// Initialize the I3C controller hardware
    fn init(&mut self, config: &mut I3cConfig);

    /// Get the bus number for this instance
    fn bus_num(&self) -> u8;

    /// Enable interrupts
    fn enable_irq(&mut self);

    /// Disable interrupts
    fn disable_irq(&mut self);

    /// Enable the I3C controller
    fn i3c_enable(&mut self, config: &I3cConfig);

    /// Disable the I3C controller
    fn i3c_disable(&mut self, is_secondary: bool);

    /// Set the controller role (primary/secondary)
    fn set_role(&mut self, is_secondary: bool);

    /// Main ISR handler
    fn i3c_aspeed_isr(&mut self, config: &mut I3cConfig);
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

    /// Read from FIFO using provided read function
    fn rd_fifo<F>(&mut self, read_word: F, out: &mut [u8])
    where
        F: FnMut() -> u32;

    /// Drain FIFO without storing data
    fn drain_fifo<F>(&mut self, read_word: F, len: usize)
    where
        F: FnMut() -> u32;

    /// Read from RX FIFO
    fn rd_rx_fifo(&mut self, out: &mut [u8]);

    /// Read from IBI FIFO
    fn rd_ibi_fifo(&mut self, out: &mut [u8]);
}

// =============================================================================
// Sub-trait: Transfer Operations
// =============================================================================

/// Transfer, CCC, and device management operations
pub trait HardwareTransfer {
    /// Set the IBI Mandatory Data Byte
    fn set_ibi_mdb(&mut self, mdb: u8);

    /// Exit halt state
    fn exit_halt(&mut self, config: &mut I3cConfig);

    /// Enter halt state
    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig);

    /// Reset controller components (FIFOs, queues, etc.)
    fn reset_ctrl(&mut self, reset: u32);

    /// Enable IBI for a device
    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cError>;

    /// Start a transfer
    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer);

    /// End a transfer
    fn end_xfer(&mut self, config: &mut I3cConfig);

    /// Get DAT position for an address
    fn get_addr_pos(&mut self, config: &I3cConfig, addr: u8) -> Option<u8>;

    /// Detach a device by DAT position
    fn detach_i3c_dev(&mut self, pos: usize);

    /// Attach a device to a DAT position
    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cError>;

    /// Execute a CCC
    fn do_ccc(&mut self, config: &mut I3cConfig, ccc: &mut CccPayload) -> Result<(), I3cError>;

    /// Execute ENTDAA (Enter Dynamic Address Assignment)
    fn do_entdaa(&mut self, config: &mut I3cConfig, index: u32) -> Result<(), I3cError>;

    /// Build commands for private transfer
    fn priv_xfer_build_cmds<'a>(
        &mut self,
        cmds: &mut [I3cCmd<'a>],
        msgs: &mut [I3cMsg<'a>],
        pos: u8,
    ) -> Result<(), I3cError>;

    /// Execute a private transfer
    fn priv_xfer(
        &mut self,
        config: &mut I3cConfig,
        pid: u64,
        msgs: &mut [I3cMsg],
    ) -> Result<(), I3cError>;

    /// Handle IBI SIR (Slave Interrupt Request)
    fn handle_ibi_sir(&mut self, config: &mut I3cConfig, addr: u8, len: usize);

    /// Handle all pending IBIs
    fn handle_ibis(&mut self, config: &mut I3cConfig);
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

    /// Handle response ready in target mode
    fn target_handle_response_ready(&mut self, config: &mut I3cConfig);

    /// Notify pending read in target mode
    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cError>;

    /// Handle CCC update in target mode
    fn target_handle_ccc_update(&mut self, config: &mut I3cConfig);
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

// ISR exports - only when isr-handlers feature is enabled
// For kernel integration, disable this feature and define ISRs in target code
#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn i3c() {
    i3c_irq_handler();
}

#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn i3c1() {
    i3c1_irq_handler();
}

#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn i3c2() {
    i3c2_irq_handler();
}

#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn i3c3() {
    i3c3_irq_handler();
}

/// I3C controller wrapping hardware register pointers.
///
/// Provides low-level hardware abstraction for I3C controllers.
/// Platform/board layer is responsible for:
/// - Clock enable (SCU)
/// - Reset deassert (SCU)
/// - Pin mux configuration
///
/// All of these must be done BEFORE constructing this instance.
pub struct Ast1060I3c<'a, Y: FnMut(u32)> {
    i3c: *const ast1060_pac::i3c::RegisterBlock,
    i3cg: *const ast1060_pac::i3cglobal::RegisterBlock,
    pub bus_num: u8,
    pub yield_ns: Y,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a, Y: FnMut(u32)> Ast1060I3c<'a, Y> {
    /// Create a new I3C instance from hardware pointers.
    ///
    /// # Safety
    ///
    /// - `i3c` and `i3cg` must be valid, non-null pointers to the respective register blocks
    /// - Both must point to the **same** I3C controller instance
    /// - `bus_num` must be 0-3 (one of the four I3C bus instances)
    /// - Pointers must remain valid for the lifetime of this instance
    /// - Caller must ensure exclusive ownership (no concurrent mutable access)
    /// - Platform must have already handled SCU setup (clock, reset)
    pub unsafe fn new(
        i3c: *const ast1060_pac::i3c::RegisterBlock,
        i3cg: *const ast1060_pac::i3cglobal::RegisterBlock,
        bus_num: u8,
        yield_ns: Y,
    ) -> Self {
        assert!(bus_num < 4, "I3C bus number must be 0-3, got {}", bus_num);
        Self {
            i3c,
            i3cg,
            bus_num,
            yield_ns,
            _lifetime: PhantomData,
        }
    }

    /// Create instance from pre-initialized hardware (no I/O).
    ///
    /// Lightweight constructor when hardware is already configured.
    /// Use after [`new`] on the same controller, or when boot firmware
    /// already initialized the I3C hardware.
    #[must_use]
    pub unsafe fn from_initialized(
        i3c: *const ast1060_pac::i3c::RegisterBlock,
        i3cg: *const ast1060_pac::i3cglobal::RegisterBlock,
        bus_num: u8,
        yield_ns: Y,
    ) -> Self {
        unsafe { Self::new(i3c, i3cg, bus_num, yield_ns) }
    }

    /// Safe access to I3C registers
    #[inline]
    fn i3c(&self) -> &ast1060_pac::i3c::RegisterBlock {
        unsafe { &*self.i3c }
    }

    /// Safe access to I3C global registers
    #[inline]
    fn i3cg(&self) -> &ast1060_pac::i3cglobal::RegisterBlock {
        unsafe { &*self.i3cg }
    }
}

// -----------------------------------------------------------------------------
// Register Helper Macros
// -----------------------------------------------------------------------------

#[allow(unused_macros)]
macro_rules! read_i3cg_reg1 {
    ($self:expr, $bus:expr) => {{
        match $bus {
            0 => $self.i3cg().i3c014().read().bits(),
            1 => $self.i3cg().i3c024().read().bits(),
            2 => $self.i3cg().i3c034().read().bits(),
            3 => $self.i3cg().i3c044().read().bits(),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

#[allow(unused_macros)]
macro_rules! write_i3cg_reg0 {
    ($self:expr, $bus:expr, |$w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg().i3c010().write(|$w| $body),
            1 => $self.i3cg().i3c020().write(|$w| $body),
            2 => $self.i3cg().i3c030().write(|$w| $body),
            3 => $self.i3cg().i3c040().write(|$w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

#[allow(unused_macros)]
macro_rules! read_i3cg_reg0 {
    ($self:expr, $bus:expr) => {{
        match $bus {
            0 => $self.i3cg().i3c010().read().bits(),
            1 => $self.i3cg().i3c020().read().bits(),
            2 => $self.i3cg().i3c030().read().bits(),
            3 => $self.i3cg().i3c040().read().bits(),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

#[allow(unused_macros)]
macro_rules! write_i3cg_reg1 {
    ($self:expr, $bus:expr, |$w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg().i3c014().write(|$w| $body),
            1 => $self.i3cg().i3c024().write(|$w| $body),
            2 => $self.i3cg().i3c034().write(|$w| $body),
            3 => $self.i3cg().i3c044().write(|$w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! modify_i3cg_reg1 {
    ($self:expr, $bus:expr, |$r:ident, $w:ident| $body:expr) => {{
        match $bus {
            0 => $self.i3cg().i3c014().modify(|$r, $w| $body),
            1 => $self.i3cg().i3c024().modify(|$r, $w| $body),
            2 => $self.i3cg().i3c034().modify(|$r, $w| $body),
            3 => $self.i3cg().i3c044().modify(|$r, $w| $body),
            _ => panic!("invalid I3C bus index: {}", $bus),
        }
    }};
}

macro_rules! i3c_dat_read {
    ($self:expr, $pos:expr) => {{
        match ($pos) {
            0 => $self.i3c().i3cd280().read().bits(),
            1 => $self.i3c().i3cd284().read().bits(),
            2 => $self.i3c().i3cd288().read().bits(),
            3 => $self.i3c().i3cd28c().read().bits(),
            4 => $self.i3c().i3cd290().read().bits(),
            5 => $self.i3c().i3cd294().read().bits(),
            6 => $self.i3c().i3cd298().read().bits(),
            7 => $self.i3c().i3cd29c().read().bits(),
            _ => 0,
        }
    }};
}

macro_rules! i3c_dat_write {
    ($self:expr, $pos:expr, |$w:ident| $body:expr) => {{
        match ($pos) {
            0 => {
                $self.i3c().i3cd280().write(|$w| $body);
            }
            1 => {
                $self.i3c().i3cd284().write(|$w| $body);
            }
            2 => {
                $self.i3c().i3cd288().write(|$w| $body);
            }
            3 => {
                $self.i3c().i3cd28c().write(|$w| $body);
            }
            4 => {
                $self.i3c().i3cd290().write(|$w| $body);
            }
            5 => {
                $self.i3c().i3cd294().write(|$w| $body);
            }
            6 => {
                $self.i3c().i3cd298().write(|$w| $body);
            }
            7 => {
                $self.i3c().i3cd29c().write(|$w| $body);
            }
            _ => { /* ignore */ }
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollError {
    Timeout,
}

pub fn poll_with_timeout<F, C, Y>(
    mut read_reg: F,
    mut condition: C,
    yield_ns: &mut Y,
    delay_ns: u32,
    max_iters: u32,
) -> Result<u32, PollError>
where
    F: FnMut() -> u32,
    C: FnMut(u32) -> bool,
    Y: FnMut(u32),
{
    for _ in 0..max_iters {
        let val = read_reg();
        if condition(val) {
            return Ok(val);
        }
        yield_ns(delay_ns);
    }
    Err(PollError::Timeout)
}

impl<'a, Y: FnMut(u32)> Ast1060I3c<'a, Y> {
    fn toggle_scl_in(&mut self, count: u32) {
        let bus = self.bus_num;
        for _ in 0..count {
            modify_i3cg_reg1!(self, bus, |r, w| unsafe {
                w.bits(r.bits() & !I3CG_REG1_SCL_IN_SW_MODE_VAL)
            });
            modify_i3cg_reg1!(self, bus, |r, w| unsafe {
                w.bits(r.bits() | I3CG_REG1_SCL_IN_SW_MODE_VAL)
            });
        }
    }

    fn gen_internal_stop(&mut self) {
        let bus = self.bus_num;
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() & !I3CG_REG1_SCL_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() & !I3CG_REG1_SDA_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() | I3CG_REG1_SCL_IN_SW_MODE_VAL)
        });
        modify_i3cg_reg1!(self, bus, |r, w| unsafe {
            w.bits(r.bits() | I3CG_REG1_SDA_IN_SW_MODE_VAL)
        });
    }

    fn enter_sw_mode(&mut self) {
        let bus = self.bus_num;
        let mut reg = read_i3cg_reg1!(self, bus);
        reg |= I3CG_REG1_SCL_IN_SW_MODE_VAL | I3CG_REG1_SDA_IN_SW_MODE_VAL;
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
        reg |= I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN;
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
    }

    fn exit_sw_mode(&mut self) {
        let bus = self.bus_num;
        let mut reg = read_i3cg_reg1!(self, bus);
        reg &= !(I3CG_REG1_SCL_IN_SW_MODE_EN | I3CG_REG1_SDA_IN_SW_MODE_EN);
        modify_i3cg_reg1!(self, bus, |_r, w| unsafe { w.bits(reg) });
    }

}

impl<'a, Y: FnMut(u32)> HardwareCore for Ast1060I3c<'a, Y> {
    #[allow(clippy::too_many_lines)]
    fn init(&mut self, config: &mut I3cConfig) {
        // PRECONDITION: Platform must have handled:
        // - SCU clock enable for this I3C controller
        // - SCU reset deassert
        // - Pin mux configuration

        self.i3c_disable(config.is_secondary);

        self.i3c().i3cd034().write(|w| {
            w.ibiqueue_sw_rst()
                .set_bit()
                .rx_buffer_sw_rst()
                .set_bit()
                .tx_buffer_sw_rst()
                .set_bit()
                .response_queue_sw_rst()
                .set_bit()
                .cmd_queue_sw_rst()
                .set_bit()
                .core_sw_rst()
                .set_bit()
        });

        let ptr = self.i3c;
        let _ = poll_with_timeout(
            move || unsafe { (*ptr).i3cd034().read().bits() },
            |val| val == 0,
            &mut self.yield_ns,
            100_000,
            1_000_000,
        );

        self.set_role(config.is_secondary);
        self.init_clock(config);

        self.i3c().i3cd03c().write(|w| unsafe { w.bits(0xffff_ffff) });
        if config.is_secondary {
            self.i3c().i3cd040().write(|w| {
                w.transfererrstaten()
                    .set_bit()
                    .respreadystatintren()
                    .set_bit()
                    .cccupdatedstaten()
                    .set_bit()
                    .dynaddrassgnstaten()
                    .set_bit()
                    .ibiupdatedstaten()
                    .set_bit()
                    .readreqrecvstaten()
                    .set_bit()
            });

            self.i3c().i3cd044().write(|w| {
                w.transfererrsignalen()
                    .set_bit()
                    .respreadysignalintren()
                    .set_bit()
                    .cccupdatedsignalen()
                    .set_bit()
                    .dynaddrassgnsignalen()
                    .set_bit()
                    .ibiupdatedsignalen()
                    .set_bit()
                    .readreqrecvsignalen()
                    .set_bit()
            });
        } else {
            self.i3c().i3cd040().write(|w| {
                w.transfererrstaten()
                    .set_bit()
                    .respreadystatintren()
                    .set_bit()
            });

            self.i3c().i3cd044().write(|w| {
                w.transfererrsignalen()
                    .set_bit()
                    .respreadysignalintren()
                    .set_bit()
            });
        }

        config.sir_allowed_by_sw = false;

        self.i3c()
            .i3cd01c()
            .write(|w| unsafe { w.ibidata_threshold_value().bits(31) });

        self.i3c()
            .i3cd020()
            .modify(|_, w| unsafe { w.rx_buffer_threshold_value().bits(0) });

        self.init_pid(config);

        config.maxdevs = self.i3c().i3cd05c().read().devaddrtabledepth().bits();
        config.free_pos = if config.maxdevs == 32 {
            u32::MAX
        } else {
            (1u32 << config.maxdevs) - 1
        };
        config.need_da = 0;

        for i in 0..(config.maxdevs) {
            i3c_dat_write!(self, i, |w| {
                w.sirreject().set_bit().mrreject().set_bit()
            });
        }

        self.i3c().i3cd02c().write(|w| unsafe { w.bits(0xffff_ffff) });
        self.i3c().i3cd030().write(|w| unsafe { w.bits(0xffff_ffff) });
        self.i3c()
            .i3cd000()
            .modify(|_, w| w.hot_join_ack_nack_ctrl().set_bit());

        if config.is_secondary {
            self.i3c()
                .i3cd004()
                .write(|w| unsafe { w.dev_static_addr().bits(9).static_addr_valid().set_bit() });
        } else {
            self.i3c()
                .i3cd004()
                .write(|w| unsafe { w.dev_dynamic_addr().bits(8).dynamic_addr_valid().set_bit() });
        }

        self.i3c_enable(config);

        if !config.is_secondary {
            self.i3c()
                .i3cd040()
                .modify(|_, w| w.ibithldstaten().set_bit());
            self.i3c()
                .i3cd044()
                .modify(|_, w| w.ibithldsignalen().set_bit());
        }
        self.i3c()
            .i3cd000()
            .modify(|_, w| w.hot_join_ack_nack_ctrl().clear_bit());

        // Safety: Ensure memory barrier and init completion before interrupts are enabled by the caller
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }

    fn bus_num(&self) -> u8 {
        self.bus_num
    }

    fn enable_irq(&mut self) {
        unsafe {
            match self.bus_num {
                0 => NVIC::unmask(ast1060_pac::Interrupt::i3c),
                1 => NVIC::unmask(ast1060_pac::Interrupt::i3c1),
                2 => NVIC::unmask(ast1060_pac::Interrupt::i3c2),
                3 => NVIC::unmask(ast1060_pac::Interrupt::i3c3),
                _ => {}
            }
        }
    }

    fn disable_irq(&mut self) {
        match self.bus_num {
            0 => NVIC::mask(ast1060_pac::Interrupt::i3c),
            1 => NVIC::mask(ast1060_pac::Interrupt::i3c1),
            2 => NVIC::mask(ast1060_pac::Interrupt::i3c2),
            3 => NVIC::mask(ast1060_pac::Interrupt::i3c3),
            _ => {}
        }
    }

    fn i3c_disable(&mut self, is_secondary: bool) {
        if self.i3c().i3cd000().read().enbl_i3cctrl().bit_is_clear() {
            return;
        }

        if is_secondary {
            self.enter_sw_mode();
        }
        self.i3c()
            .i3cd000()
            .modify(|_, w| w.enbl_i3cctrl().clear_bit());

        if is_secondary {
            self.toggle_scl_in(8);
            self.gen_internal_stop();
            self.exit_sw_mode();
        }
    }

    fn i3c_enable(&mut self, config: &I3cConfig) {
        if config.is_secondary {
            self.i3c().i3cd038().write(|w| unsafe { w.bits(0) });
            self.enter_sw_mode();
            self.i3c().i3cd000().modify(|_, w| {
                w.enbl_adaption_of_i2ci3cmode()
                    .clear_bit()
                    .ibipayloaden()
                    .set_bit()
                    .enbl_i3cctrl()
                    .set_bit()
            });
            let wait_cnt = &self.i3c().i3cd0d4().read().i3cibifree().bits();
            let wait_ns = u32::from(*wait_cnt) * config.core_period;
            (self.yield_ns)(wait_ns * 100_u32);
            self.toggle_scl_in(8);
            if self.i3c().i3cd000().read().enbl_i3cctrl().bit_is_set() {
                self.gen_internal_stop();
            }
            self.exit_sw_mode();
        } else {
            self.i3c().i3cd000().modify(|_, w| {
                w.i3cbroadcast_addr_include()
                    .set_bit()
                    .enbl_i3cctrl()
                    .set_bit()
            });
        }
    }

    fn set_role(&mut self, is_secondary: bool) {
        if is_secondary {
            self.i3c()
                .i3cd0b0()
                .modify(|_, w| unsafe { w.dev_op_mode().bits(1) });
        } else {
            self.i3c()
                .i3cd0b0()
                .modify(|_, w| unsafe { w.dev_op_mode().bits(0) });
        }
    }

    fn i3c_aspeed_isr(&mut self, config: &mut I3cConfig) {
        self.disable_irq();
        let status = self.i3c().i3cd03c().read().bits();
        if status == 0 {
            self.enable_irq();
            return;
        }

        if config.is_secondary {
            if status & INTR_DYN_ADDR_ASSGN_STAT != 0 {
                let da = self.i3c().i3cd004().read().dev_dynamic_addr().bits();
                if let Some(tc) = &mut config.target_config {
                    tc.addr = Some(da);
                }
                let _ = ibi_workq::i3c_ibi_work_enqueue_target_da_assignment(self.bus_num.into());
            }

            if (status & INTR_RESP_READY_STAT) != 0 {
                self.target_handle_response_ready(config);
            }

            if (status & INTR_CCC_UPDATED_STAT) != 0 {
                self.target_handle_ccc_update(config);
            }
        } else {
            if (status & (INTR_RESP_READY_STAT | INTR_TRANSFER_ERR_STAT | INTR_TRANSFER_ABORT_STAT))
                != 0
            {
                self.end_xfer(config);
            }

            if (status & INTR_IBI_THLD_STAT) != 0 {
                self.handle_ibis(config);
            }
        }

        self.i3c().i3cd03c().write(|w| unsafe { w.bits(status) });
        self.enable_irq();
    }
}

impl<'a, Y: FnMut(u32)> HardwareClock for Ast1060I3c<'a, Y> {
    fn init_clock(&mut self, config: &mut I3cConfig) {
        let clk_rate = config.core_clk_hz.expect("core_clk_hz must be configured");
        config.core_period = (NSEC_PER_SEC).div_ceil(clk_rate);

        let ns_to_cnt_u8 =
            |ns: u32| -> u8 { u8::try_from(ns.div_ceil(config.core_period)).unwrap_or(u8::MAX) };
        let ns_to_cnt_u16 =
            |ns: u32| -> u16 { u16::try_from(ns.div_ceil(config.core_period)).unwrap_or(u16::MAX) };

        // I2C FM
        let (fm_hi_ns, fm_lo_ns) = self.calc_i2c_clk(config.i2c_scl_hz);
        self.i3c().i3cd0bc().write(|w| unsafe {
            w.i2cfmhcnt()
                .bits(ns_to_cnt_u16(fm_hi_ns))
                .i2cfmlcnt()
                .bits(ns_to_cnt_u16(fm_lo_ns))
        });

        // I2C FMP
        let (i2c_fmp_hi_ns, i2c_fmp_lo_ns) = self.calc_i2c_clk(1_000_000);
        self.i3c().i3cd0c0().write(|w| unsafe {
            w.i2cfmphcnt()
                .bits(ns_to_cnt_u8(i2c_fmp_hi_ns))
                .i2cfmplcnt()
                .bits(ns_to_cnt_u16(i2c_fmp_lo_ns))
        });

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
        self.i3c().i3cd0b4().write(|w| unsafe {
            w.i3codhcnt()
                .bits(ns_to_cnt_u8(od_hi_ns))
                .i3codlcnt()
                .bits(ns_to_cnt_u8(od_lo_ns))
        });

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
        self.i3c().i3cd0b8().write(|w| unsafe {
            w.i3cpphcnt()
                .bits(ns_to_cnt_u8(i3c_pp_hi_ns))
                .i3cpplcnt()
                .bits(ns_to_cnt_u8(i3c_pp_lo_ns))
        });

        // SDA TX hold time
        let hold_steps = (config.sda_tx_hold_ns)
            .div_ceil(config.core_period)
            .clamp(SDA_TX_HOLD_MIN, SDA_TX_HOLD_MAX);
        let mut reg = self.i3c().i3cd0d0().read().bits();
        reg = (reg & !SDA_TX_HOLD_MASK) | ((hold_steps & 0x7) << 16);
        self.i3c().i3cd0d0().write(|w| unsafe { w.bits(reg) });

        // BUS_FREE_TIMING
        self.i3c().i3cd0d4().write(|w| unsafe { w.bits(0xffff_007c) });
    }

    fn calc_i2c_clk(&mut self, fscl_hz: u32) -> (u32, u32) {
        use core::cmp::max;

        debug_assert!(fscl_hz > 0);
        let period_ns: u32 = (1_000_000_000u32).div_ceil(fscl_hz);

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

        (hi as u32, lo)
    }

    fn init_pid(&mut self, config: &mut I3cConfig) {
        self.i3c()
            .i3cd070()
            .write(|w| unsafe { w.slvmipimfgid().bits(0x3f6).slvpiddcr().clear_bit() });

        // NOTE: rev_id must be set by caller via SCU before calling this
        // This method only handles the i3c register setup, not SCU reset/clock/etc
        self.i3c().i3cd074().write(|w| unsafe { w.bits(0xa000_0000) });
        let mut reg: u32 = self.i3c().i3cd078().read().bits();
        reg &= !SLV_DCR_MASK;
        reg |= (config.dcr << 8) | 0x66;
        self.i3c().i3cd078().write(|w| unsafe { w.bits(reg) });
    }
}

impl<'a, Y: FnMut(u32)> HardwareFifo for Ast1060I3c<'a, Y> {
    fn wr_tx_fifo(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(4);
        for chunk in &mut chunks {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            self.i3c()
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }

        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut tmp = [0u8; 4];
            tmp[..rem.len()].copy_from_slice(rem);
            let word = u32::from_le_bytes(tmp);
            self.i3c()
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }
    }

    fn rd_fifo<F>(&mut self, mut read_word: F, out: &mut [u8])
    where
        F: FnMut() -> u32,
    {
        let mut chunks = out.chunks_exact_mut(4);
        for chunk in &mut chunks {
            let val = read_word();
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let val = read_word();
            let bytes = val.to_le_bytes();
            rem.copy_from_slice(&bytes[..rem.len()]);
        }
    }

    fn drain_fifo<F>(&mut self, mut read_word: F, len: usize)
    where
        F: FnMut() -> u32,
    {
        let nwords = (len + 3) >> 2;
        for _ in 0..nwords {
            let _ = read_word();
        }
    }

    fn rd_rx_fifo(&mut self, out: &mut [u8]) {
        let ptr = self.i3c;
        self.rd_fifo(move || unsafe { (*ptr).i3cd014().read().rx_data_port().bits() }, out);
    }

    fn rd_ibi_fifo(&mut self, out: &mut [u8]) {
        let ptr = self.i3c;
        self.rd_fifo(move || unsafe { (*ptr).i3cd018().read().bits() }, out);
    }
}

impl<'a, Y: FnMut(u32)> HardwareRecovery for Ast1060I3c<'a, Y> {
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

impl<'drv, Y: FnMut(u32)> HardwareTransfer for Ast1060I3c<'drv, Y> {
    fn set_ibi_mdb(&mut self, mdb: u8) {
        self.i3c()
            .i3cd000()
            .modify(|_, w| unsafe { w.mdb().bits(mdb) });
    }

    fn exit_halt(&mut self, config: &mut I3cConfig) {
        let state = self.i3c().i3cd054().read().cmtfrstatus().bits();
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if state != expected {
            return;
        }

        self.i3c().i3cd000().modify(|_, w| w.i3cresume().set_bit());

        let ptr = self.i3c;
        let rc = poll_with_timeout(
            move || u32::from(unsafe { (*ptr).i3cd054().read().cmtfrstatus().bits() }),
            |val| val != u32::from(expected),
            &mut self.yield_ns,
            10000,
            1_000_000,
        );

        if rc.is_err() {
        }
    }

    fn enter_halt(&mut self, by_sw: bool, config: &mut I3cConfig) {
        let expected = if config.is_secondary {
            CM_TFR_STS_TARGET_HALT
        } else {
            CM_TFR_STS_MASTER_HALT
        };

        if by_sw {
            self.i3c().i3cd000().modify(|_, w| w.i3cabort().set_bit());
        }

        let ptr = self.i3c;
        let rc = poll_with_timeout(
            move || u32::from(unsafe { (*ptr).i3cd054().read().cmtfrstatus().bits() }),
            |val| val == u32::from(expected),
            &mut self.yield_ns,
            10000,
            1_000_000,
        );

        if rc.is_err() {
        }
    }

    fn reset_ctrl(&mut self, reset: u32) {
        let reg = reset & RESET_CTRL_ALL;

        if reg == 0 {
            return;
        }

        self.i3c().i3cd034().write(|w| unsafe { w.bits(reg) });
        let ptr = self.i3c;
        let rc = poll_with_timeout(
            move || unsafe { (*ptr).i3cd034().read().bits() },
            |val| val == 0,
            &mut self.yield_ns,
            10_000,
            1_000_000,
        );

        if rc.is_err() {
        }
    }

    fn ibi_enable(&mut self, config: &mut I3cConfig, addr: u8) -> Result<(), I3cDrvError> {
        let dev_idx = config
            .attached
            .find_dev_idx_by_addr(addr)
            .ok_or(I3cDrvError::NoSuchDev)?;
        let pos_opt = config
            .attached
            .pos_of(dev_idx)
            .or(config.attached.devices[dev_idx].pos);

        let pos: u8 = pos_opt.ok_or(I3cDrvError::NoDatPos)?;
        let dev = &config.attached.devices[dev_idx];
        let tgt_bcr: u32 = u32::from(dev.bcr);
        let mut reg = i3c_dat_read!(self, u32::from(pos));
        reg &= !DEV_ADDR_TABLE_SIR_REJECT;
        if tgt_bcr & I3C_BCR_IBI_PAYLOAD_HAS_DATA_BYTE != 0 {
            reg |= DEV_ADDR_TABLE_IBI_MDB | DEV_ADDR_TABLE_IBI_PEC;
        }

        i3c_dat_write!(self, pos, |w| unsafe { w.bits(reg) });

        let mut sir_reject = self.i3c().i3cd030().read().bits();
        sir_reject &= !bit(pos.into());
        self.i3c().i3cd030().write(|w| unsafe { w.bits(sir_reject) });

        self.i3c()
            .i3cd040()
            .modify(|_, w| w.ibithldstaten().set_bit());

        self.i3c()
            .i3cd044()
            .modify(|_, w| w.ibithldsignalen().set_bit());

        let events = I3C_CCC_EVT_INTR;
        // ccc_events_set requires HardwareTransfer trait bound on Self.
        // We are inside HardwareTransfer impl for Ast1060I3c.
        // Rust might have trouble inferring if Self: HardwareTransfer is not fully established yet?
        // But Ast1060I3c implements HardwareTransfer (this block).
        // However, ccc_events_set takes `&mut impl HardwareInterface`.
        // Ast1060I3c implements HardwareInterface (blanket impl over all sub-traits).
        // So this call should be valid.
        let _ = ccc_events_set(self, config, dev.dyn_addr, true, events);

        Ok(())
    }

    fn start_xfer(&mut self, config: &mut I3cConfig, xfer: &mut I3cXfer) {
        let prev = config
            .curr_xfer
            .swap(core::ptr::from_mut(xfer).cast::<()>(), Ordering::AcqRel);
        if !prev.is_null() {
        }

        xfer.ret = -1;
        xfer.done.reset();

        for cmd in xfer.cmds.iter() {
            if let Some(tx) = cmd.tx {
                let take = tx.len().min(cmd.tx_len as usize);
                if take > 0 {
                    self.wr_tx_fifo(&tx[..take]);
                }
            }
        }
        self.i3c().i3cd01c().modify(|_, w| unsafe {
            w.response_buffer_threshold_value()
                .bits(u8::try_from(xfer.cmds.len().saturating_sub(1)).unwrap_or(0))
        });

        for cmd in xfer.cmds.iter() {
            self.i3c().i3cd00c().write(|w| unsafe { w.bits(cmd.cmd_hi) });
            self.i3c().i3cd00c().write(|w| unsafe { w.bits(cmd.cmd_lo) });
        }
    }

    fn end_xfer(&mut self, config: &mut I3cConfig) {
        let p = config
            .curr_xfer
            .swap(core::ptr::null_mut(), Ordering::AcqRel);

        if p.is_null() {
            // Drain the response queue to prevent interrupt loops if no xfer is active
            let nresp = self.i3c().i3cd04c().read().respbufblr().bits() as usize;
            for _ in 0..nresp {
                let _ = self.i3c().i3cd010().read().bits();
            }
            return;
        }

        let xfer: &mut I3cXfer = unsafe { &mut *(p.cast::<I3cXfer>()) };

        let nresp = self.i3c().i3cd04c().read().respbufblr().bits() as usize;

        for _ in 0..nresp {
            let resp = self.i3c().i3cd010().read().bits();

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

            if tid >= xfer.cmds.len() {
                if rx_len > 0 {
                    { let ptr = self.i3c; self.drain_fifo(move || unsafe { (*ptr).i3cd014().read().rx_data_port().bits() }, rx_len); }
                }
                continue;
            }

            let cmd = &mut xfer.cmds[tid];
            cmd.rx_len = u32::try_from(rx_len).unwrap_or(0);
            cmd.ret = i32::try_from(err).unwrap_or(-1);

            if rx_len == 0 {
                continue;
            }

            if err == 0 {
                if let Some(rx_buf) = cmd.rx.as_deref_mut() {
                    self.rd_rx_fifo(&mut rx_buf[..rx_len]);
                } else {
                    // Drain if no buffer provided
                    { let ptr = self.i3c; self.drain_fifo(move || unsafe { (*ptr).i3cd014().read().rx_data_port().bits() }, rx_len); }
                }
            } else if rx_len > 0 {
                { let ptr = self.i3c; self.drain_fifo(move || unsafe { (*ptr).i3cd014().read().rx_data_port().bits() }, rx_len); }
            }
        }
        let mut ret = 0;
        for i in 0..nresp {
            let r = xfer.cmds[i].ret;
            if r != 0 {
                ret = r;
            }
        }

        if ret != 0 {
            self.enter_halt(false, config);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.exit_halt(config);
        }

        xfer.ret = ret;
        xfer.done.complete();
    }

    fn get_addr_pos(&mut self, config: &I3cConfig, addr: u8) -> Option<u8> {
        config
            .addrs
            .iter()
            .take(config.maxdevs as usize)
            .position(|&a| a == addr)
            .and_then(|i| u8::try_from(i).ok())
    }

    fn detach_i3c_dev(&mut self, pos: usize) {
        i3c_dat_write!(self, pos, |w| {
            w.sirreject().set_bit().mrreject().set_bit()
        });
    }

    fn attach_i3c_dev(&mut self, pos: usize, addr: u8) -> Result<(), I3cDrvError> {
        let mut da_with_parity = addr;
        if Self::even_parity(addr) {
            da_with_parity |= 1 << 7;
        }

        i3c_dat_write!(self, pos, |w| unsafe {
            w.sirreject()
                .set_bit()
                .mrreject()
                .set_bit()
                .devdynamicaddr()
                .bits(da_with_parity)
        });

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

                if data_len > 0 {
                    if let Some(d) = payload.ccc.as_ref().and_then(|c| c.data.as_deref()) {
                        cmd.tx = Some(d);
                        cmd.tx_len = u32::try_from(data_len).map_err(|_| I3cDrvError::Invalid)?;
                    }
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
                pos = match pos_ops {
                    Some(p) => p,
                    None => return Err(I3cDrvError::Invalid),
                };

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

        if !xfer.done.wait_for_us(1_000_000_000, &mut self.yield_ns) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
        }

        let ret = xfer.ret;
        if ret == i32::try_from(RESPONSE_ERROR_IBA_NACK).map_err(|_| I3cDrvError::Invalid)? {
            return Ok(());
        }

        if is_broadcast {
            if let Some(ccc_rw) = payload.ccc.as_mut() {
                if let Some(d) = ccc_rw.data.as_deref() {
                    ccc_rw.num_xfer = d.len();
                }
            }
        }

        match ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Invalid),
        }
    }

    fn do_entdaa(&mut self, config: &mut I3cConfig, pos: u32) -> Result<(), I3cDrvError> {
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

        let mut cmds = [cmd];
        let mut xfer = I3cXfer::new(&mut cmds[..]);
        xfer.ret = -1;

        self.start_xfer(config, &mut xfer);

        if !xfer.done.wait_for_us(1_000_000_000, &mut self.yield_ns) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
            return Err(I3cDrvError::Invalid);
        }

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

        for i in 0..cmds_len {
            let (is_read, ptr, len) = {
                let m = &mut msgs[i];
                let is_read = (m.flags & I3C_MSG_READ) != 0;

                if is_read {
                    let buf = match m.buf.as_deref_mut() {
                        Some(b) if !b.is_empty() => b,
                        _ => return Err(I3cDrvError::Invalid),
                    };
                    (true, buf.as_mut_ptr(), buf.len())
                } else {
                    let buf = match m.buf.as_deref() {
                        Some(b) if !b.is_empty() => b,
                        _ => return Err(I3cDrvError::Invalid),
                    };
                    m.num_xfer = u32::try_from(buf.len()).map_err(|_| I3cDrvError::Invalid)?;
                    (false, buf.as_ptr().cast_mut(), buf.len())
                }
            };

            let cmd = &mut cmds[i];
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
                let rx_slice: &'a mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
                cmd.rx = Some(rx_slice);
                cmd.rx_len = u32::try_from(len).map_err(|_| I3cDrvError::Invalid)?;
                cmd.cmd_lo |= COMMAND_PORT_READ_TRANSFER;
            } else {
                let tx_slice: &'a [u8] =
                    unsafe { core::slice::from_raw_parts(ptr.cast_const(), len) };
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

        let mut cmds: heapless::Vec<I3cCmd, MAX_CMDS> = heapless::Vec::new();
        for _ in 0..msgs.len() {
            cmds.push(I3cCmd {
                cmd_lo: 0,
                cmd_hi: 0,
                tx: None,
                rx: None,
                tx_len: 0,
                rx_len: 0,
                ret: 0,
            })
            .unwrap();
        }

        let ret = self.priv_xfer_build_cmds(cmds.as_mut_slice(), msgs, pos);
        match ret {
            Ok(()) => {}
            Err(e) => return Err(e),
        }

        let mut xfer = I3cXfer::new(cmds.as_mut_slice());
        self.start_xfer(config, &mut xfer);

        if !xfer.done.wait_for_us(1_000_000_000, &mut self.yield_ns) {
            self.enter_halt(true, config);
            self.reset_ctrl(RESET_CTRL_XFER_QUEUES);
            self.exit_halt(config);
            let _ = config
                .curr_xfer
                .swap(core::ptr::null_mut(), Ordering::AcqRel);
            return Err(I3cDrvError::Timeout);
        }

        for (i, m) in msgs.iter_mut().enumerate() {
            if (m.flags & I3C_MSG_READ) != 0 {
                m.actual_len = xfer.cmds[i].rx_len;
            }
        }

        match xfer.ret {
            0 => Ok(()),
            _ => Err(I3cDrvError::Timeout),
        }
    }

    fn handle_ibi_sir(&mut self, config: &mut I3cConfig, addr: u8, len: usize) {
        let pos = config.attached.pos_of_addr(addr);
        if pos.is_none() {
            { let ptr = self.i3c; self.drain_fifo(move || unsafe { (*ptr).i3cd018().read().bits() }, len); }
        }

        let mut ibi_buf: [u8; 2] = [0u8; 2];
        let take = core::cmp::min(len, ibi_buf.len());
        self.rd_ibi_fifo(&mut ibi_buf[..take]);
        let bus = self.bus_num as usize;
        let _ = ibi_workq::i3c_ibi_work_enqueue_target_irq(bus, addr, &ibi_buf[..take]);
    }

    fn handle_ibis(&mut self, config: &mut I3cConfig) {
        let nibis = self.i3c().i3cd04c().read().ibistatuscnt().bits();

        if nibis == 0 {
            return;
        }

        for _ in 0..nibis {
            let reg = self.i3c().i3cd018().read().bits();

            let ibi_id = field_get(reg, IBIQ_STATUS_IBI_ID, IBIQ_STATUS_IBI_ID_SHIFT);
            let ibi_data_len = field_get(
                reg,
                IBIQ_STATUS_IBI_DATA_LEN,
                IBIQ_STATUS_IBI_DATA_LEN_SHIFT,
            ) as usize;
            let ibi_addr = (ibi_id >> 1) & 0x7F;
            let rnw = (ibi_id & 1) != 0;
            if ibi_addr != 2 && rnw {
                // sirq
                self.handle_ibi_sir(config, ibi_addr as u8, ibi_data_len);
            } else if ibi_addr == 2 && !rnw {
                // hot-join
                let bus = self.bus_num as usize;
                let _ = ibi_workq::i3c_ibi_work_enqueue_hotjoin(bus);
            } else {
                // normal ibi
                { let ptr = self.i3c; self.drain_fifo(move || unsafe { (*ptr).i3cd018().read().bits() }, ibi_data_len); }
            }
        }
    }
}

impl<'a, Y: FnMut(u32)> HardwareTarget for Ast1060I3c<'a, Y> {
    fn target_tx_write(&mut self, buf: &[u8]) {
        self.wr_tx_fifo(buf);
        let cmd = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(
                COMMAND_PORT_ARG_DATA_LEN,
                u32::try_from(buf.len()).map_or(0, |v| v),
            )
            | field_prep(COMMAND_PORT_TID, Tid::TargetRdData as u32);

        self.i3c().i3cd00c().write(|w| unsafe { w.bits(cmd) });
    }

    fn target_ibi_raise_hj(&self, config: &mut I3cConfig) -> Result<(), I3cDrvError> {
        if !config.is_secondary {
            return Err(I3cDrvError::Invalid);
        }
        let hj_support = self.i3c().i3cd008().read().slvhjcap().bit();
        if !hj_support {
            return Err(I3cDrvError::Invalid);
        }

        let addr_valid = self.i3c().i3cd004().read().dynamic_addr_valid().bit();
        if addr_valid {
            return Err(I3cDrvError::Access);
        }

        self.i3c().i3cd038().write(|w| unsafe { w.bits(8) }); // set HJ request

        Ok(())
    }

    fn target_handle_response_ready(&mut self, config: &mut I3cConfig) {
        let nresp = self.i3c().i3cd04c().read().respbufblr().bits();

        for _ in 0..nresp {
            let resp = self.i3c().i3cd010().read().bits();

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
                self.enter_halt(false, config);
                self.reset_ctrl(RESET_CTRL_QUEUES);
                self.exit_halt(config);
                continue;
            }

            if rx_len != 0 {
                let mut buf: [u8; 256] = [0u8; 256];
                self.rd_rx_fifo(&mut buf[..rx_len]);
            }

            if tid == Tid::TargetIbi as usize {
                config.target_ibi_done.complete();
            }

            if tid == Tid::TargetRdData as usize {
                config.target_data_done.complete();
            }
        }
    }

    fn target_pending_read_notify(
        &mut self,
        config: &mut I3cConfig,
        buf: &[u8],
        notifier: &mut I3cIbi,
    ) -> Result<(), I3cDrvError> {
        let reg = self.i3c().i3cd038().read().bits();
        if !(config.sir_allowed_by_sw && (reg & SLV_EVENT_CTRL_SIR_EN != 0)) {
            return Err(I3cDrvError::Access);
        }

        let Some(mdb) = notifier.first_byte() else {
            return Err(I3cDrvError::Invalid);
        };

        self.set_ibi_mdb(mdb);
        if let Some(p) = notifier.payload {
            if !p.is_empty() {
                self.wr_tx_fifo(p);
            }
        }

        let payload_len = u32::try_from(notifier.payload.map_or(0, <[u8]>::len))
            .map_err(|_| I3cDrvError::Invalid)?;
        let cmd: u32 = field_prep(COMMAND_PORT_ATTR, COMMAND_ATTR_SLAVE_DATA_CMD)
            | field_prep(COMMAND_PORT_ARG_DATA_LEN, payload_len)
            | field_prep(COMMAND_PORT_TID, Tid::TargetIbi as u32);
        self.i3c().i3cd00c().write(|w| unsafe { w.bits(cmd) });

        config.target_ibi_done.reset();

        self.i3c()
            .i3cd01c()
            .modify(|_, w| unsafe { w.response_buffer_threshold_value().bits(0) });

        self.target_tx_write(buf);
        config.target_data_done.reset();

        self.i3c().i3cd08c().write(|w| w.sir().set_bit());

        if !config
            .target_ibi_done
            .wait_for_us(1_000_000_000, &mut self.yield_ns)
        {
            self.enter_halt(false, config);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.exit_halt(config);
            return Err(I3cDrvError::IoError);
        }

        if !config
            .target_data_done
            .wait_for_us(1_000_000_000, &mut self.yield_ns)
        {
            self.i3c_disable(config.is_secondary);
            self.reset_ctrl(RESET_CTRL_QUEUES);
            self.i3c_enable(config);
            return Err(I3cDrvError::Timeout);
        }

        Ok(())
    }

    fn target_handle_ccc_update(&mut self, config: &mut I3cConfig) {
        let event = self.i3c().i3cd038().read().bits();
        self.i3c().i3cd038().write(|w| unsafe { w.bits(event) });
        let reg = self.i3c().i3cd054().read().cmtfrstatus().bits();
        if reg == CM_TFR_STS_TARGET_HALT {
            self.enter_halt(true, config);
            self.exit_halt(config);
        }
    }
}
