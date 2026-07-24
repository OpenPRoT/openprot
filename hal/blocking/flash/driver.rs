// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Low-level flash driver interface.

#![no_std]

use core::num::NonZero;

use util_types::PowerOf2Usize;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Low-level flash driver interface.
///
/// This trait defines the interface for interacting with flash hardware at a low level.
/// It supports an asynchronous-style execution model using a start-poll-complete pattern:
/// 1. Start the operation (`start_erase`, `start_program`).
/// 2. Poll status (`is_busy`) or wait for interrupt.
/// 3. Finalize and check errors (`complete_op`).
///
/// Read operations (`read`) are assumed to be synchronous and blocking for simplicity.
pub trait FlashDriver {
    /// The error type returned by driver operations.
    type Error;

    /// The default page size in bytes.
    const PAGE_SIZE: usize;

    /// The maximum size of a single program operation (write window).
    /// Program operations cannot span across boundaries aligned to this size.
    const PROGRAM_WINDOW_SIZE: usize;

    /// The maximum size of a single read operation.
    const MAX_READ_SIZE: usize;

    /// The alignment required for read operations (addresses and lengths).
    const READ_ALIGNMENT: usize;

    /// The alignment required for program operations (addresses and lengths).
    const PROGRAM_ALIGNMENT: usize;

    /// Returns the total size of the flash in bytes.
    fn size(&self) -> NonZero<usize>;

    /// Returns a bitmap of supported erase block sizes.
    ///
    /// Each bit `i` represents a supported erase block size of `2^i` bytes.
    /// For example, if bit 11 is set, then 2048-byte erases are supported.
    fn erasable_sizes_bitmap(&mut self) -> Result<u32, Self::Error>;

    /// Reads data from flash synchronously.
    ///
    /// # Arguments
    /// * `start_addr`: The address to start reading from. Must be aligned to `READ_ALIGNMENT`.
    /// * `buf`: The buffer to read data into. Must have size <= `MAX_READ_SIZE` and be aligned to `READ_ALIGNMENT`.
    fn read(&mut self, start_addr: FlashAddress, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Starts an erase operation asynchronously.
    ///
    /// The operation is not guaranteed to be complete until `complete_op` is called
    /// and returns `Ok`.
    ///
    /// # Arguments
    /// * `start_addr`: The start address of the block to erase. Must be aligned to the block size.
    /// * `size`: The size of the block to erase. Must have its corresponding bit set in `ERASABLE_SIZES_BITMAP`.
    fn start_erase(
        &mut self,
        start_addr: FlashAddress,
        size: PowerOf2Usize,
    ) -> Result<(), Self::Error>;

    /// Starts a program operation asynchronously.
    ///
    /// The operation is not guaranteed to be complete until `complete_op` is called
    /// and returns `Ok`.
    ///
    /// The programmed region must not cross a `PROGRAM_WINDOW_SIZE` boundary.
    ///
    /// # Arguments
    /// * `start_address`: The address to start programming at. Must be aligned to `PROGRAM_ALIGNMENT`.
    /// * `data`: The data to program. Must have size <= `PROGRAM_WINDOW_SIZE` and be aligned to `PROGRAM_ALIGNMENT`.
    fn start_program(
        &mut self,
        start_address: FlashAddress,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    /// Returns whether the driver is currently busy with an operation.
    fn is_busy(&mut self) -> bool;

    /// Completes a pending erase or program operation and returns the result.
    ///
    /// This should be called after `is_busy` returns false to check for errors and
    /// reset the driver state.
    fn complete_op(&mut self) -> Result<(), Self::Error>;
}

/// Represents an address in flash memory.
///
/// A flash address consists of an offset within the flash address space.
#[derive(Default, Clone, Copy, PartialEq, Eq, IntoBytes, Immutable, FromBytes, KnownLayout)]
#[repr(transparent)]
pub struct FlashAddress {
    offset: u32,
}

impl core::fmt::Display for FlashAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:08x}", self.offset)
    }
}

impl core::fmt::Debug for FlashAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self, f)
    }
}

impl FlashAddress {
    /// Creates a new `FlashAddress`.
    pub const fn new(offset: u32) -> Self {
        Self { offset }
    }

    /// Returns the offset.
    pub fn offset(&self) -> u32 {
        self.offset
    }
}

impl core::ops::Add<usize> for FlashAddress {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        Self {
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
            offset: self.offset & other as u32,
        }
    }
}

impl core::ops::BitAndAssign<usize> for FlashAddress {
    fn bitand_assign(&mut self, other: usize) {
        self.offset &= other as u32;
    }
}
