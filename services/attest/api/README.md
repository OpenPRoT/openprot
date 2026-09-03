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
| `src/types.rs` | `Measurement`, `DigestAlgorithm`, `MeasurementAuthority`, `AttestConfig`, `OemId`, `HwSigner` trait, `MeasurementProvider` trait. |
| `src/consts.rs` | Fixed-capacity constants (`MAX_CERT_SIZE`, `MAX_CHAIN_LEN`, etc.). |
| `src/error.rs` | `AttestError` — shared error type for both service crates. |

## Key traits

### `AttestProducer`

The primary interface implemented by `HwAttestProducer` (and the
`SoftwareAttestProducer` stub in the producer crate).

```rust
pub trait AttestProducer: Send + Sync {
    fn generate_token(
        &self,
        nonce: &[u8],
        evidence: &[u8],
        iat: u64,
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError>;

    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
}
```

`evidence` is the raw CBOR output of the verifier service. Pass an empty slice
when no peer attestation has been performed. The bytes are embedded verbatim as
the `concise-evidence` claim (key `-70001`) in the OCP-EAT token. `iat` is a
Unix timestamp (seconds since epoch) supplied by the caller. The encoded token
is appended to the caller-supplied `out` buffer.

### `HwSigner`

Abstracts signing and certificate operations that execute inside the Caliptra
hardware boundary.

```rust
pub trait HwSigner: Send + Sync {
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    fn leaf_cert_der(&self, buf: &mut Vec<u8, MAX_CERT_SIZE>) -> Result<(), AttestError>;
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    fn caliptra_measurements(
        &self,
        out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError>;
}
```

The private Alias Key never leaves Caliptra. Production code implements this
trait via the Caliptra mailbox driver (`caliptra-sw`).

### `MeasurementProvider`

Plug in platform-specific firmware measurement sources (UEFI, BMC, etc.)
beyond the Caliptra-internal measurements.

```rust
pub trait MeasurementProvider: Send + Sync {
    fn component_name(&self) -> &str;
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
| `AttestConfig` | Producer configuration: `oemid`, `hw_model`, `hw_version`, `cert_cache_ttl`. |
| `OemId` | OEM identifier (IANA Private Enterprise Number or UUID form). |

## Cargo

```toml
[dependencies]
openprot-attest-api = { path = "services/attest/api" }
```

No additional features are required. The crate is `no_std` and depends on
`heapless` for fixed-capacity collections and `thiserror` for `AttestError`
derivation.
