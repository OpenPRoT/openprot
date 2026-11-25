// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 FMC backend for the generic flash service.
//!
//! Adapts the SMC/FMC SPI-NOR peripheral driver to `hal_flash_driver::FlashDriver`
//! so it can be wrapped by `hal_flash::BlockingFlash` and served over IPC by
//! `services_flash_server::FlashIpcServer`.

#![no_std]

use core::cell::{RefCell, RefMut};
use core::num::NonZero;

use ast10x0_peripherals::smc::{
    ChipSelect, Cs, FlashConfig, FlashGeometry, FmcReady, FmcUninit, SmcConfig, SmcController,
    SmcError, SmcInstance, SmcTopology, SpiNorFlash, SpiNorFlashDevice,
};
use hal_flash_driver::{FlashAddress, FlashDriver};
use util_error::{self as error, ErrorCode};
use util_types::{Blocking, PowerOf2Usize};

/// Compile-time descriptor for the wired FMC controller this backend drives.
struct FmcInstance;

impl SmcInstance for FmcInstance {
    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: Some(FlashConfig { spi_clock_mhz: 50 }),
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

fn map_smc_error(e: SmcError) -> ErrorCode {
    match e {
        SmcError::HardwareError => error::FLASH_AST10X0_HARDWARE_ERROR,
        SmcError::Timeout => error::FLASH_AST10X0_TIMEOUT,
        SmcError::DmaAborted => error::FLASH_AST10X0_DMA_ABORTED,
        SmcError::DmaLengthMismatch => error::FLASH_AST10X0_DMA_LENGTH_MISMATCH,
        SmcError::InvalidChipSelect => error::FLASH_AST10X0_INVALID_CHIP_SELECT,
        SmcError::InvalidCapacity => error::FLASH_AST10X0_INVALID_CAPACITY,
        SmcError::DeviceNotSupported => error::FLASH_AST10X0_DEVICE_NOT_SUPPORTED,
        SmcError::WriteProtected => error::FLASH_AST10X0_WRITE_PROTECTED,
        SmcError::WriteInProgress => error::FLASH_AST10X0_WRITE_IN_PROGRESS,
        SmcError::ControllerNotReady => error::FLASH_AST10X0_CONTROLLER_NOT_READY,
        SmcError::DmaNotEnabled => error::FLASH_AST10X0_DMA_NOT_ENABLED,
    }
}

/// No-op `Blocking` impl paired with this driver.
///
/// FMC user-mode SPI-NOR commands have no completion interrupt; the peripheral
/// driver polls the device's WIP status bit to completion inside
/// `program_page`/`erase_sector`, so `start_*` below return with the operation
/// already finished and there is nothing to wait for.
pub struct NoWaitBlocking;

impl Blocking for NoWaitBlocking {
    fn wait_for_notification(&self) {}
}

/// The FMC controller, shared by the per-chip-select flash drivers.
///
/// Both chip selects use the same FMC registers, so one controller is
/// initialized once and every [`Ast10x0FmcFlashDriver`] borrows it. Drivers
/// borrow the controller for the duration of one operation, so they must stay
/// on the single thread that owns the controller.
pub struct FmcController {
    fmc: RefCell<FmcReady<FmcInstance>>,
}

impl FmcController {
    /// Initialize the FMC and return the controller.
    ///
    /// # Safety
    /// The calling process must be the sole owner of the FMC controller
    /// (MMIO 0x7e62_0000) and its CS flash windows: with both CS0 and CS1
    /// present the 256 MiB aperture is split in half, so CS0 decodes at
    /// 0x8000_0000 and CS1 at 0x8800_0000, per the system.json5 of the image
    /// this runs in. The FMC pinmux (`PINCTRL_FMC_QUAD`) must already have been
    /// applied by the kernel target's pre-task init; this driver never touches
    /// the shared SCU. Call at most once per process.
    pub unsafe fn new() -> Result<Self, ErrorCode> {
        // SAFETY: sole ownership of the FMC hardware block per the contract above.
        let uninit = unsafe { FmcUninit::<FmcInstance>::new() }.map_err(map_smc_error)?;
        let fmc = uninit.init().map_err(map_smc_error)?;
        Ok(Self {
            fmc: RefCell::new(fmc),
        })
    }

    /// Borrow the controller, or fail instead of panicking if it is in use.
    fn borrow(&self) -> Result<RefMut<'_, FmcReady<FmcInstance>>, ErrorCode> {
        self.fmc
            .try_borrow_mut()
            .map_err(|_| error::FLASH_AST10X0_CONTROLLER_NOT_READY)
    }

    /// Return a driver for the flash on chip select `cs`.
    ///
    /// Geometry was discovered over SFDP during `new()`; it is read back off the
    /// chip select handle (no rediscovery, no recalibration).
    pub fn driver(&self, cs: ChipSelect) -> Result<Ast10x0FmcFlashDriver<'_>, ErrorCode> {
        let mut fmc = self.borrow()?;
        let geometry = cs_handle(&mut fmc, cs)?.geometry();
        let capacity = NonZero::new(geometry.capacity_bytes as usize)
            .ok_or(error::FLASH_AST10X0_INVALID_CAPACITY)?;
        let program_window_size = PowerOf2Usize::new(geometry.page_size as usize)
            .ok_or(error::FLASH_GENERIC_INVALID_PAGE_SIZE)?;
        let erasable_sizes_bitmap =
            NonZero::new(geometry.sector_size).ok_or(error::FLASH_GENERIC_ERASE_INVALID_SIZE)?;
        Ok(Ast10x0FmcFlashDriver {
            controller: self,
            cs,
            geometry,
            capacity,
            program_window_size,
            erasable_sizes_bitmap,
        })
    }
}

fn cs_handle(fmc: &mut FmcReady<FmcInstance>, cs: ChipSelect) -> Result<Cs<'_>, ErrorCode> {
    match cs {
        ChipSelect::Cs0 => fmc.cs0(),
        ChipSelect::Cs1 => fmc.cs1(),
    }
    .map_err(map_smc_error)
}

/// FMC flash driver for one chip select.
pub struct Ast10x0FmcFlashDriver<'a> {
    controller: &'a FmcController,
    cs: ChipSelect,
    geometry: FlashGeometry,
    capacity: NonZero<usize>,
    program_window_size: PowerOf2Usize,
    erasable_sizes_bitmap: NonZero<u32>,
}

/// Stable alias used by the server binary for compile-time backend selection.
pub type Backend<'a> = Ast10x0FmcFlashDriver<'a>;

impl Ast10x0FmcFlashDriver<'_> {
    /// Run `f` on this chip select's flash device.
    fn with_device<T>(
        &mut self,
        f: impl FnOnce(&mut SpiNorFlash<'_>) -> Result<T, SmcError>,
    ) -> Result<T, ErrorCode> {
        let mut fmc = self.controller.borrow()?;
        let cs = cs_handle(&mut fmc, self.cs)?;
        let mut device = SpiNorFlash::new(cs).map_err(map_smc_error)?;
        f(&mut device).map_err(map_smc_error)
    }
}

impl FlashDriver for Ast10x0FmcFlashDriver<'_> {
    type Error = ErrorCode;

    // PAGE_SIZE / PROGRAM_WINDOW_SIZE are defaulted to 0, geometry is discovered instead
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        self.capacity
    }

    /// Default erase page: one SFDP-discovered sector.
    fn page_size(&self) -> usize {
        self.geometry.sector_size as usize
    }

    /// SPI NOR program page: writes must not cross this boundary.
    fn program_window_size(&self) -> PowerOf2Usize {
        self.program_window_size
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<NonZero<u32>, Self::Error> {
        // Only sector erase is implemented by the peripheral driver.
        Ok(self.erasable_sizes_bitmap)
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), Self::Error> {
        let len = buf.len();
        let n = self.with_device(|device| device.read(start_addr.offset(), buf))?;
        if n != len {
            return Err(error::FLASH_AST10X0_SHORT_READ);
        }
        Ok(())
    }

    fn start_erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), Self::Error> {
        if size.get() != self.geometry.sector_size as usize {
            return Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE);
        }
        // Blocks until the device's WIP bit clears; see `NoWaitBlocking`.
        self.with_device(|device| device.erase_sector(start_addr.offset()))
    }

    fn start_program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), Self::Error> {
        // Blocks until the device's WIP bit clears; see `NoWaitBlocking`.
        self.with_device(|device| device.program_page(start_addr.offset(), data))?;
        Ok(())
    }

    fn is_busy(&mut self) -> bool {
        false
    }

    fn complete_op(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
