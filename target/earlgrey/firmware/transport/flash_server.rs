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
use zfmt::Zfmt;

use earlgrey_util::EarlgreyFlashAddress;
use eflash_driver::{EmbeddedFlash, Permission};
use hal_flash::{BlockingFlash, FlashAddress};
use services_flash_server::FlashIpcServer;
use spi_flash::SpiFlash;
use spi_host::SpiHost0;
use util_ipc::IpcHandle;
use util_types::Blocking;

#[derive(Zfmt)]
#[zfmt(format = "SPI Host init failed: {code:08x}")]
struct SpiHostInitFailed {
    code: u32,
}

#[derive(Zfmt)]
#[zfmt(format = "SPI Flash init failed: {code:08x}")]
struct SpiFlashInitFailed {
    code: u32,
}

struct FlashCtrlInterrupt;

impl Blocking for FlashCtrlInterrupt {
    fn wait_for_notification(&self) {
        loop {
            if let Ok(w) = syscall::object_wait(
                handle::FLASH_INTERRUPTS,
                signals::FLASH_CTRL_OP_DONE,
                Instant::MAX,
            ) {
                if w.pending_signals.contains(signals::FLASH_CTRL_OP_DONE) {
                    break;
                }
            }
        }
        let _ = syscall::interrupt_ack(handle::FLASH_INTERRUPTS, signals::FLASH_CTRL_OP_DONE);
    }
}

fn flash_server() -> Result<(), ErrorCode> {
    let mut eflash_driver =
        EmbeddedFlash::new_with_interrupts(unsafe { flash_ctrl_core::FlashCtrl::new() });
    eflash_driver.set_default_permission(Permission::FULL_ACCESS);
    for i in 5..9 {
        eflash_driver.set_info_permission(FlashAddress::info(0, i, 0), Permission::FULL_ACCESS)?;
        eflash_driver.set_info_permission(FlashAddress::info(1, i, 0), Permission::FULL_ACCESS)?;
    }
    let eflash = BlockingFlash {
        driver: eflash_driver,
        blocking: FlashCtrlInterrupt,
    };
    let mut eflash_server = FlashIpcServer::new(eflash);

    let mmio0 = unsafe { spi_host::RegisterBlock::new(SpiHost0::PTR) };
    let mut spi_host = unsafe { earlgrey_spi_host::SpiHost::new(mmio0) };
    if let Err(e) = spi_host.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI0) {
        let code = u32::from(ErrorCode::from(e));
        util_zfmt::error!(SpiHostInitFailed { code });
        return Err(ErrorCode::from(e));
    }

    let mut spi_flash = SpiFlash::new(spi_host);
    if let Err(e) = spi_flash.init() {
        util_zfmt::error!(SpiFlashInitFailed { code: u32::from(e) });
        return Err(e);
    }
    let mut spi_flash_server = FlashIpcServer::new(spi_flash);

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_UPDATEMGR_SERVICE,
        syscall::Signals::READABLE,
        1, // token 1 = EFlash updatemgr
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_USB_SERVICE,
        syscall::Signals::READABLE,
        2, // token 2 = EFlash usb
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_UPDATEMGR_SERVICE,
        syscall::Signals::READABLE,
        3, // token 3 = SPI Flash updatemgr
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_USB_SERVICE,
        syscall::Signals::READABLE,
        4, // token 4 = SPI Flash usb
    )
    .map_err(ErrorCode::kernel_error)?;

    let mut buf = [0u8; 2064];
    let eflash_updatemgr_ipc = IpcHandle::new(handle::EFLASH_UPDATEMGR_SERVICE);
    let eflash_usb_ipc = IpcHandle::new(handle::EFLASH_USB_SERVICE);
    let spi_flash_updatemgr_ipc = IpcHandle::new(handle::SPI_FLASH_UPDATEMGR_SERVICE);
    let spi_flash_usb_ipc = IpcHandle::new(handle::SPI_FLASH_USB_SERVICE);

    loop {
        let wait_result = syscall::object_wait(
            handle::FLASH_WAIT_GROUP,
            syscall::Signals::READABLE,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;

        let token = wait_result.user_data;
        if token == 1 {
            eflash_server.handle_one(&eflash_updatemgr_ipc, &mut buf)?;
        } else if token == 2 {
            eflash_server.handle_one(&eflash_usb_ipc, &mut buf)?;
        } else if token == 3 {
            spi_flash_server.handle_one(&spi_flash_updatemgr_ipc, &mut buf)?;
        } else if token == 4 {
            spi_flash_server.handle_one(&spi_flash_usb_ipc, &mut buf)?;
        }
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
