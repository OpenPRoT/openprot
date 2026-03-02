// Licensed under the Apache-2.0 license

//! SPDM Responder
//!
//! Userspace service that handles SPDM protocol exchanges using spdm-lib.

#![no_main]
#![no_std]

use userspace::entry;

#[entry]
fn entry() -> ! {
    pw_log::info!("SPDM responder starting");

    #[expect(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
