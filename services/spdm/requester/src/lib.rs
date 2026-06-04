// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM Requester Service
//!
//! This service implements the SPDM requester (client) role, which initiates
//! attestation and measurement operations with SPDM responders.
//!
//! ## Overview
//!
//! The SPDM requester sends requests to responders to:
//! - Get version information
//! - Negotiate capabilities and algorithms
//! - Challenge the responder for attestation
//! - Retrieve measurements and certificates
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────┐
//! │  Application            │
//! │  (attestation manager)  │
//! └───────────┬─────────────┘
//!             │
//!             ▼
//! ┌─────────────────────────┐
//! │  SPDM Requester         │◄── This crate
//! │  (SpdmContext wrapper)  │
//! └───────────┬─────────────┘
//!             │ SPDM messages
//!             ▼
//! ┌─────────────────────────┐
//! │  MCTP Transport         │
//! └─────────────────────────┘
//! ```

#![no_std]
#![warn(missing_docs)]

pub use openprot_spdm_common::{DEFAULT_DTS, DEFAULT_SMS};

use spdm_lib::cert_store::{PeerCertStore, SpdmCertStore};
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
use spdm_lib::protocol::{CapabilityFlags, DeviceCapabilities};

/// Supported SPDM versions (static to avoid lifetime issues)
static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

/// SPDM requester result type
pub type RequesterResult<T> = Result<T, RequesterError>;

/// SPDM requester errors
#[derive(Debug)]
pub enum RequesterError {
    /// SPDM protocol error
    SpdmError(SpdmError),
}

impl From<SpdmError> for RequesterError {
    fn from(e: SpdmError) -> Self {
        RequesterError::SpdmError(e)
    }
}

/// SPDM requester configuration.
///
/// Use [`RequesterConfig::default()`] to get defaults, modify as needed,
/// then pass to [`SpdmRequester::new()`].
///
/// # Example
///
/// ```rust,no_run
/// use openprot_spdm_requester::RequesterConfig;
///
/// // Get defaults and customize
/// let mut config = RequesterConfig::default();
/// let mut caps = RequesterConfig::default_capabilities();
/// caps.data_transfer_size = 2048;
/// config.capabilities = Some(caps);
///
/// let mut algos = RequesterConfig::default_algorithms();
/// algos.device_algorithms.base_hash_algo.set_tpm_alg_sha_512(1);
/// config.algorithms = Some(algos);
/// ```
#[derive(Default)]
pub struct RequesterConfig<'a> {
    /// Device capabilities. If `None`, default capabilities are used.
    pub capabilities: Option<DeviceCapabilities>,
    /// Device algorithms. If `None`, default algorithms are used.
    pub algorithms: Option<LocalDeviceAlgorithms<'a>>,
}

impl RequesterConfig<'_> {
    /// Get the default device capabilities for a requester.
    ///
    /// Returns capabilities that can be modified and passed back via
    /// [`RequesterConfig::capabilities`].
    pub fn default_capabilities() -> DeviceCapabilities {
        let mut flags = CapabilityFlags::default();
        flags.set_cert_cap(1);
        flags.set_chal_cap(1);
        flags.set_meas_cap(0); // Requester doesn't provide measurements
        flags.set_chunk_cap(1);

        DeviceCapabilities {
            ct_exponent: 0,
            flags,
            data_transfer_size: DEFAULT_DTS,
            max_spdm_msg_size: DEFAULT_SMS,
            include_supported_algorithms: false,
        }
    }

    /// Get the default device algorithms for a requester.
    ///
    /// Returns algorithms that can be modified and passed back via
    /// [`RequesterConfig::algorithms`].
    ///
    /// Default configuration:
    /// - Measurement: DMTF specification with SHA-384
    /// - Asymmetric: ECDSA with NIST P-384
    /// - Hash: SHA-384
    pub fn default_algorithms<'a>() -> LocalDeviceAlgorithms<'a> {
        let mut measurement_spec = MeasurementSpecification::default();
        measurement_spec.set_dmtf_measurement_spec(1);

        let mut measurement_hash_algo = MeasurementHashAlgo::default();
        measurement_hash_algo.set_tpm_alg_sha_384(1);

        let mut base_asym_algo = BaseAsymAlgo::default();
        base_asym_algo.set_tpm_alg_ecdsa_ecc_nist_p384(1);

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
}

/// SPDM requester state and configuration.
///
/// This wraps the spdm-lib `SpdmContext` and provides a simplified interface
/// for initiating SPDM protocol exchanges.
pub struct SpdmRequester<'a> {
    context: SpdmContext<'a>,
}

impl<'a> SpdmRequester<'a> {
    /// Create a new SPDM requester with platform implementations.
    ///
    /// # Arguments
    ///
    /// * `transport` - Transport layer implementation (e.g., MCTP)
    /// * `cert_store` - Certificate store with local certificates
    /// * `peer_cert_store` - Certificate store for peer (responder) certificates
    /// * `hash` - Hash implementation for protocol operations
    /// * `m1_hash` - Hash implementation for M1 transcript
    /// * `l1_hash` - Hash implementation for L1 transcript
    /// * `rng` - Random number generator
    /// * `evidence` - Evidence provider for measurements
    /// * `config` - Optional configuration (uses defaults if None)
    ///
    /// # Returns
    ///
    /// A new `SpdmRequester` instance ready to initiate protocol exchanges.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transport: &'a mut dyn SpdmTransport,
        cert_store: &'a mut dyn SpdmCertStore,
        peer_cert_store: &'a mut dyn PeerCertStore,
        hash: &'a mut dyn SpdmHash,
        m1_hash: &'a mut dyn SpdmHash,
        l1_hash: &'a mut dyn SpdmHash,
        rng: &'a mut dyn SpdmRng,
        evidence: &'a dyn SpdmEvidence,
        config: Option<RequesterConfig<'a>>,
    ) -> RequesterResult<Self> {
        let config = config.unwrap_or_default();

        // Use provided capabilities or create defaults
        let capabilities = match config.capabilities {
            Some(caps) => {
                #[cfg(debug_assertions)]
                validate_device_capabilities(&caps);
                caps
            }
            None => RequesterConfig::default_capabilities(),
        };

        // Use provided algorithms or create defaults
        let algorithms = match config.algorithms {
            Some(algos) => {
                #[cfg(debug_assertions)]
                validate_device_algorithms(&algos);
                algos
            }
            None => RequesterConfig::default_algorithms(),
        };

        // Create SPDM context
        let context = SpdmContext::new(
            &SUPPORTED_VERSIONS,
            transport,
            capabilities,
            algorithms,
            cert_store,
            Some(peer_cert_store),
            hash,
            m1_hash,
            l1_hash,
            rng,
            evidence,
        )?;

        Ok(Self { context })
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

/// Validate device capabilities in debug builds.
#[cfg(debug_assertions)]
fn validate_device_capabilities(caps: &DeviceCapabilities) {
    use spdm_lib::protocol::capabilities::MIN_DATA_TRANSFER_SIZE_V12;

    debug_assert!(
        caps.data_transfer_size >= MIN_DATA_TRANSFER_SIZE_V12,
        "data_transfer_size ({}) must be >= MIN_DATA_TRANSFER_SIZE_V12 ({})",
        caps.data_transfer_size,
        MIN_DATA_TRANSFER_SIZE_V12
    );

    debug_assert!(
        caps.max_spdm_msg_size >= caps.data_transfer_size,
        "max_spdm_msg_size ({}) must be >= data_transfer_size ({})",
        caps.max_spdm_msg_size,
        caps.data_transfer_size
    );
}

/// Validate device algorithms in debug builds.
#[cfg(debug_assertions)]
fn validate_device_algorithms(algos: &LocalDeviceAlgorithms) {
    let measurement_hash_algo = algos.device_algorithms.measurement_hash_algo;
    debug_assert!(
        measurement_hash_algo.0.count_ones() <= 1,
        "measurement_hash_algo must have at most one bit set"
    );

    let base_hash_algo = algos.device_algorithms.base_hash_algo;
    debug_assert!(
        base_hash_algo.0 != 0,
        "base_hash_algo must have at least one algorithm enabled"
    );

    let base_asym_algo = algos.device_algorithms.base_asym_algo;
    debug_assert!(
        base_asym_algo.0 != 0,
        "base_asym_algo must have at least one algorithm enabled"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RequesterConfig::default();
        assert!(config.capabilities.is_none());
        assert!(config.algorithms.is_none());
    }

    #[test]
    fn test_default_capabilities() {
        let caps = RequesterConfig::default_capabilities();
        assert_eq!(caps.flags.cert_cap(), 1);
        assert_eq!(caps.flags.chal_cap(), 1);
        assert_eq!(caps.flags.meas_cap(), 0);
        assert_eq!(caps.flags.chunk_cap(), 1);
    }

    #[test]
    fn test_default_algorithms() {
        let algos = RequesterConfig::default_algorithms();
        assert_eq!(
            algos
                .device_algorithms
                .measurement_spec
                .dmtf_measurement_spec(),
            1
        );
        assert_eq!(
            algos
                .device_algorithms
                .measurement_hash_algo
                .tpm_alg_sha_384(),
            1
        );
        assert_eq!(
            algos
                .device_algorithms
                .base_asym_algo
                .tpm_alg_ecdsa_ecc_nist_p384(),
            1
        );
        assert_eq!(algos.device_algorithms.base_hash_algo.tpm_alg_sha_384(), 1);
    }
}
