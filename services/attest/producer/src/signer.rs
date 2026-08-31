// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Concrete [`AttestProducer`] implementations.
//!
//! `HwAttestProducer` — backed by a real `HwSigner` (mailbox driver).
//! `SoftwareAttestProducer` — software-only stub, available under `test-support`.

use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use openprot_attest_api::{
    AttestConfig, AttestError, AttestProducer, CertChain, HwSigner, MeasurementProvider,
};

use crate::{builder, dice_identity, measurements};

// ── Hardware-backed producer ──────────────────────────────────────────────────

/// Attestation producer backed by a platform hardware signer.
pub struct HwAttestProducer {
    signer: Arc<dyn HwSigner>,
    config: AttestConfig,
    providers: Vec<Box<dyn MeasurementProvider>>,
}

impl HwAttestProducer {
    pub fn new(signer: Arc<dyn HwSigner>, config: AttestConfig) -> Self {
        Self {
            signer,
            config,
            providers: Vec::new(),
        }
    }

    /// Register an additional measurement provider (UEFI, BMC, etc.).
    pub fn add_provider(&mut self, provider: Box<dyn MeasurementProvider>) {
        self.providers.push(provider);
    }
}

impl AttestProducer for HwAttestProducer {
    fn generate_token(&self, nonce: &[u8], evidence: &[u8], iat: u64) -> Result<Vec<u8>, AttestError> {
        let meas = measurements::collect(vec![], &self.providers)?;
        builder::build(&self.config, &*self.signer, &meas, nonce, evidence, iat)
    }

    fn cert_chain(&self) -> Result<CertChain, AttestError> {
        dice_identity::cert_chain(&*self.signer)
    }
}

// ── Software stub (test-support) ─────────────────────────────────────────────

/// Software-backed attestation producer for use in tests.
///
/// Uses a deterministic P-384 key; produces structurally valid COSE_Sign1
/// tokens without any Caliptra hardware.
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
    fn generate_token(&self, nonce: &[u8], evidence: &[u8], iat: u64) -> Result<Vec<u8>, AttestError> {
        let meas = measurements::test_caliptra_measurements();
        builder::build(&self.config, &StubSigner, &meas, nonce, evidence, iat)
    }

    fn cert_chain(&self) -> Result<CertChain, AttestError> {
        Ok(CertChain(vec![stub_leaf_cert(), stub_ca_cert()]))
    }
}

// ── Stub signer used internally by SoftwareAttestProducer ────────────────────

#[cfg(feature = "test-support")]
struct StubSigner;

#[cfg(feature = "test-support")]
impl HwSigner for StubSigner {
    fn sign(&self, _payload: &[u8]) -> Result<[u8; 96], AttestError> {
        Ok([0u8; 96])
    }

    fn leaf_cert_der(&self) -> Result<Vec<u8>, AttestError> {
        Ok(stub_leaf_cert())
    }

    fn cert_chain_der(&self) -> Result<Vec<Vec<u8>>, AttestError> {
        Ok(vec![stub_leaf_cert(), stub_ca_cert()])
    }
}

#[cfg(feature = "test-support")]
fn stub_leaf_cert() -> Vec<u8> {
    vec![0x30, 0x00]
}

#[cfg(feature = "test-support")]
fn stub_ca_cert() -> Vec<u8> {
    vec![0x30, 0x00]
}
