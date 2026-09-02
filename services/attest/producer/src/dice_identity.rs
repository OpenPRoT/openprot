// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra DICE certificate chain retrieval.
//!
//! The chain is assembled by Caliptra during boot:
//!   Vendor CA → IDevID → LDevID → AliasFMC → AliasRT  (leaf)
//!
//! In production the `HwSigner` implementation delegates to the
//! `caliptra-sw` Rust driver.  Tests use a software-backed stub.

use heapless::Vec;

use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN};
use openprot_attest_api::{AttestError, CertChain, HwSigner};

/// Retrieve the full DICE certificate chain from the signer, ordered leaf → root.
pub fn cert_chain(signer: &dyn HwSigner) -> Result<CertChain, AttestError> {
    let mut buf: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    signer.cert_chain_der(&mut buf)?;
    if buf.len() < 2 {
        return Err(AttestError::Caliptra(
            "DICE chain must have at least two certificates (leaf + one CA)",
        ));
    }
    Ok(CertChain(buf))
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;
    use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS};
    use openprot_attest_api::AttestError;

    struct OneCert;
    struct TwoCerts;

    impl HwSigner for OneCert {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError> {
            buf.extend_from_slice(&[0x30, 0x00])
                .map_err(|_| AttestError::BufferFull)
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c.extend_from_slice(&[0x30, 0x00]).unwrap();
            buf.push(c).map_err(|_| AttestError::BufferFull)
        }
        fn caliptra_measurements(
            &self,
            _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            Ok(())
        }
    }

    impl HwSigner for TwoCerts {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError> {
            buf.extend_from_slice(&[0x30, 0x00])
                .map_err(|_| AttestError::BufferFull)
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c0.extend_from_slice(&[0x30, 0x00]).unwrap();
            let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c1.extend_from_slice(&[0x30, 0x01]).unwrap();
            buf.push(c0).map_err(|_| AttestError::BufferFull)?;
            buf.push(c1).map_err(|_| AttestError::BufferFull)
        }
        fn caliptra_measurements(
            &self,
            _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            Ok(())
        }
    }

    #[test]
    fn rejects_chain_with_fewer_than_two_certs() {
        assert!(cert_chain(&OneCert).is_err());
    }

    #[test]
    fn accepts_chain_with_two_certs() {
        let chain = cert_chain(&TwoCerts).unwrap();
        assert_eq!(chain.0.len(), 2);
    }
}
