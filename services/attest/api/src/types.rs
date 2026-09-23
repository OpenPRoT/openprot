// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::{String, Vec};
use zeroize::Zeroize;

use crate::consts::{
    MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_HW_MODEL_LEN,
    MAX_MEASUREMENTS, MAX_OEMID_LEN, MAX_VERSION_LEN,
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

/// Caller-supplied material for the software signer path.
///
/// `private_key_scalar` is the 48-byte big-endian P-384 private scalar `d`.
/// It must satisfy `1 ≤ d < n` (P-384 group order) — validated at construction
/// time by [`crate::SwSigner::new`].
///
/// `cert_chain` holds the DER-encoded certificate chain, leaf → root.  Each
/// certificate must start with `0x30` (DER SEQUENCE).  At least one certificate
/// is required.
pub struct SwSignerConfig {
    pub private_key_scalar: [u8; 48],
    pub cert_chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
}

impl Drop for SwSignerConfig {
    fn drop(&mut self) {
        self.private_key_scalar.zeroize();
    }
}

/// Producer configuration, set once at platform initialisation.
pub struct AttestConfig {
    pub oemid: OemId,
    pub hw_model: String<MAX_HW_MODEL_LEN>,
}

/// Platform-specific measurement source.
///
/// Implement for each firmware component (UEFI, BMC, etc.) the platform
/// wants to measure beyond Caliptra-internal measurements.
pub trait MeasurementProvider {
    fn measurements(&self, out: &mut Vec<Measurement, MAX_MEASUREMENTS>)
        -> Result<(), AttestError>;
}
