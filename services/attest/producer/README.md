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
| `src/builder.rs` | Assembles the CBOR claim map and constructs the `COSE_Sign1` envelope. No verifier dependency. |
| `src/cert_ueid.rs` | Minimal DER walker: extracts the TCG UEID (OID 2.23.133.5.4.4) from the Caliptra DER certificate chain and verifies consistency across all certs that carry it. |
| `src/dice_identity.rs` | Retrieves the DER-encoded DICE certificate chain from the signer and validates it for Caliptra compliance: chain length ≥ 3, X.509 v3, and tcg-dice-MultiTcbInfo extension (OID 2.23.133.5.4.5) present on all non-root certificates. |
| `src/measurements.rs` | Appends platform-registered `MeasurementProvider` outputs into the measurement buffer. Caliptra-internal measurements are written directly by `HwSigner::caliptra_measurements` before `collect()` is called. |

## Implementations

### `HwAttestProducer` (production)

Backed by an `HwSigner` implementation from the Caliptra mailbox driver.
All ES384 signing operations occur inside the Caliptra hardware boundary; the
private Alias Key is never exposed to host software.

```rust
let mut producer = HwAttestProducer::new(&caliptra_driver, config);
producer.add_provider(&uefi_measurements)?;

let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut out)?;
```

### `SoftwareAttestProducer` (feature = `test-support`)

Software-only stub for unit and integration tests. Uses a deterministic
all-zero ES384 signature and placeholder DER certificates. No Caliptra
hardware or driver is required.

```rust
#[cfg(feature = "test-support")]
let producer = SoftwareAttestProducer::new(config);
let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut out)?;
```

Enable the feature in `Cargo.toml`:

```toml
[dev-dependencies]
openprot-attest-producer = { path = "services/attest/producer", features = ["test-support"] }
```

## Token structure

`builder.rs` produces a `COSE_Sign1`-wrapped CWT conforming to the OCP-EAT
profile. The protected header carries the algorithm identifier (`-35` = ES384).
The CWT payload includes:

Only claims explicitly defined in the OCP-EAT profile are included.
Claims are written in CBOR deterministic encoding order (RFC 8949 §4.2.1).

| Claim | Key | Required | Description |
|---|---|---|---|
| `eat_nonce` | 10 | MUST | Caller-supplied freshness nonce (8–64 bytes) |
| `ueid` | 256 | OPTIONAL | Device UEID extracted from the TCG UEID extension (OID 2.23.133.5.4.4) in the Caliptra AliasRT certificate |
| `oemid` | 258 | OPTIONAL | OEM identifier (IANA PEN form) |
| `hwmodel` | 259 | OPTIONAL | Hardware model string |
| `dbgstat` | 263 | MUST | Debug status (hardcoded 3 = disabled) |
| `eat_profile` | 265 | MUST | OCP EAT profile OID `1.3.6.1.4.1.42623.1.3` |
| `measurements` | 273 | MUST | Per-component firmware measurement records |

The CWT payload is wrapped as `tag(55799, tag(61, map))` per the OCP profile requirement for self-described CBOR. The x5chain certificate chain is in the **unprotected** COSE header (key 33); only the algorithm identifier is in the protected header.

## Dependencies

| Crate | Purpose |
|---|---|
| `openprot-attest-api` | Trait and type definitions (`AttestProducer`, `HwSigner`, etc.) |
| `minicbor` | CBOR encoding of token claims and COSE_Sign1 envelope |

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
