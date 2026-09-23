// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! TCG UEID extraction from a Caliptra DICE certificate chain.
//!
//! The Caliptra DICE chain encodes the device UEID in an X.509 extension with
//! OID 2.23.133.5.4.4 (TCG UEID). The extension value is an OCTET STRING whose
//! contents are `SEQUENCE { OCTET STRING(ueid_bytes) }`.
//!
//! This module is intentionally narrow: it only handles cert structures
//! Caliptra emits. Every cert in the Caliptra chain is X.509 v3 with at least
//! one extension; a cert with no `[3]` extensions wrapper is treated as
//! malformed and returns `Err`. A cert that has extensions but lacks the TCG
//! UEID OID returns `Ok(None)` (e.g. intermediate or root CA certs).

use heapless::Vec;
use openprot_attest_api::{consts::MAX_CERT_SIZE, consts::MAX_CHAIN_LEN, AttestError};

use crate::der::{find_tag, octet_string_body, sequence_body, skip_optional_boolean, take_sequence};

/// DER encoding of OID 2.23.133.5.4.4 (TCG UEID extension).
const OID_TCG_UEID: [u8; 8] = [0x06, 0x06, 0x67, 0x81, 0x05, 0x05, 0x04, 0x04];

/// Length of the UEID value in Caliptra certs (17 bytes: 1-byte type + 16-byte ID).
pub const UEID_LEN: usize = 17;

/// Extract the UEID bytes from a single DER-encoded X.509 certificate.
///
/// Returns `Ok(None)` if the cert has a `[3]` extensions wrapper but does not
/// carry the TCG UEID OID (e.g. a root CA or intermediate without UEID).
/// Returns `Err` if the DER structure is malformed (missing extensions wrapper,
/// bad length encoding, unexpected UEID field length).
fn extract(cert_der: &[u8]) -> Result<Option<[u8; UEID_LEN]>, AttestError> {
    let tbs = sequence_body(cert_der).ok_or(AttestError::Der("cert: bad outer SEQUENCE"))?;
    let tbs_body = sequence_body(tbs).ok_or(AttestError::Der("cert: bad TBS SEQUENCE"))?;

    // All Caliptra-emitted certs are X.509 v3 with extensions. A missing [3]
    // wrapper indicates a malformed cert, not a "no UEID" condition.
    let extensions_wrapper =
        find_tag(tbs_body, 0xa3).ok_or(AttestError::Der("cert: no extensions wrapper"))?;

    let ext_seq = sequence_body(extensions_wrapper)
        .ok_or(AttestError::Der("cert: bad extensions SEQUENCE"))?;

    let mut remaining = ext_seq;
    while !remaining.is_empty() {
        let (ext_body, rest) =
            take_sequence(remaining).ok_or(AttestError::Der("cert: bad extension entry"))?;
        remaining = rest;

        if ext_body.starts_with(&OID_TCG_UEID) {
            let after_oid = &ext_body[OID_TCG_UEID.len()..];
            let extn_value_outer = skip_optional_boolean(after_oid);
            let contents = octet_string_body(extn_value_outer)
                .ok_or(AttestError::Der("ueid: bad extnValue OCTET STRING"))?;
            let inner_seq =
                sequence_body(contents).ok_or(AttestError::Der("ueid: bad inner SEQUENCE"))?;
            let ueid_bytes = octet_string_body(inner_seq)
                .ok_or(AttestError::Der("ueid: bad inner OCTET STRING"))?;
            if ueid_bytes.len() != UEID_LEN {
                return Err(AttestError::Der("ueid: unexpected length"));
            }
            let mut out = [0u8; UEID_LEN];
            out.copy_from_slice(ueid_bytes);
            return Ok(Some(out));
        }
    }
    Ok(None)
}

/// Extract the UEID from the leaf cert and verify every other cert in the chain
/// that carries a TCG UEID extension has the same value.
///
/// The leaf cert is at index 0 (AliasRT). Intermediate and root CA certs may
/// not carry the UEID extension; those return `Ok(None)` from `extract` and
/// are skipped without error.
pub fn extract_and_verify(
    chain: &Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
) -> Result<[u8; UEID_LEN], AttestError> {
    if chain.is_empty() {
        return Err(AttestError::ChainValidation("cert chain is empty"));
    }

    let leaf_ueid = extract(&chain[0])?
        .ok_or(AttestError::ChainValidation("leaf cert missing TCG UEID extension"))?;

    for cert in chain.iter().skip(1) {
        if let Some(ueid) = extract(cert)? {
            if ueid != leaf_ueid {
                return Err(AttestError::ChainValidation(
                    "UEID mismatch across certificate chain",
                ));
            }
        }
    }

    Ok(leaf_ueid)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cert_with_ueid(ueid: &[u8; UEID_LEN]) -> heapless::Vec<u8, 512> {
        let inner_os = der_tlv(0x04, ueid);
        let inner_seq = der_tlv(0x30, &inner_os);
        let extn_value = der_tlv(0x04, &inner_seq);
        let mut ext_body: heapless::Vec<u8, 64> = heapless::Vec::new();
        ext_body.extend_from_slice(&OID_TCG_UEID).unwrap();
        ext_body.extend_from_slice(&extn_value).unwrap();
        let ext_seq = der_tlv(0x30, &ext_body);
        let exts_seq = der_tlv(0x30, &ext_seq);
        let exts_wrapper = der_tlv(0xa3, &exts_seq);

        let mut tbs_body: heapless::Vec<u8, 256> = heapless::Vec::new();
        tbs_body.extend_from_slice(&[0xa0, 0x03, 0x02, 0x01, 0x02]).unwrap();
        tbs_body.extend_from_slice(&[0x02, 0x01, 0x01]).unwrap();
        for _ in 0..5 { tbs_body.extend_from_slice(&[0x30, 0x00]).unwrap(); }
        tbs_body.extend_from_slice(&exts_wrapper).unwrap();
        let tbs = der_tlv(0x30, &tbs_body);

        let mut cert_body: heapless::Vec<u8, 384> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap();
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap();
        let mut buf: heapless::Vec<u8, 512> = heapless::Vec::new();
        buf.extend_from_slice(&der_tlv(0x30, &cert_body)).unwrap();
        buf
    }

    fn der_tlv(tag: u8, value: &[u8]) -> heapless::Vec<u8, 256> {
        let mut out: heapless::Vec<u8, 256> = heapless::Vec::new();
        out.push(tag).unwrap();
        let l = value.len();
        if l < 128 { out.push(l as u8).unwrap(); }
        else if l < 256 { out.push(0x81).unwrap(); out.push(l as u8).unwrap(); }
        else { out.push(0x82).unwrap(); out.push((l >> 8) as u8).unwrap(); out.push((l & 0xff) as u8).unwrap(); }
        out.extend_from_slice(value).unwrap();
        out
    }

    #[test]
    fn extracts_ueid_from_cert() {
        let ueid = [0xABu8; UEID_LEN];
        let cert = make_cert_with_ueid(&ueid);
        assert_eq!(extract(&cert).unwrap(), Some(ueid));
    }

    #[test]
    fn returns_err_for_cert_with_no_extensions_wrapper() {
        let tbs_body = [
            0xa0, 0x03, 0x02, 0x01, 0x02,
            0x02, 0x01, 0x01,
            0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00,
        ];
        let tbs = der_tlv(0x30, &tbs_body);
        let mut cert_body: heapless::Vec<u8, 64> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap();
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap();
        let cert = der_tlv(0x30, &cert_body);
        assert!(extract(&cert).is_err());
    }

    #[test]
    fn returns_none_for_cert_with_extensions_but_no_ueid() {
        let other_oid: [u8; 5] = [0x06, 0x03, 0x55, 0x1d, 0x0e];
        let ext_val = der_tlv(0x04, &[0x04, 0x14]);
        let mut ext_body: heapless::Vec<u8, 32> = heapless::Vec::new();
        ext_body.extend_from_slice(&other_oid).unwrap();
        ext_body.extend_from_slice(&ext_val).unwrap();
        let exts_wrapper = der_tlv(0xa3, &der_tlv(0x30, &der_tlv(0x30, &ext_body)));

        let mut tbs_body: heapless::Vec<u8, 128> = heapless::Vec::new();
        tbs_body.extend_from_slice(&[0xa0, 0x03, 0x02, 0x01, 0x02]).unwrap();
        tbs_body.extend_from_slice(&[0x02, 0x01, 0x01]).unwrap();
        for _ in 0..5 { tbs_body.extend_from_slice(&[0x30, 0x00]).unwrap(); }
        tbs_body.extend_from_slice(&exts_wrapper).unwrap();
        let tbs = der_tlv(0x30, &tbs_body);

        let mut cert_body: heapless::Vec<u8, 256> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap();
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap();
        let cert = der_tlv(0x30, &cert_body);
        assert_eq!(extract(&cert).unwrap(), None);
    }

    #[test]
    fn verify_passes_when_all_certs_agree() {
        let ueid = [0x11u8; UEID_LEN];
        let cert = make_cert_with_ueid(&ueid);
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        for _ in 0..2 {
            let mut c: Vec<u8, MAX_CERT_SIZE> = Vec::new();
            c.extend_from_slice(&cert).unwrap();
            chain.push(c).unwrap();
        }
        assert_eq!(extract_and_verify(&chain).unwrap(), ueid);
    }

    #[test]
    fn verify_fails_on_mismatch() {
        let cert_a = make_cert_with_ueid(&[0x11u8; UEID_LEN]);
        let cert_b = make_cert_with_ueid(&[0x22u8; UEID_LEN]);
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        c0.extend_from_slice(&cert_a).unwrap();
        let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        c1.extend_from_slice(&cert_b).unwrap();
        chain.push(c0).unwrap();
        chain.push(c1).unwrap();
        assert!(extract_and_verify(&chain).is_err());
    }
}
