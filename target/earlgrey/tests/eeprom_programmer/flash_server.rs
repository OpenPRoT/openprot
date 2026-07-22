// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use flash_server_codegen::handle;
use pw_status::Error;
use services_flash_server::FlashIpcServer;
use spi_host::{RegisterBlock, SpiHost0};
use userspace::process_entry;
use userspace::syscall;
use userspace::syscall::Signals;
use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::IpcHandle;

fn flash_server() -> Result<(), ErrorCode> {
    let mmio0 = unsafe { RegisterBlock::new(SpiHost0::PTR) };
    let mut spi_host = unsafe { earlgrey_spi_host::SpiHost::new(mmio0) };
    if let Err(e) = spi_host.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI0) {
        return Err(ErrorCode::from(e));
    }
    let mut spi_flash = spi_flash::SpiFlash::new(spi_host);
    if let Err(e) = spi_flash.init() {
        return Err(e);
    }
    let mut spi_flash_server = FlashIpcServer::new(spi_flash);

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_USB_SERVICE,
        Signals::READABLE,
        handle::SPI_FLASH_USB_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    let mut buf = [0u8; 2064];
    let spi_flash_usb_ipc = IpcHandle::new(handle::SPI_FLASH_USB_SERVICE);

    loop {
        let wait_result =
            syscall::object_wait(handle::FLASH_WAIT_GROUP, Signals::READABLE, Instant::MAX)
                .map_err(ErrorCode::kernel_error)?;

        let channel = wait_result.user_data as u32;
        if channel == handle::SPI_FLASH_USB_SERVICE {
            let _ = spi_flash_server.handle_one(&spi_flash_usb_ipc, &mut buf);
        }
    }
}

#[process_entry("flash_server")]
fn entry() -> Result<(), Error> {
    let _ = flash_server();
    Err(Error::Unknown)
}
