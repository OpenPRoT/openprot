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
/// Maximum TX buffer size per bus (matches hardware buffer size).
const SLAVE_TX_BUF_SIZE: usize = 32;

pub struct AspeedI2cBackend {
    peripherals: ast1060_pac::Peripherals,
    /// Tracks which buses have been initialized via `init_bus()`.
    initialized: u16,
    /// Tracks which buses have slave mode configured via `configure_slave()`.
    slave_configured: u16,
    /// Pre-loaded TX data to send when master reads from us (one buffer per bus).
    slave_tx_bufs: [[u8; SLAVE_TX_BUF_SIZE]; 14],
    /// Valid byte count in each `slave_tx_bufs` slot.
    slave_tx_lens: [usize; 14],
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
            slave_tx_bufs: [[0u8; SLAVE_TX_BUF_SIZE]; 14],
            slave_tx_lens: [0usize; 14],
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
        let _i2c = Ast1060I2c::new(&ctrl, I2cConfig::with_static_clocks())
            .map_err(map_i2c_error)?;
        self.initialized |= 1 << bus;
        pw_log::info!("I2C bus {} controller initialized", bus as u8);
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
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::with_static_clocks());
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
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::with_static_clocks());
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
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::with_static_clocks());
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
        let mut i2c = Ast1060I2c::from_initialized(&ctrl, I2cConfig::with_static_clocks());
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
        pw_log::info!("I2C bus {} slave configured at address 0x{:02x}", bus as u8, addr as u8);
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
        pw_log::info!("I2C bus {} slave enabled", bus as u8);
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
        pw_log::info!("I2C bus {} slave disabled", bus as u8);
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

        let len = data.len().min(SLAVE_TX_BUF_SIZE);
        self.slave_tx_bufs[bus as usize][..len].copy_from_slice(&data[..len]);
        self.slave_tx_lens[bus as usize] = len;

        // Pre-load the hardware TX buffer and length register so it's ready for the next
        // master read. We do NOT set AST_I2CS_TX_BUFF_EN here because that would put the
        // slave into TX mode and prevent it from receiving writes. The TX_BUFF_EN flag
        // will be set later when we detect a ReadRequest event.
        let (regs, buffs) = self.controller_regs(bus)?;

        // Manually pre-load TX buffer without triggering transmission.
        // Only copy 1 byte to match the DDK's current limitation.
        let to_write = 1.min(len);
        if to_write > 0 {
            // Write directly to hardware buffer register (first DWORD)
            // The buffer is organized as 8 DWORDs, each holding up to 4 bytes
            unsafe {
                buffs.buff(0).write(|w| w.bits(data[0] as u32));

                // Set transfer length register (tx_data_byte_count = len - 1)
                regs.i2cc0c()
                    .modify(|_, w| w.tx_data_byte_count().bits((to_write - 1) as u8));
            }
            // Note: We do NOT set AST_I2CS_TX_BUFF_EN here - that happens in slave_wait_event
            // when ReadRequest is detected, to avoid blocking RX operations.
        }

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

        // Copy TX buffer to local stack before borrowing peripherals.
        let tx_len = self.slave_tx_lens[bus as usize];
        let mut tx_local = [0u8; SLAVE_TX_BUF_SIZE];
        tx_local[..tx_len].copy_from_slice(&self.slave_tx_bufs[bus as usize][..tx_len]);

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
                    return Ok((SlaveEventKind::DataReceived, n));
                }
                Some(SlaveEvent::ReadRequest) => {
                    // Hardware should already have the TX buffer armed from slave_set_response(),
                    // but call slave_write() as a fallback in case the timing worked out.
                    let _ = i2c.slave_write(&tx_local[..tx_len]);
                    return Ok((SlaveEventKind::ReadRequest, 0));
                }
                Some(SlaveEvent::DataSent { len: _ }) => {
                    // Read transaction completed — treat same as ReadRequest
                    // (data was already pre-loaded and sent by hardware).
                    return Ok((SlaveEventKind::ReadRequest, 0));
                }
                Some(SlaveEvent::Stop) => {
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
}
