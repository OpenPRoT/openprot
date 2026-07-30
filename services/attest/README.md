# services/attest

Attestation producer service for the OpenPRoT Platform Root of Trust.

This directory contains two Cargo crates and the Bazel build targets that
deliver OCP-EAT attestation token generation to the rest of the OpenPRoT
firmware stack.

## Crates

| Crate | Path | Role |
|---|---|---|
| `openprot-attest-api` | `api/` | Platform-independent traits and types. Callers depend on this crate only. |
| `openprot-attest-producer` | `producer/` | Concrete token producer backed by Caliptra hardware or a software stub. |

Neither crate depends on the verifier module or `spdm-lib`. Evidence from the
verifier is accepted as a raw CBOR byte slice (`&[u8]`), keeping the producer
decoupled from verifier internals.

## Bazel targets

```
//services/attest:attest_embedded_all   # production filegroup (api + producer)
//services/attest:attest_host_tests     # test_suite for all host-side tests
```

## Cargo build

```bash
# From the workspace root (~/openprot_attestation/)

# API crate only
cargo build -p openprot-attest-api

# Producer crate (pulls in API automatically)
cargo build -p openprot-attest-producer

# Producer with software stub enabled (no Caliptra hardware required)
cargo build -p openprot-attest-producer --features test-support

# Entire workspace
cargo build
```

## Testing

```bash
cargo test -p openprot-attest-producer --features test-support
cargo test --features test-support
```

## Relationship to the verifier

The attester and verifier are deliberately decoupled. The verifier
(`attestation/src/verifier/`) produces a CBOR-serialized `AttestEvidence`
value that is passed to `AttestProducer::generate_token` as the `evidence`
byte slice. The producer embeds it verbatim as the `concise-evidence` claim
(key `-70001`) in the outgoing OCP-EAT token. The outer `COSE_Sign1`
signature covers the embedded evidence.

## Standards

- OCP Entity Attestation Token — <https://opencomputeproject.github.io/Security/ietf-eat-profile/HEAD/>
- IETF EAT (RFC 9711) — <https://www.rfc-editor.org/rfc/rfc9711>
- CBOR Web Token (RFC 8392) — <https://www.rfc-editor.org/rfc/rfc8392>
- COSE (RFC 9052) — <https://www.rfc-editor.org/rfc/rfc9052>
