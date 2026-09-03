// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
// Integration tests for openprot-attest-producer.
// Requires: cargo test --features test-support

#![cfg(feature = "test-support")]

use heapless::{String, Vec};
use openprot_attest_api::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_TOKEN_SIZE};
use openprot_attest_api::{AttestConfig, AttestProducer, OemId};
use openprot_attest_producer::SoftwareAttestProducer;
use std::time::Duration;

fn config() -> AttestConfig {
    let mut hw_model: String<64> = String::new();
    hw_model.push_str("TestPlatform").unwrap();
    let mut hw_version: String<32> = String::new();
    hw_version.push_str("0.1.0").unwrap();
    let mut oemid_bytes: Vec<u8, 16> = Vec::new();
    oemid_bytes
        .extend_from_slice(&[0x00, 0x01, 0x47, 0xae])
        .unwrap();
    AttestConfig {
        oemid: OemId(oemid_bytes),
        hw_model,
        hw_version,
        cert_cache_ttl: Duration::from_secs(3600),
    }
}

fn generate(
    producer: &SoftwareAttestProducer,
    nonce: &[u8],
    evidence: &[u8],
) -> Vec<u8, MAX_TOKEN_SIZE> {
    let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
    producer
        .generate_token(nonce, evidence, 0, &mut out)
        .unwrap();
    out
}

fn unwrap_cose_sign1(token: &[u8]) -> std::vec::Vec<ciborium::value::Value> {
    let outer: ciborium::value::Value = ciborium::de::from_reader(token).unwrap();
    // token = tag(18, [phdr-bstr, uphdr-map, payload-bstr, sig-bstr])
    let (_, inner) = outer.as_tag().unwrap();
    inner.as_array().unwrap().clone()
}

fn decode_payload_map(
    token: &[u8],
) -> std::vec::Vec<(ciborium::value::Value, ciborium::value::Value)> {
    let arr = unwrap_cose_sign1(token);
    let payload_bytes = arr[2].as_bytes().unwrap().clone();
    // payload bytes decode to tag(61, map{...})
    let tagged: ciborium::value::Value =
        ciborium::de::from_reader(payload_bytes.as_slice()).unwrap();
    let (_, map_val) = tagged.as_tag().unwrap();
    map_val.as_map().unwrap().clone()
}

fn find_claim(
    map: &[(ciborium::value::Value, ciborium::value::Value)],
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
    let arr = unwrap_cose_sign1(&token);
    assert_eq!(arr.len(), 4);
}

#[test]
fn protected_header_is_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let arr = unwrap_cose_sign1(&token);
    assert!(arr[0].as_bytes().is_some());
}

#[test]
fn signature_is_96_zero_bytes() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let arr = unwrap_cose_sign1(&token);
    let sig = arr[3].as_bytes().unwrap();
    assert_eq!(sig.len(), 96);
    assert!(sig.iter().all(|&b| b == 0));
}

// ── Payload claims ───────────────────────────────────────────────────────────

#[test]
fn nonce_claim_matches_input() {
    let producer = SoftwareAttestProducer::new(config());
    let nonce = b"unique_nonce_bytes";
    let token = generate(&producer, nonce, &[]);
    let map = decode_payload_map(&token);
    let v = find_claim(&map, 10).unwrap(); // eat_nonce
    assert_eq!(v.as_bytes().unwrap(), nonce);
}

#[test]
fn hw_model_claim_matches_config() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload_map(&token);
    let v = find_claim(&map, 259).unwrap(); // hwmodel
    assert_eq!(v.as_text().unwrap(), "TestPlatform");
}

#[test]
fn measurements_claim_contains_three_caliptra_components() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload_map(&token);
    let v = find_claim(&map, -70000).unwrap(); // measurements
    assert_eq!(v.as_array().unwrap().len(), 3);
}

// ── Evidence embedding ───────────────────────────────────────────────────────

#[test]
fn empty_evidence_omits_evidence_claim() {
    let producer = SoftwareAttestProducer::new(config());
    let token = generate(&producer, b"n", &[]);
    let map = decode_payload_map(&token);
    assert!(find_claim(&map, -70001).is_none());
}

#[test]
fn evidence_bytes_embedded_verbatim() {
    let producer = SoftwareAttestProducer::new(config());
    let evidence = [0xDE, 0xAD, 0xBE, 0xEF];
    let token = generate(&producer, b"n", &evidence);
    let map = decode_payload_map(&token);
    let v = find_claim(&map, -70001).unwrap();
    assert_eq!(v.as_bytes().unwrap(), &evidence);
}

// ── Certificate chain ────────────────────────────────────────────────────────

#[test]
fn cert_chain_returns_two_certs() {
    let producer = SoftwareAttestProducer::new(config());
    let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
    producer.cert_chain(&mut chain).unwrap();
    assert_eq!(chain.len(), 2);
}
