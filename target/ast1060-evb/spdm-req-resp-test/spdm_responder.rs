// Licensed under the Apache-2.0 license

//! SPDM Responder Application
//!
//! Runs the SPDM responder using spdm-lib's responder API. Communicates with
//! the SPDM requester via the MCTP loopback server using IPC channels.
//!
//! The responder:
//! 1. Connects to the MCTP loopback server via an IPC channel
//! 2. Creates an `MctpSpdmTransport` in responder mode (listener for SPDM messages)
//! 3. Creates an `SpdmContext` with mock platform implementations
//! 4. Loops processing incoming SPDM requests via `responder_process_message`

#![no_std]
#![no_main]

use openprot_mctp_client::IpcMctpClient;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_log::{error, info};
use spdm_lib::codec::MessageBuf;
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

use app_spdm_responder::handle;

mod mock_platform;
use mock_platform::{MockCertStore, MockEvidence, MockHash, MockRng};

/// Create local device algorithms configuration for SPDM responder
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

fn spdm_responder_loop() -> Result<(), &'static str> {
    info!("SPDM responder starting");

    // Create MCTP client via IPC to loopback server
    let mctp_client = IpcMctpClient::new(handle::MCTP);

    // Create SPDM transport in responder mode
    let mut transport = MctpSpdmTransport::new_responder(mctp_client);

    // Initialize transport (registers MCTP listener for SPDM message type 0x05)
    if let Err(_) = transport.init_sequence() {
        error!("Failed to initialize responder transport");
        return Err("Transport init failed");
    }

    info!("SPDM transport initialized (responder mode)");

    // Create mock platform implementations
    let mut cert_store = MockCertStore::new();
    let mut hash = MockHash::new();
    let mut m1_hash = MockHash::new();
    let mut l1_hash = MockHash::new();
    let mut rng = MockRng::new();
    let evidence = MockEvidence::new();

    // Configure device capabilities
    let mut flags = CapabilityFlags::default();
    flags.set_cert_cap(1); // Certificate capability
    flags.set_chal_cap(1); // Challenge capability
    flags.set_meas_cap(2); // Measurements with signature
    flags.set_meas_fresh_cap(1); // Measurements freshness
    flags.set_chunk_cap(1); // Chunk capability

    let capabilities = DeviceCapabilities {
        ct_exponent: 0,
        flags,
        data_transfer_size: 1024,
        max_spdm_msg_size: 4096,
        include_supported_algorithms: true,
    };

    // Configure supported algorithms
    let algorithms = create_local_algorithms();

    // Supported SPDM versions
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    // Create SPDM responder context
    let mut ctx = match SpdmContext::new(
        &SUPPORTED_VERSIONS,
        &mut transport,
        capabilities,
        algorithms,
        &mut cert_store,
        None, // No peer cert store for responder
        &mut hash,
        &mut m1_hash,
        &mut l1_hash,
        &mut rng,
        &evidence,
    ) {
        Ok(ctx) => ctx,
        Err(_) => {
            error!("Failed to create SPDM responder context");
            return Err("SPDM context creation failed");
        }
    };

    info!("SPDM responder context created, entering message loop");

    // Responder loop: process incoming SPDM requests
    let mut response_buf = [0u8; 4096];
    let mut msg_buf = MessageBuf::new(&mut response_buf);
    loop {
        msg_buf.reset();
        match ctx.responder_process_message(&mut msg_buf) {
            Ok(_) => {
                info!("Processed SPDM request successfully");
            }
            Err(_) => {
                error!("Error processing SPDM request");
                // Continue processing — don't exit on individual message errors
            }
        }
    }
}

#[entry]
fn entry() -> ! {
    info!("SPDM Responder Application");

    match spdm_responder_loop() {
        Ok(_) => {
            // Responder loop should not return Ok — it loops forever
            info!("SPDM responder exited unexpectedly");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            error!("SPDM responder FAILURE: {}", e as &str);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::Unknown));
        }
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    error!("PANIC in SPDM responder");
    loop {}
}
