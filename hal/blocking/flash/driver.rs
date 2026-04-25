#![no_std]

use core::num::NonZero;

use util_error::ErrorCode;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub trait FlashDriver {
    const PAGE_SIZE: usize;
    const PROGRAM_WINDOW_SIZE: usize;
    const MAX_READ_SIZE: usize;
    const READ_ALIGNMENT: usize;
    const PROGRAM_ALIGNMENT: usize;

    fn size(&self) -> NonZero<usize>;
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode>;
    fn start_erase_page(&mut self, start_addr: FlashAddress) -> Result<(), ErrorCode>;
    fn start_program(&mut self, start_address: FlashAddress, data: &[u8]) -> Result<(), ErrorCode>;
    fn is_busy(&mut self) -> bool;
    fn complete_op(&mut self) -> Result<(), ErrorCode>;
}

#[derive(Default, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable, FromBytes, KnownLayout)]
pub struct FlashAddress {
    address: u32,
}

impl FlashAddress {
    /// Constructs a flash address for flash data pages.
    pub fn data(address: u32) -> Self {
        Self {
            address: address & 0x7FFF_FFFF,
        }
    }

    /// Constructs a flash address for flash info pages.
    pub fn info(bank: u32, page: u32, offset: u32) -> Self {
        Self {
            address: 0x8000_0000 | (bank & 0x7f) << 24 | (page & 0xff) << 16 | (offset & 0xFFFF),
        }
    }

    /// Returns whether the flash address is an info page address.
    pub fn is_info(&self) -> bool {
        self.address & 0x8000_0000 != 0
    }

    /// Returns the flash offset.  For data pages, this is the flash address.  For info pages, this
    /// is the offset within the page.
    pub fn offset(&self) -> u32 {
        if self.is_info() {
            self.address & 0xFFFF
        } else {
            self.address
        }
    }

    /// Returns the bank of a flash info page (only valid when `is_info` returns true).
    pub fn bank(&self) -> u32 {
        (self.address >> 24) & 0x7f
    }

    /// Returns the page number of a flash info page (only valid when `is_info` returns true).
    pub fn page(&self) -> u32 {
        (self.address >> 16) & 0xff
    }
}

impl core::ops::Add<usize> for FlashAddress {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        let other = other as u32;
        if self.is_info() {
            let offset = self.offset() + other;
            FlashAddress {
                address: (self.address & !0xFFFF) | (offset & 0xFFFF),
            }
        } else {
            let offset = self.offset() + other;
            FlashAddress::data(offset as u32)
        }
    }
}

impl core::ops::AddAssign<usize> for FlashAddress {
    fn add_assign(&mut self, other: usize) {
        let other = other as u32;
        if self.is_info() {
            let offset = self.offset() + other;
            self.address = (self.address & !0xFFFF) | (offset & 0xFFFF);
        } else {
            let offset = self.offset() + other;
            self.address = offset as u32;
        }
    }
}

impl core::ops::BitAnd<usize> for FlashAddress {
    type Output = Self;
    fn bitand(self, other: usize) -> Self {
        let other = other as u32;
        if self.is_info() {
            let offset = self.offset() & other;
            FlashAddress {
                address: (self.address & !0xFFFF) | (offset & 0xFFFF),
            }
        } else {
            let offset = self.offset() & other;
            FlashAddress::data(offset as u32)
        }
    }
}

impl core::ops::BitAndAssign<usize> for FlashAddress {
    fn bitand_assign(&mut self, other: usize) {
        let other = other as u32;
        if self.is_info() {
            let offset = self.offset() & other;
            self.address = (self.address & !0xFFFF) | (offset & 0xFFFF);
        } else {
            let offset = self.offset() & other;
            self.address = offset as u32;
        }
    }
}
