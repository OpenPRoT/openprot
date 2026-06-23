// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM VCA stress test — requester side.
//!
//! Repeatedly performs the full VCA handshake (GET_VERSION → GET_CAPABILITIES
//! → NEGOTIATE_ALGORITHMS) against the peer responder over real I2C/MCTP
//! transport. Runs indefinitely; the test harness observes correct cycling
//! via the TEST_RESULT sentinel on timeout or error.

#![no_main]
#![no_std]

mod mock_peer_cert_store;
mod mock_platform;

use app_spdm_requester::handle;
use mock_peer_cert_store::MockPeerCertStore;
use mock_platform::{MockCertStore, MockEvidence, MockHash, MockRng};
use openprot_mctp_api::stack::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_spdm_requester::SpdmRequester;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_status::Error;
use spdm_lib::codec::MessageBuf;
use spdm_lib::commands::algorithms::request::generate_negotiate_algorithms_request;
use spdm_lib::commands::capabilities::request::generate_capabilities_request_local;
use spdm_lib::commands::version::request::generate_get_version;
use spdm_lib::commands::version::VersionReqPayload;
use spdm_lib::platform::transport::SpdmTransport as _;
use userspace::{entry, syscall};

const RESPONDER_EID: u8 = 9;

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            pw_log::info!("SPDM requester stress test completed");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("SPDM requester stress test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    loop {}
}

fn run() -> Result<(), u32> {
    pw_log::info!("SPDM VCA stress test starting (requester)");

    let mctp_client = IpcMctpClient::new(handle::MCTP);
    let stack = Stack::new(mctp_client);

    stack.set_eid(8).map_err(|e| {
        pw_log::error!("set_eid failed: {}", e.code as u32);
        1u32
    })?;

    let mut round: u32 = 0;
    loop {
        let mut transport = MctpSpdmTransport::new_requester(&stack, RESPONDER_EID);
        transport.init_sequence().map_err(|_| {
            pw_log::error!("transport init_sequence failed on round {}", round as u32);
            2u32
        })?;

        let mut cert_store = MockCertStore::new();
        let mut peer_cert_store = MockPeerCertStore::new();
        let mut hash = MockHash::new();
        let mut m1_hash = MockHash::new();
        let mut l1_hash = MockHash::new();
        let mut rng = MockRng::new();
        let evidence = MockEvidence::new();

        let mut requester = SpdmRequester::new(
            &mut transport,
            &mut cert_store,
            &mut peer_cert_store,
            &mut hash,
            &mut m1_hash,
            &mut l1_hash,
            &mut rng,
            &evidence,
            None,
        )
        .map_err(|_| {
            pw_log::error!("SpdmRequester::new failed on round {}", round as u32);
            3u32
        })?;

        let mut buf_storage = [0u8; 4096];
        let mut buf = MessageBuf::new(&mut buf_storage);

        // GET_VERSION → VERSION
        generate_get_version(
            requester.context_mut(),
            &mut buf,
            VersionReqPayload::new(0, 0),
        )
        .map_err(|_| {
            pw_log::error!("generate_get_version failed on round {}", round as u32);
            4u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send GET_VERSION failed on round {}", round as u32);
                5u32
            })?;
        buf.reset();
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process VERSION failed on round {}", round as u32);
                6u32
            })?;

        // GET_CAPABILITIES → CAPABILITIES
        buf.reset();
        generate_capabilities_request_local(requester.context_mut(), &mut buf).map_err(|_| {
            pw_log::error!(
                "generate_capabilities_request_local failed on round {}",
                round as u32
            );
            7u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send GET_CAPABILITIES failed on round {}", round as u32);
                8u32
            })?;
        buf.reset();
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process CAPABILITIES failed on round {}", round as u32);
                9u32
            })?;

        // NEGOTIATE_ALGORITHMS → ALGORITHMS
        buf.reset();
        generate_negotiate_algorithms_request(
            requester.context_mut(),
            &mut buf,
            None,
            None,
            None,
            None,
        )
        .map_err(|_| {
            pw_log::error!(
                "generate_negotiate_algorithms_request failed on round {}",
                round as u32
            );
            10u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send NEGOTIATE_ALGORITHMS failed on round {}", round as u32);
                11u32
            })?;
        buf.reset();
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process ALGORITHMS failed on round {}", round as u32);
                12u32
            })?;

        round += 1;
        pw_log::info!("VCA round {} complete", round as u32);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("SPDM requester panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
