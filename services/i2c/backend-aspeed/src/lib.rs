// Licensed under the Apache-2.0 license

//! AST1060 I2C Backend for the OpenPRoT I2C Server
//!
//! Thin adapter wrapping [`aspeed_ddk::i2c_core`] to provide I2C operations for
//! the server dispatch loop.
//!
//! # Two-Layer Init Model
//!
//! I2C initialization is split across two trust boundaries:
//!
//! | Layer | What | Where | Why |
//! |-------|------|-------|-----|
//! | **Platform** | SCU reset, I2CG0C/I2CG10, pinmux | `entry.rs` (kernel boot) | Shared SCU registers — single-threaded, no contention |
//! | **Per-bus** | I2CC00, timing, interrupts | `AspeedI2cBackend::init_bus()` (server task) | Per-controller MMIO, owned by this server |
//!
//! # Architecture
//!
//! ```text
//! Kernel entry.rs (boot, single-threaded):
//!   init_i2c_global()       ← SCU reset + I2CG0C/I2CG10
//!   apply_pinctrl_group()   ← SCU4xx pin mux (all buses)
//!
//! Server task (per process):
//!   AspeedI2cBackend::new()           ← steal PAC peripherals
//!   backend.init_bus(0)?              ← Ast1060I2c::new() → I2CC00, timing, IRQs
//!
//!   per operation:
//!     controller_regs(bus)
//!     Ast1060I2c::from_initialized()  ← zero register writes (~50x faster)
//!     i2c.write() / read() / ...
//! ```
//!
//! # Why Not `new()` Per Operation?
//!
//! Hubris called `Ast1060I2c::new()` on every operation, re-initializing
//! controller registers each time. We call `new()` once in `init_bus()`,
//! then use `from_initialized()` per-operation for zero-overhead access.

#![no_std]

use aspeed_ddk::i2c_core::{Ast1060I2c, Controller as DdkController, I2cConfig, I2cError, SlaveConfig, SlaveEvent};
use i2c_api::{ResponseCode, SlaveEventKind};

use pw_log;

// ---------------------------------------------------------------------------
// I2C Slave Command Register Constants
// ---------------------------------------------------------------------------
// These are copied from aspeed_ddk::i2c_core::constants which is private.

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/// Map aspeed-ddk [`I2cError`] to wire protocol [`ResponseCode`].
///
/// This is the single point where hardware errors become IPC response codes.
/// The mapping is intentionally conservative — ambiguous hardware errors map
/// to `IoError` rather than more specific codes.
fn map_i2c_error(e: I2cError) -> ResponseCode {
    match e {
        I2cError::NoAcknowledge => ResponseCode::NoDevice,
        I2cError::ArbitrationLoss => ResponseCode::ArbitrationLost,
        I2cError::Timeout => ResponseCode::Timeout,
        I2cError::Busy => ResponseCode::Busy,
        I2cError::InvalidAddress => ResponseCode::InvalidAddress,
        I2cError::BusRecoveryFailed => ResponseCode::BusStuck,
        I2cError::Invalid => ResponseCode::ServerError,
        // Hardware errors without a specific wire code → IoError
        I2cError::Bus | I2cError::Overrun | I2cError::Abnormal | I2cError::SlaveError => {
            ResponseCode::IoError
        }
    }
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

/// AST1060 I2C backend owning all peripheral register blocks.
///
/// Constructed once at server startup via [`AspeedI2cBackend::new()`],
/// then each bus must be initialized via [`init_bus()`] before use.
///
/// Each operation creates a transient [`Ast1060I2c`] handle via
/// `from_initialized()` on the stack — no heap, no persistent driver state.
///
/// # Safety Model
///
/// The backend exclusively owns `ast1060_pac::Peripherals`, ensuring no
/// aliased register access. The `unsafe` is confined to [`new()`] where
/// `Peripherals::steal()` is called.

/// Maximum RX buffer size for buffered slave notifications (SMBus max payload).
const SLAVE_RX_BUF_SIZE: usize = 255;

/// Per-bus state for interrupt-driven slave receive notifications.
///
/// When enabled, `drain_slave_rx()` is called from the IRQ handler to latch
/// incoming data into `rx_buf` without any polling loop. The server then
/// signals the registered client via `raise_peer_user_signal`; the client
/// retrieves the data with `get_buffered_slave_message()`.
///
/// # Why a flat buffer instead of a ring buffer
///
/// The AST1060 hardware packet mode automatically NACKs new master writes
/// until DMA is re-armed. MCTP DSP0236 flow control also prevents back-to-back
/// bursts. A single flat buffer is therefore sufficient and matches the Hubris
/// reference implementation.
#[derive(Clone, Copy)]
struct SlaveNotificationState {
    /// Whether interrupt-driven notification is active for this bus.
    enabled: bool,
    /// Flat receive buffer — holds at most one MCTP packet at a time.
    rx_buf: [u8; SLAVE_RX_BUF_SIZE],
    /// Number of valid bytes currently in `rx_buf` (0 means empty).
    rx_len: usize,
}

impl SlaveNotificationState {
    const fn new() -> Self {
        Self {
            enabled: false,
            rx_buf: [0u8; SLAVE_RX_BUF_SIZE],
            rx_len: 0,
        }
    }
}

const EMPTY_NOTIF_STATE: SlaveNotificationState = SlaveNotificationState::new();

pub struct AspeedI2cBackend {
    peripherals: ast1060_pac::Peripherals,
    /// Tracks which buses have been initialized via `init_bus()`.
    initialized: u16,
    /// Tracks which buses have slave mode configured via `configure_slave()`.
    slave_configured: u16,
    /// Per-bus state for interrupt-driven slave receive notifications.
    slave_notification: [SlaveNotificationState; 14],
}

impl AspeedI2cBackend {
    /// Create the backend by stealing PAC peripherals.
    ///
    /// # Safety
    ///
    /// Must only be called once. Caller must ensure exclusive access to I2C
    /// peripherals for the lifetime of this backend.
    ///
    /// After construction, call [`init_bus()`] for each bus this server owns.
    pub unsafe fn new() -> Self {
        Self {
            peripherals: unsafe { ast1060_pac::Peripherals::steal() },
            initialized: 0,
            slave_configured: 0,
            slave_notification: [EMPTY_NOTIF_STATE; 14],
        }
    }

    /// Initialize a single I2C bus controller.
    ///
    /// Performs per-controller hardware setup: I2CC00 reset, master enable,
    /// timing configuration, and interrupt enable. Must be called once per bus
    /// before any operations on that bus.
    ///
    /// # Preconditions
    ///
    /// Platform init (`entry.rs`) must have already run:
    /// - `init_i2c_global()` — SCU reset, I2CG0C, I2CG10
    /// - `apply_pinctrl_group()` — SCU pin mux for this bus
    ///
    /// # Errors
    ///
    /// Returns `ResponseCode::InvalidBus` if `bus > 13`, or
    /// `ResponseCode::IoError` if hardware init fails.
    pub fn init_bus(&mut self, bus: u8) -> Result<(), ResponseCode> {
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        // Ast1060I2c::new() calls init_hardware() which configures:
        //   - I2CC00: reset, master enable, bus auto-release
        //   - Timing registers per I2cConfig
        //   - I2CM10/I2CM14: interrupt enable/clear
        let _i2c = Ast1060I2c::new(&ctrl, I2cConfig::default())
            .map_err(map_i2c_error)?;
        self.initialized |= 1 << bus;
        pw_log::info!("I2C bus {} controller initialized", bus as u32);
        Ok(())
    }

    /// Check whether a bus has been initialized.
    #[inline]
    fn is_bus_initialized(&self, bus: u8) -> bool {
        bus < 14 && (self.initialized & (1 << bus)) != 0
    }

    /// Map bus index (0–13) to PAC register block references.
    ///
    /// The AST1060 has 14 I2C controllers. Bus indices beyond 13 return
    /// `ResponseCode::InvalidBus`.
    fn controller_regs(
        &self,
        bus: u8,
    ) -> Result<
        (
            &ast1060_pac::i2c::RegisterBlock,
            &ast1060_pac::i2cbuff::RegisterBlock,
        ),
        ResponseCode,
    > {
        match bus {
            0 => Ok((&self.peripherals.i2c, &self.peripherals.i2cbuff)),
            1 => Ok((&self.peripherals.i2c1, &self.peripherals.i2cbuff1)),
            2 => Ok((&self.peripherals.i2c2, &self.peripherals.i2cbuff2)),
            3 => Ok((&self.peripherals.i2c3, &self.peripherals.i2cbuff3)),
            4 => Ok((&self.peripherals.i2c4, &self.peripherals.i2cbuff4)),
            5 => Ok((&self.peripherals.i2c5, &self.peripherals.i2cbuff5)),
            6 => Ok((&self.peripherals.i2c6, &self.peripherals.i2cbuff6)),
            7 => Ok((&self.peripherals.i2c7, &self.peripherals.i2cbuff7)),
            8 => Ok((&self.peripherals.i2c8, &self.peripherals.i2cbuff8)),
            9 => Ok((&self.peripherals.i2c9, &self.peripherals.i2cbuff9)),
            10 => Ok((&self.peripherals.i2c10, &self.peripherals.i2cbuff10)),
            11 => Ok((&self.peripherals.i2c11, &self.peripherals.i2cbuff11)),
            12 => Ok((&self.peripherals.i2c12, &self.peripherals.i2cbuff12)),
            13 => Ok((&self.peripherals.i2c13, &self.peripherals.i2cbuff13)),
            _ => Err(ResponseCode::InvalidBus),
        }
    }

    // -----------------------------------------------------------------------
    // Master operations
    // -----------------------------------------------------------------------

    /// Write data to an I2C device.
    ///
    /// Creates a transient `Ast1060I2c` via `from_initialized()` for the
    /// specified bus, performs the write, and drops the handle.
    ///
    /// # Errors
    ///
    /// Returns `ResponseCode::ServerError` if the bus was not initialized
    /// via [`init_bus()`].
    pub fn write(&mut self, bus: u8, addr: u8, data: &[u8]) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.write(addr, data).map_err(map_i2c_error)
    }

    /// Read data from an I2C device.
    pub fn read(&mut self, bus: u8, addr: u8, buf: &mut [u8]) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.read(addr, buf).map_err(map_i2c_error)
    }

    /// Write-then-read (combined transaction with repeated START).
    pub fn write_read(
        &mut self,
        bus: u8,
        addr: u8,
        wr: &[u8],
        rd: &mut [u8],
    ) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.write_read(addr, wr, rd).map_err(map_i2c_error)
    }

    /// Attempt bus recovery (clock stretching / stuck SDA).
    pub fn recover_bus(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.recover_bus().map_err(map_i2c_error)
    }

    // -----------------------------------------------------------------------
    // Slave operations
    // -----------------------------------------------------------------------

    /// Configure a bus as an I2C slave at the given 7-bit address.
    ///
    /// The bus must have been initialized via [`init_bus()`] first.
    /// After configuration, call [`enable_slave()`] to start receiving.
    pub fn configure_slave(&mut self, bus: u8, addr: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        let config = SlaveConfig::new(addr).map_err(map_i2c_error)?;
        i2c.configure_slave(&config).map_err(map_i2c_error)?;
        self.slave_configured |= 1 << bus;
        pw_log::info!("I2C bus {} slave configured at address 0x{:02x}", bus as u32, addr as u32);
        Ok(())
    }

    /// Enable slave receive mode on a previously configured bus.
    pub fn enable_slave(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if (self.slave_configured & (1 << bus)) == 0 {
            return Err(ResponseCode::NotInitialized);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.enable_slave();
        pw_log::info!("I2C bus {} slave enabled", bus as u32);
        Ok(())
    }

    /// Disable slave receive mode on a bus.
    pub fn disable_slave(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        i2c.disable_slave();
        pw_log::info!("I2C bus {} slave disabled", bus as u32);
        Ok(())
    }

    /// Pre-load the TX buffer for a bus.
    ///
    /// Data stored here will be sent to the master when it issues a read
    /// transaction to our slave address, as part of `slave_wait_event`.
    pub fn slave_set_response(&mut self, bus: u8, data: &[u8]) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if (self.slave_configured & (1 << bus)) == 0 {
            return Err(ResponseCode::NotInitialized);
        }

        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());
        let n = i2c.slave_write(data).map_err(map_i2c_error)?;
        pw_log::info!("slave_set_response slave write length should be 1 (len={})", n as u8);
        Ok(())
    }

    /// Block until the next slave event, handling reads automatically.
    ///
    /// On `DataReceived`, reads bytes into `rx_buf` and returns
    /// `(SlaveEventKind::DataReceived, n)` where `n` is the byte count.
    ///
    /// On `ReadRequest`, sends the pre-loaded TX buffer (set via
    /// [`slave_set_response`]) and returns `(SlaveEventKind::ReadRequest, 0)`.
    ///
    /// On `Stop`, returns `(SlaveEventKind::Stop, 0)`.
    pub fn slave_wait_event(
        &mut self,
        bus: u8,
        rx_buf: &mut [u8],
    ) -> Result<(SlaveEventKind, usize), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if (self.slave_configured & (1 << bus)) == 0 {
            return Err(ResponseCode::NotInitialized);
        }

        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());

        const POLL_BUDGET: usize = 10_000;
        for _ in 0..POLL_BUDGET {
            match i2c.handle_slave_interrupt() {
                Some(SlaveEvent::DataReceived { len: _ }) => {
                    let n = i2c.slave_read(rx_buf).map_err(map_i2c_error)?;
                    pw_log::info!("slave_wait_event:: SlaveEvent::DataReceived (len={})", n as u8);
                    return Ok((SlaveEventKind::DataReceived, n));
                }
                Some(SlaveEvent::ReadRequest) => {
                    // TX buffer was pre-armed in slave_set_response(), so the hardware
                    // should respond automatically. We just need to wait for DataSent.
                    pw_log::info!("slave_wait_event:: SlaveEvent::ReadRequest");
                    continue;
                }
                Some(SlaveEvent::DataSent { len: _ }) => {
                    pw_log::info!("slave_wait_event:: SlaveEvent::DataSent");
                    return Ok((SlaveEventKind::DataSent, 1));
                }
                Some(SlaveEvent::Stop) => {
                    pw_log::info!("slave_wait_event:: SlaveEvent::Stop");
                    return Ok((SlaveEventKind::Stop, 0));
                }
                Some(SlaveEvent::WriteRequest) | None => {
                    continue;
                }
            }
        }

        Err(ResponseCode::Timeout)
    }

    /// Poll for received slave data, blocking until data arrives or timeout.
    ///
    /// Returns the number of bytes written into `buf`, or 0 if a Stop was
    /// received before any data (e.g. a probe). Returns `ResponseCode::Timeout`
    /// if no activity is seen within the polling budget.
    pub fn slave_receive(&mut self, bus: u8, buf: &mut [u8]) -> Result<usize, ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if (self.slave_configured & (1 << bus)) == 0 {
            return Err(ResponseCode::NotInitialized);
        }
        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());

        // Polling loop — spins until a relevant event or budget exhausted.
        // At 200 MHz, ~10_000 iterations ≈ a few hundred microseconds.
        const POLL_BUDGET: usize = 10_000;
        for _ in 0..POLL_BUDGET {
            match i2c.handle_slave_interrupt() {
                Some(SlaveEvent::DataReceived { len: _ }) => {
                    let n = i2c.slave_read(buf).map_err(map_i2c_error)?;
                    return Ok(n);
                }
                Some(SlaveEvent::Stop) => {
                    // Stop without data (e.g. probe or zero-length write).
                    return Ok(0);
                }
                Some(SlaveEvent::WriteRequest) | Some(SlaveEvent::ReadRequest) => {
                    // Transaction in progress — keep polling for data.
                    continue;
                }
                Some(SlaveEvent::DataSent { len: _ }) => {
                    // Master read from us; not relevant for receive path.
                    continue;
                }
                None => {
                    // No hardware event yet — keep polling.
                    continue;
                }
            }
        }

        Err(ResponseCode::Timeout)
    }

    // -----------------------------------------------------------------------
    // Interrupt-driven slave notification (Phase 3)
    // -----------------------------------------------------------------------

    /// Enable interrupt-driven slave receive notifications for a bus.
    ///
    /// After this call, `drain_slave_rx()` should be invoked from the server's
    /// IRQ handler whenever the I2C interrupt fires. The hardware interrupt was
    /// already enabled by `enable_slave()`; this simply arms the backend flat
    /// buffer so it is ready to latch received packets.
    pub fn enable_slave_notification(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if (self.slave_configured & (1 << bus)) == 0 {
            return Err(ResponseCode::NotInitialized);
        }
        self.slave_notification[bus as usize].enabled = true;
        self.slave_notification[bus as usize].rx_len = 0;
        pw_log::info!("I2C bus {} slave notification enabled", bus as u32);
        Ok(())
    }

    /// Disable interrupt-driven slave receive notifications for a bus.
    pub fn disable_slave_notification(&mut self, bus: u8) -> Result<(), ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        self.slave_notification[bus as usize].enabled = false;
        self.slave_notification[bus as usize].rx_len = 0;
        pw_log::info!("I2C bus {} slave notification disabled", bus as u32);
        Ok(())
    }

    /// Consume one hardware slave interrupt and latch any received data.
    ///
    /// Called once from the server's IRQ handler — no polling loop.  If a
    /// `DataReceived` event is pending, the bytes are copied into the per-bus
    /// flat buffer and the byte count is returned.  Returns `Ok(0)` for any
    /// non-data event so the caller can distinguish a real receive from a
    /// spurious wakeup:
    ///
    /// | Event | Return |
    /// |---|---|
    /// | `DataReceived` | `Ok(n)` where n > 0 |
    /// | `Stop` | `Ok(0)` — master ended transaction, no data |
    /// | `WriteRequest` | `Ok(0)` — address phase only; `DataReceived` follows |
    /// | `ReadRequest` / `DataSent` | `Ok(0)` — master-read path, not RX |
    /// | None | `Ok(0)` — no event pending |
    ///
    /// # Errors
    ///
    /// Returns `ResponseCode::ServerError` if the bus is not initialized, or
    /// `ResponseCode::NotInitialized` if notification has not been enabled for
    /// this bus via `enable_slave_notification()`.
    pub fn drain_slave_rx(&mut self, bus: u8) -> Result<usize, ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if !self.slave_notification[bus as usize].enabled {
            return Err(ResponseCode::NotInitialized);
        }

        let (regs, buffs) = self.controller_regs(bus)?;
        let ctrl = aspeed_ddk::i2c_core::I2cController {
            controller: DdkController(bus),
            registers: regs,
            buff_registers: buffs,
        };
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::default());

        // Use a local stack buffer so we can release the peripheral borrow
        // before writing into self.slave_notification.
        let mut local_rx = [0u8; SLAVE_RX_BUF_SIZE];
        let len = match i2c.handle_slave_interrupt() {
            Some(SlaveEvent::DataReceived { len: _ }) => {
                // Data is ready in the hardware buffer — read it out.
                i2c.slave_read(&mut local_rx).map_err(map_i2c_error)?
            }
            Some(SlaveEvent::Stop) => {
                // Master ended the transaction without data (e.g. probe or
                // zero-length write). Not an error; nothing to latch.
                0
            }
            Some(SlaveEvent::WriteRequest) => {
                // Address phase seen but DataReceived not yet posted.
                // The hardware will fire another interrupt when data arrives.
                0
            }
            Some(SlaveEvent::ReadRequest) | Some(SlaveEvent::DataSent { len: _ }) => {
                // Master-read path; not relevant for the slave-RX notification.
                0
            }
            None => 0,
        };
        drop(i2c);

        if len > 0 {
            let state = &mut self.slave_notification[bus as usize];
            state.rx_buf[..len].copy_from_slice(&local_rx[..len]);
            state.rx_len = len;
        }

        Ok(len)
    }

    /// Copy the buffered slave receive data into `buf` and clear the buffer.
    ///
    /// Call this after `drain_slave_rx()` returns a non-zero length, i.e. from
    /// the server's `SlaveReceive` IPC handler after a notification fires.
    ///
    /// Returns the number of bytes written into `buf`, truncated to
    /// `buf.len()` if the caller's buffer is smaller than the received packet.
    ///
    /// Returns `ResponseCode::Busy` if the buffer is empty (no data has been
    /// latched since the last call or since `enable_slave_notification()`).
    pub fn get_buffered_slave_message(
        &mut self,
        bus: u8,
        buf: &mut [u8],
    ) -> Result<usize, ResponseCode> {
        if !self.is_bus_initialized(bus) {
            return Err(ResponseCode::ServerError);
        }
        if !self.slave_notification[bus as usize].enabled {
            return Err(ResponseCode::NotInitialized);
        }

        let state = &mut self.slave_notification[bus as usize];
        let len = state.rx_len;
        if len == 0 {
            return Err(ResponseCode::Busy);
        }

        let copy_len = len.min(buf.len());
        buf[..copy_len].copy_from_slice(&state.rx_buf[..copy_len]);
        state.rx_len = 0;

        Ok(copy_len)
    }
}
