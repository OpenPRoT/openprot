// Licensed under the Apache-2.0 license

//! SPDM Responder Application for AST1060-EVB
//!
//! This application provides SPDM (Security Protocol and Data Model) responder
//! functionality for device attestation and secure communication.

#![no_std]
#![no_main]

use ast1060_cert_store::Ast1060CertStore;
use ast1060_evidence::Ast1060Evidence;
use openprot_mctp_client::IpcMctpClient;
use openprot_spdm_hash::SpdmCryptoHash;
use openprot_spdm_rng::SpdmCryptoRng;
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_log::info;
use pw_status::Result;
use spdm_lib::codec::MessageBuf;
use spdm_lib::context::SpdmContext;
use spdm_lib::protocol::{
    AeadCipherSuite, AlgorithmPriorityTable, BaseAsymAlgo, BaseHashAlgo, CapabilityFlags,
    DeviceAlgorithms, DeviceCapabilities, DheNamedGroup, KeySchedule, LocalDeviceAlgorithms,
    MeasurementHashAlgo, MeasurementSpecification, MelSpecification, OtherParamSupport,
    ReqBaseAsymAlg, SpdmVersion,
};
use userspace::entry;
use userspace::syscall;

use app_spdm_responder::handle;

/// Create local device algorithms configuration for SPDM
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

/// SPDM responder main loop
fn spdm_responder_loop() -> Result<()> {
    info!("SPDM Responder starting...");

    // Initialize platform implementations
    let mut cert_store = Ast1060CertStore::new(handle::CRYPTO);
    let mut hash = SpdmCryptoHash::new(handle::CRYPTO);
    let mut m1_hash = SpdmCryptoHash::new(handle::CRYPTO);
    let mut l1_hash = SpdmCryptoHash::new(handle::CRYPTO);
    let mut rng = SpdmCryptoRng::new(handle::CRYPTO);
    let evidence = Ast1060Evidence::new();

    info!("Platform implementations initialized");

    // Initialize MCTP transport
    let mctp_client = IpcMctpClient::new(handle::MCTP);
    let mut transport = MctpSpdmTransport::new_responder(mctp_client);

    // Configure device capabilities
    let mut flags = CapabilityFlags::default();
    flags.set_cert_cap(1);      // Certificate capability
    flags.set_chal_cap(1);      // Challenge capability
    flags.set_meas_cap(2);      // Measurements with signature (2 = with signature)
    flags.set_meas_fresh_cap(1); // Measurements freshness capability
    flags.set_chunk_cap(1);     // Chunk capability

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
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V11];

    // Create SPDM context directly (no wrapper)
    let mut context = match SpdmContext::new(
        &SUPPORTED_VERSIONS,
        &mut transport,
        capabilities,
        algorithms,
        &mut cert_store,
        None, // No peer cert store needed for responder
        &mut hash,
        &mut m1_hash,
        &mut l1_hash,
        &mut rng,
        &evidence,
    ) {
        Ok(ctx) => ctx,
        Err(_e) => {
            pw_log::error!("Failed to create SPDM context");
            return Err(pw_status::Error::Unknown);
        }
    };

    info!("SPDM context created, entering message loop");

    // Process SPDM messages
    // Buffer and MessageBuf must live as long as context due to lifetime constraint
    let mut buffer = [0u8; 4096];
    let mut message_buf = MessageBuf::new(&mut buffer);

    loop {
        if let Err(_e) = context.responder_process_message(&mut message_buf) {
            // Continue processing - don't exit on errors
            // Errors are expected during normal operation (e.g., malformed requests)
        }
    }
}

#[entry]
fn entry() -> ! {
    if let Err(e) = spdm_responder_loop() {
        pw_log::error!("SPDM responder error: {:04x}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
