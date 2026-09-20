<!-- SPDX-License-Identifier: Apache-2.0 -->

# openprot-attest-api

Platform-independent trait and type definitions for the OpenPRoT attestation
producer service.

Callers and other OpenPRoT services depend **only** on this crate. It has no
dependency on the producer implementation, the verifier module, or `spdm-lib`.

## Purpose

This crate defines the stable interface boundary for attestation token
generation. By depending on `openprot-attest-api` rather than
`openprot-attest-producer`, services can be tested with any `AttestProducer`
implementation — including the `SoftwareAttestProducer` stub — without pulling
in hardware dependencies.

## Source files

| File | Contents |
|---|---|
| `src/lib.rs` | Public re-exports. `#![no_std]` `#![forbid(unsafe_code)]`. |
| `src/traits.rs` | `AttestProducer` trait. |
| `src/hw_abstraction.rs` | `HwSigner` trait; `SwSigner` struct with P-384 scalar validation. |
| `src/types.rs` | `Measurement`, `DigestAlgorithm`, `MeasurementAuthority`, `AttestConfig`, `OemId`, `MeasurementProvider` trait, `SignerKind` enum, `SwSignerConfig` struct. |
| `src/consts.rs` | Fixed-capacity constants (`MAX_CERT_SIZE`, `MAX_CHAIN_LEN`, etc.). |
| `src/error.rs` | `AttestError` — shared error type for both service crates. |

## Key traits

### `AttestProducer`

The primary interface implemented by `HwAttestProducer` (and the
`SoftwareAttestProducer` stub in the producer crate).

```rust
pub trait AttestProducer {
    fn generate_token(
        &self,
        nonce: &[u8],
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError>;

    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
}
```

The encoded token is appended to the caller-supplied `out` buffer.

### `HwSigner`

Abstracts signing and certificate operations that execute inside the Caliptra
hardware boundary.

```rust
pub trait HwSigner {
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    fn measurements(
        &self,
        out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError>;
}
```

The private Alias Key never leaves Caliptra. Production code implements this
trait via the Caliptra mailbox driver (`caliptra-sw`).

### `SwSigner`

Holds a caller-supplied P-384 private scalar and DER certificate chain for
software-only signing (no Caliptra hardware required).  Constructed via
`SwSigner::new(SwSignerConfig { ... })`, which validates:

- Scalar is not all zeros (`d ≥ 1`).
- Scalar is less than the P-384 group order (`d < n`).
- Cert chain contains at least one certificate.
- Every certificate begins with `0x30` (DER SEQUENCE tag).

Returns `Err(AttestError::InvalidKey)` on any violation.

### `MeasurementProvider`

Plug in platform-specific firmware measurement sources (UEFI, BMC, etc.)
beyond the Caliptra-internal measurements.

```rust
pub trait MeasurementProvider {
    fn measurements(
        &self,
        out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError>;
}
```

## Key types

| Type | Description |
|---|---|
| `Measurement` | Single firmware measurement: component name, version, digest algorithm, digest bytes, measurement authority. |
| `DigestAlgorithm` | `Sha384` or `Sha512`. |
| `MeasurementAuthority` | `Caliptra` (hardware-measured) or `Platform` (software-registered). |
| `AttestConfig` | Producer configuration: `oemid`, `hw_model`, `cert_cache_ttl`, `signer_kind`. |
| `OemId` | OEM identifier (IANA Private Enterprise Number or UUID form). |
| `SignerKind` | `Hardware` (Caliptra mailbox) or `Software` (caller-supplied key via `SwSigner`). |
| `SwSignerConfig` | Input to `SwSigner::new`: 48-byte P-384 scalar and DER cert chain. |

## Cargo

```toml
[dependencies]
openprot-attest-api = { path = "services/attest/api" }
```

No additional features are required. The crate is `no_std` and depends on
`heapless` for fixed-capacity collections and `thiserror` for `AttestError`
derivation.
