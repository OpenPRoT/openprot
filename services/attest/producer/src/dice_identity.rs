// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra DICE certificate chain retrieval.
//!
//! The chain is assembled by Caliptra during boot:
//!   Vendor CA → IDevID → LDevID → AliasFMC → AliasRT  (leaf)
//!
//! In production the `CaliptraSigner` implementation delegates to the
//! `caliptra-sw` Rust driver.  Tests use a software-backed stub.

use openprot_attest_api::{AttestError, CaliptraSigner, CertChain};

/// Retrieve the full DICE certificate chain from Caliptra, ordered leaf → root.
pub fn cert_chain(
    signer: &dyn CaliptraSigner,
) -> Result<CertChain, AttestError> {
    let chain = signer.cert_chain_der()?;
    if chain.len() < 2 {
        return Err(AttestError::Caliptra(
            "DICE certificate chain must have at least two certificates (leaf + one CA)".into(),
        ));
    }
    Ok(CertChain(chain))
}

/// Return just the DER-encoded Alias (leaf) certificate.
pub fn alias_cert(signer: &dyn CaliptraSigner) -> Result<Vec<u8>, AttestError> {
    signer.alias_cert_der()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OneCert;
    struct TwoCerts;

    impl CaliptraSigner for OneCert {
        fn sign_es384(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn alias_cert_der(&self) -> Result<Vec<u8>, AttestError> {
            Ok(vec![0x30, 0x00])
        }
        fn cert_chain_der(&self) -> Result<Vec<Vec<u8>>, AttestError> {
            Ok(vec![vec![0x30, 0x00]])
        }
    }

    impl CaliptraSigner for TwoCerts {
        fn sign_es384(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn alias_cert_der(&self) -> Result<Vec<u8>, AttestError> {
            Ok(vec![0x30, 0x00])
        }
        fn cert_chain_der(&self) -> Result<Vec<Vec<u8>>, AttestError> {
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

    #[test]
    fn alias_cert_returns_leaf() {
        assert_eq!(alias_cert(&TwoCerts).unwrap(), vec![0x30, 0x00]);
    }
}
