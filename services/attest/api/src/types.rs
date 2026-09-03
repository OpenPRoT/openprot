// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::time::Duration;

use heapless::{String, Vec};

use crate::consts::{
    MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_HW_MODEL_LEN,
    MAX_HW_VERSION_LEN, MAX_MEASUREMENTS, MAX_OEMID_LEN, MAX_VERSION_LEN,
};
use crate::error::AttestError;

/// OEM identifier (IANA Private Enterprise Number or UUID form).
#[derive(Clone, Debug)]
pub struct OemId(pub Vec<u8, MAX_OEMID_LEN>);

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
    pub component: String<MAX_COMPONENT_LEN>,
    pub version: String<MAX_VERSION_LEN>,
    pub digest_alg: DigestAlgorithm,
    pub digest: Vec<u8, MAX_DIGEST_LEN>,
    pub authority: MeasurementAuthority,
}

/// Producer configuration, set once at platform initialisation.
pub struct AttestConfig {
    pub oemid: OemId,
    pub hw_model: String<MAX_HW_MODEL_LEN>,
    pub hw_version: String<MAX_HW_VERSION_LEN>,
    pub cert_cache_ttl: Duration,
}

/// Hardware-backed signing operations.
///
/// Production: implemented by a platform mailbox driver.
/// Testing: implement with a software key (`SoftwareAttestProducer` in the
/// producer crate behind `test-support`).
pub trait HwSigner: Send + Sync {
    /// Sign `payload` with the platform alias key. Returns raw (r‖s) bytes.
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    /// Return the DER-encoded leaf certificate.
    fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError>;
    /// Return the full DER-encoded certificate chain, leaf → root.
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    /// Return Caliptra-internal firmware measurements (ROM, FMC, runtime, etc.).
    fn caliptra_measurements(
        &self,
        out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError>;
}

/// Platform-specific measurement source.
///
/// Implement for each firmware component (UEFI, BMC, etc.) the platform
/// wants to measure beyond Caliptra-internal measurements.
pub trait MeasurementProvider: Send + Sync {
    fn component_name(&self) -> &str;
    fn measurements(&self, out: &mut Vec<Measurement, MAX_MEASUREMENTS>)
        -> Result<(), AttestError>;
}
