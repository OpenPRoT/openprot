// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use sysmgr_codegen::{handle};
use pw_status::Error;
use userspace::{entry, syscall};

use earlgrey_sysmgr_server::SysmgrServer;
use util_error::ErrorCode;
use util_ipc::IpcChannel;

fn sysmgr_server() -> Result<(), ErrorCode> {
    let mut sysmgr = SysmgrServer::new()?;
    let mut buf = [0u8; 512];
    let ipc = IpcChannel::new(handle::SYSMGR_SERVICE);
    sysmgr.run(&ipc, &mut buf)
}

#[entry]
fn entry() -> ! {
    let ret = sysmgr_server().map_err(|e| {
        pw_log::error!("❌ FAILED: {:08x}", u32::from(e) as u32);
        Error::Unknown
    });
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
