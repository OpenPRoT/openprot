// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Minimal DER/X.509 primitives shared by `cert_ueid` and `dice_identity`.

use openprot_attest_api::AttestError;

/// Parse a DER length field at `data[0..]`. Returns `(length, bytes_consumed)`.
pub(crate) fn parse_length(data: &[u8]) -> Option<(usize, usize)> {
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
pub(crate) fn sequence_body(data: &[u8]) -> Option<&[u8]> {
    if data.first()? != &0x30 {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    data.get(1 + consumed..1 + consumed + len)
}

/// Return the body and remaining bytes after consuming the first SEQUENCE in `data`.
pub(crate) fn take_sequence(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if data.first()? != &0x30 {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    let body = data.get(1 + consumed..1 + consumed + len)?;
    let rest = data.get(1 + consumed + len..)?;
    Some((body, rest))
}

/// Return the body of the first OCTET STRING at the start of `data`.
pub(crate) fn octet_string_body(data: &[u8]) -> Option<&[u8]> {
    if data.first()? != &0x04 {
        return None;
    }
    let (len, consumed) = parse_length(&data[1..])?;
    data.get(1 + consumed..1 + consumed + len)
}

/// Scan `data` for the first TLV with the given tag, returning its value bytes.
pub(crate) fn find_tag(data: &[u8], tag: u8) -> Option<&[u8]> {
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
pub(crate) fn skip_optional_boolean(data: &[u8]) -> &[u8] {
    if data.first() == Some(&0x01) {
        if let Some((len, consumed)) = parse_length(&data[1..]) {
            let end = 1 + consumed + len;
            if end <= data.len() {
                return &data[end..];
            }
        }
    }
    data
}

/// Return `true` if `cert_der` is an X.509 v3 certificate (version field = 2).
pub(crate) fn is_x509_v3(cert_der: &[u8]) -> bool {
    const V3: [u8; 5] = [0xA0, 0x03, 0x02, 0x01, 0x02];
    sequence_body(cert_der)
        .and_then(sequence_body)
        .and_then(|b| b.get(..5))
        .is_some_and(|v| v == V3)
}

/// Return `true` if `cert_der` contains an X.509 extension whose OID bytes
/// (full DER TLV) match `oid`. Returns `false` if the certificate carries no
/// extensions at all (valid for root CA certs).
pub(crate) fn has_extension_oid(cert_der: &[u8], oid: &[u8]) -> Result<bool, AttestError> {
    let tbs = sequence_body(cert_der).ok_or(AttestError::Der("bad outer SEQUENCE"))?;
    let tbs_body = sequence_body(tbs).ok_or(AttestError::Der("bad TBS SEQUENCE"))?;
    let ext_wrapper = match find_tag(tbs_body, 0xa3) {
        Some(e) => e,
        None => return Ok(false),
    };
    let ext_seq = sequence_body(ext_wrapper).ok_or(AttestError::Der("bad extensions SEQUENCE"))?;
    let mut remaining = ext_seq;
    while !remaining.is_empty() {
        let (ext_body, rest) =
            take_sequence(remaining).ok_or(AttestError::Der("bad extension entry"))?;
        remaining = rest;
        if ext_body.starts_with(oid) {
            return Ok(true);
        }
    }
    Ok(false)
}
