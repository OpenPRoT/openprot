// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OCP-EAT COSE_Sign1 token assembly.
//!
//! Token structure (CBOR diagnostic notation):
//!
//! ```text
//! 18(                        ; COSE_Sign1
//!   [
//!     << { 1: -35, 33: [cert-chain...] } >>,   ; protected header
//!     {},                                        ; unprotected header
//!     << 61( { ...CWT claims... } ) >>,          ; payload (EAT CWT)
//!     h'...'                                     ; ES384 signature
//!   ]
//! )
//! ```
//!
//! Claim key numbers follow RFC 9711 and the OCP-EAT profile.

use ciborium::value::Value;
use openprot_attest_api::{
    AttestConfig, AttestError, CaliptraSigner, DigestAlgorithm, Measurement,
};

// Registered EAT claim keys (RFC 9711 / RFC 8392)
const CLAIM_ISS: i64 = 1;
const CLAIM_IAT: i64 = 6;
const CLAIM_NONCE: i64 = 10;
const CLAIM_UEID: i64 = 256;
const CLAIM_OEMID: i64 = 258;
const CLAIM_HWMODEL: i64 = 259;
const CLAIM_HWVER: i64 = 260;
const CLAIM_DBGSTAT: i64 = 263;
const CLAIM_SWNAME: i64 = 14;
const CLAIM_SWVER: i64 = 15;

// OCP-EAT private claim labels
const CLAIM_MEASUREMENTS: i64 = -70000;
const CLAIM_EVIDENCE: i64 = -70001;

// COSE algorithm identifier for ES384
const ALG_ES384: i64 = -35;
// COSE header key for x5chain
const HDR_X5CHAIN: i64 = 33;

/// Build and sign a complete OCP-EAT token.
///
/// `evidence_cbor` is a raw CBOR byte slice from the verifier service.
/// Pass an empty slice when no peer evidence is available.
pub(crate) fn build(
    config: &AttestConfig,
    signer: &dyn CaliptraSigner,
    measurements: &[Measurement],
    nonce: &[u8],
    evidence_cbor: &[u8],
) -> Result<Vec<u8>, AttestError> {
    let chain = signer.cert_chain_der()?;

    let mut claims: Vec<(Value, Value)> = Vec::new();

    claims.push((
        Value::Integer(CLAIM_ISS.into()),
        Value::Text("https://openprot.example/caliptra/device".into()),
    ));

    let iat = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    claims.push((
        Value::Integer(CLAIM_IAT.into()),
        Value::Integer((iat as i64).into()),
    ));

    claims.push((
        Value::Integer(CLAIM_NONCE.into()),
        Value::Bytes(nonce.to_vec()),
    ));

    // UEID: type RAND (0x01 prefix) + 28 placeholder bytes
    let mut ueid = vec![0x01u8];
    ueid.extend_from_slice(&[0u8; 28]);
    claims.push((Value::Integer(CLAIM_UEID.into()), Value::Bytes(ueid)));

    claims.push((
        Value::Integer(CLAIM_OEMID.into()),
        Value::Bytes(config.oemid.0.clone()),
    ));

    claims.push((
        Value::Integer(CLAIM_HWMODEL.into()),
        Value::Text(config.hw_model.clone()),
    ));
    claims.push((
        Value::Integer(CLAIM_HWVER.into()),
        Value::Array(vec![
            Value::Text(config.hw_version.clone()),
            Value::Integer(1i64.into()),
        ]),
    ));

    // dbgstat = 3 (disabled)
    claims.push((
        Value::Integer(CLAIM_DBGSTAT.into()),
        Value::Integer(3i64.into()),
    ));

    let sw_names: Vec<Value> = measurements
        .iter()
        .map(|m| Value::Text(m.component.clone()))
        .collect();
    let sw_vers: Vec<Value> = measurements
        .iter()
        .map(|m| {
            Value::Array(vec![
                Value::Text(m.version.clone()),
                Value::Integer(1i64.into()),
            ])
        })
        .collect();
    claims.push((Value::Integer(CLAIM_SWNAME.into()), Value::Array(sw_names)));
    claims.push((Value::Integer(CLAIM_SWVER.into()), Value::Array(sw_vers)));

    let meas_array: Vec<Value> = measurements
        .iter()
        .map(|m| {
            let alg: i64 = match m.digest_alg {
                DigestAlgorithm::Sha384 => -43,
                DigestAlgorithm::Sha512 => -44,
            };
            Value::Array(vec![
                Value::Text(m.component.clone()),
                Value::Integer(alg.into()),
                Value::Bytes(m.digest.clone()),
            ])
        })
        .collect();
    claims.push((
        Value::Integer(CLAIM_MEASUREMENTS.into()),
        Value::Array(meas_array),
    ));

    // Embed pre-serialized verifier evidence verbatim
    if !evidence_cbor.is_empty() {
        claims.push((
            Value::Integer(CLAIM_EVIDENCE.into()),
            Value::Bytes(evidence_cbor.to_vec()),
        ));
    }

    // CBOR-encode CWT payload
    let mut payload_bytes = Vec::new();
    ciborium::ser::into_writer(&Value::Map(claims), &mut payload_bytes)
        .map_err(|e| AttestError::Cbor(e.to_string()))?;

    // Sign with Caliptra Alias Key
    let sig = signer.sign_es384(&payload_bytes)?;

    // Protected header: { alg: ES384, x5chain: [cert-chain] }
    let protected_header = Value::Map(vec![
        (
            Value::Integer(1i64.into()),
            Value::Integer(ALG_ES384.into()),
        ),
        (
            Value::Integer(HDR_X5CHAIN.into()),
            Value::Array(chain.into_iter().map(Value::Bytes).collect()),
        ),
    ]);
    let mut protected_header_bytes = Vec::new();
    ciborium::ser::into_writer(&protected_header, &mut protected_header_bytes)
        .map_err(|e| AttestError::Cbor(e.to_string()))?;

    // COSE_Sign1 = [protected-bstr, unprotected-map, payload-bstr, sig-bstr]
    let cose_sign1 = Value::Array(vec![
        Value::Bytes(protected_header_bytes),
        Value::Map(vec![]),
        Value::Bytes(payload_bytes),
        Value::Bytes(sig.to_vec()),
    ]);

    let mut out = Vec::new();
    ciborium::ser::into_writer(&cose_sign1, &mut out)
        .map_err(|e| AttestError::Cbor(e.to_string()))?;

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_attest_api::{DigestAlgorithm, MeasurementAuthority, OemId};
    use std::time::Duration;

    struct TestSigner;

    impl CaliptraSigner for TestSigner {
        fn sign_es384(&self, _: &[u8]) -> Result<[u8; 96], AttestError> {
            Ok([0u8; 96])
        }
        fn alias_cert_der(&self) -> Result<Vec<u8>, AttestError> {
            Ok(vec![0x30, 0x00])
        }
        fn cert_chain_der(&self) -> Result<Vec<Vec<u8>>, AttestError> {
            Ok(vec![vec![0x30, 0x00], vec![0x30, 0x00]])
        }
    }

    fn config() -> AttestConfig {
        AttestConfig {
            oemid: OemId(vec![0x00, 0x01, 0x47, 0xae]),
            hw_model: "TestModel".into(),
            hw_version: "1.0.0".into(),
            cert_cache_ttl: Duration::from_secs(3600),
        }
    }

    fn meas() -> Vec<Measurement> {
        vec![Measurement {
            component: "Test ROM".into(),
            version: "1.0.0".into(),
            digest_alg: DigestAlgorithm::Sha384,
            digest: vec![0xAAu8; 48],
            authority: MeasurementAuthority::Caliptra,
        }]
    }

    fn decode_payload(token: &[u8]) -> Vec<(Value, Value)> {
        let outer: Value = ciborium::de::from_reader(token).unwrap();
        let payload_bytes = outer.as_array().unwrap()[2].as_bytes().unwrap().clone();
        let payload: Value = ciborium::de::from_reader(payload_bytes.as_slice()).unwrap();
        payload.as_map().unwrap().clone()
    }

    fn find_claim(map: &[(Value, Value)], key: i64) -> Option<&Value> {
        map.iter()
            .find(|(k, _)| k.as_integer().and_then(|i| i64::try_from(i).ok()) == Some(key))
            .map(|(_, v)| v)
    }

    #[test]
    fn output_is_four_element_cbor_array() {
        let token = build(&config(), &TestSigner, &meas(), b"nonce", &[]).unwrap();
        let value: Value = ciborium::de::from_reader(token.as_slice()).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 4);
    }

    #[test]
    fn nonce_appears_in_payload() {
        let token = build(&config(), &TestSigner, &meas(), b"testnonce", &[]).unwrap();
        let map = decode_payload(&token);
        let v = find_claim(&map, 10).unwrap(); // CLAIM_NONCE = 10
        assert_eq!(v.as_bytes().unwrap(), b"testnonce");
    }

    #[test]
    fn empty_evidence_omits_evidence_claim() {
        let token = build(&config(), &TestSigner, &meas(), b"n", &[]).unwrap();
        let map = decode_payload(&token);
        assert!(find_claim(&map, -70001).is_none());
    }

    #[test]
    fn non_empty_evidence_included_verbatim() {
        let evidence = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let token = build(&config(), &TestSigner, &meas(), b"n", &evidence).unwrap();
        let map = decode_payload(&token);
        let v = find_claim(&map, -70001).unwrap();
        assert_eq!(v.as_bytes().unwrap(), &evidence);
    }

    #[test]
    fn hw_model_in_payload() {
        let token = build(&config(), &TestSigner, &meas(), b"n", &[]).unwrap();
        let map = decode_payload(&token);
        let v = find_claim(&map, 259).unwrap(); // CLAIM_HWMODEL = 259
        assert_eq!(v.as_text().unwrap(), "TestModel");
    }
}
