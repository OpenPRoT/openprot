// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_main]
#![no_std]

use app_flash_server::handle;
use flash_backend::{Backend, NoWaitBlocking};
use hal_flash::BlockingFlash;
use services_flash_server::FlashIpcServer;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;
use util_ipc::IpcHandle;

/// IPC buffer: must hold the largest request/response. Reads are bounded by
/// the client's buffer and this size; 4 KiB of payload + opcode/status headroom.
const IPC_BUF_SIZE: usize = 4352;

#[entry]
fn entry() {
    // SAFETY: this process is the sole owner of the FMC/CS0-window mappings
    // declared in system.json5, the kernel target applied the FMC pinmux
    // before starting any process, and this runs once.
    let driver = match unsafe { Backend::new() } {
        Ok(d) => d,
        Err(e) => {
            pw_log::error!("flash server: FMC init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
            loop {}
        }
    };
    let flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let mut server = FlashIpcServer::new(flash);
    let mut buf = [0u8; IPC_BUF_SIZE];

    pw_log::info!("flash server: ready");
    loop {
        if syscall::object_wait(handle::FLASH, Signals::READABLE, Instant::MAX).is_err() {
            continue;
        }
        if let Err(e) = server.handle_one(&IpcHandle::new(handle::FLASH), &mut buf) {
            pw_log::error!("flash server: request failed: {:08x}", e.0.get() as u32);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
