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
| `src/signing_abstraction.rs` | `HwSigner` trait; `SwSigner` struct with P-384 key validation and ECDSA signing. |
| `src/types.rs` | `Measurement`, `DigestAlgorithm`, `MeasurementAuthority`, `AttestConfig`, `OemId`, `MeasurementProvider` trait, `SwSignerConfig` struct. |
| `src/consts.rs` | Fixed-capacity constants (`MAX_CERT_SIZE`, `MAX_CHAIN_LEN`, etc.). |
| `src/error.rs` | `AttestError` — shared error type for both service crates. |

## Key traits

### `AttestProducer`

The primary interface implemented by `HwAttestProducer`, `SwAttestProducer`,
and the `SoftwareAttestProducer` stub in the producer crate.

```rust
pub trait AttestProducer {
    fn generate_token(
        &self,
        nonce: &[u8],
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError>;

    /// `buf` is cleared before being populated with the chain (leaf → root).
    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
}
```

### `HwSigner`

Abstracts signing and certificate operations backed by the Caliptra hardware
boundary.

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

Holds a caller-supplied P-384 private key and DER certificate chain for
software signing (no Caliptra hardware required). Constructed via
`SwSigner::new(SwSignerConfig { ... })`, which validates:

- Scalar is a valid P-384 private key (`1 ≤ d < n`), via `p384::ecdsa::SigningKey::from_bytes`.
- Cert chain contains at least one certificate.
- Every certificate begins with `0x30` (DER SEQUENCE tag).

Returns `Err(AttestError::InvalidKey)` on any violation. The key is stored as
a `SigningKey` which zeroizes on drop. `SwSignerConfig` also zeroizes the raw
scalar on drop.

`sign()` produces a real ECDSA P-384 signature (SHA-384 prehash, RFC 6979
deterministic nonce) over the COSE `Sig_Structure`.

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
| `AttestConfig` | Producer configuration: `oemid`, `hw_model`. |
| `OemId` | OEM identifier (IANA Private Enterprise Number or UUID form). |
| `SwSignerConfig` | Input to `SwSigner::new`: 48-byte P-384 scalar and DER cert chain. Zeroizes scalar on drop. |

## Error variants

| Variant | Meaning |
|---|---|
| `Mailbox` | Caliptra mailbox communication failure. |
| `Der` | DER parse error (malformed certificate structure). |
| `ChainValidation` | DICE chain structural or compliance failure. |
| `InvalidNonce` | Nonce length outside the 8–64 byte range. |
| `Cbor` | CBOR encoding error. |
| `BufferFull` | Fixed-size buffer capacity exceeded. |
| `Cose` | COSE signing error. |
| `Provider` | Measurement provider error. |
| `InvalidKey` | Invalid P-384 key material supplied to `SwSigner::new`. |

## Cargo

```toml
[dependencies]
openprot-attest-api = { path = "services/attest/api" }
```

The crate is `no_std` and depends on `heapless` for fixed-capacity
collections, `thiserror` for `AttestError`, `p384` + `sha2` for ECDSA
signing, and `zeroize` for key material cleanup.
