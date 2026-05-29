// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Contained block-device facade layered on top of `SpiNorFlash`.

use crate::smc::device::flash::{JedecId, SpiNorFlash, SpiNorFlashDevice};
use crate::smc::types::{FlashConfig, SmcError};

/// Geometry and limits exposed by the block facade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockDeviceInfo {
    pub capacity_bytes: usize,
    pub page_size: usize,
    pub sector_size: usize,
    pub block_size: usize,
}

/// Minimal block-oriented facade over a `SpiNorFlash` device.
pub struct SpiNorBlockDevice<'a, 'b> {
    flash: &'a mut SpiNorFlash<'b>,
    cfg: FlashConfig,
}

impl<'a, 'b> SpiNorBlockDevice<'a, 'b> {
    /// Build a block facade from an existing `SpiNorFlash` plus known config.
    pub fn from_flash(
        flash: &'a mut SpiNorFlash<'b>,
        cfg: FlashConfig,
    ) -> Result<Self, SmcError> {
        let expected = cfg_capacity_bytes(cfg)?;
        let actual = SpiNorFlashDevice::capacity_bytes(flash)?;
        if expected != actual {
            return Err(SmcError::InvalidCapacity);
        }
        if cfg.page_size == 0 || cfg.sector_size == 0 || cfg.block_size == 0 {
            return Err(SmcError::InvalidCapacity);
        }
        Ok(Self { flash, cfg })
    }

    /// Build a block facade by mapping a JEDEC ID to a known flash profile.
    pub fn from_jedec_id(
        flash: &'a mut SpiNorFlash<'b>,
        jedec: JedecId,
    ) -> Result<Self, SmcError> {
        let cfg = cfg_from_jedec(jedec)?;
        Self::from_flash(flash, cfg)
    }

    /// Read bytes from the block device.
    pub fn read_blocks(&self, address: u32, out: &mut [u8]) -> Result<usize, SmcError> {
        SpiNorFlashDevice::read(self.flash, address, out)
    }

    /// Program bytes using the underlying page-program path.
    pub fn write_blocks(&mut self, address: u32, data: &[u8]) -> Result<usize, SmcError> {
        if data.is_empty() {
            return Ok(0);
        }
        let page = self.cfg.page_size as usize;
        if (address as usize) % page != 0 {
            return Err(SmcError::InvalidCapacity);
        }
        self.flash.program(address, data)
    }

    /// Erase bytes using sector-granularity contract.
    pub fn erase_blocks(&mut self, address: u32, length: u32) -> Result<(), SmcError> {
        if length == 0 {
            return Ok(());
        }
        let sector = self.cfg.sector_size;
        if !address.is_multiple_of(sector) || !length.is_multiple_of(sector) {
            return Err(SmcError::InvalidCapacity);
        }
        self.flash.erase_range(address, length as usize)
    }

    /// Return block-device geometry.
    pub fn info(&self) -> Result<BlockDeviceInfo, SmcError> {
        Ok(BlockDeviceInfo {
            capacity_bytes: cfg_capacity_bytes(self.cfg)?,
            page_size: self.cfg.page_size as usize,
            sector_size: self.cfg.sector_size as usize,
            block_size: self.cfg.block_size as usize,
        })
    }

    /// Read JEDEC ID from the underlying flash.
    pub fn jedec(&self) -> Result<JedecId, SmcError> {
        self.flash.jedec()
    }
}

fn cfg_capacity_bytes(cfg: FlashConfig) -> Result<usize, SmcError> {
    (cfg.capacity_mb as usize)
        .checked_mul(1024 * 1024)
        .ok_or(SmcError::InvalidCapacity)
}

fn cfg_from_jedec(jedec: JedecId) -> Result<FlashConfig, SmcError> {
    match (jedec.manufacturer, jedec.memory_type, jedec.capacity_code) {
        (0xEF, 0x40, 0x17) => Ok(FlashConfig::winbond_w25q64()),
        (0xEF, 0x40, 0x18) => Ok(FlashConfig {
            capacity_mb: 16,
            page_size: 256,
            sector_size: 4096,
            block_size: 65536,
            spi_clock_mhz: 25,
        }),
        (0xEF, 0x40, 0x19) => Ok(FlashConfig::winbond_w25q256()),
        _ => Err(SmcError::DeviceNotSupported),
    }
}