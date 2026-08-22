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
    ChipSelect, FlashConfig, FmcReady, FmcUninit, SmcConfig, SmcController, SmcError, SmcTopology,
    SpiNorFlash, SpiNorFlashDevice,
};
use hal_flash_driver::{FlashAddress, FlashDriver};
use util_error::{self as error, ErrorCode};
use util_types::{Blocking, PowerOf2Usize};

/// CS0 flash device configuration (W25Q64-class, 8 MiB).
///
/// Matches the hardware-verified configuration used by
/// //target/ast10x0/tests/smc/write.
const CS0_CONFIG: FlashConfig = FlashConfig {
    capacity_mb: 8,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
    spi_clock_mhz: 50,
};

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

/// FMC CS0 flash driver.
pub struct Ast10x0FmcFlashDriver {
    fmc: FmcReady,
}

/// Stable alias used by the server binary for compile-time backend selection.
pub type Backend = Ast10x0FmcFlashDriver;

impl Ast10x0FmcFlashDriver {
    /// Initialize the FMC and return a ready driver.
    ///
    /// # Safety
    /// The calling process must be the sole owner of the FMC controller
    /// (MMIO 0x7e62_0000) and its CS0 flash window (0x8000_0000), per the
    /// system.json5 of the image this runs in. The FMC pinmux
    /// (`PINCTRL_FMC_QUAD`) must already have been applied by the kernel
    /// target's pre-task init; this driver never touches the shared SCU.
    /// Call at most once per process.
    pub unsafe fn new() -> Result<Self, ErrorCode> {
        let config = SmcConfig {
            controller_id: SmcController::Fmc,
            cs0: Some(CS0_CONFIG),
            cs1: None,
            dma_enabled: false,
            enable_interrupts: false,
            topology: SmcTopology::BootSpi { master_idx: 0 },
        };
        // SAFETY: sole ownership of the FMC hardware block per the contract above.
        let uninit = unsafe { FmcUninit::new(config) }.map_err(map_smc_error)?;
        let mut fmc = uninit.init().map_err(map_smc_error)?;
        fmc.spi_nor_read_init(ChipSelect::Cs0)
            .map_err(map_smc_error)?;
        Ok(Self { fmc })
    }

    fn device(&mut self) -> Result<SpiNorFlash<'_>, ErrorCode> {
        SpiNorFlash::from_fmc_cs(&mut self.fmc, CS0_CONFIG, ChipSelect::Cs0).map_err(map_smc_error)
    }
}

impl FlashDriver for Ast10x0FmcFlashDriver {
    type Error = ErrorCode;

    /// Default erase page: one 4 KiB sector.
    const PAGE_SIZE: usize = CS0_CONFIG.sector_size as usize;
    /// SPI NOR program page: writes must not cross a 256-byte boundary.
    const PROGRAM_WINDOW_SIZE: usize = CS0_CONFIG.page_size as usize;
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        NonZero::new(CS0_CONFIG.capacity_mb as usize * 1024 * 1024).unwrap()
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Only 4 KiB sector erase is implemented by the peripheral driver.
        Ok(1u32 << CS0_CONFIG.sector_size.trailing_zeros())
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
        if size.get() != CS0_CONFIG.sector_size as usize {
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
