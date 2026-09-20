// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;

use crate::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS};
use crate::error::AttestError;
use crate::types::{Measurement, SwSignerConfig};

/// Hardware-backed signing operations implemented by a Caliptra mailbox driver.
///
/// The private Alias Key never leaves the Caliptra hardware boundary.
/// Testing: implement with a software key (`SoftwareAttestProducer` in the
/// producer crate behind `test-support`).
pub trait HwSigner {
    /// Sign `payload` with the platform alias key. Returns raw (r‖s) bytes.
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    /// Return the full DER-encoded certificate chain, leaf → root.
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    /// Return Caliptra-internal firmware measurements (ROM, FMC, runtime, etc.).
    fn measurements(&self, out: &mut Vec<Measurement, MAX_MEASUREMENTS>)
        -> Result<(), AttestError>;
}

// P-384 group order n (FIPS 186-4), big-endian:
// FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEC7634D81F4372DDF581A0DB248B0A77AECEC196ACCC52973
const P384_ORDER: [u8; 48] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xC7, 0x63, 0x4D, 0x81, 0xF4, 0x37, 0x2D, 0xDF,
    0x58, 0x1A, 0x0D, 0xB2, 0x48, 0xB0, 0xA7, 0x7A, 0xEC, 0xEC, 0x19, 0x6A, 0xCC, 0xC5, 0x29, 0x73,
];

/// Validates and holds a caller-supplied P-384 private scalar and cert chain.
///
/// Constructed via [`SwSigner::new`], which checks that:
/// - The scalar is not all zeros (`d ≥ 1`).
/// - The scalar is less than the P-384 group order (`d < n`).
/// - The cert chain contains at least one certificate.
/// - Every certificate begins with `0x30` (DER SEQUENCE tag).
pub struct SwSigner {
    private_key_scalar: [u8; 48],
    chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
}

impl SwSigner {
    /// Construct a `SwSigner` from caller-supplied key material.
    ///
    /// Returns `Err(AttestError::InvalidKey)` if the scalar or cert chain
    /// fails validation.
    pub fn new(config: SwSignerConfig) -> Result<Self, AttestError> {
        let d = &config.private_key_scalar;

        // d must not be zero.
        if d.iter().all(|&b| b == 0) {
            return Err(AttestError::InvalidKey("private key scalar is zero"));
        }

        // d must be less than the P-384 group order (big-endian comparison).
        if d >= &P384_ORDER {
            return Err(AttestError::InvalidKey(
                "private key scalar >= P-384 group order",
            ));
        }

        // Cert chain must contain at least one certificate.
        if config.cert_chain.is_empty() {
            return Err(AttestError::InvalidKey("cert chain must not be empty"));
        }

        // Every cert must start with the DER SEQUENCE tag.
        for cert in &config.cert_chain {
            if cert.first() != Some(&0x30) {
                return Err(AttestError::InvalidKey("cert is not a DER SEQUENCE"));
            }
        }

        Ok(Self {
            private_key_scalar: config.private_key_scalar,
            chain: config.cert_chain,
        })
    }
}

impl HwSigner for SwSigner {
    /// Returns a zeroed 96-byte signature (r‖s placeholder).
    ///
    /// Production use must replace this with a real ECDSA P-384 implementation
    /// using the stored private scalar.
    fn sign(&self, _payload: &[u8]) -> Result<[u8; 96], AttestError> {
        let _ = &self.private_key_scalar;
        Ok([0u8; 96])
    }

    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        for cert in &self.chain {
            buf.push(cert.clone())
                .map_err(|_| AttestError::BufferFull)?;
        }
        Ok(())
    }

    fn measurements(
        &self,
        _out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError> {
        Ok(())
    }
}
