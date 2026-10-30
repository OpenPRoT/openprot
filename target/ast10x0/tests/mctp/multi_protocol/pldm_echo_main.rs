// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Echo task standing in for an PLDM application: one MCTP client of the
//! shared server on its own IPC channel, message type 0x01.

#![no_std]
#![no_main]

use userspace::entry;

use app_pldm_echo::handle;

#[entry]
fn entry() {
    echo_task::serve(handle::MCTP, handle::CTL, echo_task::MSG_TYPE_PLDM)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
