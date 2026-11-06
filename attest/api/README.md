<!-- SPDX-License-Identifier: Apache-2.0 -->

# openprot-attest-api

Platform-independent trait and type definitions for the OpenPRoT attestation
producer.

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
| `src/signing_abstraction.rs` | `Signer` trait — hardware-backed signing abstraction. |
| `src/types.rs` | `Measurement`, `DigestAlgorithm`, `MeasurementAuthority`, `AttestConfig`, `OemId`, `MeasurementProvider` trait. |
| `src/consts.rs` | Fixed-capacity constants (`MAX_CERT_SIZE`, `MAX_CHAIN_LEN`, etc.). |
| `src/error.rs` | `AttestError` — shared error type for both crates. |

## Key traits

### `AttestProducer`

The primary interface implemented by `HwAttestProducer` and the
`SoftwareAttestProducer` stub in the producer crate.

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

### `Signer`

Abstracts signing and certificate operations backed by an external hardware
security boundary. The private Alias Key never leaves the hardware. Production
code implements this trait via the platform hardware driver.

```rust
pub trait Signer {
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

### `MeasurementProvider`

Plug in platform-specific firmware measurement sources (UEFI, BMC, etc.)
beyond the hardware-internal measurements.

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
| `AttestConfig` | Producer configuration: `oemid`, `hw_model`, `ueid_check`. |
| `OemId` | OEM identifier (IANA Private Enterprise Number or UUID form). |

## `AttestConfig` fields

| Field | Type | Description |
|---|---|---|
| `oemid` | `OemId` | OEM identifier included in the token as the `oemid` claim (key 258). |
| `hw_model` | `String<MAX_HW_MODEL_LEN>` | Hardware model string included as the `hwmodel` claim (key 259). |
| `ueid_check` | `bool` | Controls whether the TCG UEID extension is extracted and included in the token (see below). |

### `ueid_check`

When `true`, `generate_token` extracts the TCG UEID extension
(OID 2.23.133.5.4.4) from the leaf certificate, verifies that every other
certificate in the chain that carries the extension has the same value, and
includes the result as the `ueid` claim (EAT key 256) in the token.
A leaf certificate that lacks the extension, or a chain with mismatched UEID
values, causes `generate_token` to return `Err(AttestError::ChainValidation)`.

When `false`, the check is skipped entirely and the `ueid` claim is omitted
from the token. Use this when the platform certificate chain does not carry the
TCG UEID extension.

```rust
// UEID extracted from cert chain and included in token
let config = AttestConfig {
    oemid: OemId(oemid_bytes),
    hw_model,
    ueid_check: true,
};

// UEID claim omitted; cert chain not required to carry TCG UEID extension
let config = AttestConfig {
    oemid: OemId(oemid_bytes),
    hw_model,
    ueid_check: false,
};
```

## Error variants

| Variant | Meaning |
|---|---|
| `Mailbox` | Hardware mailbox communication failure. |
| `Der` | DER parse error (malformed certificate structure). |
| `ChainValidation` | DICE chain structural or compliance failure. |
| `InvalidNonce` | Nonce length outside the 8–64 byte range. |
| `Cbor` | CBOR encoding error. |
| `BufferFull` | Fixed-size buffer capacity exceeded. |
| `Cose` | COSE signing error. |
| `Provider` | Measurement provider error. |

## Cargo

```toml
[dependencies]
openprot-attest-api = { path = "attest/api" }
```

The crate is `no_std` and depends only on `heapless` and `thiserror`.
