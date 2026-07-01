// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM VCA stress test — responder side.
//!
//! Continuously processes incoming SPDM messages from the peer requester.
//! Runs indefinitely alongside the requester's VCA loop.

#![no_main]
#![no_std]

mod mock_platform;

use app_spdm_responder::handle;
use mock_platform::{MockCertStore, MockEvidence, MockHash, MockRng};
use openprot_mctp_api::stack::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_spdm_responder::SpdmResponder;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_status::Error;
use spdm_lib::codec::MessageBuf;
use spdm_lib::platform::transport::SpdmTransport as _;
use userspace::{entry, syscall};

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            pw_log::info!("SPDM responder stress test completed");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("SPDM responder stress test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    loop {}
}

fn run() -> Result<(), u32> {
    pw_log::info!("SPDM VCA stress test starting (responder)");

    let mctp_client = IpcMctpClient::new(handle::MCTP);
    let stack = Stack::new(mctp_client);

    stack.set_eid(9).map_err(|e| {
        pw_log::error!("set_eid failed: {}", e.code as u32);
        1u32
    })?;

    let mut transport = MctpSpdmTransport::new_responder(&stack);
    transport.init_sequence().map_err(|_| {
        pw_log::error!("transport init_sequence failed");
        2u32
    })?;

    let mut cert_store = MockCertStore::new();
    let mut hash = MockHash::new();
    let mut m1_hash = MockHash::new();
    let mut l1_hash = MockHash::new();
    let mut rng = MockRng::new();
    let evidence = MockEvidence::new();

    let mut responder = SpdmResponder::new(
        &mut transport,
        &mut cert_store,
        &mut hash,
        &mut m1_hash,
        &mut l1_hash,
        &mut rng,
        &evidence,
        None,
    )
    .map_err(|_| {
        pw_log::error!("SpdmResponder::new failed");
        3u32
    })?;

    static mut BUF: [u8; 4096] = [0u8; 4096];
    // SAFETY: single-threaded; BUF is not aliased across calls.
    let buf_slice: &'static mut [u8] = unsafe { &mut *core::ptr::addr_of_mut!(BUF) };
    let mut buf = MessageBuf::new(buf_slice);

    loop {
        buf.reset();
        if responder
            .context_mut()
            .responder_process_message(&mut buf)
            .is_err()
        {
            pw_log::error!("process_message failed");
            return Err(4u32);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("SPDM responder panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
