// Licensed under the Apache-2.0 license

//! PLDM Requester Application (ast10x0)
//!
//! Drives the PLDM firmware-update FD-initiator role using [`PldmRequester`]
//! over an MCTP loopback transport.  Calls [`PldmRequester::run_once`] in a
//! loop until the command interface has no more pending requests or an error
//! occurs.

#![no_std]
#![no_main]

use openprot_mctp_api::MctpClient;
use openprot_pldm_service::requester::PldmRequester;
use openprot_pldm_service::transport::MctpPldmTransport;
use pw_log::{error, info};
use userspace::entry;
use app_pldm_requester::handle;

use test_config::{RESPONDER_EID, TIMEOUT_MILLIS, CAPS};

/// Maximum number of [`PldmRequester::run_once`] iterations before the test
/// declares success.  Acts as a loop guard for the FD firmware-update phases.
const MAX_ITERS: usize = 64;

fn pldm_requester_test() -> Result<(), &'static str> {
    let transport = MctpPldmTransport::new(MctpClient::new(handle::MCTP));
    let mut requester = PldmRequester::new(&CAPS);
    let mut buf = [0u8; 1024];

    for i in 0..MAX_ITERS {
        match requester.run_once(&transport, RESPONDER_EID, &mut buf, TIMEOUT_MILLIS) {
            Ok(()) => {
                info!("PLDM requester: run_once iteration {} complete", i);
            }
            Err(e) => {
                error!("PLDM requester: run_once failed at iteration {}: {:?}", i, e);
                return Err("run_once failed");
            }
        }
    }

    info!("PLDM requester: all {} iterations complete", MAX_ITERS);
    Ok(())
}

#[entry]
fn entry() -> ! {
    info!("PLDM Requester Application");

    match pldm_requester_test() {
        Ok(_) => {
            info!("SUCCESS: All PLDM requester tests passed");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            error!("FAILURE: {}", e as &str);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Unknown));
        }
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
