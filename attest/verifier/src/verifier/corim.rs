// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! CoRIM (Concise Reference Integrity Manifest) reference value provider.
//!
//! Parses a CBOR-encoded CoRIM file and exposes its reference values through
//! the [`ReferenceValueProvider`] trait so they can be used by the verifier's
//! appraisal engine.
//!
//! # Supported CoRIM subset
//!
//! Only the reference-value triple path is parsed:
//!
//! ```text
//! corim-map {
//!   1: [ comid-map {          ; comid list
//!     4: {                    ; triples-map
//!       1: [                  ; reference-value-triples
//!         [                   ; reference-triple-record
//!           environment-map,  ; identifies the component
//!           [measurement-map] ; holds digests
//!         ]
//!       ]
//!     }
//!   }]
//! }
//! ```
//!
//! CBOR tags 501 (tagged-corim) and 505 (tagged-comid) are stripped
//! automatically; both tagged and untagged forms are accepted.
//!
//! ## Component name extraction
//!
//! From the environment-map → class-map:
//! - Key 3 (`model`, text) is used directly as the component name.
//! - Key 5 (`index`, uint) is formatted as `"spdm-measurement-block-{n}"`.
//!
//! ## Digest algorithm mapping (COSE algorithm IDs)
//!
//! | CoRIM alg ID | Algorithm |
//! |---|---|
//! | `-43` | SHA-384 |
//! | `-44` | SHA-512 |
//!
//! Entries with unknown algorithm IDs are silently skipped.

use std::collections::HashMap;

use ciborium::value::Value;

use crate::verifier::ReferenceValueProvider;
use openprot_attest_api::{
    consts::{MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_VERSION_LEN},
    DigestAlgorithm, Measurement, MeasurementAuthority,
};

// ── CorimRvp ──────────────────────────────────────────────────────────────

/// Reference Value Provider backed by a CoRIM CBOR file.
///
/// Construct with [`CorimRvp::from_cbor`], then register with a
/// [`crate::verifier::Verifier`] via [`crate::verifier::Verifier::add_rvp`].
pub struct CorimRvp {
    /// Map from component name to its list of accepted (alg, digest) pairs.
    values: HashMap<String, Vec<(DigestAlgorithm, Vec<u8>)>>,
}

impl CorimRvp {
    /// Parse a CBOR-encoded CoRIM byte buffer and extract reference values.
    ///
    /// Returns an error string if the bytes are not valid CBOR or the
    /// top-level CoRIM structure is malformed.  Unknown or unsupported
    /// inner structures are skipped without error.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, String> {
        let top: Value = ciborium::de::from_reader(bytes)
            .map_err(|e| format!("CoRIM CBOR decode error: {e}"))?;

        let corim_map = unwrap_tag(top);
        let corim_entries = as_map(&corim_map).ok_or("CoRIM top level must be a CBOR map")?;

        // Key 1 = comid list
        let comid_list = map_get(corim_entries, 1)
            .and_then(|v| v.as_array())
            .ok_or("CoRIM missing comid list (key 1)")?;

        let mut values: HashMap<String, Vec<(DigestAlgorithm, Vec<u8>)>> = HashMap::new();

        for comid_val in comid_list {
            let comid_map = unwrap_tag(comid_val.clone());
            let comid_entries = match as_map(&comid_map) {
                Some(e) => e,
                None => continue,
            };

            // Key 4 = triples-map
            let triples_map_val = match map_get(comid_entries, 4) {
                Some(v) => v,
                None => continue,
            };
            let triples_entries = match as_map(triples_map_val) {
                Some(e) => e,
                None => continue,
            };

            // Key 1 = reference-value-triples
            let ref_triples = match map_get(triples_entries, 1).and_then(|v| v.as_array()) {
                Some(a) => a,
                None => continue,
            };

            for triple in ref_triples {
                let record = match triple.as_array().filter(|a| a.len() >= 2) {
                    Some(a) => a,
                    None => continue,
                };

                let env_map_val = &record[0];
                let meas_list_val = &record[1];

                let component_name = match extract_component_name(env_map_val) {
                    Some(n) => n,
                    None => continue,
                };

                let meas_list = match meas_list_val.as_array() {
                    Some(a) => a,
                    None => continue,
                };

                for meas_map_val in meas_list {
                    let meas_entries = match as_map(meas_map_val) {
                        Some(e) => e,
                        None => continue,
                    };

                    // Key 1 = mval-map
                    let mval_val = match map_get(meas_entries, 1) {
                        Some(v) => v,
                        None => continue,
                    };
                    let mval_entries = match as_map(mval_val) {
                        Some(e) => e,
                        None => continue,
                    };

                    // Key 2 = digests: [[alg-id, bytes], ...]
                    let digests = match map_get(mval_entries, 2).and_then(|v| v.as_array()) {
                        Some(a) => a,
                        None => continue,
                    };

                    for digest_entry in digests {
                        let pair = match digest_entry.as_array().filter(|a| a.len() >= 2) {
                            Some(a) => a,
                            None => continue,
                        };

                        let alg_id = match pair[0].as_integer().and_then(|i| i64::try_from(i).ok())
                        {
                            Some(i) => i,
                            None => continue,
                        };
                        let digest_alg = match alg_id {
                            -43 => DigestAlgorithm::Sha384,
                            -44 => DigestAlgorithm::Sha512,
                            _ => continue, // unsupported algorithm — skip silently
                        };
                        let digest_bytes = match pair[1].as_bytes() {
                            Some(b) => b.clone(),
                            None => continue,
                        };

                        values
                            .entry(component_name.clone())
                            .or_default()
                            .push((digest_alg, digest_bytes));
                    }
                }
            }
        }

        Ok(Self { values })
    }

    /// Number of components with reference values loaded from this CoRIM.
    pub fn component_count(&self) -> usize {
        self.values.len()
    }
}

impl ReferenceValueProvider for CorimRvp {
    fn reference_values(&self, component: &str) -> Option<std::vec::Vec<Measurement>> {
        let entries = self.values.get(component)?;
        let measurements = entries
            .iter()
            .filter_map(|(alg, digest)| {
                let comp = heapless::String::<MAX_COMPONENT_LEN>::try_from(component).ok()?;
                let ver = heapless::String::<MAX_VERSION_LEN>::new();
                let mut dig = heapless::Vec::<u8, MAX_DIGEST_LEN>::new();
                dig.extend_from_slice(digest).ok()?;
                Some(Measurement {
                    component: comp,
                    version: ver,
                    digest_alg: *alg,
                    digest: dig,
                    authority: MeasurementAuthority::Caliptra,
                })
            })
            .collect();
        Some(measurements)
    }
}

// ── CBOR helpers ──────────────────────────────────────────────────────────

/// Strip CBOR tag wrapper (tag 501 = tagged-corim, tag 505 = tagged-comid,
/// or any other tag), returning the inner value.
fn unwrap_tag(v: Value) -> Value {
    match v {
        Value::Tag(_, inner) => *inner,
        other => other,
    }
}

/// Borrow the inner entries of a CBOR map value.
fn as_map(v: &Value) -> Option<&[(Value, Value)]> {
    v.as_map().map(|m| m.as_slice())
}

/// Look up an integer key in a CBOR map, returning a reference to the value.
fn map_get(entries: &[(Value, Value)], key: i64) -> Option<&Value> {
    entries
        .iter()
        .find(|(k, _)| k == &Value::Integer(key.into()))
        .map(|(_, v)| v)
}

/// Extract a component name from an environment-map CBOR value.
///
/// Checks class-map key 3 (model, text) first, then key 5 (index, uint).
fn extract_component_name(env_val: &Value) -> Option<String> {
    let env_entries = as_map(env_val)?;

    // Key 1 = class-map
    let class_val = map_get(env_entries, 1)?;
    let class_entries = as_map(class_val)?;

    // Prefer key 3 (model text) as the component name.
    if let Some(model) = map_get(class_entries, 3).and_then(|v| v.as_text()) {
        return Some(model.to_string());
    }

    // Fall back to key 5 (index uint) → "spdm-measurement-block-{n}".
    if let Some(idx) = map_get(class_entries, 5)
        .and_then(|v| v.as_integer())
        .and_then(|i| u64::try_from(i).ok())
    {
        return Some(format!("spdm-measurement-block-{idx}"));
    }

    None
}
