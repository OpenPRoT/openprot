<!-- SPDX-License-Identifier: Apache-2.0 -->

# openprot-attest-verifier

Host-side OCP-EAT attestation token verifier for OpenPRoT.

Validates COSE_Sign1-wrapped EAT tokens received from peers, appraises
firmware measurements against reference values, and returns structured
`AttestEvidence` for embedding in the platform's own OCP-EAT token.

## Source files

| File | Contents |
|---|---|
| `src/lib.rs` | Public re-exports. |
| `src/verifier/mod.rs` | `Verifier`, `VerifyConfig`, `VerifyError`, `ReferenceValueProvider`, `appraise_measurements`, `Disposition`, `ComponentDisposition`. |
| `src/verifier/eat_verifier.rs` | COSE_Sign1 EAT token validation (structural, nonce, freshness, measurements). |
| `src/verifier/evidence.rs` | `AttestEvidence` struct; CBOR serialization/deserialization. |
| `src/verifier/corim.rs` | `CorimRvp` — Reference Value Provider backed by a CoRIM CBOR file. |
| `tools/openprot-verify/src/main.rs` | CLI tool: verify OCP-EAT tokens against trust anchors. |

## Key types

### `Verifier`

```rust
let verifier = Verifier::new(config)?;
verifier.add_rvp(Box::new(CorimRvp::from_cbor(&corim_bytes)?));

let evidence = verifier.verify_token(&token_bytes, &nonce)?;
```

### `ReferenceValueProvider`

```rust
pub trait ReferenceValueProvider: Send + Sync {
    fn reference_values(&self, component: &str) -> Option<Vec<Measurement>>;
}
```

Implement this trait to supply reference values from any source.
`CorimRvp` provides a CBOR CoRIM-backed implementation.

### `AttestEvidence`

Returned by `verify_token`. Contains:

| Field | Type | Description |
|---|---|---|
| `peer_ueid` | `Vec<u8>` | UEID from the peer's DICE leaf certificate. |
| `nonce` | `Vec<u8>` | Freshness nonce from the token. |
| `disposition` | `Disposition` | `Pass`, `Fail`, or `Indeterminate`. |
| `component_results` | `HashMap<String, ComponentDisposition>` | Per-component outcomes. |
| `appraisal_timestamp` | `u64` | Unix timestamp of appraisal. |

## CoRIM reference values

```rust
let corim_bytes = std::fs::read("platform.corim")?;
let rvp = CorimRvp::from_cbor(&corim_bytes)?;
verifier.add_rvp(Box::new(rvp));
```

CoRIM component names are resolved from class-map key 3 (model text) or
key 5 (index uint, formatted as `spdm-measurement-block-{n}`).

## CLI tool

### `openprot-verify`

```bash
openprot-verify --trust-anchor root.der [--trust-anchor <DER_FILE> ...]
                [--nonce <HEX>] [--max-age <SECS>] [--max-chain-depth <N>]
                [--json] <TOKEN_FILE>
```

Exit codes: 0=Pass, 1=error, 2=Fail, 3=Indeterminate.

## Dependencies

| Crate | Purpose |
|---|---|
| `openprot-attest-api` | `Measurement`, `DigestAlgorithm`, `MeasurementAuthority` types |
| `ciborium` | CBOR decoding for CoRIM, EAT token, and `AttestEvidence` |
| `heapless` | Fixed-capacity buffers for `Measurement` field construction |
| `thiserror` | `VerifyError` derive |

## Cargo build

```bash
cargo build -p openprot-attest-verifier
cargo run --bin openprot-verify -- --help
```

## Bazel targets

```
//attest/verifier:attest_verifier   # library
//attest/verifier:openprot_verify   # CLI verifier
//attest:attest_verifier_all        # all of the above
```
