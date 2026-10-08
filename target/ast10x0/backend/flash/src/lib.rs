// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 FMC and SPI1 backends for the generic flash service.
//!
//! Adapts the SMC SPI-NOR peripheral driver to `hal_flash_driver::FlashDriver`
//! so it can be wrapped by `hal_flash::BlockingFlash` and served over IPC by
//! `services_flash_server::FlashIpcServer`. Each consumer names the controller
//! it drives: `Ast10x0FmcFlashDriver` for the flash on FMC CS1,
//! `Ast10x0FmcCs0FlashDriver` for the boot flash on FMC CS0, and
//! `Ast10x0Spi1FlashDriver` for the shared staging flash on SPI1 CS0.

#![no_std]

use core::marker::PhantomData;
use core::num::NonZero;

use ast10x0_peripherals::smc::{
    FlashConfig, FlashGeometry, FmcReady, FmcUninit, GeometrySource, Pinned, SmcConfig,
    SmcController, SmcError, SmcInstance, SmcTopology, SpiNorFlash, SpiNorFlashDevice, SpiReady,
    SpiUninit,
};
use hal_flash_driver::{FlashAddress, FlashDriver};
use util_error::{self as error, ErrorCode};
use util_region::{Mmap, Region, Unmapped};
use util_types::{Blocking, PowerOf2Usize};

/// The shared staging flash on SPI1 CS0: a 64 MiB part with 4 KiB sectors.
const STAGING_FLASH_GEOMETRY: FlashGeometry = FlashGeometry {
    capacity_bytes: 0x0400_0000,
    page_size: 256,
    sector_size: 4096,
    block_size: 65536,
};

/// Compile-time descriptor for the wired SPI1 controller this backend drives.
///
/// `R` names the register region the hosting process was granted, so the
/// controller's base address comes from that image's `system.json5`.
struct Spi1Instance<R: Mmap>(PhantomData<R>);

impl<R: Mmap> SmcInstance for Spi1Instance<R> {
    type Regs = R;

    const CONTROLLER: SmcController = SmcController::Spi1;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 25 }),
        cs1: None,
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::HostSpi { master_idx: 0 },
    };

    /// Pinned, so `init` reaches the device without an SFDP read: the part's
    /// BFP table runs past the 256 bytes `Discover` reads, which it rejects.
    type Cs0Geometry = Pinned<STAGING_FLASH_GEOMETRY>;
}

/// Geometry source for the served chip (CS0). `Pinned` here makes the reported
/// geometry a compile-time constant; `Discover` reports the SFDP-read value.
type Cs0Geometry<R> = <Spi1Instance<R> as SmcInstance>::Cs0Geometry;

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
/// User-mode SPI-NOR commands have no completion interrupt; the peripheral
/// driver polls the device's WIP status bit to completion inside
/// `program_page`/`erase_sector`, so `start_*` below return with the operation
/// already finished and there is nothing to wait for.
pub struct NoWaitBlocking;

impl Blocking for NoWaitBlocking {
    fn wait_for_notification(&self) {}
}

/// SPI1 flash driver.
pub struct Ast10x0Spi1FlashDriver<R: Mmap> {
    spi: SpiReady<Spi1Instance<R>>,
    geometry: FlashGeometry,
}

impl<R: Mmap> Ast10x0Spi1FlashDriver<R> {
    /// Initialize SPI1 from the regions mapped to this process.
    ///
    /// Takes the register block and the CS0 decode window, which `init` lights
    /// up. Consuming the tokens is what bounds this to one driver per process.
    /// `CONFIG` leaves CS1 off, so no window is needed for it.
    ///
    /// The SPI1 pinmux and the SPIM0 route to the staging flash must already have
    /// been applied by the kernel target's pre-task init; this driver never
    /// touches the shared SCU.
    pub fn new<Cs0>(regs: Region<R>, cs0_window: Region<Cs0>) -> Result<Self, ErrorCode>
    where
        Cs0: Mmap,
    {
        let uninit =
            SpiUninit::<Spi1Instance<R>>::new(regs, cs0_window, Region::<Unmapped>::unmapped())
                .map_err(map_smc_error)?;
        let mut spi = uninit.init().map_err(map_smc_error)?;
        let geometry = {
            let cs0 = spi.cs0().map_err(map_smc_error)?;
            cs0.geometry()
        };
        NonZero::new(geometry.capacity_bytes as usize)
            .ok_or(error::FLASH_AST10X0_INVALID_CAPACITY)?;
        Ok(Self { spi, geometry })
    }

    fn device(&mut self) -> Result<SpiNorFlash<'_>, ErrorCode> {
        let cs = self.spi.cs0().map_err(map_smc_error)?;
        SpiNorFlash::new(cs).map_err(map_smc_error)
    }

    /// Read the device's JEDEC ID.
    ///
    /// The reported geometry is pinned, so this is the only call that proves the
    /// bus actually reaches the part: all-zero means chip select never asserted,
    /// all-ones means the line is floating.
    pub fn jedec_id(&mut self) -> Result<[u8; 3], ErrorCode> {
        self.device()?.jedec_id().map_err(map_smc_error)
    }
}

impl<R: Mmap> FlashDriver for Ast10x0Spi1FlashDriver<R> {
    type Error = ErrorCode;

    // PAGE_SIZE / PROGRAM_WINDOW_SIZE are defaulted to 0, geometry is discovered instead
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        NonZero::new(Cs0Geometry::<R>::geometry(&self.geometry).capacity_bytes as usize)
            .expect("capacity validated in new()")
    }

    /// Default erase page: one SFDP-discovered sector.
    fn page_size(&self) -> usize {
        Cs0Geometry::<R>::geometry(&self.geometry).sector_size as usize
    }

    /// SPI NOR program page: writes must not cross this boundary.
    fn program_window_size(&self) -> usize {
        Cs0Geometry::<R>::geometry(&self.geometry).page_size as usize
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Only sector erase is implemented by the peripheral driver.
        Ok(1u32
            << Cs0Geometry::<R>::geometry(&self.geometry)
                .sector_size
                .trailing_zeros())
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

/// Compile-time descriptor for the FMC driving the boot flash on CS0.
///
/// CS1 is left off so CS0 decodes from the aperture base rather than the
/// aperture being split in half.
struct FmcCs0Instance<R: Mmap>(PhantomData<R>);

impl<R: Mmap> SmcInstance for FmcCs0Instance<R> {
    type Regs = R;

    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: None,
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

/// Geometry source for the boot chip (CS0). `Pinned` here makes the reported
/// geometry a compile-time constant; `Discover` reports the SFDP-read value.
type FmcCs0Geometry<R> = <FmcCs0Instance<R> as SmcInstance>::Cs0Geometry;

/// FMC CS0 boot flash driver.
pub struct Ast10x0FmcCs0FlashDriver<R: Mmap> {
    fmc: FmcReady<FmcCs0Instance<R>>,
    geometry: FlashGeometry,
}

impl<R: Mmap> Ast10x0FmcCs0FlashDriver<R> {
    /// Initialize the FMC from the regions mapped to this process.
    ///
    /// The FMC pinmux (`PINCTRL_FMC_QUAD`) must already have been applied by the
    /// kernel target's pre-task init; this driver never touches the shared SCU.
    pub fn new<Cs0>(regs: Region<R>, cs0_window: Region<Cs0>) -> Result<Self, ErrorCode>
    where
        Cs0: Mmap,
    {
        let uninit =
            FmcUninit::<FmcCs0Instance<R>>::new(regs, cs0_window, Region::<Unmapped>::unmapped())
                .map_err(map_smc_error)?;
        let mut fmc = uninit.init().map_err(map_smc_error)?;
        let geometry = {
            let cs0 = fmc.cs0().map_err(map_smc_error)?;
            cs0.geometry()
        };
        NonZero::new(geometry.capacity_bytes as usize)
            .ok_or(error::FLASH_AST10X0_INVALID_CAPACITY)?;
        Ok(Self { fmc, geometry })
    }

    /// Read the device's JEDEC ID: all-zero means chip select never asserted,
    /// all-ones means the line is floating.
    pub fn jedec_id(&mut self) -> Result<[u8; 3], ErrorCode> {
        self.device()?.jedec_id().map_err(map_smc_error)
    }

    fn device(&mut self) -> Result<SpiNorFlash<'_>, ErrorCode> {
        let cs = self.fmc.cs0().map_err(map_smc_error)?;
        SpiNorFlash::new(cs).map_err(map_smc_error)
    }
}

impl<R: Mmap> FlashDriver for Ast10x0FmcCs0FlashDriver<R> {
    type Error = ErrorCode;

    // PAGE_SIZE / PROGRAM_WINDOW_SIZE are defaulted to 0, geometry is discovered instead
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        NonZero::new(FmcCs0Geometry::<R>::geometry(&self.geometry).capacity_bytes as usize)
            .expect("capacity validated in new()")
    }

    /// Default erase page: one SFDP-discovered sector.
    fn page_size(&self) -> usize {
        FmcCs0Geometry::<R>::geometry(&self.geometry).sector_size as usize
    }

    /// SPI NOR program page: writes must not cross this boundary.
    fn program_window_size(&self) -> usize {
        FmcCs0Geometry::<R>::geometry(&self.geometry).page_size as usize
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Only sector erase is implemented by the peripheral driver.
        Ok(1u32
            << FmcCs0Geometry::<R>::geometry(&self.geometry)
                .sector_size
                .trailing_zeros())
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

/// Compile-time descriptor for the wired FMC controller this backend drives.
///
/// `R` names the register region the hosting process was granted, so the
/// controller's base address comes from that image's `system.json5`.
struct FmcInstance<R: Mmap>(PhantomData<R>);

impl<R: Mmap> SmcInstance for FmcInstance<R> {
    type Regs = R;

    const CONTROLLER: SmcController = SmcController::Fmc;
    const CONFIG: SmcConfig = SmcConfig {
        cs0: Some(FlashConfig { spi_clock_mhz: 50 }),
        cs1: Some(FlashConfig { spi_clock_mhz: 50 }),
        dma_enabled: false,
        enable_interrupts: false,
        topology: SmcTopology::BootSpi { master_idx: 0 },
    };
}

/// Geometry source for the served chip (CS1). `Pinned` here makes the reported
/// geometry a compile-time constant; `Discover` reports the SFDP-read value.
type Cs1Geometry<R> = <FmcInstance<R> as SmcInstance>::Cs1Geometry;

/// FMC flash driver.
pub struct Ast10x0FmcFlashDriver<R: Mmap> {
    fmc: FmcReady<FmcInstance<R>>,
    geometry: FlashGeometry,
}

impl<R: Mmap> Ast10x0FmcFlashDriver<R> {
    /// Initialize the FMC from the regions mapped to this process.
    ///
    /// Takes the register block and both CS decode windows, which `init` lights
    /// up: with CS0 and CS1 both present the 256 MiB aperture is split in half,
    /// so CS0 decodes low and the served CS1 flash decodes at the midpoint.
    /// Consuming the tokens is what bounds this to one driver per process.
    ///
    /// The FMC pinmux (`PINCTRL_FMC_QUAD`) must already have been applied by the
    /// kernel target's pre-task init; this driver never touches the shared SCU.
    pub fn new<Cs0, Cs1>(
        regs: Region<R>,
        cs0_window: Region<Cs0>,
        cs1_window: Region<Cs1>,
    ) -> Result<Self, ErrorCode>
    where
        Cs0: Mmap,
        Cs1: Mmap,
    {
        let uninit = FmcUninit::<FmcInstance<R>>::new(regs, cs0_window, cs1_window)
            .map_err(map_smc_error)?;
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

impl<R: Mmap> FlashDriver for Ast10x0FmcFlashDriver<R> {
    type Error = ErrorCode;

    // PAGE_SIZE / PROGRAM_WINDOW_SIZE are defaulted to 0, geometry is discovered instead
    const MAX_READ_SIZE: usize = 4096;
    const READ_ALIGNMENT: usize = 4;
    const PROGRAM_ALIGNMENT: usize = 1;

    fn size(&self) -> NonZero<usize> {
        NonZero::new(Cs1Geometry::<R>::geometry(&self.geometry).capacity_bytes as usize)
            .expect("capacity validated in new()")
    }

    /// Default erase page: one SFDP-discovered sector.
    fn page_size(&self) -> usize {
        Cs1Geometry::<R>::geometry(&self.geometry).sector_size as usize
    }

    /// SPI NOR program page: writes must not cross this boundary.
    fn program_window_size(&self) -> usize {
        Cs1Geometry::<R>::geometry(&self.geometry).page_size as usize
    }

    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error> {
        // Only sector erase is implemented by the peripheral driver.
        Ok(1u32
            << Cs1Geometry::<R>::geometry(&self.geometry)
                .sector_size
                .trailing_zeros())
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
