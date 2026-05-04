// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Backend trait that platform flash drivers implement.
//!
//! The shape mirrors the `FlashStorage` HIL from caliptra-mcu-sw but is
//! synchronous and buffer-borrowing rather than callback-based: the
//! server runtime drives concurrency, so backends only need to expose
//! a blocking-or-`WouldBlock` surface.

use crate::protocol::{FlashError, FlashGeometry, FlashRegion, GeometryFlags, RegionAttrs};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendError {
    InvalidOperation,
    InvalidAddress,
    InvalidLength,
    BufferTooSmall,
    Busy,
    Timeout,
    /// Backend cannot complete synchronously at this time; the server
    /// runtime should retry after `OPERATION_COMPLETE` fires.
    WouldBlock,
    /// Media-level failure (program/erase verify fail, ECC uncorrectable, …).
    IoError,
    /// Region is write-protected, locked, or otherwise refused.
    NotPermitted,
    InternalError,
}

impl From<BackendError> for FlashError {
    fn from(value: BackendError) -> Self {
        match value {
            BackendError::InvalidOperation => FlashError::InvalidOperation,
            BackendError::InvalidAddress => FlashError::InvalidAddress,
            BackendError::InvalidLength => FlashError::InvalidLength,
            BackendError::BufferTooSmall => FlashError::BufferTooSmall,
            BackendError::Busy => FlashError::Busy,
            BackendError::Timeout => FlashError::Timeout,
            BackendError::WouldBlock => FlashError::WouldBlock,
            BackendError::IoError => FlashError::IoError,
            BackendError::NotPermitted => FlashError::NotPermitted,
            BackendError::InternalError => FlashError::InternalError,
        }
    }
}

/// Static description of the flash region a backend exposes. Reported
/// to clients via `GetCapacity`.
///
/// The per-call payload cap is *not* part of `FlashInfo`: it is fixed
/// by the protocol (`MAX_PAYLOAD_SIZE`) and the same for every
/// backend. Clients reference the constant directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlashInfo {
    /// Total addressable bytes [0, capacity).
    pub capacity: u32,
    /// Smallest erasable unit, in bytes. Erase requests must be aligned
    /// and sized in multiples of this value.
    pub erase_size: u32,
}

pub trait FlashBackend {
    /// Per-call routing key. Single-CS backends set this to `()`; multi-CS
    /// backends set it to a CS index (e.g. `ChipSelect`) so the server
    /// runtime can dispatch each channel to the right device on a shared
    /// controller.
    type RouteKey: Copy;

    /// Static layout/capability of the device selected by `key`.
    fn info(&self, key: Self::RouteKey) -> FlashInfo;

    /// Wire-shaped geometry for the device selected by `key`. Powers the
    /// `GetGeometry` opcode.
    ///
    /// Default derives from `info()`: a single erase granularity (the
    /// one already advertised in `FlashInfo`), 256-byte page,
    /// address-width inferred from capacity, no capability flags. A
    /// backend that supports multiple erase granules (4 K + 64 K, etc.)
    /// or has cross-flash DMA / hash-eligibility should override.
    fn geometry(&self, key: Self::RouteKey) -> Result<FlashGeometry, BackendError> {
        let info = self.info(key);
        let erase_bitmap = if info.erase_size != 0 && info.erase_size.is_power_of_two() {
            info.erase_size
        } else {
            0
        };
        let address_width: u8 = if info.capacity > 0x0100_0000 { 4 } else { 3 };
        Ok(FlashGeometry::new(
            info.capacity,
            256,
            erase_bitmap,
            info.erase_size,
            address_width,
            GeometryFlags::empty(),
        ))
    }

    /// Logical regions exposed by the device selected by `key`. Powers
    /// the `GetRegions` opcode. Writes up to `out.len()` records into
    /// `out` and returns the number written.
    ///
    /// Default reports a single whole-chip region with no protection
    /// attributes. Backends that carve sub-regions (e.g. ROT internal
    /// active / recovery / state / AFM) override.
    fn regions(
        &self,
        key: Self::RouteKey,
        out: &mut [FlashRegion],
    ) -> Result<usize, BackendError> {
        if out.is_empty() {
            return Err(BackendError::BufferTooSmall);
        }
        let info = self.info(key);
        out[0] = FlashRegion::new(0, 0, info.capacity, RegionAttrs::WHOLE_CHIP);
        Ok(1)
    }

    /// Probe whether the flash device selected by `key` is present and
    /// responsive.
    ///
    /// Default implementation assumes presence so existing backends remain
    /// source-compatible until they opt into a hardware-backed probe.
    fn exists(&mut self, _key: Self::RouteKey) -> Result<bool, BackendError> {
        Ok(true)
    }

    /// Read up to `out.len()` bytes from the device selected by `key`,
    /// starting at the device-relative `address`, into `out`. Returns the
    /// number of bytes actually read.
    fn read(
        &mut self,
        key: Self::RouteKey,
        address: u32,
        out: &mut [u8],
    ) -> Result<usize, BackendError>;

    /// Write `data` to the device selected by `key`, starting at the
    /// device-relative `address`. Returns the number of bytes actually
    /// written.
    fn write(
        &mut self,
        key: Self::RouteKey,
        address: u32,
        data: &[u8],
    ) -> Result<usize, BackendError>;

    /// Erase `length` bytes on the device selected by `key`, starting at
    /// the device-relative `address`. Both must be multiples of
    /// `FlashInfo::erase_size`.
    fn erase(
        &mut self,
        key: Self::RouteKey,
        address: u32,
        length: u32,
    ) -> Result<(), BackendError>;

    /// Arm whatever interrupt sources the backend uses to signal that
    /// a previously-blocked operation can now resume. The backend
    /// owns the choice of which controller bits to flip; the runtime
    /// only expresses "about to wait." Interrupts are controller-wide,
    /// not per-CS, so this method does not take a `RouteKey`.
    /// Synchronous-only backends implement this as `Ok(())` so the
    /// choice is visible at the impl site rather than silently
    /// inherited.
    fn enable_interrupts(&mut self) -> Result<(), BackendError>;

    /// Disarm the interrupt sources armed by `enable_interrupts`. Same
    /// constraints: required, no default, no `RouteKey`.
    fn disable_interrupts(&mut self) -> Result<(), BackendError>;
}
