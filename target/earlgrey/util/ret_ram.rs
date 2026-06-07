// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Retention SRAM layout and access for Earlgrey.
//!
//! Retention SRAM is a 4KiB memory region that persists data across warm resets
//! (like watchdog resets or software-initiated resets). It is used to pass
//! boot logs from early boot stages to the application, and to request Boot Services.

use crate::boot_log::BootLog;
use crate::boot_svc::BootSvc;
use crate::rom_error::RomError;
use crate::tags::RetRamVersion;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// The memory layout of the Retention SRAM.
///
/// This structure maps exactly to the 4KiB retention RAM.
#[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct RetRam {
    /// The layout version of the retention RAM.
    pub version: RetRamVersion,
    /// Reset reasons captured by the bootloader.
    pub reset_reasons: u32,
    /// Boot Services request/response payload area.
    pub boot_svc: BootSvc,
    /// Reserved space.
    pub reserved: [u8; 1652],
    /// The boot log populated by ROM and ROM_EXT.
    pub boot_log: BootLog,
    /// The shutdown reason for the last reset.
    pub last_shutdown_reason: RomError,
    /// Owner-specific persistent storage area.
    pub owner: [u8; 2048],
}

impl RetRam {
    /// Returns a static mutable reference to the Retention RAM.
    ///
    /// # Safety
    ///
    /// This function performs a raw pointer cast to the physical address of the
    /// Retention SRAM (`0x4060_0000`). It is safe to call in userspace only if
    /// the process has been granted MMIO access to this page (e.g. mapped in `system.json5`).
    pub unsafe fn mut_ref() -> &'static mut RetRam {
        unsafe {
            let rr = core::slice::from_raw_parts_mut(
                top_earlgrey::RAM_RET_AON_BASE_ADDR as *mut u8,
                top_earlgrey::RAM_RET_AON_SIZE_BYTES,
            );
            RetRam::mut_from_bytes(rr).unwrap()
        }
    }
}
