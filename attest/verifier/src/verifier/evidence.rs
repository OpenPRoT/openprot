// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! [`AttestEvidence`] type and CBOR serialization.
//!
//! An `AttestEvidence` record is returned by the verifier after appraising a
//! peer's SPDM measurements. It is then passed to the attestation producer,
//! which embeds it as a `concise-evidence` claim in the platform's OCP-EAT
//! token. The producer's outer COSE_Sign1 provides integrity protection.
//!
//! `AttestEvidence` is **not** itself a token and is **not** separately signed.

use std::collections::HashMap;

use ciborium::value::Value;

use crate::verifier::{ComponentDisposition, Disposition, VerifyError};

// CBOR map keys for AttestEvidence serialization
const KEY_PEER_UEID: i64 = 1;
const KEY_NONCE: i64 = 2;
const KEY_DISPOSITION: i64 = 3;
const KEY_APPRAISAL_TS: i64 = 4;
const KEY_COMPONENTS: i64 = 5;

/// Structured appraisal result produced by the verifier after an SPDM
/// attestation exchange.
///
/// Returned by [`crate::verifier::Verifier::verify_spdm`] and
/// [`crate::verifier::Verifier::verify_token`].
#[derive(Clone, Debug)]
pub struct AttestEvidence {
    /// UEID of the appraised peer device (from the peer's DICE certificate).
    pub peer_ueid: Vec<u8>,
    /// Freshness nonce echoed from the peer's signed measurement response;
    /// binds this evidence record to the specific SPDM session.
    pub nonce: Vec<u8>,
    /// Overall appraisal outcome.
    pub disposition: Disposition,
    /// Per-component appraisal outcomes, keyed by component name.
    pub component_results: HashMap<String, ComponentDisposition>,
    /// Unix timestamp (seconds) at which the appraisal was performed.
    pub appraisal_timestamp: u64,
}

/// Serialize an [`AttestEvidence`] record to a CBOR [`Value`] for embedding
/// in the producer's EAT token.
pub fn to_cbor(ev: &AttestEvidence) -> Result<Value, VerifyError> {
    let disp_int: i64 = match ev.disposition {
        Disposition::Pass => 0,
        Disposition::Fail => 1,
        Disposition::Indeterminate => 2,
    };

    let component_map: Vec<(Value, Value)> = ev
        .component_results
        .iter()
        .map(|(name, disp)| {
            let d: i64 = match disp {
                ComponentDisposition::Pass => 0,
                ComponentDisposition::Fail => 1,
                ComponentDisposition::Unknown => 2,
            };
            (Value::Text(name.clone()), Value::Integer(d.into()))
        })
        .collect();

    Ok(Value::Map(vec![
        (
            Value::Integer(KEY_PEER_UEID.into()),
            Value::Bytes(ev.peer_ueid.clone()),
        ),
        (
            Value::Integer(KEY_NONCE.into()),
            Value::Bytes(ev.nonce.clone()),
        ),
        (
            Value::Integer(KEY_DISPOSITION.into()),
            Value::Integer(disp_int.into()),
        ),
        (
            Value::Integer(KEY_APPRAISAL_TS.into()),
            Value::Integer((ev.appraisal_timestamp as i64).into()),
        ),
        (
            Value::Integer(KEY_COMPONENTS.into()),
            Value::Map(component_map),
        ),
    ]))
}

/// Deserialize an [`AttestEvidence`] record from a CBOR [`Value`].
///
/// Used by the optional audit store and the host-side CLI verifier tool.
pub fn from_cbor(v: &Value) -> Result<AttestEvidence, VerifyError> {
    let map = v
        .as_map()
        .ok_or_else(|| VerifyError::EvidenceCbor("expected a CBOR map".into()))?;

    macro_rules! get_bytes {
        ($key:expr) => {
            map.iter()
                .find(|(k, _)| k == &Value::Integer($key.into()))
                .and_then(|(_, v)| v.as_bytes().cloned())
                .ok_or_else(|| VerifyError::EvidenceCbor(format!("missing key {}", $key)))?
        };
    }
    macro_rules! get_int {
        ($key:expr) => {
            map.iter()
                .find(|(k, _)| k == &Value::Integer($key.into()))
                .and_then(|(_, v)| v.as_integer().and_then(|i| i64::try_from(i).ok()))
                .ok_or_else(|| VerifyError::EvidenceCbor(format!("missing key {}", $key)))?
        };
    }

    let peer_ueid = get_bytes!(KEY_PEER_UEID);
    let nonce = get_bytes!(KEY_NONCE);
    let disp_int = get_int!(KEY_DISPOSITION);
    let ts = get_int!(KEY_APPRAISAL_TS) as u64;

    let disposition = match disp_int {
        0 => Disposition::Pass,
        1 => Disposition::Fail,
        _ => Disposition::Indeterminate,
    };

    let comp_map = map
        .iter()
        .find(|(k, _)| k == &Value::Integer(KEY_COMPONENTS.into()))
        .and_then(|(_, v)| v.as_map())
        .ok_or_else(|| VerifyError::EvidenceCbor("missing component_results".into()))?;

    let mut component_results = HashMap::new();
    for (k, v) in comp_map {
        let name = k
            .as_text()
            .ok_or_else(|| VerifyError::EvidenceCbor("component key must be text".into()))?
            .to_string();
        let d = v
            .as_integer()
            .and_then(|i| i64::try_from(i).ok())
            .unwrap_or(2);
        let cd = match d {
            0 => ComponentDisposition::Pass,
            1 => ComponentDisposition::Fail,
            _ => ComponentDisposition::Unknown,
        };
        component_results.insert(name, cd);
    }

    Ok(AttestEvidence {
        peer_ueid,
        nonce,
        disposition,
        component_results,
        appraisal_timestamp: ts,
    })
}
