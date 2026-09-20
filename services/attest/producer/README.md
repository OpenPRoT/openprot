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
| `src/signer.rs` | `HwAttestProducer`, `SwAttestProducer`, and `SoftwareAttestProducer` (feature = `test-support`). |
| `src/builder.rs` | Assembles the CBOR claim map and constructs the `COSE_Sign1` envelope. Used by all three producers. |
| `src/cert_ueid.rs` | Minimal DER walker: extracts the TCG UEID (OID 2.23.133.5.4.4) from the DER certificate chain and verifies consistency across all certs that carry it. |
| `src/dice_identity.rs` | Retrieves the DER-encoded DICE certificate chain from the signer and validates it for Caliptra compliance: chain length ≥ 3, X.509 v3, and tcg-dice-MultiTcbInfo extension (OID 2.23.133.5.4.5) present on all non-root certificates. Used by `HwAttestProducer` only. |
| `src/measurements.rs` | Appends platform-registered `MeasurementProvider` outputs into the measurement buffer. |

## Implementations

### `HwAttestProducer` (production, hardware key)

Backed by an `HwSigner` implementation from the Caliptra mailbox driver.
All ES384 signing operations occur inside the Caliptra hardware boundary; the
private Alias Key is never exposed to host software.  Enforces full DICE chain
validation (length ≥ 3, X.509 v3, tcg-dice-MultiTcbInfo on all non-root certs).

```rust
let mut producer = HwAttestProducer::new(&caliptra_driver, config);
producer.add_provider(&uefi_measurements)?;

let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut out)?;
```

### `SwAttestProducer` (production, software key)

Backed by a [`SwSigner`](../api/src/hw_abstraction.rs) holding a caller-supplied
P-384 private scalar and DER certificate chain.  No Caliptra hardware required.
DICE chain validation is skipped (SW-generated certs carry no DICE extensions).
If the leaf certificate carries the TCG UEID extension (OID 2.23.133.5.4.4) it
is extracted; otherwise a zeroed placeholder is used in the token.

```rust
use openprot_attest_api::{SwSigner, SwSignerConfig, SignerKind};
use heapless::Vec;

// Build the cert chain (at least one cert, each starting with 0x30).
let mut chain: Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN> = Vec::new();
// ... push DER-encoded certs ...

let sw_config = SwSignerConfig {
    private_key_scalar: my_48_byte_scalar,
    cert_chain: chain,
};
let signer = SwSigner::new(sw_config)?;  // validates scalar and certs

let config = AttestConfig {
    oemid,
    hw_model,
    cert_cache_ttl,
    signer_kind: SignerKind::Software,
};
let producer = SwAttestProducer::new(signer, config);

let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut out)?;
```

`SwSigner::new` returns `Err(AttestError::InvalidKey)` if the scalar is zero,
≥ the P-384 group order, or any certificate fails the DER SEQUENCE check.

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
profile. The protected header carries the algorithm identifier (`-35` = ES384)
and the `x5chain` certificate chain. The CWT payload includes:

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
| `concise-evidence` | -70001 | OPTIONAL | CBOR-serialized verifier appraisal results (omitted if no peer attestation) |

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
