// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 FMC backend for the generic flash service.
//!
//! Adapts the SMC/FMC SPI-NOR peripheral driver to `hal_flash_driver::FlashDriver`
//! so it can be wrapped by `hal_flash::BlockingFlash` and served over IPC by
//! `services_flash_server::FlashIpcServer`.

#![no_std]

use core::num::NonZero;

use ast10x0_peripherals::smc::{
    FlashConfig, FlashGeometry, FmcReady, FmcUninit, SmcConfig, SmcController, SmcError,
    SmcInstance, SmcTopology, SpiNorFlash, SpiNorFlashDevice,
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

/// FMC flash driver.
pub struct Ast10x0FmcFlashDriver {
    fmc: FmcReady<FmcInstance>,
    geometry: FlashGeometry,
}

/// Stable alias used by the server binary for compile-time backend selection.
pub type Backend = Ast10x0FmcFlashDriver;

impl Ast10x0FmcFlashDriver {
    /// Initialize the FMC and return a ready driver.
    ///
    /// # Safety
    /// The calling process must be the sole owner of the FMC controller
    /// (MMIO 0x7e62_0000) and its CS flash windows: with both CS0 and CS1
    /// present the 256 MiB aperture is split in half, so CS0 decodes at
    /// 0x8000_0000 and the served CS1 flash at 0x8800_0000, per the
    /// system.json5 of the image this runs in. The FMC pinmux
    /// (`PINCTRL_FMC_QUAD`) must already have been applied by the kernel
    /// target's pre-task init; this driver never touches the shared SCU.
    /// Call at most once per process.
    pub unsafe fn new() -> Result<Self, ErrorCode> {
        // SAFETY: sole ownership of the FMC hardware block per the contract above.
        let uninit = unsafe { FmcUninit::<FmcInstance>::new() }.map_err(map_smc_error)?;
        let mut fmc = uninit.init().map_err(map_smc_error)?;
        // Geometry was discovered over SFDP during `init()`; read it back off the
        // CS1 handle (no rediscovery, no recalibration).
        let geometry = {
            let cs1 = fmc.cs1().map_err(map_smc_error)?;
            cs1.geometry()
        };
        NonZero::new(geometry.capacity_bytes as usize)
            .ok_or(error::FLASH_AST10X0_INVALID_CAPACITY)?;
        Ok(Self { fmc, geometry })
    }

    fn device(&mut self) -> Result<SpiNorFlash<'_>, ErrorCode> {
        let cs = self.fmc.cs1().map_err(map_smc_error)?;
        SpiNorFlash::new(cs).map_err(map_smc_error)
    }
}

impl FlashDriver for Ast10x0FmcFlashDriver {
    type Error = ErrorCode;

    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        NonZero::new(self.geometry.capacity_bytes as usize).expect("capacity validated in new()")
    }

    /// Default erase page: one SFDP-discovered sector.
    fn page_size(&self) -> usize {
        self.geometry.sector_size as usize
    }

    /// SPI NOR program page: writes must not cross this boundary.
    fn program_window_size(&self) -> usize {
        self.geometry.page_size as usize
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Only sector erase is implemented by the peripheral driver.
        Ok(1u32 << self.geometry.sector_size.trailing_zeros())
    }

    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), Self::Error> {
        let len = buf.len();
        let n = self
            .device()?
            .read(start_addr.offset(), buf)
            .map_err(map_smc_error)?;
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
        self.device()?
            .erase_sector(start_addr.offset())
            .map_err(map_smc_error)
    }

    fn start_program(&mut self, start_addr: FlashAddress, data: &[u8]) -> Result<(), Self::Error> {
        // Blocks until the device's WIP bit clears; see `NoWaitBlocking`.
        self.device()?
            .program_page(start_addr.offset(), data)
            .map_err(map_smc_error)?;
        Ok(())
    }

    fn is_busy(&mut self) -> bool {
        false
    }

    fn complete_op(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
