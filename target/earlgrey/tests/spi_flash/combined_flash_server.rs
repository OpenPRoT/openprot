// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use combined_flash_server_codegen::{handle, signals};
use earlgrey_util::EarlgreyFlashAddress;
use eflash_driver::{EmbeddedFlash, Permission};
use hal_flash::BlockingFlash;
use hal_flash_driver::FlashAddress;
use pw_status::Error;
use services_flash_server::FlashIpcServer;
use spi_flash::SpiFlash;
use spi_host::SpiHost0;
use userspace::time::Instant;
use userspace::{entry, syscall};
use util_error::{ErrorCode, KERNEL_ERROR_INTERNAL};
use util_ipc::IpcHandle;
use util_types::Blocking;

// 1. EFlash Interrupt Blocker
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

fn run_server() -> Result<(), ErrorCode> {
    pw_log::info!("combined_server: initializing EFlash driver");
    let mut eflash_driver =
        EmbeddedFlash::new_with_interrupts(unsafe { flash_ctrl_core::FlashCtrl::new() });
    eflash_driver.set_default_permission(Permission::FULL_ACCESS);
    // Grant info page permissions as well (same as standard eflash server)
    for i in 5..9 {
        eflash_driver.set_info_permission(FlashAddress::info(0, i, 0), Permission::FULL_ACCESS)?;
        eflash_driver.set_info_permission(FlashAddress::info(1, i, 0), Permission::FULL_ACCESS)?;
    }

    let eflash = BlockingFlash {
        driver: eflash_driver,
        blocking: FlashCtrlInterrupt,
    };
    let mut eflash_server = FlashIpcServer::new(eflash);

    pw_log::info!("combined_server: initializing SPI Host");
    let spi_host0 = unsafe { SpiHost0::new() };
    let mut spi_host = earlgrey_spi_host::SpiHost::new_spi_host0(spi_host0);
    if let Err(e) = spi_host.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI0) {
        let err_num = match e {
            earlgrey_spi_host::SpiError::InvalidTransaction => 1,
            earlgrey_spi_host::SpiError::FifoOverflow => 2,
            earlgrey_spi_host::SpiError::FifoUnderflow => 3,
            earlgrey_spi_host::SpiError::Timeout => 4,
            earlgrey_spi_host::SpiError::HardwareError => 5,
        };
        pw_log::error!("combined_server: SPI Host init failed: {}", err_num);
        return Err(KERNEL_ERROR_INTERNAL);
    }

    pw_log::info!("combined_server: initializing SpiFlash driver");
    let mut spi_flash = SpiFlash::new(spi_host);
    if let Err(e) = spi_flash.init() {
        pw_log::error!(
            "combined_server: SPI Flash init failed: 0x{:x}",
            u32::from(e)
        );
        return Err(e);
    }
    let mut spi_flash_server = FlashIpcServer::new(spi_flash);

    pw_log::info!("combined_server: registering wait group ports");
    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_SERVICE,
        syscall::Signals::READABLE,
        1, // token 1 = EFlash
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_SERVICE,
        syscall::Signals::READABLE,
        2, // token 2 = SPI Flash
    )
    .map_err(ErrorCode::kernel_error)?;

    let mut buf = [0u8; 2064];
    let eflash_ipc = IpcHandle::new(handle::EFLASH_SERVICE);
    let spi_flash_ipc = IpcHandle::new(handle::SPI_FLASH_SERVICE);

    pw_log::info!("combined_server: entering main wait_group loop");
    loop {
        let wait_result = syscall::object_wait(
            handle::FLASH_WAIT_GROUP,
            syscall::Signals::READABLE,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;

        let token = wait_result.user_data;
        if token == 1 {
            eflash_server.handle_one(&eflash_ipc, &mut buf)?;
        } else if token == 2 {
            spi_flash_server.handle_one(&spi_flash_ipc, &mut buf)?;
        }
    }
}

#[entry]
fn entry() -> Result<(), Error> {
    pw_log::info!("🔄 COMBINED FLASH SERVER START");
    let ret = run_server();

    let ret = match ret {
        Ok(()) => {
            pw_log::info!("✅ COMBINED FLASH SERVER PASS");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ COMBINED FLASH SERVER FAIL: {:08x}", u32::from(e));
            Err(Error::Unknown)
        }
    };
    ret
}

util_panic::make_panic_handler!();
