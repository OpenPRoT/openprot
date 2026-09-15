// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra DICE certificate chain retrieval and compliance validation.
//!
//! The chain is assembled by Caliptra during boot (stored leaf → root):
//!   index 0: AliasRT (leaf)
//!   index 1: AliasFMC
//!   index 2: LDevID
//!   index 3: IDevID  (optional, fuse-provisioned)
//!   index 4: Vendor CA (optional, standard root CA)

use heapless::Vec;

use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN};
use openprot_attest_api::{AttestError, HwSigner};

use crate::cert_ueid::{has_extension_oid, is_x509_v3};

/// DER encoding of OID 2.23.133.5.4.5 (tcg-dice-MultiTcbInfo).
const OID_TCG_MULTI_TCBINFO: [u8; 8] = [0x06, 0x06, 0x67, 0x81, 0x05, 0x05, 0x04, 0x05];

/// Retrieve the full DICE certificate chain from the signer and validate it.
pub fn cert_chain(
    signer: &dyn HwSigner,
) -> Result<Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>, AttestError> {
    let mut buf: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    signer.cert_chain_der(&mut buf)?;
    validate_chain(&buf)?;
    Ok(buf)
}

/// Verify that `chain` meets the minimum structural and Caliptra DICE requirements.
///
/// Checks applied to every certificate:
/// - Chain length ≥ 3 (AliasRT + AliasFMC + LDevID minimum).
/// - First byte is `0x30` (DER SEQUENCE).
/// - X.509 v3 (TBS version field = `A0 03 02 01 02`).
///
/// Additionally, every cert **except the last** (root CA) must carry the
/// `tcg-dice-MultiTcbInfo` extension (OID 2.23.133.5.4.5).  This extension is
/// present in all Caliptra 1.x and 2.x DICE-generated certificates (AliasRT,
/// AliasFMC, LDevID, IDevID).  The root Vendor CA cert is a standard X.509
/// cert that does not carry DICE extensions and is therefore exempt.
pub fn validate_chain(chain: &[Vec<u8, MAX_CERT_SIZE>]) -> Result<(), AttestError> {
    if chain.len() < 3 {
        return Err(AttestError::Caliptra(
            "DICE chain must have at least 3 certificates",
        ));
    }
    for (i, cert) in chain.iter().enumerate() {
        if cert.first() != Some(&0x30) {
            return Err(AttestError::Caliptra("cert: not a DER SEQUENCE"));
        }
        if !is_x509_v3(cert) {
            return Err(AttestError::Caliptra("cert: not X.509 v3"));
        }
        let is_root = i == chain.len() - 1;
        if !is_root && !has_extension_oid(cert, &OID_TCG_MULTI_TCBINFO)? {
            return Err(AttestError::Caliptra(
                "cert: missing tcg-dice-MultiTcbInfo extension",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;
    use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS};
    use openprot_attest_api::AttestError;

    use crate::signer::STUB_CERT;

    // ── HwSigner stubs ────────────────────────────────────────────────────────

    struct OneCert;
    struct TwoCerts;
    struct ThreeDiceCerts;

    impl HwSigner for OneCert {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c.extend_from_slice(&STUB_CERT).unwrap();
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
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c0.extend_from_slice(&STUB_CERT).unwrap();
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

    impl HwSigner for ThreeDiceCerts {
        fn sign(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn cert_chain_der(
            &self,
            buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
        ) -> Result<(), AttestError> {
            // leaf (AliasRT) and intermediate (AliasFMC): carry MultiTcbInfo
            // root (LDevID in this minimal stub): standard v3 cert, no MultiTcbInfo
            buf.push(make_dice_cert(true, true))
                .map_err(|_| AttestError::BufferFull)?;
            buf.push(make_dice_cert(true, true))
                .map_err(|_| AttestError::BufferFull)?;
            buf.push(make_dice_cert(true, false))
                .map_err(|_| AttestError::BufferFull)
        }
        fn caliptra_measurements(
            &self,
            _out: &mut Vec<openprot_attest_api::Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            Ok(())
        }
    }

    // ── DER test helpers ──────────────────────────────────────────────────────

    fn der_tlv(tag: u8, value: &[u8]) -> heapless::Vec<u8, 256> {
        let mut out: heapless::Vec<u8, 256> = heapless::Vec::new();
        out.push(tag).unwrap();
        let l = value.len();
        if l < 128 {
            out.push(l as u8).unwrap();
        } else if l < 256 {
            out.push(0x81).unwrap();
            out.push(l as u8).unwrap();
        } else {
            out.push(0x82).unwrap();
            out.push((l >> 8) as u8).unwrap();
            out.push((l & 0xff) as u8).unwrap();
        }
        out.extend_from_slice(value).unwrap();
        out
    }

    /// Build a minimal DER cert for testing.
    ///
    /// `is_v3`: include the X.509 v3 version field (`A0 03 02 01 02`).
    /// `has_multitcbinfo`: embed a tcg-dice-MultiTcbInfo extension.
    fn make_dice_cert(is_v3: bool, has_multitcbinfo: bool) -> heapless::Vec<u8, MAX_CERT_SIZE> {
        // Optional MultiTcbInfo extension
        let mut ext_items: heapless::Vec<u8, 64> = heapless::Vec::new();
        if has_multitcbinfo {
            let oid = OID_TCG_MULTI_TCBINFO;
            let extn_val = der_tlv(0x04, &der_tlv(0x30, &[])); // OCTET STRING { SEQUENCE{} }
            let mut ext_body: heapless::Vec<u8, 32> = heapless::Vec::new();
            ext_body.extend_from_slice(&oid).unwrap();
            ext_body.extend_from_slice(&extn_val).unwrap();
            ext_items
                .extend_from_slice(&der_tlv(0x30, &ext_body))
                .unwrap();
        }
        let ext_wrapper = der_tlv(0xa3, &der_tlv(0x30, &ext_items));

        // TBSCertificate body
        let mut tbs_body: heapless::Vec<u8, 128> = heapless::Vec::new();
        if is_v3 {
            tbs_body
                .extend_from_slice(&[0xa0, 0x03, 0x02, 0x01, 0x02])
                .unwrap();
        }
        tbs_body.extend_from_slice(&[0x02, 0x01, 0x01]).unwrap(); // serialNumber INTEGER 1
        for _ in 0..5 {
            tbs_body.extend_from_slice(&[0x30, 0x00]).unwrap(); // placeholder fields
        }
        tbs_body.extend_from_slice(&ext_wrapper).unwrap();

        // Outer cert SEQUENCE { TBS, algId, signature }
        let mut cert_body: heapless::Vec<u8, 256> = heapless::Vec::new();
        cert_body
            .extend_from_slice(&der_tlv(0x30, &tbs_body))
            .unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap(); // placeholder algId
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap(); // placeholder sig

        let mut out: heapless::Vec<u8, MAX_CERT_SIZE> = heapless::Vec::new();
        out.extend_from_slice(&der_tlv(0x30, &cert_body)).unwrap();
        out
    }

    // ── cert_chain() integration tests ───────────────────────────────────────

    #[test]
    fn rejects_chain_shorter_than_three() {
        assert!(cert_chain(&OneCert).is_err());
        assert!(cert_chain(&TwoCerts).is_err());
    }

    #[test]
    fn accepts_valid_three_cert_dice_chain() {
        let chain = cert_chain(&ThreeDiceCerts).unwrap();
        assert_eq!(chain.len(), 3);
    }

    // ── validate_chain() unit tests ───────────────────────────────────────────

    fn make_chain(specs: &[(bool, bool)]) -> Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> {
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        for &(is_v3, has_mti) in specs {
            chain.push(make_dice_cert(is_v3, has_mti)).unwrap();
        }
        chain
    }

    #[test]
    fn validate_rejects_non_sequence_cert() {
        let mut chain = make_chain(&[(true, true), (true, true), (true, false)]);
        // Corrupt the first byte of the leaf cert.
        chain[0][0] = 0x04;
        assert!(validate_chain(&chain).is_err());
    }

    #[test]
    fn validate_rejects_non_v3_cert() {
        // Leaf is v1 (no version field).
        let chain = make_chain(&[(false, true), (true, true), (true, false)]);
        let err = validate_chain(&chain).unwrap_err();
        assert!(matches!(err, AttestError::Caliptra(_)));
    }

    #[test]
    fn validate_rejects_dice_cert_missing_multitcbinfo() {
        // Leaf is v3 but lacks the MultiTcbInfo extension.
        let chain = make_chain(&[(true, false), (true, true), (true, false)]);
        let err = validate_chain(&chain).unwrap_err();
        assert!(matches!(err, AttestError::Caliptra(_)));
    }

    #[test]
    fn validate_root_cert_allowed_without_multitcbinfo() {
        // Root (last cert) is not required to carry MultiTcbInfo.
        let chain = make_chain(&[(true, true), (true, true), (true, false)]);
        assert!(validate_chain(&chain).is_ok());
    }

    #[test]
    fn validate_accepts_four_cert_chain_with_root_exempt() {
        // 4-cert chain: leaf, two intermediates, root without MultiTcbInfo.
        let chain = make_chain(&[(true, true), (true, true), (true, true), (true, false)]);
        assert!(validate_chain(&chain).is_ok());
    }
}
