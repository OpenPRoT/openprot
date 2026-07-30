// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::error::AttestError;

/// OEM identifier (IANA Private Enterprise Number or UUID form).
#[derive(Clone, Debug)]
pub struct OemId(pub Vec<u8>);

#[derive(Clone, Copy, Debug)]
pub enum DigestAlgorithm {
    Sha384,
    Sha512,
}

#[derive(Clone, Copy, Debug)]
pub enum MeasurementAuthority {
    Caliptra,
    Platform,
}

/// A single firmware measurement record to include in the EAT token.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub component:  String,
    pub version:    String,
    pub digest_alg: DigestAlgorithm,
    pub digest:     Vec<u8>,
    pub authority:  MeasurementAuthority,
}

/// DER-encoded certificate chain ordered leaf → root.
pub struct CertChain(pub Vec<Vec<u8>>);

/// Producer configuration, set once at platform initialisation.
pub struct AttestConfig {
    pub oemid:          OemId,
    pub hw_model:       String,
    pub hw_version:     String,
    pub cert_cache_ttl: Duration,
}

/// Hardware-backed signing operations.
///
/// Production: implemented by a Caliptra mailbox driver.
/// Testing: implement with a software key (`SoftwareAttestProducer` in the
/// producer crate behind `test-support`).
pub trait CaliptraSigner: Send + Sync {
    /// Sign `payload` with the Alias Key (ES384). Returns raw (r‖s) bytes.
    fn sign_es384(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    /// Return the DER-encoded Alias (leaf) certificate.
    fn alias_cert_der(&self) -> Result<Vec<u8>, AttestError>;
    /// Return the full DER-encoded certificate chain, leaf → root.
    fn cert_chain_der(&self) -> Result<Vec<Vec<u8>>, AttestError>;
}

/// Platform-specific measurement source.
///
/// Implement for each firmware component (UEFI, BMC, etc.) the platform
/// wants to measure beyond Caliptra-internal measurements.
pub trait MeasurementProvider: Send + Sync {
    fn component_name(&self) -> &str;
    fn measurements(&self) -> Result<Vec<Measurement>, AttestError>;
}
