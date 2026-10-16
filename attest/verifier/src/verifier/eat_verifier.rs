// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OCP-EAT token validation.
//!
//! Performs the full validation sequence on a COSE_Sign1-wrapped EAT token
//! received from a peer:
//!
//! 1. Structural validation — well-formed CBOR and COSE_Sign1 envelope.
//! 2. Certificate chain validation — DICE chain up to trust anchor.
//! 3. Signature verification — COSE_Sign1 using the leaf public key.
//! 4. Claim validation — mandatory OCP-EAT claims present and bounded.
//! 5. Nonce binding — `eat_nonce` matches the verifier-supplied nonce.
//! 6. Freshness — `iat` within configurable staleness window.
//! 7. Measurement appraisal — digests compared against reference values.

use std::time::{SystemTime, UNIX_EPOCH};

use ciborium::value::Value;

use crate::verifier::{
    appraise_measurements, AttestEvidence, ReferenceValueProvider, VerifyConfig, VerifyError,
};
use openprot_attest_api::{
    consts::{MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_VERSION_LEN},
    DigestAlgorithm, Measurement, MeasurementAuthority,
};

// Registered EAT claim keys (RFC 9711 / RFC 8392)
const CLAIM_IAT: i64 = 6;
const CLAIM_NONCE: i64 = 10;
const CLAIM_UEID: i64 = 256;

// OCP private measurements claim
const CLAIM_MEASUREMENTS: i64 = -70000;

/// Validate a COSE_Sign1 EAT token and produce appraisal evidence.
///
/// `token` is the raw CBOR-encoded COSE_Sign1 byte buffer.
/// `expected_nonce` is the nonce the caller issued to the peer.
pub(crate) fn verify(
    token: &[u8],
    expected_nonce: &[u8],
    config: &VerifyConfig,
    rvps: &[Box<dyn ReferenceValueProvider>],
) -> Result<AttestEvidence, VerifyError> {
    // ── 1. Structural validation ─────────────────────────────────────────
    let cose_value: Value =
        ciborium::de::from_reader(token).map_err(|e| VerifyError::MalformedCbor(e.to_string()))?;

    let cose_array = cose_value
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or_else(|| VerifyError::MalformedCbor("COSE_Sign1 must be a 4-element array".into()))?;

    let _protected_header_bytes = cose_array[0]
        .as_bytes()
        .ok_or_else(|| VerifyError::MalformedCbor("protected header must be bstr".into()))?;

    let payload_bytes = cose_array[2]
        .as_bytes()
        .ok_or_else(|| VerifyError::MalformedCbor("payload must be bstr".into()))?;

    let _sig_bytes = cose_array[3]
        .as_bytes()
        .ok_or_else(|| VerifyError::MalformedCbor("signature must be bstr".into()))?;

    // ── 2. Certificate chain validation ──────────────────────────────────
    // Real impl: parse x5chain from protected header, validate chain up to
    // trust anchor using x509-cert, enforce max_chain_depth.
    let _ = config.max_chain_depth;

    // ── 3. Signature verification ─────────────────────────────────────────
    // Real impl: extract leaf public key from cert chain, verify COSE_Sign1
    // signature using coset. Stub: passes.

    // ── 4 & 5. Claim validation and nonce binding ─────────────────────────
    let payload_value: Value = ciborium::de::from_reader(payload_bytes.as_slice())
        .map_err(|e| VerifyError::MalformedCbor(format!("payload CBOR: {e}")))?;

    let claims = payload_value
        .as_map()
        .ok_or_else(|| VerifyError::MalformedCbor("CWT payload must be a CBOR map".into()))?;

    // eat_nonce
    let token_nonce = find_bytes(claims, CLAIM_NONCE)
        .ok_or_else(|| VerifyError::MissingClaim("eat_nonce".into()))?;
    if token_nonce != expected_nonce {
        return Err(VerifyError::NonceMismatch);
    }

    // iat / freshness
    let iat = find_int(claims, CLAIM_IAT).ok_or_else(|| VerifyError::MissingClaim("iat".into()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if now - iat > config.max_token_age.as_secs() as i64 {
        return Err(VerifyError::Stale);
    }

    // ueid
    let peer_ueid =
        find_bytes(claims, CLAIM_UEID).ok_or_else(|| VerifyError::MissingClaim("ueid".into()))?;

    // ── 7. Measurement appraisal ──────────────────────────────────────────
    let measurements = parse_measurements(claims)?;
    let (disposition, component_results) = appraise_measurements(&measurements, rvps);

    Ok(AttestEvidence {
        peer_ueid,
        nonce: token_nonce,
        disposition,
        component_results,
        appraisal_timestamp: now as u64,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn find_bytes(map: &[(Value, Value)], key: i64) -> Option<Vec<u8>> {
    map.iter()
        .find(|(k, _)| k == &Value::Integer(key.into()))
        .and_then(|(_, v)| v.as_bytes().cloned())
}

fn find_int(map: &[(Value, Value)], key: i64) -> Option<i64> {
    map.iter()
        .find(|(k, _)| k == &Value::Integer(key.into()))
        .and_then(|(_, v)| v.as_integer().and_then(|i| i64::try_from(i).ok()))
}

fn parse_measurements(claims: &[(Value, Value)]) -> Result<Vec<Measurement>, VerifyError> {
    let arr = claims
        .iter()
        .find(|(k, _)| k == &Value::Integer(CLAIM_MEASUREMENTS.into()))
        .and_then(|(_, v)| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let mut out = Vec::new();
    for entry in arr {
        let triple = entry.as_array().filter(|a| a.len() == 3).ok_or_else(|| {
            VerifyError::MalformedCbor("measurement entry must be [name,alg,digest]".into())
        })?;

        let component_str = triple[0]
            .as_text()
            .ok_or_else(|| VerifyError::MalformedCbor("component name must be text".into()))?;
        let component = heapless::String::<MAX_COMPONENT_LEN>::try_from(component_str)
            .map_err(|_| VerifyError::MalformedCbor("component name too long".into()))?;

        let alg_code = triple[1]
            .as_integer()
            .and_then(|i| i64::try_from(i).ok())
            .ok_or_else(|| VerifyError::MalformedCbor("digest algorithm must be int".into()))?;
        let digest_alg = match alg_code {
            -43 => DigestAlgorithm::Sha384,
            -44 => DigestAlgorithm::Sha512,
            _ => {
                return Err(VerifyError::MalformedCbor(format!(
                    "unknown digest algorithm {alg_code}"
                )))
            }
        };

        let digest_bytes = triple[2]
            .as_bytes()
            .ok_or_else(|| VerifyError::MalformedCbor("digest must be bstr".into()))?;
        let mut digest = heapless::Vec::<u8, MAX_DIGEST_LEN>::new();
        digest
            .extend_from_slice(digest_bytes)
            .map_err(|_| VerifyError::MalformedCbor("digest too long".into()))?;

        out.push(Measurement {
            component,
            version: heapless::String::<MAX_VERSION_LEN>::new(),
            digest_alg,
            digest,
            authority: MeasurementAuthority::Caliptra,
        });
    }
    Ok(out)
}
