<!-- SPDX-License-Identifier: Apache-2.0 -->

# openprot-attest-producer

Concrete OCP-EAT attestation token producer for OpenPRoT.

Implements the `AttestProducer` trait from `openprot-attest-api` with a
hardware-backed production implementation and a software stub for tests.

## Source files

| File | Contents |
|---|---|
| `src/lib.rs` | Public re-exports; feature gates. |
| `src/signer.rs` | `HwAttestProducer` and `SoftwareAttestProducer` (feature = `test-support`). |
| `src/builder.rs` | Assembles the CBOR claim map and constructs the `COSE_Sign1` envelope. |
| `src/der.rs` | Generic DER/X.509 primitives shared by `cert_ueid` and `dice_identity`. |
| `src/cert_ueid.rs` | Extracts the TCG UEID (OID 2.23.133.5.4.4) from the leaf cert and verifies consistency across the chain. |
| `src/dice_identity.rs` | Retrieves and validates the DICE certificate chain: length ≥ 3, X.509 v3, tcg-dice-MultiTcbInfo on all non-root certs. |
| `src/measurements.rs` | Appends platform-registered `MeasurementProvider` outputs into the measurement buffer. |

## Implementations

### `HwAttestProducer` (production)

Backed by a `Signer` implementation from the platform hardware driver.
All ES384 signing operations occur inside the hardware security boundary; the
private Alias Key is never exposed to host software. Enforces full DICE chain
validation (length ≥ 3, X.509 v3, tcg-dice-MultiTcbInfo on all non-root certs).

```rust
let mut producer = HwAttestProducer::new(&hw_driver, config);
producer.add_provider(&uefi_measurements)?;

let mut out: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut out)?;
```

### `SoftwareAttestProducer` (feature = `test-support`)

Software-only stub for unit and integration tests. Uses a deterministic
all-zero ES384 signature and placeholder DER certificates. No external
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
openprot-attest-producer = { path = "attest/producer", features = ["test-support"] }
```

## Integrating a hardware target

`HwAttestProducer` is hardware-agnostic: it calls three methods on the
platform-supplied `Signer` implementation and does no hardware I/O itself.
To wire up a real hardware security subsystem, implement the `Signer` trait
from `openprot-attest-api`:

```rust
pub trait Signer {
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    fn measurements(&self, out: &mut Vec<Measurement, MAX_MEASUREMENTS>)
        -> Result<(), AttestError>;
}
```

### Caliptra example

Caliptra is a RISC-V Root of Trust (RoT) accessible from host firmware via a
memory-mapped mailbox. The three `Signer` methods map to mailbox commands:

| `Signer` method | Caliptra mailbox command | Notes |
|---|---|---|
| `sign(payload)` | `ECDSA384_SIGN` | Caliptra hashes `payload` internally and signs with the AliasRT private key — the key never leaves the RoT. Returns 96-byte raw r‖s. |
| `cert_chain_der(buf)` | `GET_IDEV_CERT` / `CERTIFY_KEY` | Retrieve the full DICE certificate chain (VendorCA → IDevID → LDevID → AliasFMC → AliasRT). |
| `measurements(out)` | `FW_INFO` / `GET_MEASUREMENT` | Read Caliptra's RT journey measurements (ROM, FMC, runtime firmware digests). |

A minimal implementation using a Caliptra mailbox driver crate:

```rust
use openprot_attest_api::{AttestError, Measurement, Signer};

pub struct CaliptraSigner<'a> {
    mailbox: &'a CaliptraMailbox,
}

impl<'a> CaliptraSigner<'a> {
    pub fn new(mailbox: &'a CaliptraMailbox) -> Self {
        Self { mailbox }
    }
}

impl Signer for CaliptraSigner<'_> {
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError> {
        // Caliptra accepts the raw payload and performs SHA-384 + ECDSA internally.
        self.mailbox
            .ecdsa384_sign(payload)
            .map_err(|_| AttestError::Mailbox("ECDSA384_SIGN failed"))
    }

    fn cert_chain_der(
        &self,
        buf: &mut heapless::Vec<heapless::Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError> {
        // GET_IDEV_CERT returns the full DICE chain in one response on recent
        // Caliptra firmware; older firmware may require GET_IDEV_CERT +
        // CERTIFY_KEY calls to reconstruct it.
        let chain = self
            .mailbox
            .get_idev_cert()
            .map_err(|_| AttestError::Mailbox("GET_IDEV_CERT failed"))?;
        buf.clear();
        for der in chain {
            buf.push(der).map_err(|_| AttestError::BufferFull)?;
        }
        Ok(())
    }

    fn measurements(
        &self,
        out: &mut heapless::Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError> {
        // FW_INFO returns ROM, FMC, and runtime measurement digests.
        let info = self
            .mailbox
            .fw_info()
            .map_err(|_| AttestError::Mailbox("FW_INFO failed"))?;
        for m in info.measurements() {
            out.push(m).map_err(|_| AttestError::BufferFull)?;
        }
        Ok(())
    }
}
```

Wiring it into the producer at platform initialisation:

```rust
let signer = CaliptraSigner::new(&CALIPTRA_MAILBOX);
let mut producer = HwAttestProducer::new(&signer, attest_config);

// Optionally register additional measurement providers (UEFI, BMC, etc.).
producer.add_provider(&uefi_measurements)?;

let mut token: Vec<u8, MAX_TOKEN_SIZE> = Vec::new();
producer.generate_token(&nonce, &mut token)?;
```

### Extending to other hardware targets

Any platform RoT that exposes ES384 signing and DICE certificate retrieval can
implement `Signer`. The mailbox protocol, register layout, and driver API are
entirely within the `Signer` impl; the producer library is unchanged. Platforms
without a DICE chain can implement `cert_chain_der` to return a single
self-signed attestation certificate and set `AttestConfig::ueid_check = false`
to omit the UEID binding.

## Token structure

`builder.rs` produces a `COSE_Sign1`-wrapped CWT conforming to the OCP-EAT
profile. The algorithm identifier (`-35` = ES384) is in the protected header.
The `x5chain` certificate chain is in the **unprotected** header (key 33, not
covered by the signature, per the OCP-EAT profile).

Claims are written in CBOR deterministic encoding order (RFC 8949 §4.2.1).

| Claim | Key | Presence | Description |
|---|---|---|---|
| `eat_nonce` | 10 | always | Caller-supplied freshness nonce (8–64 bytes). |
| `ueid` | 256 | when `AttestConfig.ueid_check = true` | Device UEID from the TCG UEID extension in the leaf cert. |
| `oemid` | 258 | always | OEM identifier (IANA PEN form). |
| `hwmodel` | 259 | always | Hardware model string. |
| `dbgstat` | 263 | always | Debug status (hardcoded `3` = disabled). |
| `eat_profile` | 265 | always | OCP EAT profile OID `1.3.6.1.4.1.42623.1.3`. |
| `measurements` | 273 | always | Per-component firmware measurement records. |

The CWT payload is wrapped as `tag(55799, tag(61, map))` per the OCP profile
requirement for self-described CBOR.

## Dependencies

| Crate | Purpose |
|---|---|
| `openprot-attest-api` | Trait and type definitions (`AttestProducer`, `Signer`, `AttestError`, etc.) |
| `minicbor` | `no_std` CBOR encoding of token claims and `COSE_Sign1` envelope |

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
//attest/producer:attest_producer                   # production library
//attest/producer:attest_producer_unit_test         # unit tests
//attest/producer:attest_producer_integration_test  # integration tests
```
