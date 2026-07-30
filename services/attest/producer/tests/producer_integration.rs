// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
// Integration tests for openprot-attest-producer.
// Requires: cargo test --features test-support

#![cfg(feature = "test-support")]

use openprot_attest_api::{AttestConfig, AttestProducer, OemId};
use openprot_attest_producer::SoftwareAttestProducer;
use std::time::Duration;

fn config() -> AttestConfig {
    AttestConfig {
        oemid: OemId(vec![0x00, 0x01, 0x47, 0xae]),
        hw_model: "TestPlatform".into(),
        hw_version: "0.1.0".into(),
        cert_cache_ttl: Duration::from_secs(3600),
    }
}

fn decode_payload_map(
    token: &[u8],
) -> Vec<(ciborium::value::Value, ciborium::value::Value)> {
    let outer: ciborium::value::Value =
        ciborium::de::from_reader(token).unwrap();
    let payload_bytes =
        outer.as_array().unwrap()[2].as_bytes().unwrap().clone();
    let payload: ciborium::value::Value =
        ciborium::de::from_reader(payload_bytes.as_slice()).unwrap();
    payload.as_map().unwrap().clone()
}

fn find_claim(
    map: &[(ciborium::value::Value, ciborium::value::Value)],
    key: i64,
) -> Option<ciborium::value::Value> {
    map.iter()
        .find(|(k, _)| {
            k.as_integer().and_then(|i| i64::try_from(i).ok()) == Some(key)
        })
        .map(|(_, v)| v.clone())
}

// ── Token structure ──────────────────────────────────────────────────────────

#[test]
fn token_is_four_element_cbor_array() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"testnonce12345678", &[]).unwrap();
    let value: ciborium::value::Value =
        ciborium::de::from_reader(token.as_slice()).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 4);
}

#[test]
fn protected_header_is_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"n", &[]).unwrap();
    let outer: ciborium::value::Value =
        ciborium::de::from_reader(token.as_slice()).unwrap();
    assert!(outer.as_array().unwrap()[0].as_bytes().is_some());
}

#[test]
fn signature_is_96_zero_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"n", &[]).unwrap();
    let outer: ciborium::value::Value =
        ciborium::de::from_reader(token.as_slice()).unwrap();
    let sig = outer.as_array().unwrap()[3].as_bytes().unwrap();
    assert_eq!(sig.len(), 96);
    assert!(sig.iter().all(|&b| b == 0));
}

// ── Payload claims ───────────────────────────────────────────────────────────

#[test]
fn nonce_claim_matches_input() {
    let producer = SoftwareAttestProducer::new(config());
    let nonce = b"unique_nonce_bytes";
    let token = producer.generate_token(nonce, &[]).unwrap();
    let map = decode_payload_map(&token);
    let v = find_claim(&map, 10).unwrap(); // eat_nonce
    assert_eq!(v.as_bytes().unwrap(), nonce);
}

#[test]
fn hw_model_claim_matches_config() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"n", &[]).unwrap();
    let map = decode_payload_map(&token);
    let v = find_claim(&map, 259).unwrap(); // hwmodel
    assert_eq!(v.as_text().unwrap(), "TestPlatform");
}

#[test]
fn measurements_claim_contains_three_caliptra_components() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"n", &[]).unwrap();
    let map = decode_payload_map(&token);
    let v = find_claim(&map, -70000).unwrap(); // measurements
    assert_eq!(v.as_array().unwrap().len(), 3);
}

// ── Evidence embedding ───────────────────────────────────────────────────────

#[test]
fn empty_evidence_omits_evidence_claim() {
    let producer = SoftwareAttestProducer::new(config());
    let token = producer.generate_token(b"n", &[]).unwrap();
    let map = decode_payload_map(&token);
    assert!(find_claim(&map, -70001).is_none());
}

#[test]
fn evidence_bytes_embedded_verbatim() {
    let producer = SoftwareAttestProducer::new(config());
    let evidence = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let token = producer.generate_token(b"n", &evidence).unwrap();
    let map = decode_payload_map(&token);
    let v = find_claim(&map, -70001).unwrap();
    assert_eq!(v.as_bytes().unwrap(), &evidence);
}

// ── Certificate chain ────────────────────────────────────────────────────────

#[test]
fn cert_chain_returns_two_certs() {
    let producer = SoftwareAttestProducer::new(config());
    let chain = producer.cert_chain().unwrap();
    assert_eq!(chain.0.len(), 2);
}
