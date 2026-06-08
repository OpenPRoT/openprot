//! Low-level flash driver interface.

#![no_std]

use core::num::NonZero;

use util_types::PowerOf2Usize;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Low-level flash driver interface.
///
/// This trait defines the interface for interacting with flash hardware at a low level.
/// It supports asynchronous-style operations (start/is_busy/complete) but can also be
/// implemented for synchronous drivers.
pub trait FlashDriver {
    type Error;
    /// A bitmap of supported erase block sizes.
    ///
    /// Each bit i represents a supported erase block size of 2^i bytes.
    const ERASABLE_SIZES_BITMAP: u32;
    /// The maximum size of a single program operation.
    const PROGRAM_WINDOW_SIZE: usize;
    /// The maximum size of a single read operation.
    const MAX_READ_SIZE: usize;
    /// The alignment required for read operations.
    const READ_ALIGNMENT: usize;
    /// The alignment required for program operations.
    const PROGRAM_ALIGNMENT: usize;

    /// Returns the total size of the flash in bytes.
    fn size(&self) -> NonZero<usize>;

    /// Reads data from flash.
    ///
    /// # Arguments
    /// * `start_addr`: The address to start reading from.
    /// * `buf`: The buffer to read data into.
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Starts an erase operation.
    ///
    /// # Arguments
    /// * `start_addr`: The start address of the block to erase.
    /// * `size`: The size of the block to erase.
    fn start_erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), Self::Error>;

    /// Starts a program operation.
    ///
    /// # Arguments
    /// * `start_address`: The address to start programming at.
    /// * `data`: The data to program.
    fn start_program(
        &mut self,
        start_address: FlashAddress,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    /// Returns whether the driver is currently busy with an operation.
    fn is_busy(&mut self) -> bool;

    /// Completes a pending operation and returns the result.
    fn complete_op(&mut self) -> Result<(), Self::Error>;
}

/// Represents an address in flash memory.
///
/// A flash address consists of a device identifier and an offset within that
/// device's address space.
#[derive(Default, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable, FromBytes, KnownLayout)]
#[repr(C)]
pub struct FlashAddress {
    device_id: u32,
    offset: u32,
}

impl core::fmt::Display for FlashAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:0x{:08x}", self.device_id, self.offset)
    }
}

impl core::fmt::Debug for FlashAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl FlashAddress {
    /// Creates a new `FlashAddress`.
    pub const fn new(device_id: u32, offset: u32) -> Self {
        Self { device_id, offset }
    }

    /// Returns the device identifier.
    pub fn device_id(&self) -> u32 {
        self.device_id
    }

    /// Returns the offset within the device's address space.
    pub fn offset(&self) -> u32 {
        self.offset
    }
}

impl core::ops::Add<usize> for FlashAddress {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        Self {
            device_id: self.device_id,
            offset: self.offset + other as u32,
        }
    }
}

impl core::ops::AddAssign<usize> for FlashAddress {
    fn add_assign(&mut self, other: usize) {
        self.offset += other as u32;
    }
}

impl core::ops::BitAnd<usize> for FlashAddress {
    type Output = Self;
    fn bitand(self, other: usize) -> Self {
        Self {
            device_id: self.device_id,
            offset: self.offset & other as u32,
        }
    }
}

impl core::ops::BitAndAssign<usize> for FlashAddress {
    fn bitand_assign(&mut self, other: usize) {
        self.offset &= other as u32;
    }
}
