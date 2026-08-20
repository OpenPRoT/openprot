<!-- SPDX-License-Identifier: Apache-2.0 -->

# openprot-attest-producer

Concrete OCP-EAT attestation token producer for OpenPRoT.

Implements the `AttestProducer` trait from `openprot-attest-api` with two
backends: a hardware-backed producer that calls the Caliptra mailbox driver,
and a software stub for testing without physical hardware.

## Source files

| File | Contents |
|---|---|
| `src/lib.rs` | Public re-exports; feature gates. |
| `src/signer.rs` | `HwAttestProducer` and `SoftwareAttestProducer` (feature = `test-support`). |
| `src/builder.rs` | Assembles the CBOR claim map, embeds raw evidence bytes, constructs the `COSE_Sign1` envelope. No verifier dependency. |
| `src/dice_identity.rs` | Wraps the Caliptra mailbox calls that return the DER-encoded DICE certificate chain. |
| `src/measurements.rs` | Aggregates Caliptra-internal firmware measurements with platform-registered `MeasurementProvider` outputs. |

## Implementations

### `HwAttestProducer` (production)

Backed by a `CaliptraSigner` implementation from the Caliptra mailbox driver.
All ES384 signing operations occur inside the Caliptra hardware boundary; the
private Alias Key is never exposed to host software.

```rust
let producer = HwAttestProducer::new(Arc::new(caliptra_driver), config);
producer.add_provider(Box::new(UefiFirmwareMeasurements));

let token: Vec<u8> = producer.generate_token(&nonce, &evidence_cbor)?;
```

### `SoftwareAttestProducer` (feature = `test-support`)

Software-only stub for unit and integration tests. Uses a deterministic
all-zero ES384 signature and placeholder DER certificates. No Caliptra
hardware or driver is required.

```rust
#[cfg(feature = "test-support")]
let producer = SoftwareAttestProducer::new(config);
let token = producer.generate_token(&nonce, &[])?;
```

Enable the feature in `Cargo.toml`:

```toml
[dev-dependencies]
openprot-attest-producer = { path = "services/attest/producer", features = ["test-support"] }
```

## Token structure

`builder.rs` produces a `COSE_Sign1`-wrapped CWT conforming to the OCP-EAT
profile. The protected header carries the algorithm identifier (`-35` = ES384)
and the `x5chain` certificate chain. The CWT payload includes:

| Claim | Key | Description |
|---|---|---|
| `iss` | 1 | Issuer derived from Caliptra device identity |
| `iat` | 6 | Token creation timestamp |
| `eat_nonce` | 10 | Caller-supplied freshness nonce (min 32 bytes) |
| `ueid` | 256 | Unique Entity Identifier from Caliptra CDI |
| `oemid` | 258 | OEM identifier (IANA PEN form) |
| `hwmodel` | 259 | Hardware model string |
| `hwversion` | 260 | Hardware version string |
| `dbgstat` | 263 | Debug status |
| `measurements` | -70000 | Per-component firmware measurement records |
| `concise-evidence` | -70001 | CBOR-serialized verifier appraisal results (omitted if no peer attestation) |

## Dependencies

| Crate | Purpose |
|---|---|
| `openprot-attest-api` | Trait and type definitions (`AttestProducer`, `CaliptraSigner`, etc.) |
| `ciborium` | CBOR encoding of token claims and evidence embedding |
| `thiserror` | `AttestError` derivation |
| `zeroize` | Zero-on-drop for intermediate key material and sensitive buffers |

## Cargo build

```bash
# Production build
cargo build -p openprot-attest-producer

# With software stub
cargo build -p openprot-attest-producer --features test-support

# Tests (software stub required)
cargo test -p openprot-attest-producer --features test-support
```

## Bazel targets

```
//services/attest/producer:attest_producer               # production library
//services/attest/producer:attest_producer_unit_test     # unit tests
//services/attest/producer:attest_producer_integration_test  # integration tests
```
