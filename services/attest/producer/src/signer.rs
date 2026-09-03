// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Concrete [`AttestProducer`] implementations.
//!
//! `HwAttestProducer` — backed by a real `HwSigner` (mailbox driver).
//! `SoftwareAttestProducer` — software-only stub, available under `test-support`.

use heapless::Vec;

use openprot_attest_api::consts::{
    MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS, MAX_PROVIDERS, MAX_TOKEN_SIZE,
};
use openprot_attest_api::{
    AttestConfig, AttestError, AttestProducer, HwSigner, MeasurementProvider,
};

use crate::{builder, dice_identity, measurements};

// ── Hardware-backed producer ──────────────────────────────────────────────────

/// Attestation producer backed by a platform hardware signer.
pub struct HwAttestProducer<'a> {
    signer: &'a dyn HwSigner,
    config: AttestConfig,
    providers: Vec<&'a dyn MeasurementProvider, MAX_PROVIDERS>,
}

impl<'a> HwAttestProducer<'a> {
    pub fn new(signer: &'a dyn HwSigner, config: AttestConfig) -> Self {
        Self {
            signer,
            config,
            providers: Vec::new(),
        }
    }

    pub fn add_provider(
        &mut self,
        provider: &'a dyn MeasurementProvider,
    ) -> Result<(), AttestError> {
        self.providers
            .push(provider)
            .map_err(|_| AttestError::BufferFull)
    }
}

impl AttestProducer for HwAttestProducer<'_> {
    fn generate_token(
        &self,
        nonce: &[u8],
        evidence: &[u8],
        iat: u64,
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError> {
        let mut caliptra_meas: Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS> = Vec::new();
        self.signer.caliptra_measurements(&mut caliptra_meas)?;
        let mut meas: Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS> = Vec::new();
        measurements::collect(&caliptra_meas, &self.providers, &mut meas)?;
        builder::build(&self.config, self.signer, &meas, nonce, evidence, iat, out)
    }

    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        dice_identity::cert_chain(self.signer).map(|c| *buf = c.0)
    }
}

// ── Software stub (test-support) ─────────────────────────────────────────────

/// Software-backed attestation producer for use in tests.
///
/// Produces structurally valid COSE_Sign1 tokens without any hardware.
#[cfg(feature = "test-support")]
pub struct SoftwareAttestProducer {
    config: AttestConfig,
}

#[cfg(feature = "test-support")]
impl SoftwareAttestProducer {
    pub fn new(config: AttestConfig) -> Self {
        Self { config }
    }
}

#[cfg(feature = "test-support")]
impl AttestProducer for SoftwareAttestProducer {
    fn generate_token(
        &self,
        nonce: &[u8],
        evidence: &[u8],
        iat: u64,
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError> {
        let meas = measurements::test_caliptra_measurements();
        builder::build(&self.config, &StubSigner, &meas, nonce, evidence, iat, out)
    }

    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        let mut leaf: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        leaf.extend_from_slice(&STUB_CERT)
            .map_err(|_| AttestError::BufferFull)?;
        let mut ca: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        ca.extend_from_slice(&STUB_CERT)
            .map_err(|_| AttestError::BufferFull)?;
        buf.push(leaf).map_err(|_| AttestError::BufferFull)?;
        buf.push(ca).map_err(|_| AttestError::BufferFull)
    }
}

// ── Stub signer used internally by SoftwareAttestProducer ────────────────────

/// Placeholder DER: an empty SEQUENCE. Not a parseable certificate.
#[cfg(feature = "test-support")]
const STUB_CERT: [u8; 2] = [0x30, 0x00];

#[cfg(feature = "test-support")]
struct StubSigner;

#[cfg(feature = "test-support")]
impl HwSigner for StubSigner {
    fn sign(&self, _payload: &[u8]) -> Result<[u8; 96], AttestError> {
        Ok([0u8; 96])
    }
    fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError> {
        buf.extend_from_slice(&STUB_CERT)
            .map_err(|_| AttestError::BufferFull)
    }
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        let mut leaf: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        leaf.extend_from_slice(&STUB_CERT).unwrap();
        let mut ca: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        ca.extend_from_slice(&STUB_CERT).unwrap();
        buf.push(leaf).map_err(|_| AttestError::BufferFull)?;
        buf.push(ca).map_err(|_| AttestError::BufferFull)
    }
    fn caliptra_measurements(
        &self,
        out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError> {
        let stub = measurements::test_caliptra_measurements();
        for m in stub {
            out.push(m).map_err(|_| AttestError::BufferFull)?;
        }
        Ok(())
    }
}
