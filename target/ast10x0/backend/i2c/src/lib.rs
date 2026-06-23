// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 backend for the i2c userspace driver server.
//!
//! This crate is a *thin adapter*, not a reimplementation. The in-tree
//! `ast10x0_peripherals::i2c::Ast1060I2c` driver already implements
//! `embedded_hal::i2c::I2c<SevenBitAddress>` (see
//! `target/ast10x0/peripherals/i2c/hal_impl.rs`), so the server can drive a
//! decoded wire transaction straight through it with **no typestate shim**.
//!
//! All this crate adds is:
//!  1. the bus-index → `(I2C, I2CBUFF)` register-pointer mapping for the 14
//!     AST1060 controllers,
//!  2. [`init_bus`] — the per-controller hardware bring-up the board calls
//!     eagerly for **every** wired bus, and
//!  3. per-bus open constructors ([`open_bus`] / [`open_bus_dma`]) that wrap an
//!     already-initialized controller (no re-init).
//!
//! ## Initialization split (settled)
//!
//! Per-controller config (`I2CC00` master-enable, `configure_timing`,
//! interrupts) depends on each bus's [`I2cConfig`], which is **board
//! topology**, so it now lives in the board descriptor. `Ast10x0Board::init()`
//! does subsystem bring-up (pin-mux / SCU clock+reset / `init_i2c_global`)
//! **and then eagerly calls [`init_bus`] for every wired bus** (DMA buses
//! included — `init_hardware()` does not touch the DMA buffer). The server
//! therefore opens each bus with the cheap no-init [`Ast1060I2c::from_initialized`]
//! path; `new()` is no longer used.
//!
//! DMA exception: the non-cached SRAM transfer buffer cannot live in a
//! `&'static` descriptor, so DMA buses are opened with [`open_bus_dma`], which
//! takes a server-owned `&'static mut` `.ram_nc` buffer. Register init for DMA
//! buses still happens in [`init_bus`] like any other bus.
//!
//! The server holds **one driver instance per bus it owns** (one IPC channel
//! per bus — see `i2c_server`). Slave/target mode is intentionally absent: the
//! wire protocol (`i2c_api::protocol`) only carries whole `Transaction`s.

#![no_std]

use ast1060_pac::{i2c::RegisterBlock, i2cbuff::RegisterBlock as BuffRegisterBlock};
use embedded_hal::i2c::{ErrorType, I2c, Operation, SevenBitAddress};
use i2c_api::seam::I2cSlaveEvent;
use openprot_hal_blocking::i2c_hardware::slave::{I2cIsrEvent, I2cSlaveBuffer, I2cSlaveCore};
use openprot_hal_blocking::i2c_hardware::I2cBusRecovery;

pub use ast10x0_peripherals::i2c::{
    Ast1060I2c, Ast1060I2cRegisters, ClockConfig, I2cConfig, I2cError, I2cSpeed, I2cXferMode,
};

/// The yield closure type stored in every bus driver.
///
/// A non-capturing `fn(u32)` (zero-sized) so `BusDriver` is a single concrete
/// type the server can store homogeneously. The server thread is the only user
/// of the bus, so a busy-wait spin between status polls is acceptable.
pub type Yield = fn(u32);

/// The concrete driver type the server owns, one per bus.
pub type BusDriver = Ast1060I2cBackend;

/// Highest AST1060 I2C controller index (controllers 0..=13).
pub const MAX_BUS: u8 = 13;

fn spin(_ns: u32) {
    core::hint::spin_loop();
}

/// AST10x0 I2C backend — owns MMIO pointers and DMA buffers for one bus,
/// constructs a transient [`Ast1060I2c`] driver per HAL operation.
///
/// This is the [`BusDriver`] held by the server. `make_driver()` re-creates
/// a thin `Ast1060I2c` each call so no per-operation state survives across HAL
/// method boundaries. DMA buffers are reborrowed for each driver's lifetime
/// and released when the transient driver is dropped at end of call.
pub struct Ast1060I2cBackend {
    regs: Ast1060I2cRegisters,
    config: I2cConfig,
    /// Master DMA staging buffer; `None` for buffer-mode (non-DMA) buses.
    master_dma_buf: Option<&'static mut [u8]>,
    /// Slave DMA receive buffer; `None` for buffer-mode buses.
    slave_dma_buf: Option<&'static mut [u8]>,
    /// Mirrored slave enable state — serves `I2cSlaveCore::is_slave_mode_enabled(&self)`.
    slave_enabled: bool,
    /// Mirrored slave address — serves `I2cSlaveCore::slave_address(&self)`.
    slave_addr: Option<SevenBitAddress>,
}

impl Ast1060I2cBackend {
    /// Construct a transient [`Ast1060I2c`] scoped to `&'_ mut self`.
    ///
    /// DMA buffers are reborrowed (`as_deref_mut`) so the transient driver's
    /// lifetime is bounded by the `&mut self` borrow — the driver cannot
    /// escape the HAL method that calls this. When both DMA buffers are
    /// present the DMA-capable constructor is used; otherwise the buffer-mode
    /// constructor is used.
    fn make_driver(&mut self) -> Ast1060I2c<'_, Yield> {
        let regs = self.regs;
        if let (Some(m), Some(s)) = (
            self.master_dma_buf.as_deref_mut(),
            self.slave_dma_buf.as_deref_mut(),
        ) {
            Ast1060I2c::from_initialized_with_dma(regs, &self.config, m, s, spin as Yield)
        } else {
            Ast1060I2c::from_initialized(regs, &self.config, spin as Yield)
        }
    }
}

impl ErrorType for Ast1060I2cBackend {
    type Error = I2cError;
}

impl I2c<SevenBitAddress> for Ast1060I2cBackend {
    fn write(&mut self, address: SevenBitAddress, bytes: &[u8]) -> Result<(), Self::Error> {
        self.make_driver().write(address, bytes)
    }

    fn read(&mut self, address: SevenBitAddress, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.make_driver().read(address, buffer)
    }

    fn write_read(
        &mut self,
        address: SevenBitAddress,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.make_driver().write_read(address, bytes, buffer)
    }

    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.make_driver().transaction(address, operations)
    }
}

impl I2cSlaveCore<SevenBitAddress> for Ast1060I2cBackend {
    fn configure_slave_address(&mut self, addr: SevenBitAddress) -> Result<(), Self::Error> {
        self.make_driver().configure_slave_address(addr)?;
        self.slave_addr = Some(addr);
        Ok(())
    }

    fn enable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.make_driver().enable_slave_mode()?;
        self.slave_enabled = true;
        Ok(())
    }

    fn disable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.make_driver().disable_slave_mode()?;
        self.slave_enabled = false;
        Ok(())
    }

    fn is_slave_mode_enabled(&self) -> bool {
        self.slave_enabled
    }

    fn slave_address(&self) -> Option<SevenBitAddress> {
        self.slave_addr
    }
}

impl I2cSlaveBuffer<SevenBitAddress> for Ast1060I2cBackend {
    fn read_slave_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.make_driver().read_slave_buffer(buffer)
    }

    fn write_slave_response(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.make_driver().write_slave_response(data)
    }

    fn poll_slave_data(&mut self) -> Result<Option<usize>, Self::Error> {
        self.make_driver().poll_slave_data()
    }

    fn clear_slave_buffer(&mut self) -> Result<(), Self::Error> {
        self.make_driver().clear_slave_buffer()
    }

    fn tx_buffer_space(&self) -> Result<usize, Self::Error> {
        Ok(32)
    }

    fn rx_buffer_count(&self) -> Result<usize, Self::Error> {
        // Conservative: use poll_slave_data() for the actual drain path.
        Ok(0)
    }
}

impl I2cBusRecovery for Ast1060I2cBackend {
    fn recover_bus(&mut self) -> Result<(), Self::Error> {
        self.make_driver().recover_bus()
    }
}

/// Explicit impl so the server-runtime receives the actual hardware event kind
/// (ReadRequest, Stop, etc.) rather than the default DataReceived from
/// `poll_slave_data()`. The inner `Ast1060I2c::try_next_slave_event` is an
/// inherent method; routing through this explicit trait impl is the only way
/// to reach it via trait dispatch (a blanket on I2cSlaveBuffer would shadow it).
impl I2cSlaveEvent for Ast1060I2cBackend {
    fn try_next_slave_event(&mut self) -> Result<Option<(I2cIsrEvent, usize)>, Self::Error> {
        self.make_driver().try_next_slave_event()
    }
}

/// Resolve a bus index to its `(I2C, I2CBUFF)` register-block pointers.
///
/// AST1060 exposes 14 controllers; instances 1..=13 are `derivedFrom` I2C0 in
/// the PAC, so every `::ptr()` is the same `*const RegisterBlock` type.
fn regs_for(bus: u8) -> Option<(*const RegisterBlock, *const BuffRegisterBlock)> {
    use ast1060_pac as p;
    Some(match bus {
        0 => (p::I2c::ptr(), p::I2cbuff::ptr()),
        1 => (p::I2c1::ptr(), p::I2cbuff1::ptr()),
        2 => (p::I2c2::ptr(), p::I2cbuff2::ptr()),
        3 => (p::I2c3::ptr(), p::I2cbuff3::ptr()),
        4 => (p::I2c4::ptr(), p::I2cbuff4::ptr()),
        5 => (p::I2c5::ptr(), p::I2cbuff5::ptr()),
        6 => (p::I2c6::ptr(), p::I2cbuff6::ptr()),
        7 => (p::I2c7::ptr(), p::I2cbuff7::ptr()),
        8 => (p::I2c8::ptr(), p::I2cbuff8::ptr()),
        9 => (p::I2c9::ptr(), p::I2cbuff9::ptr()),
        10 => (p::I2c10::ptr(), p::I2cbuff10::ptr()),
        11 => (p::I2c11::ptr(), p::I2cbuff11::ptr()),
        12 => (p::I2c12::ptr(), p::I2cbuff12::ptr()),
        13 => (p::I2c13::ptr(), p::I2cbuff13::ptr()),
        _ => return None,
    })
}

/// Per-controller hardware bring-up for one bus.
///
/// Runs `init_hardware()` (controller.rs:193-240: `I2CC00` master-enable,
/// `configure_timing`, interrupt enable) against controller `bus`. Called
/// **by the board**, eagerly, for every wired bus — including DMA buses,
/// since register init is independent of the DMA buffer.
///
/// The transient driver is dropped; only the hardware registers persist. The
/// server later re-wraps the same registers via [`open_bus`] /
/// [`open_bus_dma`] with no re-init.
///
/// # Precondition
///
/// Subsystem bring-up (pin-mux / SCU clock+reset / `init_i2c_global`) has
/// already run — i.e. called from `Ast10x0Board::init()` after that sequence.
///
/// # Safety
///
/// Exclusive access to controller `bus` for the duration of the call; `bus`
/// must be `<= MAX_BUS`.
pub unsafe fn init_bus(bus: u8, config: &I2cConfig) -> Result<(), I2cError> {
    let (regs, buff) = regs_for(bus).ok_or(I2cError::Invalid)?;
    // SAFETY: the single MMIO-pointer perimeter — PAC pointers for the same
    // controller `bus`, valid for the program; caller upholds exclusive
    // access and prior subsystem init.
    let mmio = unsafe { Ast1060I2cRegisters::new(regs, buff) };
    let mut i2c = Ast1060I2c::from_initialized(mmio, config, spin as Yield);
    i2c.init_hardware(config)
}

/// Open a BufferMode controller the board has already initialized.
///
/// Cheap no-init wrap ([`Ast1060I2c::from_initialized`]): [`init_bus`] (called
/// by the board for this bus) already did the per-controller register
/// config. Returns the bare [`Ast1060I2c`], which already satisfies
/// `embedded_hal::i2c::I2c<SevenBitAddress>` — the server is generic over that
/// trait and never names this type.
///
/// `config` must be the **same** entry the board used for this bus (it sets
/// the driver struct's mode fields; mismatch would desync struct vs hardware).
///
/// For DMA-mode buses use [`open_bus_dma`] instead.
///
/// # Precondition
///
/// `Ast10x0Board::init()` (which calls [`init_bus`] for every wired bus) has
/// run exactly once before this call.
///
/// # Safety
///
/// The caller must guarantee exclusive ownership of controller `bus` for the
/// returned driver's lifetime (the i2c server thread is the sole owner). `bus`
/// must be `<= MAX_BUS`.
pub unsafe fn open_bus(bus: u8, config: &I2cConfig) -> Result<BusDriver, I2cError> {
    let (regs, buff) = regs_for(bus).ok_or(I2cError::Invalid)?;
    // SAFETY: single MMIO-pointer perimeter; caller upholds exclusive
    // ownership and prior board init (incl. `init_bus`).
    let mmio = unsafe { Ast1060I2cRegisters::new(regs, buff) };
    Ok(Ast1060I2cBackend {
        regs: mmio,
        config: *config,
        master_dma_buf: None,
        slave_dma_buf: None,
        slave_enabled: false,
        slave_addr: None,
    })
}

/// Open a DmaMode controller the board has already initialized, attaching a
/// caller-owned non-cached SRAM transfer buffer.
///
/// Same no-init wrap as [`open_bus`] ([`Ast1060I2c::from_initialized_with_dma`]);
/// the only addition is the DMA buffer, which **cannot** live in the
/// `&'static` board descriptor and so is owned by the server binary (one
/// `#[link_section = ".ram_nc"]` buffer per DMA bus).
///
/// # Safety
///
/// As [`open_bus`], plus: `dma_buf` must reside in non-cached SRAM the DMA
/// engine and CPU see coherently, and be uniquely owned by this bus for the
/// driver's lifetime.
pub unsafe fn open_bus_dma(
    bus: u8,
    config: &I2cConfig,
    master_dma_buf: &'static mut [u8],
    slave_dma_buf: &'static mut [u8],
) -> Result<BusDriver, I2cError> {
    let (regs, buff) = regs_for(bus).ok_or(I2cError::Invalid)?;
    // SAFETY: single MMIO-pointer perimeter; both buffers are non-cached +
    // uniquely owned per the contract.
    let mmio = unsafe { Ast1060I2cRegisters::new(regs, buff) };
    Ok(Ast1060I2cBackend {
        regs: mmio,
        config: *config,
        master_dma_buf: Some(master_dma_buf),
        slave_dma_buf: Some(slave_dma_buf),
        slave_enabled: false,
        slave_addr: None,
    })
}
