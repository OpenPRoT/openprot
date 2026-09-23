<!-- SPDX-License-Identifier: Apache-2.0 -->

# attest service

OCP-EAT attestation token producer for OpenPRoT.

## Crates

| Crate | Path | Purpose |
|---|---|---|
| `openprot-attest-api` | `api/` | Platform-independent traits, types, and error definitions. Callers depend only on this crate. |
| `openprot-attest-producer` | `producer/` | Concrete `AttestProducer` implementations (`HwAttestProducer`, `SwAttestProducer`, `SoftwareAttestProducer`). |

## Dependency structure

```
application / verifier service
    └── openprot-attest-api   (traits + types only)
            └── openprot-attest-producer  (production implementations)
```

Platform code selects an implementation at construction time:

- **`HwAttestProducer`** — backed by the Caliptra mailbox driver; all signing inside the hardware boundary.
- **`SwAttestProducer`** — backed by a caller-supplied P-384 key (`SwSigner`); no Caliptra required.
- **`SoftwareAttestProducer`** — software stub for tests (feature = `test-support`).

See each crate's README for API details and usage examples.
