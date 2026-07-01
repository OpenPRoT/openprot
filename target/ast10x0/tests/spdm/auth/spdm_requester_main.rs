// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM auth stress test — requester side.
//!
//! Repeatedly performs the full SPDM handshake sequence:
//!   VCA (GET_VERSION → GET_CAPABILITIES → NEGOTIATE_ALGORITHMS)
//!   Auth (GET_DIGESTS → GET_CERTIFICATE → CHALLENGE → CHALLENGE_AUTH)
//!
//! The mock cert store returns a fixed signature; the requester accepts it
//! without cryptographic verification, making this a protocol-layer stress
//! test rather than a crypto correctness test.

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
use spdm_lib::commands::certificate::request::generate_get_certificate;
use spdm_lib::commands::challenge::request::generate_challenge_request;
use spdm_lib::commands::challenge::MeasurementSummaryHashType;
use spdm_lib::commands::digests::request::generate_digest_request;
use spdm_lib::commands::version::request::generate_get_version;
use spdm_lib::commands::version::VersionReqPayload;
use spdm_lib::platform::transport::SpdmTransport as _;
use userspace::{entry, syscall};

const RESPONDER_EID: u8 = 9;

/// Maximum certificate chain size (bytes).
const MAX_CERT_SIZE: u16 = 512;

#[entry]
fn entry() {
    match run() {
        Ok(()) => {
            pw_log::info!("SPDM auth stress test completed");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("SPDM auth stress test FAILED: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
        }
    }
    loop {}
}

fn run() -> Result<(), u32> {
    pw_log::info!("SPDM auth stress test starting (requester)");

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

        // ── GET_VERSION → VERSION ─────────────────────────────────────────

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
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process VERSION failed on round {}", round as u32);
                6u32
            })?;

        // ── GET_CAPABILITIES → CAPABILITIES ──────────────────────────────

        buf.reset();
        generate_capabilities_request_local(requester.context_mut(), &mut buf).map_err(|_| {
            pw_log::error!(
                "generate_capabilities_request failed on round {}",
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
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process CAPABILITIES failed on round {}", round as u32);
                9u32
            })?;

        // ── NEGOTIATE_ALGORITHMS → ALGORITHMS ────────────────────────────

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
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process ALGORITHMS failed on round {}", round as u32);
                12u32
            })?;

        // ── GET_DIGESTS → DIGESTS ─────────────────────────────────────────

        buf.reset();
        generate_digest_request(requester.context_mut(), &mut buf).map_err(|_| {
            pw_log::error!("generate_digest_request failed on round {}", round as u32);
            13u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send GET_DIGESTS failed on round {}", round as u32);
                14u32
            })?;
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process DIGESTS failed on round {}", round as u32);
                15u32
            })?;

        // ── GET_CERTIFICATE → CERTIFICATE ────────────────────────────────
        // Request the full certificate chain in one shot (offset=0, length=MAX_CERT_SIZE).

        buf.reset();
        generate_get_certificate(
            requester.context_mut(),
            &mut buf,
            0,
            0,
            MAX_CERT_SIZE,
            false,
        )
        .map_err(|_| {
            pw_log::error!("generate_get_certificate failed on round {}", round as u32);
            16u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send GET_CERTIFICATE failed on round {}", round as u32);
                17u32
            })?;
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process CERTIFICATE failed on round {}", round as u32);
                18u32
            })?;

        // ── CHALLENGE → CHALLENGE_AUTH ────────────────────────────────────
        // The mock responder signs with a fixed value; we accept without
        // cryptographic verification (protocol-layer stress test only).

        let mut nonce = [0u8; 32];
        requester
            .context_mut()
            .get_random_bytes(&mut nonce)
            .map_err(|_| {
                pw_log::error!("get_random_bytes failed on round {}", round as u32);
                19u32
            })?;

        buf.reset();
        generate_challenge_request(
            requester.context_mut(),
            &mut buf,
            0,
            MeasurementSummaryHashType::None,
            nonce,
            None,
        )
        .map_err(|_| {
            pw_log::error!(
                "generate_challenge_request failed on round {}",
                round as u32
            );
            20u32
        })?;
        requester
            .context_mut()
            .requester_send_request(&mut buf, RESPONDER_EID)
            .map_err(|_| {
                pw_log::error!("send CHALLENGE failed on round {}", round as u32);
                21u32
            })?;
        requester
            .context_mut()
            .requester_process_message(&mut buf)
            .map_err(|_| {
                pw_log::error!("process CHALLENGE_AUTH failed on round {}", round as u32);
                22u32
            })?;

        // The signature bytes remain in buf; we accept them without cryptographic
        // verification — this is a protocol-layer stress test, not a crypto test.
        requester.context_mut().set_authenticated();

        round += 1;
        pw_log::info!("auth round {} complete", round as u32);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("SPDM requester panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
