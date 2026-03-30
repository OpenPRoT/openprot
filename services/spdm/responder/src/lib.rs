// Licensed under the Apache-2.0 license

//! SPDM Responder Service
//!
//! This service implements the SPDM responder (server) role, which responds
//! to attestation and measurement requests from SPDM requesters.
//!
//! ## Overview
//!
//! The SPDM responder receives requests and provides:
//! - Version and capability information
//! - Certificate chains for authentication
//! - Device measurements
//! - Challenge-response attestation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────┐
//! │  MCTP Transport         │
//! │  (incoming messages)    │
//! └───────────┬─────────────┘
//!             │ SPDM requests
//!             ▼
//! ┌─────────────────────────┐
//! │  SPDM Responder         │◄── This crate
//! │  (SpdmContext wrapper)  │
//! └───────────┬─────────────┘
//!             │
//!             ▼
//! ┌─────────────────────────────────────┐
//! │  Platform Implementations           │
//! │  - CertStore (certificates)         │
//! │  - Hash (SHA-384)                   │
//! │  - RNG (random numbers)             │
//! │  - Evidence (measurements)          │
//! │  - Transport (MCTP)                 │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use spdm_responder::SpdmResponder;
//!
//! // Create platform implementations
//! let cert_store = Ast1060CertStore::new(crypto_handle);
//! let hash = SpdmCryptoHash::new(crypto_handle);
//! let rng = SpdmCryptoRng::new(crypto_handle);
//! let evidence = Ast1060Evidence::new();
//! let transport = MctpSpdmTransport::new(mctp_client);
//!
//! // Create responder
//! let mut responder = SpdmResponder::new(
//!     transport,
//!     cert_store,
//!     hash,
//!     rng,
//!     evidence,
//! )?;
//!
//! // Process messages in loop
//! loop {
//!     responder.process_message()?;
//! }
//! ```

#![no_std]

use spdm_lib::cert_store::SpdmCertStore;
use spdm_lib::codec::MessageBuf;
use spdm_lib::context::SpdmContext;
use spdm_lib::error::SpdmError;
use spdm_lib::platform::evidence::SpdmEvidence;
use spdm_lib::platform::hash::SpdmHash;
use spdm_lib::platform::rng::SpdmRng;
use spdm_lib::platform::transport::SpdmTransport;
use spdm_lib::protocol::algorithms::{
    AeadCipherSuite, AlgorithmPriorityTable, BaseAsymAlgo, BaseHashAlgo, DeviceAlgorithms,
    DheNamedGroup, KeySchedule, LocalDeviceAlgorithms, MeasurementHashAlgo,
    MeasurementSpecification, MelSpecification, OtherParamSupport, ReqBaseAsymAlg,
};
use spdm_lib::protocol::version::SpdmVersion;

/// Supported SPDM versions (static to avoid lifetime issues)
static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V11];
use spdm_lib::protocol::{CapabilityFlags, DeviceCapabilities};

/// Maximum SPDM message size
const MAX_SPDM_MSG_SIZE: usize = 4096;

/// SPDM responder result type
pub type ResponderResult<T> = Result<T, ResponderError>;

/// SPDM responder errors
#[derive(Debug)]
pub enum ResponderError {
    /// SPDM protocol error
    SpdmError(SpdmError),
    /// Message buffer error
    BufferError,
}

impl From<SpdmError> for ResponderError {
    fn from(e: SpdmError) -> Self {
        ResponderError::SpdmError(e)
    }
}

/// SPDM responder configuration
#[derive(Debug, Clone, Copy)]
pub struct ResponderConfig {
    /// CT exponent for timing
    pub ct_exponent: u8,
    /// Data transfer size
    pub data_transfer_size: u32,
    /// Maximum SPDM message size
    pub max_spdm_msg_size: u32,
}

impl Default for ResponderConfig {
    fn default() -> Self {
        Self {
            ct_exponent: 0,
            data_transfer_size: 1024,
            max_spdm_msg_size: MAX_SPDM_MSG_SIZE as u32,
        }
    }
}

/// SPDM responder state and configuration.
///
/// This wraps the spdm-lib `SpdmContext` and provides a simplified interface
/// for processing SPDM messages.
pub struct SpdmResponder<'a> {
    context: SpdmContext<'a>,
}

impl<'a> SpdmResponder<'a> {
    /// Create a new SPDM responder with platform implementations.
    ///
    /// # Arguments
    ///
    /// * `transport` - Transport layer implementation (e.g., MCTP)
    /// * `cert_store` - Certificate store with device certificates
    /// * `hash` - Hash implementation for protocol operations
    /// * `m1_hash` - Hash implementation for M1 transcript
    /// * `l1_hash` - Hash implementation for L1 transcript
    /// * `rng` - Random number generator
    /// * `evidence` - Evidence provider for measurements
    /// * `config` - Optional configuration (uses defaults if None)
    ///
    /// # Returns
    ///
    /// A new `SpdmResponder` instance ready to process messages.
    pub fn new(
        transport: &'a mut dyn SpdmTransport,
        cert_store: &'a mut dyn SpdmCertStore,
        hash: &'a mut dyn SpdmHash,
        m1_hash: &'a mut dyn SpdmHash,
        l1_hash: &'a mut dyn SpdmHash,
        rng: &'a mut dyn SpdmRng,
        evidence: &'a dyn SpdmEvidence,
        config: Option<ResponderConfig>,
    ) -> ResponderResult<Self> {
        let config = config.unwrap_or_default();

        // Create device capabilities
        let capabilities = create_device_capabilities(config);

        // Create local algorithms
        let algorithms = create_local_algorithms();

        // Create SPDM context
        let context = SpdmContext::new(
            &SUPPORTED_VERSIONS,
            transport,
            capabilities,
            algorithms,
            cert_store,
            None, // No peer cert store needed for responder
            hash,
            m1_hash,
            l1_hash,
            rng,
            evidence,
        )?;

        Ok(Self { context })
    }

    /// Process a single SPDM message.
    ///
    /// This method:
    /// 1. Receives a request via the transport layer
    /// 2. Processes it through the SPDM context
    /// 3. Sends the response back via the transport layer
    ///
    /// # Arguments
    ///
    /// * `buffer` - Message buffer (must be at least MAX_SPDM_MSG_SIZE bytes)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if message processed successfully
    /// - `Err(ResponderError)` on error
    ///
    /// # Note
    ///
    /// This should be called in a loop to continuously process messages.
    /// Transport errors indicate connection closed.
    pub fn process_message(&mut self, buffer: &'a mut [u8]) -> ResponderResult<()> {
        let mut message_buf = MessageBuf::new(buffer);
        self.context.responder_process_message(&mut message_buf)?;
        Ok(())
    }

    /// Get reference to the underlying SPDM context.
    ///
    /// This allows direct access to context state if needed.
    pub fn context(&self) -> &SpdmContext<'a> {
        &self.context
    }

    /// Get mutable reference to the underlying SPDM context.
    ///
    /// This allows direct manipulation of context state if needed.
    pub fn context_mut(&mut self) -> &mut SpdmContext<'a> {
        &mut self.context
    }
}

/// Create SPDM device capabilities based on configuration.
fn create_device_capabilities(config: ResponderConfig) -> DeviceCapabilities {
    let mut flags_value = 0u32;

    // Certificate capability
    flags_value |= 1 << 1; // CERT_CAP

    // Challenge capability
    flags_value |= 1 << 2; // CHAL_CAP

    // Measurements capability (with signature)
    flags_value |= 2 << 3; // MEAS_CAP (0b10 = measurements with signature)

    // Measurements freshness capability
    flags_value |= 1 << 5; // MEAS_FRESH_CAP

    // Chunk capability
    flags_value |= 1 << 17; // CHUNK_CAP

    let flags = CapabilityFlags::new(flags_value);

    DeviceCapabilities {
        ct_exponent: config.ct_exponent,
        flags,
        data_transfer_size: config.data_transfer_size,
        max_spdm_msg_size: config.max_spdm_msg_size,
        include_supported_algorithms: true,
    }
}

/// Create local device algorithms configuration.
///
/// Configures supported cryptographic algorithms:
/// - Measurement: DMTF specification with SHA-384
/// - Asymmetric: ECDSA with NIST P-384
/// - Hash: SHA-384
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
