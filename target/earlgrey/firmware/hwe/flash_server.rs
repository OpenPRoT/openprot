// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use flash_server_codegen::{handle, signals};
use pw_status::Error;
use userspace::time::Instant;
use userspace::{process_entry, syscall};
use util_error::{AsStatus, ErrorCode};
use util_zfmt::messages::{ProcessExit, ProcessStart};

use earlgrey_util::EarlgreyFlashAddress;
use eflash_driver::{EmbeddedFlash, Permission};
use hal_flash::{BlockingFlash, FlashAddress};
use services_flash_server::FlashIpcServer;
use util_blocking::pigweed::BlockingInterrupt;
use util_ipc::IpcHandle;

fn flash_server() -> Result<(), ErrorCode> {
    let mut driver =
        EmbeddedFlash::new_with_interrupts(unsafe { flash_ctrl_core::FlashCtrl::new() });
    driver.set_default_permission(Permission::FULL_ACCESS);
    for i in 5..9 {
        driver.set_info_permission(FlashAddress::info(0, i, 0), Permission::FULL_ACCESS)?;
        driver.set_info_permission(FlashAddress::info(1, i, 0), Permission::FULL_ACCESS)?;
    }
    let flash = BlockingFlash {
        driver,
        blocking: BlockingInterrupt {
            handle: handle::FLASH_INTERRUPTS,
            signals: signals::FLASH_CTRL_OP_DONE,
        },
    };
    let mut flash_server = FlashIpcServer::new(flash);
    let mut buf = [0u8; 2064];
    let ipc = IpcHandle::new(handle::FLASH_SERVICE);
    loop {
        syscall::object_wait(
            handle::FLASH_SERVICE,
            syscall::Signals::READABLE,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;
        flash_server.handle_one(&ipc, &mut buf)?;
    }
}

#[process_entry("flash_server")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart {
        name: "flash_server"
    });
    let ret = flash_server();
    util_zfmt::error!(ProcessExit {
        name: "flash_server",
        status: ret.as_status()
    });

    Err(Error::Unknown)
}
