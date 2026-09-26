// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;
use p384::ecdsa::{signature::DigestSigner, Signature, SigningKey};
use sha2::{Digest, Sha384};

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

/// Validates and holds a caller-supplied P-384 private key and cert chain.
///
/// Constructed via [`SwSigner::new`], which checks that:
/// - The scalar is a valid P-384 private key (`1 ≤ d < n`, via `SigningKey::from_bytes`).
/// - The cert chain contains at least one certificate.
/// - Every certificate begins with `0x30` (DER SEQUENCE tag).
///
/// The private key is held as a [`SigningKey`], which zeroizes on drop.
pub struct SwSigner {
    signing_key: SigningKey,
    chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
}

impl SwSigner {
    /// Construct a `SwSigner` from caller-supplied key material.
    ///
    /// Returns `Err(AttestError::InvalidKey)` if the scalar or cert chain
    /// fails validation.
    pub fn new(config: SwSignerConfig) -> Result<Self, AttestError> {
        let signing_key = SigningKey::from_bytes(config.private_key_scalar.as_ref().into())
            .map_err(|_| AttestError::InvalidKey("invalid P-384 scalar"))?;

        if config.cert_chain.is_empty() {
            return Err(AttestError::InvalidKey("cert chain must not be empty"));
        }
        for cert in &config.cert_chain {
            if cert.first() != Some(&0x30) {
                return Err(AttestError::InvalidKey("cert is not a DER SEQUENCE"));
            }
        }

        // Clone cert_chain out before config drops (and zeroizes the scalar).
        let chain = config.cert_chain.clone();
        Ok(Self { signing_key, chain })
    }
}

impl HwSigner for SwSigner {
    /// Signs `payload` with ECDSA P-384 (SHA-384 prehash). Returns raw r‖s (96 bytes).
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError> {
        let digest = Sha384::new_with_prefix(payload);
        let sig: Signature = self.signing_key.sign_digest(digest);
        let bytes = sig.to_bytes();
        let mut out = [0u8; 96];
        out.copy_from_slice(&bytes);
        Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SwSignerConfig;

    fn valid_scalar() -> [u8; 48] {
        // A fixed non-zero scalar well below the P-384 group order.
        let mut s = [0u8; 48];
        s[47] = 1;
        s
    }

    fn valid_chain() -> Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> {
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        let mut cert: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        cert.push(0x30).unwrap();
        chain.push(cert).unwrap();
        chain
    }

    #[test]
    fn rejects_zero_scalar() {
        let config = SwSignerConfig {
            private_key_scalar: [0u8; 48],
            cert_chain: valid_chain(),
        };
        assert!(matches!(
            SwSigner::new(config),
            Err(AttestError::InvalidKey(_))
        ));
    }

    #[test]
    fn rejects_scalar_equal_to_group_order() {
        // P-384 group order n — must be rejected (d must be < n).
        let order: [u8; 48] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xC7, 0x63, 0x4D, 0x81,
            0xF4, 0x37, 0x2D, 0xDF, 0x58, 0x1A, 0x0D, 0xB2, 0x48, 0xB0, 0xA7, 0x7A, 0xEC, 0xEC,
            0x19, 0x6A, 0xCC, 0xC5, 0x29, 0x73,
        ];
        let config = SwSignerConfig {
            private_key_scalar: order,
            cert_chain: valid_chain(),
        };
        assert!(matches!(
            SwSigner::new(config),
            Err(AttestError::InvalidKey(_))
        ));
    }

    #[test]
    fn rejects_empty_cert_chain() {
        let config = SwSignerConfig {
            private_key_scalar: valid_scalar(),
            cert_chain: Vec::new(),
        };
        assert!(matches!(
            SwSigner::new(config),
            Err(AttestError::InvalidKey(_))
        ));
    }

    #[test]
    fn rejects_cert_without_der_sequence_tag() {
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        let mut bad_cert: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        bad_cert.push(0x04).unwrap(); // OCTET STRING tag, not SEQUENCE
        chain.push(bad_cert).unwrap();
        let config = SwSignerConfig {
            private_key_scalar: valid_scalar(),
            cert_chain: chain,
        };
        assert!(matches!(
            SwSigner::new(config),
            Err(AttestError::InvalidKey(_))
        ));
    }

    #[test]
    fn accepts_valid_config() {
        let config = SwSignerConfig {
            private_key_scalar: valid_scalar(),
            cert_chain: valid_chain(),
        };
        assert!(SwSigner::new(config).is_ok());
    }
}
