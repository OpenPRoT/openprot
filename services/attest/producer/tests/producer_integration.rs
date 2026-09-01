// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
// Integration tests for openprot-attest-producer.

#![cfg(feature = "test-support")]

use heapless::Vec;
use openprot_attest_api::consts::{
    MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_HW_MODEL_LEN, MAX_HW_VERSION_LEN, MAX_OEMID_LEN,
    MAX_TOKEN_SIZE,
};
use openprot_attest_api::{AttestConfig, AttestProducer, OemId};
use openprot_attest_producer::SoftwareAttestProducer;

fn config() -> AttestConfig {
    let mut hw_model: heapless::String<MAX_HW_MODEL_LEN> = heapless::String::new();
    hw_model.push_str("TestPlatform").unwrap();
    let mut hw_version: heapless::String<MAX_HW_VERSION_LEN> = heapless::String::new();
    hw_version.push_str("0.1.0").unwrap();
    let mut oemid_bytes: Vec<u8, MAX_OEMID_LEN> = Vec::new();
    oemid_bytes
        .extend_from_slice(&[0x00, 0x01, 0x47, 0xae])
        .unwrap();
    AttestConfig {
        oemid: OemId(oemid_bytes),
        hw_model,
        hw_version,
        cert_cache_ttl: core::time::Duration::from_secs(3600),
    }
}

fn generate(
    producer: &SoftwareAttestProducer,
    nonce: &[u8],
    evidence: &[u8],
) -> Vec<u8, MAX_TOKEN_SIZE> {
    let mut out = Vec::new();
    producer
        .generate_token(nonce, evidence, 0, &mut out)
        .unwrap();
    out
}

// ── Helpers: decode the COSE_Sign1 structure with ciborium (std, test-only) ──

fn decode_outer(token: &[u8]) -> (Vec<u8, 4096>, Vec<u8, 4096>) {
    let outer: ciborium::value::Value = ciborium::de::from_reader(token).unwrap();
    let arr = outer.as_array().unwrap();
    let payload_bytes = arr[2].as_bytes().unwrap().clone();
    let sig_bytes = arr[3].as_bytes().unwrap().clone();
    let mut p: Vec<u8, 4096> = Vec::new();
    p.extend_from_slice(&payload_bytes).unwrap();
    let mut s: Vec<u8, 4096> = Vec::new();
    s.extend_from_slice(&sig_bytes).unwrap();
    (p, s)
}

fn decode_payload(token: &[u8]) -> Vec<(ciborium::value::Value, ciborium::value::Value), 32> {
    let (payload, _) = decode_outer(token);
    let map: ciborium::value::Value = ciborium::de::from_reader(payload.as_slice()).unwrap();
    let mut v = Vec::new();
    for pair in map.as_map().unwrap() {
        v.push(pair.clone()).unwrap();
    }
    v
}

fn find_claim(
    map: &Vec<(ciborium::value::Value, ciborium::value::Value), 32>,
    key: i64,
) -> Option<ciborium::value::Value> {
    map.iter()
        .find(|(k, _)| k.as_integer().and_then(|i| i64::try_from(i).ok()) == Some(key))
        .map(|(_, v)| v.clone())
}

// ── Token structure ──────────────────────────────────────────────────────────

#[test]
fn token_is_four_element_cbor_array() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"testnonce12345678", &[]);
    let value: ciborium::value::Value = ciborium::de::from_reader(token.as_slice()).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 4);
}

#[test]
fn protected_header_is_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let outer: ciborium::value::Value = ciborium::de::from_reader(token.as_slice()).unwrap();
    assert!(outer.as_array().unwrap()[0].as_bytes().is_some());
}

#[test]
fn signature_is_96_zero_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let (_, sig) = decode_outer(&token);
    assert_eq!(sig.len(), 96);
    assert!(sig.iter().all(|&b| b == 0));
}

// ── Payload claims ───────────────────────────────────────────────────────────

#[test]
fn nonce_claim_matches_input() {
    let producer = SoftwareAttestProducer::new(config());
    let nonce = b"unique_nonce_bytes";
    let token = generate(&producer, nonce, &[]);
    let map = decode_payload(&token);
    let v = find_claim(&map, 10).unwrap();
    assert_eq!(v.as_bytes().unwrap(), nonce);
}

#[test]
fn hw_model_claim_matches_config() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload(&token);
    let v = find_claim(&map, 259).unwrap();
    assert_eq!(v.as_text().unwrap(), "TestPlatform");
}

#[test]
fn measurements_claim_contains_three_caliptra_components() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload(&token);
    let v = find_claim(&map, -70000).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
}

// ── Evidence embedding ───────────────────────────────────────────────────────

#[test]
fn empty_evidence_omits_evidence_claim() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload(&token);
    assert!(find_claim(&map, -70001).is_none());
}

#[test]
fn evidence_bytes_embedded_verbatim() {
    let producer = SoftwareAttestProducer::new(config());
    let evidence = [0xDE, 0xAD, 0xBE, 0xEF];
    let token = generate(&producer, b"n", &evidence);
    let map = decode_payload(&token);
    let v = find_claim(&map, -70001).unwrap();
    assert_eq!(v.as_bytes().unwrap(), &evidence);
}

// ── Certificate chain ────────────────────────────────────────────────────────

#[test]
fn cert_chain_returns_two_certs() {
    let producer = SoftwareAttestProducer::new(config());
    let mut buf: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    producer.cert_chain(&mut buf).unwrap();
    assert_eq!(buf.len(), 2);
}
