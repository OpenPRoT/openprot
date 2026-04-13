// Licensed under the Apache-2.0 license

//! SPDM Requester Application
//!
//! Runs the SPDM requester flow using spdm-lib's requester API. Communicates
//! with the SPDM responder via the MCTP loopback server using IPC channels.
//!
//! The requester:
//! 1. Connects to the MCTP loopback server via an IPC channel
//! 2. Creates an `MctpSpdmTransport` in requester mode (targeting responder EID)
//! 3. Creates an `SpdmContext` with mock platform implementations
//! 4. Executes the SPDM VCA flow:
//!    - GET_VERSION → VERSION
//!    - GET_CAPABILITIES → CAPABILITIES
//!    - NEGOTIATE_ALGORITHMS → ALGORITHMS
//! 5. Reports success/failure and shuts down

#![no_std]
#![no_main]

use openprot_mctp_client::IpcMctpClient;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_log::{error, info};
use spdm_lib::codec::MessageBuf;
use spdm_lib::commands::algorithms::request::generate_negotiate_algorithms_request;
use spdm_lib::commands::capabilities::request::generate_capabilities_request_local;
use spdm_lib::commands::version::VersionReqPayload;
use spdm_lib::commands::version::request::generate_get_version;
use spdm_lib::context::SpdmContext;
use spdm_lib::platform::transport::SpdmTransport;
use spdm_lib::protocol::{
    AeadCipherSuite, AlgorithmPriorityTable, BaseAsymAlgo, BaseHashAlgo, CapabilityFlags,
    DeviceAlgorithms, DeviceCapabilities, DheNamedGroup, KeySchedule, LocalDeviceAlgorithms,
    MeasurementHashAlgo, MeasurementSpecification, MelSpecification, OtherParamSupport,
    ReqBaseAsymAlg, SpdmVersion,
};
use userspace::entry;
use userspace::syscall;

use app_spdm_requester::handle;

mod mock_platform;
use mock_platform::{MockCertStore, MockEvidence, MockHash, MockRng};

/// Remote EID of the SPDM responder
const RESPONDER_EID: u8 = 42;

/// Create local device algorithms configuration for SPDM requester
fn create_local_algorithms<'a>() -> LocalDeviceAlgorithms<'a> {
    // Measurement specification (DMTF)
    let mut measurement_spec = MeasurementSpecification::default();
    measurement_spec.set_dmtf_measurement_spec(1);

    // Measurement hash algorithm (SHA-384)
    let mut measurement_hash_algo = MeasurementHashAlgo::default();
    measurement_hash_algo.set_tpm_alg_sha_384(1);

    // Base asymmetric algorithm (ECDSA P-384)
    let mut base_asym_algo = BaseAsymAlgo::default();
    base_asym_algo.set_tpm_alg_ecdsa_ecc_nist_p384(1);

    // Base hash algorithm (SHA-384)
    let mut base_hash_algo = BaseHashAlgo::default();
    base_hash_algo.set_tpm_alg_sha_384(1);

    let device_algorithms = DeviceAlgorithms {
        measurement_spec,
        other_param_support: OtherParamSupport::default(),
        measurement_hash_algo,
        base_asym_algo,
        base_hash_algo,
        mel_specification: MelSpecification::default(),
        dhe_group: DheNamedGroup::default(),
        aead_cipher_suite: AeadCipherSuite::default(),
        req_base_asym_algo: ReqBaseAsymAlg::default(),
        key_schedule: KeySchedule::default(),
    };

    let algorithm_priority_table = AlgorithmPriorityTable {
        measurement_specification: None,
        opaque_data_format: None,
        base_asym_algo: None,
        base_hash_algo: None,
        mel_specification: None,
        dhe_group: None,
        aead_cipher_suite: None,
        req_base_asym_algo: None,
        key_schedule: None,
    };

    LocalDeviceAlgorithms {
        device_algorithms,
        algorithm_priority_table,
    }
}

fn spdm_requester_test() -> Result<(), &'static str> {
    info!("SPDM requester starting");
    info!("========================================");

    // Create MCTP client via IPC to loopback server
    let mctp_client = IpcMctpClient::new(handle::MCTP);

    // Create SPDM transport in requester mode (targeting responder EID)
    let mut transport = MctpSpdmTransport::new_requester(mctp_client, RESPONDER_EID);

    // Initialize transport (establishes MCTP request handle)
    if let Err(_) = transport.init_sequence() {
        error!("Failed to initialize requester transport");
        return Err("Transport init failed");
    }

    info!(
        "SPDM transport initialized (requester mode, target EID {})",
        RESPONDER_EID as u32
    );

    // Create mock platform implementations
    // Note: The requester needs a cert store for its own identity (even if unused
    // in the VCA flow) and the context constructor requires it.
    let mut cert_store = MockCertStore::new();
    let mut hash = MockHash::new();
    let mut m1_hash = MockHash::new();
    let mut l1_hash = MockHash::new();
    let mut rng = MockRng::new();
    let evidence = MockEvidence::new();

    // Configure requester capabilities
    let mut flags = CapabilityFlags::default();
    flags.set_cert_cap(1);
    flags.set_chal_cap(1);
    flags.set_meas_cap(0); // Requester doesn't need measurement capability
    flags.set_chunk_cap(1);

    let capabilities = DeviceCapabilities {
        ct_exponent: 0,
        flags,
        data_transfer_size: 1024,
        max_spdm_msg_size: 4096,
        include_supported_algorithms: true,
    };

    let algorithms = create_local_algorithms();

    // Supported SPDM versions
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    // Create SPDM requester context
    // Note: peer_cert_store is None for now (VCA flow doesn't need it).
    // For GET_DIGESTS/GET_CERTIFICATE flows, a PeerCertStore would be required.
    let mut ctx = match SpdmContext::new(
        &SUPPORTED_VERSIONS,
        &mut transport,
        capabilities,
        algorithms,
        &mut cert_store,
        None, // TODO: Add MockPeerCertStore when testing beyond VCA
        &mut hash,
        &mut m1_hash,
        &mut l1_hash,
        &mut rng,
        &evidence,
    ) {
        Ok(ctx) => ctx,
        Err(_) => {
            error!("Failed to create SPDM requester context");
            return Err("SPDM context creation failed");
        }
    };

    info!("SPDM requester context created");
    info!("========================================");

    // Buffer for SPDM message encoding/decoding
    let mut buf = [0u8; 4096];
    let mut msg_buf = MessageBuf::new(&mut buf);

    // ── Step 1: GET_VERSION ──────────────────────────────────────────────
    info!("Step 1: GET_VERSION");
    {
        generate_get_version(&mut ctx, &mut msg_buf, VersionReqPayload::new(0, 0))
            .map_err(|_| "Failed to generate GET_VERSION request")?;
        ctx.requester_send_request(&mut msg_buf, RESPONDER_EID)
            .map_err(|_| "Failed to send GET_VERSION request")?;
    }
    {
        msg_buf.reset();
        ctx.requester_process_message(&mut msg_buf)
            .map_err(|_| "Failed to process VERSION response")?;
    }
    info!("  GET_VERSION completed successfully");

    // ── Step 2: GET_CAPABILITIES ─────────────────────────────────────────
    info!("Step 2: GET_CAPABILITIES");
    {
        msg_buf.reset();
        generate_capabilities_request_local(&mut ctx, &mut msg_buf)
            .map_err(|_| "Failed to generate GET_CAPABILITIES request")?;
        ctx.requester_send_request(&mut msg_buf, RESPONDER_EID)
            .map_err(|_| "Failed to send GET_CAPABILITIES request")?;
    }
    {
        msg_buf.reset();
        ctx.requester_process_message(&mut msg_buf)
            .map_err(|_| "Failed to process CAPABILITIES response")?;
    }
    info!("  GET_CAPABILITIES completed successfully");

    // ── Step 3: NEGOTIATE_ALGORITHMS ─────────────────────────────────────
    info!("Step 3: NEGOTIATE_ALGORITHMS");
    {
        msg_buf.reset();
        generate_negotiate_algorithms_request(&mut ctx, &mut msg_buf, None, None, None, None)
            .map_err(|_| "Failed to generate NEGOTIATE_ALGORITHMS request")?;
        ctx.requester_send_request(&mut msg_buf, RESPONDER_EID)
            .map_err(|_| "Failed to send NEGOTIATE_ALGORITHMS request")?;
    }
    {
        msg_buf.reset();
        ctx.requester_process_message(&mut msg_buf)
            .map_err(|_| "Failed to process ALGORITHMS response")?;
    }
    info!("  NEGOTIATE_ALGORITHMS completed successfully");

    info!("========================================");
    info!("VCA (Version, Capabilities, Algorithms) flow completed!");

    Ok(())
}

#[entry]
fn entry() -> ! {
    info!("SPDM Requester Application");

    match spdm_requester_test() {
        Ok(_) => {
            info!("SUCCESS: All SPDM requester tests passed");
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
    error!("PANIC in SPDM requester");
    loop {}
}
