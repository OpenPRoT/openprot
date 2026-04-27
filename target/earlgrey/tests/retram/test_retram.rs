// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
//use test_retram_codegen::{handle, signals};
//use userspace::time::Instant;
use userspace::{entry, syscall };
use pw_status::{ Result};
use earlgrey_util::ret_ram::RetRam;
use earlgrey_util::{CheckDigest, GetData};
use earlgrey_util::tags::BootSlot;
use earlgrey_util::boot_svc::NextBl0SlotRequest;
use util_console::println;

use sha2::{Sha256, Digest};
use rstmgr::RstmgrAon;


fn test_retram() -> Result<()> {
    let mut rstmgr = unsafe { RstmgrAon::new() };
    let retram = unsafe { RetRam::mut_ref() };
    println!("Reset Reasons = {:08x}", retram.reset_reasons);
    println!("BootLog = {:#?}", retram.boot_log);
    let ok = retram.boot_log.check_digest(|data| Sha256::digest(data).into());
    println!("ok = {}", ok as bool);


    if retram.reset_reasons == 1 {
        // If power on reset, reset once more.
        println!("Preparing to boot slot B");
        let next: &mut NextBl0SlotRequest = retram.boot_svc.get_mut();
        next.next_bl0_slot = BootSlot::Unspecified;
        next.primary_bl0_slot = BootSlot::SlotB;
        println!("Set digest on boot_svc");
        retram.boot_svc.set_digest(|data| Sha256::digest(data).into());

        println!("Reboot");
        rstmgr.regs_mut().reset_req().write(|_| 6u32.into());
    }
    Ok(())
}

#[entry]
fn entry() -> ! {
    // Since this is written as a test, shut down with the return status from `main()`.
    let ret = test_retram();
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
