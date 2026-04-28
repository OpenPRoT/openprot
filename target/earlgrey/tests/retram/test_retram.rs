// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use test_retram_codegen::{handle};
use userspace::{entry, syscall };
use pw_status::{ Error};
use earlgrey_sysmgr_client::SysmgrClient;
use earlgrey_util::tags::BootSlot;
use util_error::ErrorCode;
use util_ipc::IpcChannel;
use util_console::println;


fn test_retram() -> Result<(),ErrorCode> {
    let sysmgr = SysmgrClient::new(IpcChannel::new(handle::SYSMGR_SERVICE));

    let boot_info = sysmgr.get_boot_info()?;
    println!("BootInfo = {:#?}", boot_info);

    if boot_info.reset.reason == 1 {
        // If power on reset, reset once more.
        println!("Preparing to boot slot B");
        sysmgr.set_boot_policy(BootSlot::SlotB, BootSlot::Unspecified)?;

        println!("Reboot");
        sysmgr.request_reboot()?;
    }
    Ok(())
}

#[entry]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING");

    // Log that an error occurred so that the app that caused the shutdown is logged.
    let ret = match test_retram() {
        Ok(()) => {
            pw_log::info!("✅ PASSED");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ FAILED: {:08x}", u32::from(e) as u32);
            Err(Error::Unknown)
        }
    };

    // Since this is written as a test, shut down with the return status from `main()`.
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
