// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Serves the primary flash of the emulator or the FPGA on the `flash`
//! channel. The `fpga` feature selects the FPGA's controller.

#![no_main]
#![no_std]

use app_flash_server::handle;
use flash_server_runtime::Channel;
use hal_flash::BlockingFlash;
use openprot_flash_server::Server;
use pw_status::{Error, Result};
use userspace::{entry, syscall};
#[cfg(not(feature = "fpga"))]
use veer_flash::EmulatorFlashCtrl as FlashCtrl;
#[cfg(feature = "fpga")]
use veer_flash::FpgaFlashCtrl as FlashCtrl;
use veer_flash::{NoWait, SoftFlash};

/// Holds one request or reply, including its 4-byte opcode or status.
const BUF_SIZE: usize = 1024;

#[entry]
fn entry() -> Result<()> {
    // SAFETY: the system config maps the controller's registers into this
    // process, and no other process or the kernel accesses them.
    let driver = SoftFlash::new(unsafe { FlashCtrl::primary() });
    let flash = BlockingFlash {
        driver,
        blocking: NoWait,
    };
    let mut servers = [Server::new(flash)];
    let mut channels = [Channel::new(handle::FLASH)];
    let mut buf = [0; BUF_SIZE];
    // `run` returns only if setting up the wait group or waiting on it fails.
    let result = flash_server_runtime::run(handle::WG, &mut channels, &mut servers, &mut buf);
    pw_log::error!("Flash server stopped");
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}
