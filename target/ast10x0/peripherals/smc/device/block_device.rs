// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Contained block-device facade layered on top of `SpiNorFlash`.

use crate::smc::device::flash::{JedecId, SpiNorFlash, SpiNorFlashDevice};
use crate::smc::types::SmcError;

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
}

impl<'a, 'b> SpiNorBlockDevice<'a, 'b> {
    /// Build a block facade from an existing `SpiNorFlash`.
    pub fn from_flash(flash: &'a mut SpiNorFlash<'b>) -> Result<Self, SmcError> {
        let g = flash.geometry();
        if g.page_size == 0 || g.sector_size == 0 || g.block_size == 0 {
            return Err(SmcError::InvalidCapacity);
        }
        Ok(Self { flash })
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
        let page = self.flash.geometry().page_size as usize;
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
        let sector = self.flash.geometry().sector_size;
        if !address.is_multiple_of(sector) || !length.is_multiple_of(sector) {
            return Err(SmcError::InvalidCapacity);
        }
        self.flash.erase_range(address, length as usize)
    }

    /// Return block-device geometry.
    pub fn info(&self) -> Result<BlockDeviceInfo, SmcError> {
        let g = self.flash.geometry();
        Ok(BlockDeviceInfo {
            capacity_bytes: g.capacity_bytes as usize,
            page_size: g.page_size as usize,
            sector_size: g.sector_size as usize,
            block_size: g.block_size as usize,
        })
    }

    /// Read JEDEC ID from the underlying flash.
    pub fn jedec(&self) -> Result<JedecId, SmcError> {
        self.flash.jedec()
    }
}
