<!-- SPDX-License-Identifier: Apache-2.0 -->

# attest

OpenPRoT OCP-EAT attestation token producer and verifier.

Three crates, no external git dependencies:

| Crate | Path | Purpose |
|---|---|---|
| `openprot-attest-api` | `attest/api` | Platform-independent traits and types |
| `openprot-attest-producer` | `attest/producer` | Concrete token producer implementations |
| `openprot-attest-verifier` | `attest/verifier` | Host-side EAT token verifier and appraisal engine |

## Bazel targets

```
//attest:attest_embedded_all   # production library group (api + producer)
//attest:attest_host_tests     # all unit + integration tests
//attest:attest_verifier_all   # verifier library and CLI tool
```
