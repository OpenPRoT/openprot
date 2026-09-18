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

use crate::{builder, cert_ueid, dice_identity, measurements};

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
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError> {
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        self.signer.cert_chain_der(&mut chain)?;
        dice_identity::validate_chain(&chain)?;
        let ueid = cert_ueid::extract_and_verify(&chain)?;

        let mut meas: Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS> = Vec::new();
        self.signer.measurements(&mut meas)?;
        measurements::collect(&self.providers, &mut meas)?;
        builder::build(&self.config, self.signer, &ueid, &meas, nonce, out)
    }

    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        dice_identity::cert_chain(self.signer).map(|c| *buf = c)
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
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError> {
        // Stub UEID: type EAT_RAND (0x01) followed by 16 deterministic bytes.
        let stub_ueid = [
            0x01, 0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x11, 0x22, 0x33, 0x44,
            0x55, 0x66, 0x77,
        ];
        let meas = measurements::test_measurements();
        builder::build(&self.config, &StubSigner, &stub_ueid, &meas, nonce, out)
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
#[cfg(any(test, feature = "test-support"))]
pub(crate) const STUB_CERT: [u8; 2] = [0x30, 0x00];

#[cfg(feature = "test-support")]
struct StubSigner;

#[cfg(feature = "test-support")]
impl HwSigner for StubSigner {
    fn sign(&self, _payload: &[u8]) -> Result<[u8; 96], AttestError> {
        Ok([0u8; 96])
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
    fn measurements(
        &self,
        out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError> {
        let stub = measurements::test_measurements();
        for m in stub {
            out.push(m).map_err(|_| AttestError::BufferFull)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;
    use heapless::{String, Vec};
    use openprot_attest_api::consts::{
        MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS, MAX_TOKEN_SIZE,
    };
    use openprot_attest_api::{AttestConfig, AttestError, OemId};

    /// A signer that returns a two-cert chain of stub DER (below the 3-cert minimum).
    struct ShortChainSigner;

    impl HwSigner for ShortChainSigner {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            for _ in 0..2 {
                let mut c: Vec<u8, MAX_CERT_SIZE> = Vec::new();
                c.extend_from_slice(&STUB_CERT).unwrap();
                buf.push(c).map_err(|_| AttestError::BufferFull)?;
            }
            Ok(())
        }
        fn measurements(
            &self,
            _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            Ok(())
        }
    }

    fn config() -> AttestConfig {
        let mut hw_model: String<64> = String::new();
        hw_model.push_str("TestModel").unwrap();
        let mut oemid_bytes: Vec<u8, 16> = Vec::new();
        oemid_bytes
            .extend_from_slice(&[0x00, 0x01, 0x47, 0xae])
            .unwrap();
        AttestConfig {
            oemid: OemId(oemid_bytes),
            hw_model,
            cert_cache_ttl: Duration::from_secs(3600),
        }
    }

    #[test]
    fn generate_token_rejects_chain_with_fewer_than_three_certs() {
        let producer = HwAttestProducer::new(&ShortChainSigner, config());
        let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
        let err = producer.generate_token(b"testnonce", &mut out).unwrap_err();
        assert!(matches!(err, AttestError::Mailbox(_)));
    }
}
