// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reads the ROM's handoff table from a service, through the mapping of the
//! ROM table in the system config, and checks what the caliptra-mcu-sw ROM
//! records for a boot of this image.

#![no_std]
#![no_main]

use kernel_config::HANDOFF_BASE;
use pw_status::Error;
use userspace::{entry, syscall};
use veer_handoff::{FirmwareBootType, Handoff, RomCapabilities};

fn run() -> Result<(), &'static str> {
    // SAFETY: the system config maps the ROM table for this process, and no
    // code in this image writes it.
    let handoff = unsafe { Handoff::at(HANDOFF_BASE) }.map_err(|_| "no handoff table")?;
    let boot_type = handoff
        .firmware_boot_type()
        .map_err(|_| "no firmware boot type")?;
    let capabilities = handoff.rom_capabilities().ok_or("no ROM capabilities")?;
    pw_log::info!(
        "handoff table {}.{}: boot type {}, ROM capabilities {:#x}",
        handoff.major_version() as u32,
        handoff.minor_version() as u32,
        boot_type.name() as &str,
        capabilities.bits() as u32,
    );

    // The ROM records the source of every image that it starts.
    if boot_type == FirmwareBootType::Unknown {
        return Err("the boot type is unknown");
    }
    // The ROM sets this bit in every build (`mcu_rom_capabilities` in
    // `rom/src/cold_boot.rs` of caliptra-mcu-sw).
    if !capabilities.contains(RomCapabilities::STREAMING_BOOT_I3C) {
        return Err("the ROM does not report streaming boot");
    }
    Ok(())
}

#[entry]
fn entry() -> pw_status::Result<()> {
    let result = run().map_err(|reason| {
        pw_log::error!("handoff test failed: {}", reason as &str);
        Error::Internal
    });
    // The kernel logs PASS or FAIL, which the test runner looks for.
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in handoff test");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
