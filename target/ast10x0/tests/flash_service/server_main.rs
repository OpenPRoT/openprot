// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Serves the FMC CS0 flash on the `flash_cs0` channel and the CS1 flash on the
//! `flash_cs1` channel through the flash server runtime. Both drivers share one
//! FMC controller in this process.

#![no_main]
#![no_std]

use app_flash_server::handle;
use ast10x0_peripherals::smc::ChipSelect;
use flash_backend::{Backend, FmcController, NoWaitBlocking};
use flash_server_runtime::Channel;
use hal_flash::BlockingFlash;
use hal_flash_driver::FlashDriver;
use openprot_flash_server::Server;
use pw_status::{Error, Result};
use userspace::{entry, syscall};

/// The 4-byte opcode of a request or status of a reply.
const HEADER_LEN: usize = 4;

/// Holds one request or reply: the header and the largest read the driver
/// serves in one call.
const BUF_SIZE: usize = HEADER_LEN + Backend::MAX_READ_SIZE;

/// Creates the flash for `cs`, or logs why it could not.
fn flash(
    controller: &FmcController,
    cs: ChipSelect,
) -> Option<BlockingFlash<Backend<'_>, NoWaitBlocking>> {
    match controller.driver(cs) {
        Ok(driver) => Some(BlockingFlash {
            driver,
            blocking: NoWaitBlocking,
        }),
        Err(e) => {
            pw_log::error!(
                "flash server: CS{} init failed: {:08x}",
                cs as u8,
                e.0.get() as u32
            );
            None
        }
    }
}

#[entry]
fn entry() -> Result<()> {
    // SAFETY: this process is the sole owner of the FMC and CS window mappings
    // declared in system.json5, the kernel target applied the FMC pinmux before
    // starting any process, and this runs once.
    let controller = match unsafe { FmcController::new() } {
        Ok(controller) => controller,
        Err(e) => {
            pw_log::error!("flash server: FMC init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            return Err(Error::Internal);
        }
    };
    let (Some(cs0), Some(cs1)) = (
        flash(&controller, ChipSelect::Cs0),
        flash(&controller, ChipSelect::Cs1),
    ) else {
        let _ = syscall::debug_shutdown(Err(Error::Internal));
        return Err(Error::Internal);
    };
    // `servers[i]` serves `channels[i]`.
    let mut servers = [Server::new(cs0), Server::new(cs1)];
    let mut channels = [
        Channel::new(handle::FLASH_CS0),
        Channel::new(handle::FLASH_CS1),
    ];
    let mut buf = [0; BUF_SIZE];

    pw_log::info!("flash server: ready");
    // `run` returns only if setting up the wait group or waiting on it fails.
    let result = flash_server_runtime::run(handle::WG, &mut channels, &mut servers, &mut buf);
    pw_log::error!("flash server stopped");
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}
