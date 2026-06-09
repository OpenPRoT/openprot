// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM Responder test application.
//!
//! This application exercises the SPDM responder implementation using
//! spdm-lib over the MCTP transport layer.
//!
//! ## Test Flow
//!
//! 1. Initialize MCTP transport via IPC to mctp_server
//! 2. Create SPDM responder with platform implementations
//! 3. Process incoming SPDM messages
//! 4. Report pass/fail via debug_shutdown

#![no_main]
#![no_std]

use app_spdm_responder::handle;
use openprot_mctp_api::stack::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_status::Error;
use spdm_lib::platform::transport::SpdmTransport;
use userspace::{entry, syscall};

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            pw_log::info!("SPDM responder test PASSED");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("SPDM responder test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    loop {}
}

fn run() -> Result<(), u32> {
    pw_log::info!("SPDM responder test starting");

    // Create MCTP client over IPC
    let mctp_client = IpcMctpClient::new(handle::MCTP);
    let stack = Stack::new(mctp_client);

    // Set local EID
    stack.set_eid(8).map_err(|e| {
        pw_log::error!("Stack set_eid failed: {}", e.code as u32);
        1u32
    })?;

    pw_log::info!("MCTP stack initialized with EID 8");

    // Create SPDM transport in responder mode
    let mut transport = MctpSpdmTransport::new_responder(&stack);

    // Initialize transport (registers listener for SPDM message type)
    transport.init_sequence().map_err(|_| {
        pw_log::error!("Transport init failed");
        2u32
    })?;

    pw_log::info!("SPDM transport initialized");

    // For now, just verify we can initialize the transport.
    // Full responder testing requires platform implementations for:
    // - CertStore (certificate management)
    // - Hash (SHA-384/512)
    // - RNG (random number generation)
    // - Evidence (measurements)
    //
    // TODO: Add stub implementations and exercise full SPDM flow.

    pw_log::info!("SPDM responder initialization complete");

    Ok(())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("SPDM responder panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
