// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use combined_flash_server_codegen::{handle, signals};
use earlgrey_platform::flash_mux::{
    SwitchMuxFinOp, IPC_OP_FLASH_SWITCH_MUX_FIN_NOTICE, IPC_OP_FLASH_SWITCH_MUX_NOTICE,
};
use earlgrey_util::EarlgreyFlashAddress;
use eflash_driver::{EmbeddedFlash, Permission};
use hal_flash::BlockingFlash;
use hal_flash_driver::FlashAddress;
use services_flash_server::FlashIpcServer;
use spi_flash::SpiFlash;
use spi_host::{SpiHost0, SpiHost1};
use userspace::time::Instant;
use userspace::{entry, syscall};
use util_error::{self as error, ErrorCode};
use util_ipc::{IpcChannel, IpcHandle};
use util_panic as _;
use util_types::{Blocking, Opcode};
use zerocopy::{FromBytes, IntoBytes};

// EFlash Interrupt Blocker
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

fn respond_inaccessible(ipc: &IpcHandle, buf: &mut [u8]) -> Result<(), ErrorCode> {
    let _ = ipc.read(0, buf).map_err(ErrorCode::kernel_error)?;
    let status = error::FLASH_GENERIC_INACCESSIBLE.0.get();
    ipc.respond(&[status.as_bytes()])
        .map_err(ErrorCode::kernel_error)?;
    Ok(())
}

fn handle_mux_control(
    ipc: &IpcHandle,
    buf: &mut [u8],
    accessible_flash_bitmap: &mut u8,
    spi_flash0: &mut SpiFlash<earlgrey_spi_host::SpiHost>,
    spi_flash1: &mut SpiFlash<earlgrey_spi_host::SpiHost>,
) -> Result<(), ErrorCode> {
    let len = ipc.read(0, buf).map_err(ErrorCode::kernel_error)?;
    let (opcode_bytes, reqrsp) = buf.split_at_mut(core::mem::size_of::<Opcode>());
    let opcode = Opcode::read_from_bytes(opcode_bytes).map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
    let req_len = len.saturating_sub(core::mem::size_of::<Opcode>());

    match opcode {
        IPC_OP_FLASH_SWITCH_MUX_NOTICE => {
            *accessible_flash_bitmap = 0;
            ipc.respond(&[0u32.as_bytes()])
                .map_err(ErrorCode::kernel_error)?;
        }
        IPC_OP_FLASH_SWITCH_MUX_FIN_NOTICE => {
            let req_data = reqrsp.get(..req_len).ok_or(error::IPC_ERROR_BAD_REQ_LEN)?;
            let op = SwitchMuxFinOp::read_from_bytes(req_data)
                .map_err(|_| error::IPC_ERROR_BAD_REQ_LEN)?;
            if op.accessible_flash_bitmap == 0 {
                let status = error::IPC_ERROR_BAD_REQ.0.get();
                ipc.respond(&[status.as_bytes()])
                    .map_err(ErrorCode::kernel_error)?;
                return Ok(());
            }
            let mut status = 0u32;
            if (op.accessible_flash_bitmap & 0x1) != 0 {
                if let Err(e) = spi_flash0.init() {
                    status = e.0.get();
                }
            }
            if status == 0 && (op.accessible_flash_bitmap & 0x2) != 0 {
                if let Err(e) = spi_flash1.init() {
                    status = e.0.get();
                }
            }
            if status == 0 {
                *accessible_flash_bitmap = op.accessible_flash_bitmap;
            }
            ipc.respond(&[status.as_bytes()])
                .map_err(ErrorCode::kernel_error)?;
        }
        _ => {
            let status = error::IPC_ERROR_UNKNOWN_OP.0.get();
            ipc.respond(&[status.as_bytes()])
                .map_err(ErrorCode::kernel_error)?;
        }
    }
    Ok(())
}

fn run_server() -> Result<(), ErrorCode> {
    // 1. Initialize EFlash driver.
    pw_log::info!("combined_server: initializing EFlash driver");
    // SAFETY: We have exclusive access to FlashCtrl in this test process.
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

    // 2. Initialize SPI Hosts.
    pw_log::info!("combined_server: initializing SPI Hosts");
    // SAFETY: We have exclusive access to SPI_HOST0 in this test process.
    let mmio0 = unsafe { spi_host::RegisterBlock::new(SpiHost0::PTR) };
    let mut spi_host0 = unsafe { earlgrey_spi_host::SpiHost::new(mmio0) };
    if let Err(e) = spi_host0.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI0) {
        pw_log::error!(
            "combined_server: SPI Host 0 init failed: 0x{:x}",
            u32::from(ErrorCode::from(e))
        );
        return Err(ErrorCode::from(e));
    }

    // SAFETY: We have exclusive access to SPI_HOST1 in this test process.
    let mmio1 = unsafe { spi_host::RegisterBlock::new(SpiHost1::PTR) };
    let mut spi_host1 = unsafe { earlgrey_spi_host::SpiHost::new(mmio1) };
    if let Err(e) = spi_host1.init(&earlgrey_spi_host::SpiConfig::DEFAULT_SPI1) {
        pw_log::error!(
            "combined_server: SPI Host 1 init failed: 0x{:x}",
            u32::from(ErrorCode::from(e))
        );
        return Err(ErrorCode::from(e));
    }

    // 3. Initialize SpiFlash drivers (uninitialized at boot, initialized upon MUX notification).
    let mut spi_flash0 = SpiFlash::new(spi_host0);
    let mut spi_flash1 = SpiFlash::new(spi_host1);

    // 4. Register wait group ports.
    pw_log::info!("combined_server: registering wait group ports");
    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::EFLASH_SERVICE,
        syscall::Signals::READABLE,
        handle::EFLASH_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH0_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_FLASH0_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH1_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_FLASH1_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_GENERIC_FLASH_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_GENERIC_FLASH_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::FLASH_WAIT_GROUP,
        handle::SPI_FLASH_MUX_SERVICE,
        syscall::Signals::READABLE,
        handle::SPI_FLASH_MUX_SERVICE as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    let mut buf = [0u8; 2064];
    let eflash_ipc = IpcHandle::new(handle::EFLASH_SERVICE);
    let spi_flash0_ipc = IpcHandle::new(handle::SPI_FLASH0_SERVICE);
    let spi_flash1_ipc = IpcHandle::new(handle::SPI_FLASH1_SERVICE);
    let spi_generic_flash_ipc = IpcHandle::new(handle::SPI_GENERIC_FLASH_SERVICE);
    let flash_mux_ipc = IpcHandle::new(handle::SPI_FLASH_MUX_SERVICE);
    // External SPI Flash channels start in quiescent state (bitmap = 0) until Platform Service configures MUX.
    let mut accessible_flash_bitmap: u8 = 0;

    // 5. Enter main wait_group loop.
    pw_log::info!("combined_server: entering main wait_group loop");
    loop {
        let wait_result = syscall::object_wait(
            handle::FLASH_WAIT_GROUP,
            syscall::Signals::READABLE,
            Instant::MAX,
        )
        .map_err(ErrorCode::kernel_error)?;

        let token = wait_result.user_data;
        if token == handle::EFLASH_SERVICE as usize {
            eflash_server.handle_one(&eflash_ipc, &mut buf)?;
        } else if token == handle::SPI_FLASH0_SERVICE as usize {
            if (accessible_flash_bitmap & 0x1) == 0 {
                respond_inaccessible(&spi_flash0_ipc, &mut buf)?;
            } else {
                FlashIpcServer::new(&mut spi_flash0).handle_one(&spi_flash0_ipc, &mut buf)?;
            }
        } else if token == handle::SPI_FLASH1_SERVICE as usize {
            if (accessible_flash_bitmap & 0x2) == 0 {
                respond_inaccessible(&spi_flash1_ipc, &mut buf)?;
            } else {
                FlashIpcServer::new(&mut spi_flash1).handle_one(&spi_flash1_ipc, &mut buf)?;
            }
        } else if token == handle::SPI_GENERIC_FLASH_SERVICE as usize {
            if (accessible_flash_bitmap & 0x1) != 0 {
                FlashIpcServer::new(&mut spi_flash0)
                    .handle_one(&spi_generic_flash_ipc, &mut buf)?;
            } else if (accessible_flash_bitmap & 0x2) != 0 {
                FlashIpcServer::new(&mut spi_flash1)
                    .handle_one(&spi_generic_flash_ipc, &mut buf)?;
            } else {
                respond_inaccessible(&spi_generic_flash_ipc, &mut buf)?;
            }
        } else if token == handle::SPI_FLASH_MUX_SERVICE as usize {
            handle_mux_control(
                &flash_mux_ipc,
                &mut buf,
                &mut accessible_flash_bitmap,
                &mut spi_flash0,
                &mut spi_flash1,
            )?;
        }
    }
}

#[entry]
fn entry() -> Result<(), pw_status::Error> {
    pw_log::info!("🔄 COMBINED FLASH SERVER START");
    let ret = run_server();

    let ret = match ret {
        Ok(()) => {
            pw_log::info!("✅ COMBINED FLASH SERVER PASS");
            Ok(())
        }
        Err(e) => {
            pw_log::error!("❌ COMBINED FLASH SERVER FAIL: {:08x}", u32::from(e));
            Err(pw_status::Error::Unknown)
        }
    };
    ret
}
