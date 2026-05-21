// Licensed under the Apache-2.0 license

//! PLDM Responder Application (ast10x0)
//!
//! Runs the PLDM responder role using [`PldmResponder`] over an MCTP loopback
//! transport.  Calls [`PldmResponder::run_once`] in a loop, processing one
//! inbound PLDM request per iteration and replying in-place.

#![no_std]
#![no_main]

use openprot_mctp_api::MctpClient;
use openprot_pldm_service::responder::PldmResponder;
use openprot_pldm_service::transport::MctpPldmTransport;
use pw_log::{error, info};
use userspace::entry;
use test_config::{TIMEOUT_MILLIS, CAPS};

use app_pldm_responder::handle;

fn pldm_responder_loop() -> Result<(), &'static str> {
    info!("PLDM responder starting");

    let transport = MctpPldmTransport::new(MctpClient::new(handle::MCTP));
    let mut responder = PldmResponder::new(&CAPS);
    let mut buf = [0u8; 1024];

    loop {
        match responder.run_once(&transport, &mut buf, TIMEOUT_MILLIS) {
            Ok(()) => {
                info!("PLDM responder: processed one request");
            }
            Err(e) => {
                error!("PLDM responder: run_once failed: {:?}", e);
                return Err("run_once failed");
            }
        }
    }
}

#[entry]
fn entry() -> ! {
    info!("PLDM Responder Application");

    match pldm_responder_loop() {
        Ok(_) => {
            info!("PLDM responder exited unexpectedly");
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
