// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Minimal DER walker for extracting the TCG UEID from a Caliptra DER cert.
//!
//! The Caliptra DICE chain encodes the device UEID in an X.509 extension with
//! OID 2.23.133.5.4.4 (TCG UEID). The extension value is an OCTET STRING whose
//! contents are `SEQUENCE { OCTET STRING(ueid_bytes) }`.
//!
//! This module walks the raw DER without an external ASN.1 library, following
//! only the tags and length fields needed to reach the extension value. It is
//! intentionally narrow: it only handles the cert structures Caliptra emits.

use openprot_attest_api::{AttestError, consts::MAX_CHAIN_LEN, consts::MAX_CERT_SIZE};
use heapless::Vec;

/// DER tag bytes.
const TAG_SEQUENCE: u8 = 0x30;
const TAG_OCTET_STRING: u8 = 0x04;

/// DER encoding of OID 2.23.133.5.4.4 (TCG UEID extension).
const OID_TCG_UEID: [u8; 8] = [0x06, 0x06, 0x67, 0x81, 0x05, 0x05, 0x04, 0x04];

/// Length of the UEID value in Caliptra certs (17 bytes: 1-byte type + 16-byte ID).
pub const UEID_LEN: usize = 17;

/// Extract the UEID bytes from a single DER-encoded X.509 certificate.
///
/// Returns `None` if the cert does not carry the TCG UEID extension (e.g. a
/// root CA cert that pre-dates the DICE chain). Returns an error if the DER is
/// malformed or the UEID field has an unexpected length.
pub fn extract(cert_der: &[u8]) -> Result<Option<[u8; UEID_LEN]>, AttestError> {
    // cert DER: SEQUENCE { TBSCertificate, AlgorithmIdentifier, BIT STRING }
    let tbs = sequence_body(cert_der).ok_or(AttestError::Caliptra("cert: bad outer SEQUENCE"))?;

    // TBSCertificate: SEQUENCE { version [0], serialNumber, signature, issuer,
    //                            validity, subject, spki, [3] extensions }
    let tbs_body = sequence_body(tbs).ok_or(AttestError::Caliptra("cert: bad TBS SEQUENCE"))?;

    // Walk TBS fields to find the [3] EXPLICIT extensions wrapper (tag 0xa3).
    let extensions_wrapper = find_tag(tbs_body, 0xa3)
        .ok_or(AttestError::Caliptra("cert: no extensions"))?;

    // [3] wraps a SEQUENCE of Extension SEQUENCEs.
    let ext_seq = sequence_body(extensions_wrapper)
        .ok_or(AttestError::Caliptra("cert: bad extensions SEQUENCE"))?;

    // Walk each Extension SEQUENCE looking for the TCG UEID OID.
    let mut remaining = ext_seq;
    while !remaining.is_empty() {
        let (ext_body, rest) = take_sequence(remaining)
            .ok_or(AttestError::Caliptra("cert: bad extension entry"))?;
        remaining = rest;

        // Extension ::= SEQUENCE { extnID OBJECT IDENTIFIER, extnValue OCTET STRING }
        // (critical BOOLEAN is optional; skip it if present)
        if ext_body.starts_with(&OID_TCG_UEID) {
            // Found our extension. Skip the OID (8 bytes), skip optional critical
            // boolean, then read the extnValue OCTET STRING.
            let after_oid = &ext_body[OID_TCG_UEID.len()..];
            let extn_value_outer = skip_optional_boolean(after_oid);

            // extnValue is OCTET STRING { contents }
            let contents = octet_string_body(extn_value_outer)
                .ok_or(AttestError::Caliptra("ueid: bad extnValue OCTET STRING"))?;

            // Contents: SEQUENCE { OCTET STRING(ueid_bytes) }
            let inner_seq = sequence_body(contents)
                .ok_or(AttestError::Caliptra("ueid: bad inner SEQUENCE"))?;
            let ueid_bytes = octet_string_body(inner_seq)
                .ok_or(AttestError::Caliptra("ueid: bad inner OCTET STRING"))?;

            if ueid_bytes.len() != UEID_LEN {
                return Err(AttestError::Caliptra("ueid: unexpected length"));
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
/// not carry the UEID extension; those are skipped without error.
pub fn extract_and_verify(
    chain: &Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
) -> Result<[u8; UEID_LEN], AttestError> {
    if chain.is_empty() {
        return Err(AttestError::Caliptra("cert chain is empty"));
    }

    // The leaf cert (index 0) must carry the UEID extension.
    let leaf_ueid = extract(&chain[0])?
        .ok_or(AttestError::Caliptra("leaf cert missing TCG UEID extension"))?;

    // Every other cert that carries the extension must match.
    for cert in chain.iter().skip(1) {
        if let Some(ueid) = extract(cert)? {
            if ueid != leaf_ueid {
                return Err(AttestError::Caliptra(
                    "UEID mismatch across certificate chain",
                ));
            }
        }
    }

    Ok(leaf_ueid)
}

// ── DER primitives ────────────────────────────────────────────────────────────

/// Parse a DER length field at `data[0..]`. Returns `(length, bytes_consumed)`.
fn parse_length(data: &[u8]) -> Option<(usize, usize)> {
    let first = *data.first()?;
    if first < 0x80 {
        Some((first as usize, 1))
    } else {
        let n = (first & 0x7f) as usize;
        if n == 0 || n > core::mem::size_of::<usize>() || data.len() < 1 + n {
            return None;
        }
        let mut len = 0usize;
        for &b in &data[1..1 + n] {
            len = len.checked_shl(8)?.checked_add(b as usize)?;
        }
        Some((len, 1 + n))
    }
}

/// Return the body of the first SEQUENCE found at the start of `data`.
fn sequence_body(data: &[u8]) -> Option<&[u8]> {
    if data.first()? != &TAG_SEQUENCE {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    data.get(1 + consumed..1 + consumed + len)
}

/// Return the body and remaining bytes after consuming the first SEQUENCE in `data`.
fn take_sequence(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if data.first()? != &TAG_SEQUENCE {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    let body = data.get(1 + consumed..1 + consumed + len)?;
    let rest = data.get(1 + consumed + len..)?;
    Some((body, rest))
}

/// Return the body of the first OCTET STRING at the start of `data`.
fn octet_string_body(data: &[u8]) -> Option<&[u8]> {
    if data.first()? != &TAG_OCTET_STRING {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    data.get(1 + consumed..1 + consumed + len)
}

/// Scan `data` for the first TLV with the given tag, returning its value bytes.
fn find_tag(data: &[u8], tag: u8) -> Option<&[u8]> {
    let mut pos = 0;
    while pos < data.len() {
        let t = data[pos];
        let (len, consumed) = parse_length(data.get(pos + 1..)?)?;
        if t == tag {
            return data.get(pos + 1 + consumed..pos + 1 + consumed + len);
        }
        pos = pos.checked_add(1 + consumed + len)?;
    }
    None
}

/// If `data` starts with a DER BOOLEAN TLV, skip past it and return the rest.
fn skip_optional_boolean(data: &[u8]) -> &[u8] {
    const TAG_BOOLEAN: u8 = 0x01;
    if data.first() == Some(&TAG_BOOLEAN) {
        if let Some((len, consumed)) = parse_length(&data[1..]) {
            let end = 1 + consumed + len;
            if end <= data.len() {
                return &data[end..];
            }
        }
    }
    data
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal DER-encoded cert containing a TCG UEID extension.
    /// Structure: SEQUENCE { SEQUENCE { ...[3] { SEQUENCE { ext } }... }, alg, sig }
    fn make_cert_with_ueid(ueid: &[u8; UEID_LEN]) -> heapless::Vec<u8, 512> {
        let mut buf: heapless::Vec<u8, 512> = heapless::Vec::new();

        // Build extnValue contents: SEQUENCE { OCTET STRING(ueid) }
        let inner_os = der_tlv(TAG_OCTET_STRING, ueid);
        let inner_seq = der_tlv(TAG_SEQUENCE, &inner_os);
        // extnValue: OCTET STRING { inner_seq }
        let extn_value = der_tlv(TAG_OCTET_STRING, &inner_seq);
        // Extension: SEQUENCE { OID, extnValue }
        let mut ext_body: heapless::Vec<u8, 64> = heapless::Vec::new();
        ext_body.extend_from_slice(&OID_TCG_UEID).unwrap();
        ext_body.extend_from_slice(&extn_value).unwrap();
        let ext_seq = der_tlv(TAG_SEQUENCE, &ext_body);

        // Extensions SEQUENCE
        let exts_seq = der_tlv(TAG_SEQUENCE, &ext_seq);
        // [3] EXPLICIT wrapper
        let exts_wrapper = der_tlv(0xa3, &exts_seq);

        // Minimal TBS stub: version + serial placeholder + extensions wrapper
        let tbs_version = [0xa0, 0x03, 0x02, 0x01, 0x02]; // [0] v3
        let tbs_serial = [0x02, 0x01, 0x01]; // INTEGER 1
        let tbs_alg = [0x30, 0x00]; // empty SEQUENCE placeholder
        let tbs_issuer = [0x30, 0x00];
        let tbs_validity = [0x30, 0x00];
        let tbs_subject = [0x30, 0x00];
        let tbs_spki = [0x30, 0x00];
        let mut tbs_body: heapless::Vec<u8, 256> = heapless::Vec::new();
        tbs_body.extend_from_slice(&tbs_version).unwrap();
        tbs_body.extend_from_slice(&tbs_serial).unwrap();
        tbs_body.extend_from_slice(&tbs_alg).unwrap();
        tbs_body.extend_from_slice(&tbs_issuer).unwrap();
        tbs_body.extend_from_slice(&tbs_validity).unwrap();
        tbs_body.extend_from_slice(&tbs_subject).unwrap();
        tbs_body.extend_from_slice(&tbs_spki).unwrap();
        tbs_body.extend_from_slice(&exts_wrapper).unwrap();
        let tbs = der_tlv(TAG_SEQUENCE, &tbs_body);

        // Outer cert SEQUENCE { TBS, placeholder alg, placeholder sig }
        let mut cert_body: heapless::Vec<u8, 384> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap(); // alg
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap(); // BIT STRING
        let cert = der_tlv(TAG_SEQUENCE, &cert_body);
        buf.extend_from_slice(&cert).unwrap();
        buf
    }

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

    #[test]
    fn extracts_ueid_from_cert() {
        let ueid = [0xABu8; UEID_LEN];
        let cert = make_cert_with_ueid(&ueid);
        let result = extract(&cert).unwrap();
        assert_eq!(result, Some(ueid));
    }

    #[test]
    fn returns_err_for_cert_with_no_extensions_wrapper() {
        // Caliptra certs always carry v3 extensions; a cert without [3] is malformed.
        let tbs_body = [
            0xa0, 0x03, 0x02, 0x01, 0x02, // version v3
            0x02, 0x01, 0x01, // serial
            0x30, 0x00, // alg
            0x30, 0x00, // issuer
            0x30, 0x00, // validity
            0x30, 0x00, // subject
            0x30, 0x00, // spki
        ];
        let tbs = der_tlv(TAG_SEQUENCE, &tbs_body);
        let mut cert_body: heapless::Vec<u8, 64> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap();
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap();
        let cert = der_tlv(TAG_SEQUENCE, &cert_body);
        assert!(extract(&cert).is_err());
    }

    #[test]
    fn returns_none_for_cert_with_extensions_but_no_ueid() {
        // A cert with an extensions wrapper but a different OID → None.
        // Build a dummy extension with a different OID (e.g. subjectKeyIdentifier 2.5.29.14).
        let other_oid: [u8; 5] = [0x06, 0x03, 0x55, 0x1d, 0x0e]; // OID 2.5.29.14
        let ext_val = der_tlv(TAG_OCTET_STRING, &[0x04, 0x14]);
        let mut ext_body: heapless::Vec<u8, 32> = heapless::Vec::new();
        ext_body.extend_from_slice(&other_oid).unwrap();
        ext_body.extend_from_slice(&ext_val).unwrap();
        let ext_seq = der_tlv(TAG_SEQUENCE, &ext_body);
        let exts_seq = der_tlv(TAG_SEQUENCE, &ext_seq);
        let exts_wrapper = der_tlv(0xa3, &exts_seq);

        let tbs_version = [0xa0, 0x03, 0x02, 0x01, 0x02];
        let tbs_serial = [0x02, 0x01, 0x01];
        let mut tbs_body: heapless::Vec<u8, 128> = heapless::Vec::new();
        tbs_body.extend_from_slice(&tbs_version).unwrap();
        tbs_body.extend_from_slice(&tbs_serial).unwrap();
        for _ in 0..5 { tbs_body.extend_from_slice(&[0x30, 0x00]).unwrap(); }
        tbs_body.extend_from_slice(&exts_wrapper).unwrap();
        let tbs = der_tlv(TAG_SEQUENCE, &tbs_body);

        let mut cert_body: heapless::Vec<u8, 256> = heapless::Vec::new();
        cert_body.extend_from_slice(&tbs).unwrap();
        cert_body.extend_from_slice(&[0x30, 0x00]).unwrap();
        cert_body.extend_from_slice(&[0x03, 0x01, 0x00]).unwrap();
        let cert = der_tlv(TAG_SEQUENCE, &cert_body);
        assert_eq!(extract(&cert).unwrap(), None);
    }

    #[test]
    fn verify_passes_when_all_certs_agree() {
        let ueid = [0x11u8; UEID_LEN];
        let cert = make_cert_with_ueid(&ueid);
        let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
        let mut c0: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        c0.extend_from_slice(&cert).unwrap();
        let mut c1: Vec<u8, MAX_CERT_SIZE> = Vec::new();
        c1.extend_from_slice(&cert).unwrap();
        chain.push(c0).unwrap();
        chain.push(c1).unwrap();
        let result = extract_and_verify(&chain).unwrap();
        assert_eq!(result, ueid);
    }

    #[test]
    fn verify_fails_on_mismatch() {
        let ueid_a = [0x11u8; UEID_LEN];
        let ueid_b = [0x22u8; UEID_LEN];
        let cert_a = make_cert_with_ueid(&ueid_a);
        let cert_b = make_cert_with_ueid(&ueid_b);
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
