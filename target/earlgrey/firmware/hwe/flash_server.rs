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
use embedded_hal::spi::SpiDevice;
use hal_flash::{BlockingFlash, FlashAddress};
use services_flash_opcode::{JedecIdResp, IPC_OP_FLASH_READ_ID};
use services_flash_server::FlashIpcServer;
use spi_flash::SpiFlash;
use spi_host::{SpiHost0, SpiHost1};
use util_ipc::{IpcChannel, IpcHandle};
use util_types::{Blocking, Opcode};
use zerocopy::{FromBytes, IntoBytes};

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

    let mut spi_host = unsafe {
        // SAFETY: we have exclusive access to the spi_host0 peripheral.
        earlgrey_spi_host::SpiHost::new(spi_host::RegisterBlock::new(SpiHost0::PTR))
    };
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

    let mmio1 = unsafe { spi_host::RegisterBlock::new(SpiHost1::PTR) };
    let mut spi_host1 = unsafe { earlgrey_spi_host::SpiHost::new(mmio1) };
    if let Err(e) = spi_host1.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI1) {
        let code = u32::from(ErrorCode::from(e));
        util_zfmt::error!(SpiHostInitFailed { code });
        return Err(ErrorCode::from(e));
    }

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_UPDATEMGR_SERVICE,
        syscall::Signals::READABLE,
        handle::EFLASH_UPDATEMGR_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_USB_SERVICE,
        syscall::Signals::READABLE,
        handle::EFLASH_USB_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_UPDATEMGR_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_FLASH_UPDATEMGR_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_USB_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_FLASH_USB_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::FLASH_PLATFORM_SERVICE,
        syscall::Signals::READABLE,
        handle::FLASH_PLATFORM_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    let mut buf = [0u8; 2064];
    let eflash_updatemgr_ipc = IpcHandle::new(handle::EFLASH_UPDATEMGR_SERVICE);
    let eflash_usb_ipc = IpcHandle::new(handle::EFLASH_USB_SERVICE);
    let spi_flash_updatemgr_ipc = IpcHandle::new(handle::SPI_FLASH_UPDATEMGR_SERVICE);
    let spi_flash_usb_ipc = IpcHandle::new(handle::SPI_FLASH_USB_SERVICE);
    let flash_platform_ipc = IpcHandle::new(handle::FLASH_PLATFORM_SERVICE);

    loop {
        let wait_result = syscall::object_wait(
            handle::FLASH_WAIT_GROUP,
            syscall::Signals::READABLE,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;

        let channel = wait_result.user_data as u32;
        if channel == handle::EFLASH_UPDATEMGR_SERVICE {
            eflash_server.handle_one(&eflash_updatemgr_ipc, &mut buf)?;
        } else if channel == handle::EFLASH_USB_SERVICE {
            eflash_server.handle_one(&eflash_usb_ipc, &mut buf)?;
        } else if channel == handle::SPI_FLASH_UPDATEMGR_SERVICE {
            spi_flash_server.handle_one(&spi_flash_updatemgr_ipc, &mut buf)?;
        } else if channel == handle::SPI_FLASH_USB_SERVICE {
            spi_flash_server.handle_one(&spi_flash_usb_ipc, &mut buf)?;
        } else if channel == handle::FLASH_PLATFORM_SERVICE {
            let n = flash_platform_ipc
                .read(0, &mut buf)
                .map_err(ErrorCode::kernel_error)?;
            if n >= core::mem::size_of::<Opcode>() {
                let (op_bytes, req_data) = buf.split_at_mut(core::mem::size_of::<Opcode>());
                let op = Opcode::read_from_bytes(op_bytes).unwrap_or(Opcode::new(*b"\0\0\0\0"));
                if op == IPC_OP_FLASH_READ_ID {
                    let eeprom_idx = req_data.first().copied().unwrap_or(0);
                    let mut jedec = JedecIdResp::default();
                    let mut raw = [0u8; 3];
                    let res = if eeprom_idx == 0 {
                        spi_flash_server.flash_mut().read_jedec_id(&mut raw)
                    } else if eeprom_idx == 1 {
                        let mut ops = [
                            embedded_hal::spi::Operation::Write(&[0x9F]),
                            embedded_hal::spi::Operation::Read(&mut raw),
                        ];
                        spi_host1
                            .transaction(&mut ops)
                            .map_err(|_| util_error::FLASH_GENERIC_BUSY)
                    } else {
                        Err(util_error::FLASH_GENERIC_INVALID_SIZE)
                    };
                    let status = match res {
                        Ok(()) => {
                            jedec.manufacturer = raw[0];
                            jedec.memory_type = raw[1];
                            jedec.capacity_code = raw[2];
                            0u32
                        }
                        Err(e) => e.0.get(),
                    };
                    let _ = flash_platform_ipc.respond(&[status.as_bytes(), jedec.as_bytes()]);
                }
            }
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
