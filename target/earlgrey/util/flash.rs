// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Earlgrey-specific flash address utilities.

use hal_flash_driver::FlashAddress;

/// A trait for constructing and inspecting Earlgrey-specific flash addresses.
pub trait EarlgreyFlashAddress {
    /// Constructs a flash address for flash data pages.
    fn data(address: u32) -> FlashAddress;

    /// Constructs a flash address for flash info pages.
    ///
    /// NOTE: Currently this packs the bank and page into the offset logically.
    /// In the future, we may optimize this to use the flash-controller native
    /// address representation directly to simplify the driver implementation.
    fn info(bank: u32, page: u32, offset: u32) -> FlashAddress;

    /// Returns whether the flash address is an info page address.
    fn is_info(&self) -> bool;

    /// Returns the bank of a flash info page.
    fn bank(&self) -> u32;

    /// Returns the page number of a flash info page.
    fn page(&self) -> u32;

    /// Returns the flash offset. For data pages, this is the absolute flash
    /// address. For info pages, this is the offset within the info page.
    fn earlgrey_offset(&self) -> u32;
}

impl EarlgreyFlashAddress for FlashAddress {
    fn data(address: u32) -> FlashAddress {
        FlashAddress::new(address & 0x7FFFFFFF)
    }

    fn info(bank: u32, page: u32, offset: u32) -> FlashAddress {
        let offset = 0x80000000 | (bank & 0x7f) << 24 | (page & 0xff) << 16 | (offset & 0xFFFF);
        FlashAddress::new(offset)
    }

    fn is_info(&self) -> bool {
        (self.offset() & 0x80000000) != 0
    }

    fn bank(&self) -> u32 {
        (self.offset() >> 24) & 0x7f
    }

    fn page(&self) -> u32 {
        (self.offset() >> 16) & 0xff
    }

    fn earlgrey_offset(&self) -> u32 {
        if self.is_info() {
            self.offset() & 0xFFFF
        } else {
            self.offset() & 0x7FFFFFFF
        }
    }
}
