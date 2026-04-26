// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
use flash_server_codegen::{handle, signals};
use pw_status::{Error};
use userspace::time::Instant;
use userspace::{entry, syscall};

use hal_flash::{BlockingFlash, FlashAddress};
use services_flash_server::FlashIpcServer;
use util_types::Blocking;
use util_ipc::IpcChannel;
use util_error::ErrorCode;
use eflash_driver::{EmbeddedFlash, Permission};

struct FlashCtrlInterrupt;

impl Blocking for FlashCtrlInterrupt {
    fn wait_for_notification(&self) {
        loop {
            let w = syscall::object_wait(
                handle::FLASH_INTERRUPTS,
                signals::FLASH_CTRL_OP_DONE,
                Instant::MAX,
            )
            .unwrap();
            if w.pending_signals.contains(signals::FLASH_CTRL_OP_DONE) {
                break;
            }
        }
        let _ = syscall::interrupt_ack(handle::FLASH_INTERRUPTS, signals::FLASH_CTRL_OP_DONE);
    }
}

fn flash_server() -> Result<(), ErrorCode> {
    let mut driver = EmbeddedFlash::new_with_interrupts(
        unsafe { flash_ctrl_core::FlashCtrl::new() });
    driver.set_default_permission(Permission::FULL_ACCESS);
    for i in 5..9 {
        driver.set_info_permission(FlashAddress::info(0, i, 0), Permission::FULL_ACCESS)?;
        driver.set_info_permission(FlashAddress::info(1, i, 0), Permission::FULL_ACCESS)?;
    }
    let flash = BlockingFlash {
        driver,
        blocking: FlashCtrlInterrupt,
    };
    let mut flash_server = FlashIpcServer::new(flash);
    let mut buf = [0u8; 2056];
    let ipc = IpcChannel::new(handle::FLASH_SERVICE);
    flash_server.run(&ipc, &mut buf)
}

#[entry]
fn entry() -> ! {
    pw_log::info!("🔄 RUNNING");
    let ret = flash_server();

    // Log that an error occurred so that the app that caused the shutdown is logged.
    let ret = match ret {
        Ok(()) => { pw_log::info!("✅ PASSED"); Ok(()) }
        Err(e) => { pw_log::error!("❌ FAILED: {:08x}", u32::from(e) as u32); Err(Error::Unknown) }
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
