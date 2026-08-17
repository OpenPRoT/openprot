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
//!  1. [`i2c_bus`] — const-wire a bus from its two SCL/SDA pin tokens, and
//!  2. [`open_bus_dma`] — bring up a server's DmaMode controller from its two pin tokens (init hardware,
//!     then wrap with non-cached DMA buffers) in one step, so a driver can never front an uninit controller.
//!
//! Per-controller config (`I2CC00` master-enable, `configure_timing`, interrupts) depends on each bus's
//! [`I2cConfig`] (board topology), so each server brings up its own bus — the board does not. The DMA
//! buffers are non-cached SRAM (`&'static mut` `.ram_nc`), which cannot live in a `&'static` descriptor,
//! so the server hands them to [`open_bus_dma`].
//!
//! The server holds **one driver instance per bus it owns** (one IPC channel
//! per bus — see `i2c_server`). Slave/target mode is intentionally absent: the
//! wire protocol (`i2c_api::protocol`) only carries whole `Transaction`s.

#![no_std]

use ast10x0_peripherals::scu::{scu414_30, scu414_31, scu418_0, scu418_1};
use embedded_hal::i2c::{ErrorType, I2c, Operation, SevenBitAddress};
use i2c_api::seam::I2cSlaveEvent;
use openprot_hal::i2c::{I2cBus, I2cScl, I2cSda};
use openprot_hal::resource::{Pin, Routes};
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

/// Controller 1's bus: SCL2/SDA2 on SCU414[30:31], base carried by the pins (silicon-fixed).
pub type I2c1Bus = I2cBus<scu414_30, scu414_31, Ast1060I2cRegisters>;

/// Controller 2's bus: SCL3/SDA3 on SCU418[0:1], base carried by the pins (silicon-fixed).
pub type I2c2Bus = I2cBus<scu418_0, scu418_1, Ast1060I2cRegisters>;

/// A DMA transfer buffer proven to sit in non-cached SRAM and be uniquely owned — the type-level
/// discharge of [`open_bus_dma`]'s buffer contract. Mint one only via [`non_cached_buf!`].
pub struct NonCachedBuf(&'static mut [u8]);

impl NonCachedBuf {
    /// Wrap a `.ram_nc` buffer; the macro is the sole caller and upholds placement + take-once.
    #[doc(hidden)]
    #[must_use]
    pub fn __from_ram_nc(buf: &'static mut [u8]) -> Self {
        Self(buf)
    }
}

/// Declare a `.ram_nc` DMA buffer of `$len` bytes and mint its [`NonCachedBuf`] once (`None` on any
/// later call). The only `unsafe` for a server's DMA buffers, authored here so the binary has none.
#[macro_export]
macro_rules! non_cached_buf {
    ($len:expr) => {{
        #[unsafe(link_section = ".ram_nc")]
        static mut BUF: [u8; $len] = [0u8; $len];
        static TAKEN: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);
        if TAKEN.swap(true, ::core::sync::atomic::Ordering::AcqRel) {
            ::core::option::Option::None
        } else {
            // SAFETY: the take-once guard yields a single &'static mut, and `.ram_nc` places it in
            // non-cached SRAM — exactly the two facts `NonCachedBuf` stands for.
            ::core::option::Option::Some($crate::NonCachedBuf::__from_ram_nc(unsafe {
                &mut *::core::ptr::addr_of_mut!(BUF)
            }))
        }
    }};
}

fn spin(_ns: u32) {
    core::hint::spin_loop();
}

/// Per-controller register bring-up over an already-minted handle (safe).
///
/// Runs `init_hardware()` (`I2CC00` master-enable, `configure_timing`, interrupt enable). The
/// transient driver is dropped; only the hardware registers persist.
fn init_regs(mmio: Ast1060I2cRegisters, config: &I2cConfig) -> Result<(), I2cError> {
    let mut i2c = Ast1060I2c::from_initialized(mmio, config, spin as Yield);
    i2c.init_hardware(config)
}

/// Bind an I2C bus from its two SCL/SDA pin tokens; the bus's routes ride in via `MuxRoutes`, applied by `route`.
#[must_use]
pub const fn i2c_bus<Scl: Routes<I2cScl> + Pin, Sda: Routes<I2cSda> + Pin>(
    scl: Scl,
    sda: Sda,
) -> I2cBus<Scl, Sda, Ast1060I2cRegisters> {
    let regs = Ast1060I2cRegisters::from_pins(&scl, &sda);
    I2cBus::new(scl, sda, regs)
}

/// AST10x0 I2C backend — owns MMIO pointers and DMA buffers for one bus,
/// constructs a transient [`Ast1060I2c`] driver per HAL operation.
///
/// This is the [`BusDriver`] held by the server. `make_driver()` re-creates
/// a thin `Ast1060I2c` each call so no per-operation state survives across HAL
/// method boundaries. DMA buffers are reborrowed for each driver's lifetime
/// and released when the transient driver is dropped at end of call.
pub struct Ast1060I2cBackend {
    /// This bus's controller register façade, bound from the pin tokens the server holds.
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

/// Bring up and open a DmaMode bus from its two pin tokens in one step — init hardware (master-enable,
/// timing, interrupts) then wrap with the caller's non-cached DMA buffers, so a driver can never front
/// an uninitialized controller; naming the SCL/SDA pins binds that already-muxed controller at compile time.
pub fn open_bus_dma<Scl: Routes<I2cScl>, Sda: Routes<I2cSda>>(
    scl: Scl,
    sda: Sda,
    config: &I2cConfig,
    master_dma_buf: NonCachedBuf,
    slave_dma_buf: NonCachedBuf,
) -> Result<BusDriver, I2cError> {
    let regs = Ast1060I2cRegisters::from_pins(&scl, &sda);
    init_regs(regs, config)?;
    Ok(Ast1060I2cBackend {
        regs,
        config: *config,
        master_dma_buf: Some(master_dma_buf.0),
        slave_dma_buf: Some(slave_dma_buf.0),
        slave_enabled: false,
        slave_addr: None,
    })
}
