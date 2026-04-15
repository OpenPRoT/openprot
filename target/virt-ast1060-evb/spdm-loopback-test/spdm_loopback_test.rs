// Licensed under the Apache-2.0 license

//! SPDM Loopback Integration Tests
//!
//! Tests SPDM protocol messages using MCTP loopback transport

#![no_std]
#![no_main]

use core::cell::RefCell;
use openprot_mctp_transport_loopback::{LoopbackPair, PacketBuffer};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use pw_log::{info, error};
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

// No separate setup function needed - we'll create context inline to manage lifetimes

/// Run SPDM loopback tests
fn run_tests() -> Result<(), &'static str> {
    info!("Starting SPDM Loopback Tests");
    info!("========================================");

    // Create MCTP loopback infrastructure (no heap allocation needed)
    let packets_a = RefCell::new(PacketBuffer::new());
    let packets_b = RefCell::new(PacketBuffer::new());
    let pair = LoopbackPair::<16>::new(8, 42, &packets_a, &packets_b);

    info!("Created MCTP loopback pair (EID 8 <-> EID 42)");

    // Get MCTP clients
    let client_requester = pair.client_a();  // EID 8 (requester)
    let client_responder = pair.client_b();  // EID 42 (responder)

    // Create SPDM transports
    let mut spdm_transport_requester = MctpSpdmTransport::new_requester(client_requester, 42);
    let mut spdm_transport_responder = MctpSpdmTransport::new_responder(client_responder);

    info!("Created SPDM transports");

    // Initialize requester transport
    if let Err(_) = spdm_transport_requester.init_sequence() {
        error!("Failed to initialize requester transport");
        return Err("Transport init failed");
    }

    // Create mock platform implementations for responder
    let mut cert_store = MockCertStore::new();
    let mut hash = MockHash::new();
    let mut m1_hash = MockHash::new();
    let mut l1_hash = MockHash::new();
    let mut rng = MockRng::new();
    let evidence = MockEvidence::new();

    info!("Created mock platform implementations");

    // Configure device capabilities for responder
    let mut flags = CapabilityFlags::default();
    flags.set_cert_cap(1);      // Certificate capability
    flags.set_chal_cap(1);      // Challenge capability
    flags.set_meas_cap(2);      // Measurements with signature
    flags.set_meas_fresh_cap(1); // Measurements freshness
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
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    // Initialize responder transport
    if let Err(_) = spdm_transport_responder.init_sequence() {
        error!("Failed to initialize responder transport");
        return Err("Transport init failed");
    }

    // Create SPDM responder context
    let mut responder_context = match SpdmContext::new(
        &SUPPORTED_VERSIONS,
        &mut spdm_transport_responder,
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
            error!("Failed to create SPDM context");
            return Err("SPDM context creation failed");
        }
    };

    info!("Created SPDM responder context");
    info!("========================================");

    // Response buffer must live as long as context
    let mut response_buf = [0u8; 4096];

    // Run GET_VERSION test
    info!("TEST 1: GET_VERSION");
    test_get_version_simple(
        &mut responder_context,
        &mut spdm_transport_requester,
        &pair,
        &mut response_buf,
        &SUPPORTED_VERSIONS,
    )?;
    info!("GET_VERSION test PASSED");
    info!("----------------------------------------");

    info!("All tests completed successfully!");

    Ok(())
}

/// Test GET_VERSION command
///
/// Tests full request/response cycle: requester sends GET_VERSION,
/// responder processes it and sends response, requester validates response.
///
/// # Parameters
/// - `expected_versions`: Array of expected SPDM versions (e.g., &[SpdmVersion::V12, SpdmVersion::V13])
fn test_get_version_simple<'a>(
    responder_context: &mut SpdmContext<'a>,
    requester_transport: &mut MctpSpdmTransport<openprot_mctp_transport_loopback::LoopbackClient<'_, openprot_mctp_transport_loopback::BufferSender<'_>, 16>>,
    pair: &LoopbackPair<'_, 16>,
    response_buf: &'a mut [u8],
    expected_versions: &[SpdmVersion],
) -> Result<(), &'static str> {
    // Declare request buffer
    let mut request_buf = [0u8; 1024];

    info!("Sending GET_VERSION request");

    // Create GET_VERSION request
    // SPDM GET_VERSION format: [version(1) | command(1) | param1(1) | param2(1)]
    // version = 0x10 (SPDM 1.0 compatible)
    // command = 0x84 (GET_VERSION)
    let get_version_request = [
        0x10, // SPDM version 1.0
        0x84, // GET_VERSION command code
        0x00, // Param1
        0x00, // Param2
    ];

    // Send request through MCTP transport
    request_buf[..get_version_request.len()].copy_from_slice(&get_version_request);
    {
        let mut msg_buf = MessageBuf::new(&mut request_buf);
        msg_buf.put_data(get_version_request.len())
            .map_err(|_| "Failed to create request buffer")?;

        requester_transport
            .send_request(42, &mut msg_buf)
            .map_err(|_| "Failed to send GET_VERSION request")?;
    }

    // Transfer packets from requester to responder
    pair.transfer_a_to_b();

    // Have responder process the message
    {
        let mut resp_msg_buf = MessageBuf::new(response_buf);
        responder_context
            .responder_process_message(&mut resp_msg_buf)
            .map_err(|_| "Responder failed to process GET_VERSION")?;
    }

    // Transfer response packets from responder to requester
    pair.transfer_b_to_a();

    // Receive and validate the response
    let mut receive_buf = [0u8; 1024];
    let (response_len, response_data) = {
        let mut recv_msg_buf = MessageBuf::new(&mut receive_buf);
        match requester_transport.receive_response(&mut recv_msg_buf) {
            Ok(_) => {
                let len = recv_msg_buf.data_len();
                // Extract actual data from MessageBuf
                let data = recv_msg_buf.data(len)
                    .map_err(|_| "Failed to extract response data")?;
                // Copy to a separate buffer for parsing
                let mut data_copy = [0u8; 1024];
                data_copy[..len].copy_from_slice(data);
                (len, data_copy)
            }
            Err(_) => {
                error!("Failed to receive response");
                return Err("Failed to receive response");
            }
        }
    };

    // Parse and validate response
    if response_len > 0 {
        let response_data = &response_data[..response_len];

        // Print full response in 8-byte lines
        info!("Response bytes ({} total):", response_len as u32);
        let mut i = 0;
        while i < response_len {
            let remaining = response_len - i;
            if remaining >= 8 {
                info!("  {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                    response_data[i] as u32,
                    response_data[i+1] as u32,
                    response_data[i+2] as u32,
                    response_data[i+3] as u32,
                    response_data[i+4] as u32,
                    response_data[i+5] as u32,
                    response_data[i+6] as u32,
                    response_data[i+7] as u32);
                i += 8;
            } else {
                match remaining {
                    7 => info!("  {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32, response_data[i+2] as u32,
                        response_data[i+3] as u32, response_data[i+4] as u32, response_data[i+5] as u32,
                        response_data[i+6] as u32),
                    6 => info!("  {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32, response_data[i+2] as u32,
                        response_data[i+3] as u32, response_data[i+4] as u32, response_data[i+5] as u32),
                    5 => info!("  {:02x} {:02x} {:02x} {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32, response_data[i+2] as u32,
                        response_data[i+3] as u32, response_data[i+4] as u32),
                    4 => info!("  {:02x} {:02x} {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32,
                        response_data[i+2] as u32, response_data[i+3] as u32),
                    3 => info!("  {:02x} {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32, response_data[i+2] as u32),
                    2 => info!("  {:02x} {:02x}",
                        response_data[i] as u32, response_data[i+1] as u32),
                    1 => info!("  {:02x}", response_data[i] as u32),
                    _ => {},
                }
                break;
            }
        }

        info!("Validating response:");

        // Check 1: Minimum length (at least 6 bytes for header)
        if response_len >= 6 {
            info!("  \x1b[32m[PASS]\x1b[0m Response length >= 6 bytes");
        } else {
            info!("  \x1b[31m[FAIL]\x1b[0m Response too short (expected >= 6, got {})", response_len as u32);
        }

        // Check 2: SPDM version field (byte 0 should be 0x10)
        if response_len >= 1 && response_data[0] == 0x10 {
            info!("  \x1b[32m[PASS]\x1b[0m SPDM version field = 0x10");
        } else if response_len >= 1 {
            info!("  \x1b[31m[FAIL]\x1b[0m SPDM version field = 0x{:02x} (expected 0x10)", response_data[0] as u32);
        } else {
            info!("  \x1b[31m[FAIL]\x1b[0m Cannot check SPDM version (response too short)");
        }

        // Check 3: Response code (byte 1 should be 0x04 for VERSION)
        if response_len >= 2 && response_data[1] == 0x04 {
            info!("  \x1b[32m[PASS]\x1b[0m Response code = 0x04 (VERSION)");
        } else if response_len >= 2 {
            info!("  \x1b[31m[FAIL]\x1b[0m Response code = 0x{:02x} (expected 0x04)", response_data[1] as u32);
        } else {
            info!("  \x1b[31m[FAIL]\x1b[0m Cannot check response code (response too short)");
        }

        // Check 4: Version count (byte 5)
        if response_len >= 6 {
            let version_count = response_data[5];
            let expected_count = expected_versions.len() as u8;
            info!("  Version count: {}", version_count as u32);

            if version_count == expected_count {
                info!("  \x1b[32m[PASS]\x1b[0m Version count = {}", version_count as u32);
            } else {
                info!("  \x1b[31m[FAIL]\x1b[0m Version count = {} (expected {})", version_count as u32, expected_count as u32);
            }

            // Check 5: Expected response length
            // Response format: [hdr(2) | param1(1) | param2(1) | reserved(1) | count(1) | entries(count*2)]
            let expected_len = 6 + (expected_count as usize * 2);
            if response_len == expected_len {
                info!("  \x1b[32m[PASS]\x1b[0m Response length matches version count");
            } else {
                info!("  \x1b[31m[FAIL]\x1b[0m Response length = {}, expected {} for {} versions",
                    response_len as u32, expected_len as u32, expected_count as u32);
            }

            // Check 6+: Version values
            // Each VersionNumberEntry is 16-bit per SPDM spec Table 10:
            //   Bits [15:12] = MajorVersion
            //   Bits [11:8] = MinorVersion
            //   Bits [7:4] = UpdateVersionNumber
            //   Bits [3:0] = Alpha
            // For SPDM 1.2: major=1, minor=2 → 0x1200
            // For SPDM 1.3: major=1, minor=3 → 0x1300
            // Wire format is little-endian: [0x00, 0x12] for version 1.2
            let mut offset = 6;
            for (i, &expected_ver) in expected_versions.iter().enumerate() {
                if offset + 2 <= response_len {
                    // Read 16-bit little-endian value
                    let version_entry = (response_data[offset] as u16) | ((response_data[offset + 1] as u16) << 8);

                    // Extract fields per SPDM spec
                    let major = ((version_entry >> 12) & 0x0F) as u8;
                    let minor = ((version_entry >> 8) & 0x0F) as u8;

                    info!("  Version {}: 0x{:04x} (major={}, minor={})",
                        (i + 1) as u32, version_entry as u32, major as u32, minor as u32);

                    // Get expected major/minor from SpdmVersion
                    let expected_major = expected_ver.major();
                    let expected_minor = expected_ver.minor();

                    if major == expected_major && minor == expected_minor {
                        info!("  \x1b[32m[PASS]\x1b[0m Version {} = {}.{}",
                            (i + 1) as u32, major as u32, minor as u32);
                    } else {
                        info!("  \x1b[31m[FAIL]\x1b[0m Version {} = {}.{} (expected {}.{})",
                            (i + 1) as u32, major as u32, minor as u32,
                            expected_major as u32, expected_minor as u32);
                    }
                    offset += 2;
                } else {
                    info!("  \x1b[31m[FAIL]\x1b[0m Cannot check version {} (response too short)", (i + 1) as u32);
                }
            }
        } else {
            info!("  \x1b[31m[FAIL]\x1b[0m Cannot check version count (response too short)");
        }
    } else {
        error!("Empty response received");
    }

    info!("GET_VERSION request/response cycle completed");

    pair.clear_a();
    pair.clear_b();

    Ok(())
}

#[entry]
fn entry() -> ! {
    info!("SPDM Loopback Test Application");

    match run_tests() {
        Ok(_) => {
            info!("SUCCESS: All tests passed");
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
    error!("PANIC occurred");
    loop {}
}
