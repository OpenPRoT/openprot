// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra DICE certificate chain retrieval.
//!
//! The chain is assembled by Caliptra during boot:
//!   Vendor CA → IDevID → LDevID → AliasFMC → AliasRT  (leaf)
//!
//! In production the `HwSigner` implementation delegates to the
//! `caliptra-sw` Rust driver.  Tests use a software-backed stub.

use openprot_attest_api::{AttestError, CertChain, HwSigner};

/// Retrieve the full DICE certificate chain from the signer, ordered leaf → root.
pub fn cert_chain(signer: &dyn HwSigner) -> Result<CertChain, AttestError> {
    let chain = signer.cert_chain_der()?;
    if chain.len() < 2 {
        return Err(AttestError::Caliptra(
            "DICE certificate chain must have at least two certificates (leaf + one CA)".into(),
        ));
    }
    Ok(CertChain(chain))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    struct OneCert;
    struct TwoCerts;

    impl HwSigner for OneCert {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn leaf_cert_der(&self) -> Result<alloc::vec::Vec<u8>, AttestError> {
            Ok(vec![0x30, 0x00])
        }
        fn cert_chain_der(&self) -> Result<alloc::vec::Vec<alloc::vec::Vec<u8>>, AttestError> {
            Ok(vec![vec![0x30, 0x00]])
        }
    }

    impl HwSigner for TwoCerts {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn leaf_cert_der(&self) -> Result<alloc::vec::Vec<u8>, AttestError> {
            Ok(vec![0x30, 0x00])
        }
        fn cert_chain_der(&self) -> Result<alloc::vec::Vec<alloc::vec::Vec<u8>>, AttestError> {
            Ok(vec![vec![0x30, 0x00], vec![0x30, 0x01]])
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
