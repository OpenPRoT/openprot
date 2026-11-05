# AGENTS.md

Guidance for AI coding agents working in the OpenPRoT repository. Derived from `docs/src/`; the docs and `workflows.json` remain authoritative.

## Project overview

OpenPRoT is an open-source, `no_std` Rust firmware project for Platform Root-of-Trust devices. It provides HAL traits plus services/protocols for platform security (SPDM, MCTP, PLDM, attestation, firmware update). It is built on [Pigweed](https://pigweed.dev): `pw_kernel` (microkernel), `pw_log`, `pw_status`, and the Bazel build with the `./pw` launcher.

Embedded, security-critical firmware: favor correctness and reviewability over cleverness.

## Build, test, and lint

The repo is a Bazel module (`MODULE.bazel`). There is **no Cargo workspace**; do not use `cargo` directly. Prerequisite: Bazel (bazelisk recommended). All other dependencies are managed by Bazel.

`./pw` is a wrapper around `bazelisk run //:pw -- "$@"` and dispatches to groups in `workflows.json`:

```bash
./pw presubmit         # format check + presubmit_checks + clippy (run before every commit; CI gate)
./pw format            # apply rustfmt / black / buildifier / clang-format
./pw default           # build everything buildable on a dev host, no tests
./pw ci                # build + host tests, skipping hardware and verilator targets
./pw upstream_pigweed  # ci + Earlgrey verilator tests (for Pigweed bumps)
bazel build //docs     # build mdbook docs
bazel run //docs:serve # serve docs at http://localhost:8000
```

Direct Bazel usage:

```bash
bazel test //...                                                 # host tests
bazel build //... --build_tag_filters=-hardware,-disabled        # practical wildcard build
bazel build //... --build_tag_filters=-hardware,-disabled,-verilator
bazel test //path/to:target_name                                 # single target
bazel test --config=virt_ast10x0 //target/ast10x0/...            # AST10x0 QEMU tests
```

- Tag filters are required for wildcard builds; `//...` includes hardware-only and verilator targets that fail on dev machines.
- `//third_party/caliptra/...` is intentionally excluded from wildcard builds (crate-universe workspaces collide). Build into Caliptra paths explicitly when needed.
- Clippy runs as a `rules_rust` aspect: `bazelisk build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //... -//third_party/caliptra/...`
- Tags: `hardware` (real silicon), `verilator` (simulator), `disabled`, `manual`, `kernel` (pw_kernel system images).

## Repository layout

```
openprot/            Main application crate
drivers/usart/       Target-agnostic USART API, client, server
hal/{async,blocking,nb}/   HAL trait crates by execution model
platform/traits/     OS-abstraction traits
platform/impls/      Concrete impls per host environment
services/{i2c,mctp,orchestrator,spdm,storage,telemetry}/
target/{ast10x0,earlgrey,mock,veer}/   Silicon-specific backends, linker scripts, system images, test runners
third_party/{pigweed,caliptra,crates_io}/
presubmit/           Python presubmit scripts
docs/                mdbook sources
```

Placement rules:
- Hardware-facing contracts → `hal/` or a driver's public API.
- Host-OS contracts → `platform/traits/`; concrete host impls → `platform/impls/`.
- Reusable protocol/service logic → `services/`.
- Silicon-specific backends, peripherals, linker scripts, system images, test runners → `target/<name>/`.
- Example: `drivers/usart/` defines the interface; `target/ast10x0/backend/usart/` implements it.
- Per-driver/service `README.md` files document public seams, invariants, and host-test targets.

## Targets and images

- Each `target/<name>/` has its own linker script, entry point, `defs.bzl` helpers, register definitions, and runners.
- When adding a target or binary, use that target's `defs.bzl` helpers rather than hand-rolled `rust_binary`, and always set `target_compatible_with = TARGET_COMPATIBLE_WITH` so wildcard host builds skip target-only crates.
- `system.json5` declares the process topology; Pigweed's `system_image()` combines it with kernel and application crates.
- Host tests live next to the platform-independent API/service. Emulator, simulator, and hardware tests live under `target/<name>/` with their runner config.
- Earlgrey: QEMU tests run on every PR; Verilator tests are gated by the `verilator` tag. VeeR runs on the Caliptra emulator via `target/veer/tooling/caliptra_runner.bzl`.
- Pigweed is pinned via `git_override` in `MODULE.bazel`. Rust crate deps come from three crate_universe workspaces: `@rust_crates`, `@rust_caliptra_crates` (embedded), `@rust_caliptra_crates_host`. Caliptra version bumps go through `third_party/caliptra/uprev.py`.

## Coding style

Run `./pw format` to apply all formatters.

**Rust** (rustfmt with Pigweed's `rustfmt.toml`). Review enforces:
- `no_std` only. No heap types (`Vec`, `String`, `HashMap`, `Box`); use fixed arrays and `heapless`.
- Panic-free: no `unwrap`, `expect`, `panic!`, or direct `[]` indexing in production paths. Use `get()`, pattern matching, `Result`/`Option`.
- Checked arithmetic: `checked_*`/`saturating_*`/`wrapping_*` instead of bare `+ - *` where overflow is possible.
- MMIO uses `read_volatile`/`write_volatile` and goes through HAL traits, never raw register access.
- Every `unsafe` block needs a `// SAFETY:` comment explaining why invariants hold.
- Constant-time crypto: use `subtle` for secret comparison and `zeroize` to bound secret lifetimes.
- See `.github/copilot-instructions.md` for the full list.

**Python**: PEP8, formatted with `black`.
**Starlark**: bzl style guide, formatted with `buildifier`.
**C/C++**: Google C++ Style with exceptions: 4-space indent, `snake_case` function names, pointer asterisk attached to the variable (`int *foo`). Formatted with `clang-format`.

## Contribution process

- **Small change** (bug fix, minor improvement, docs): optional issue, then a PR.
- **Large change** (architecture, boot process, API/register layout, security posture, major tooling): needs an RFC (GitHub issue titled `[RFC] …` covering scope, rationale, tradeoffs, timeline, test plan, maintenance) approved by the OpenPRoT TSC before PRs go upstream. If unsure, file an issue first.
- Fork-and-PR workflow; split large features into small PRs (ideally < 500 lines).
- Documentation must accompany new functionality; crypto changes need a thorough test plan; stay compatible with supported tool versions.
- Contributions require the CHIPS CLA. Docs live in `docs/` (mdbook) and are built with `bazel build //docs`.
- Run `./pw presubmit` before committing.

## Design notes (`docs/src/design/`)

**Pigweed integration** (`pigweed-overview.md`)
- Pigweed is a Bazel module pinned to an explicit commit via `git_override` in `MODULE.bazel`. Use the canonical `pigweed.googlesource.com` remote; the GitHub mirror is read-only and not authoritative.
- Consumed pieces: `pw_kernel`, `pw_log` (logging everywhere), `pw_status` (`Result<T, pw_status::Error>` is the return type of every syscall and IPC entry point), and Pigweed-owned Rust and C/C++ toolchains (host clang, `rv32imc` RISC-V clang).
- Pigweed's default `rust_crates` lacks `riscv32imc-unknown-none-elf`, so OpenPRoT overrides it to point at its own `@rust_crates`.

**pw_kernel IPC** (`pw-kernel-ipc.md`; worked example in `target/veer/ipc/`)
- A channel has two endpoints owned by different processes: a *handler* (server) and an *initiator* (client). Declare them as `objects` (`channel_handler` / `channel_initiator` with `handler_process` and `handler_object_name`) in `system.json5`; codegen exposes a `handle::<NAME>` constant per process.
- Initiator: `syscall::channel_transact(handle, &send, &mut recv, deadline)`. `Instant::MAX` blocks forever; pass a finite deadline to bound it.
- Handler loop: `object_wait(READABLE)` → `channel_read` → `channel_respond`. Respond exactly once per read or the initiator stays blocked.
- One initiator and one handler per channel; fan-in needs multiple channels to the same handler process, which waits on each.
- Wire format is raw bytes with no IDL. Use fixed-size or length-prefixed layouts, stack buffers, byte-stable encoding (e.g. `zerocopy`) with request/response types in a shared crate. Treat requests as untrusted and validate everything. Handler loops must never panic (a panic is a DoS for every client).

**Orchestrator** (`design/orchestrator/`, code in `services/orchestrator/`)
- The eRoT boot-sequence controller: verifies each platform component's firmware and releases it from reset in order, then governs attestation, update, and recovery. One instance per discrete eRoT running OpenPRoT; the chain config is the only per-deployment difference.
- `services/orchestrator/sm` is a **pure state machine** (reducer): it does no I/O. Outside information arrives only as `Event` payloads; every action is an `Effect` value carried out by the platform. Internal follow-ups are emitted as `Effect::Emit(event)` so they show in the effect trace. The core hard-codes no board values; the platform supplies the trust chain and the recovery-retry cap at startup.
- States: `PowerOnReset`, `PreSupervision`, `AwaitingReady`, `Ready`, `Updating`, `Recovering`, `Locked`, under the `SupervisingPlatform` superstate. The state diagram in `orchestrator-machine.md` is the single source of truth. `Rot<N, E>` is the shared storage, `Sink` the context, and the doc has an invariant catalogue.
- Two-tier verification model: eRoT gate plus optional iRoT gate. `ComponentKind::Active` (has an iRoT) waits in `AwaitingReady` for `ComponentReady`; `Passive` advances right after `ReleaseReset`. `FailurePolicy::Isolable` skips a failed component (held in reset) without blocking; `Cascading` plus `ComponentAttrs::depends_on` also skips dependents. `ComponentAttrs::recovery_region` (`RegionId`) groups components restored together. A `Timeout(ComponentId)` in `AwaitingReady` leads to `Recovering`. Recovery is a re-boot, and retry-cap exhaustion leads to `Locked`.
- Platform half: `services/orchestrator/capabilities` (narrow contracts such as `BootControl`), `adapters/hal` (HAL bindings), and `config` (board device table schema; values in `target/<board>/devices.rs`, validated at compile time). Rules: *decide what, never how* (no pin or register knowledge in policy; porting means a new table plus HAL), and *protection survives an orchestrator crash* (the SPI monitor filters in hardware and its write filter stays armed until first fetch).

## Specification notes (`docs/src/specification/`)

OpenPRoT spec v0.5 (WIP, an OCP specification plus open-source implementation); several pages are Draft/TBD.

- **Device abstraction (DDK)**: generic Rust traits for I/O peripherals (SMBus/I2C monitor/filter, delay) and crypto accelerators (AES, ECC/ECDSA, digest, RSA). Goals: platform- and execution-model-agnostic (blocking, async, and `nb` trait crates). Principles: minimalism, zero cost, composability, robust error handling, separate control and data paths. Usable as a low-level driver, a kernel-driver proxy, or a peripheral server task proxy.
- **Threat model**: assets are firmware integrity/authenticity, crypto operation integrity, anti-rollback counters, symmetric and private keys, boot measurements, update payloads, and policies. Attacker profile follows JIL (physical and remote; perturbation, side-channel, test features, RNG, software, isolation). Integrators must supply a threat table (Threat ID, Description, Target Assets, Mitigations, Verification).
- **Firmware resiliency**: aligned with NIST SP 800-193 (protect, detect, recover); the PRoT and connected-device sections are TBD.
- **MCTP**: transport for all DMTF protocols; minimum version 1.3.1 (DSP0236). SMBus binding (DSP0237) is mandatory; I3C, PCIe-VDM, and USB are recommended. Required commands: Set/Get Endpoint ID, Get MCTP Version Support, Get Message Type Support, Get Vendor Defined Message Support, and 0xF0-0xFF. A TCP binding is provided for development.
- **SPDM**: used for all attestation, both as requester (downstream) and responder (upstream). Baseline SPDM 1.2 and aligned with OCP Attestation Spec 1.1. Responders report a single version. Mandatory commands: `GET_VERSION`, `GET_CAPABILITIES`, `NEGOTIATE_ALGORITHMS`, `RESPOND_IF_READY`. Also required: `GET_DIGESTS`, `GET_CERTIFICATE`, `CHALLENGE`, `GET_MEASUREMENTS`, `GET_CSR`, `SET_CERTIFICATE`, `CHUNK_SEND`, `CHUNK_GET`. Hardware must support ECDSA P-384 and SHA3-384. Evidence is RATS EAT (as CWT) or a TCG DICE Concise Evidence manifest TOC; EAT goes in measurement block 0xF0 when there is no manifest.
- **PLDM**: responder for Type 0 (Base 1.2.0), Type 2 (Platform Monitoring 1.3.0; build-time JSON PDRs, security sensors only, no effectors), and Type 5 (Firmware Update 1.3.0, with the required inventory and update commands).
- **Firmware update**: PLDM Type 5 is the primary mechanism, covering OpenPRoT's own firmware and downstream devices. Updates must be authenticated and authorized, with a recovery path on failure. The spec defines the package header, component identifiers, and the update process.
- **Attestation**: maps to the IETF RATS architecture. OpenPRoT has a dual role: *Attester* (SPDM responder returning a signed EAT with an OCP profile and TCG Concise Evidence) and *Verifier* (SPDM requester plus a local verifier; native OCP EAT path, plugin path for other formats). Certificate chain and key derivation are hardware-defined and out of firmware scope. Covers DICE/DPE, reference values and endorsements (CoRIM), and device identity provisioning.
- **Use cases**: legacy-device integration (TPM, direct and side-by-side) and the module / single-node / heterogeneous-compute deployment models.

## Further reading

`docs/src/`: `architecture.md`, `usage.md`, `coding-style.md`, `development-process.md`, plus the full `design/` and `specification/` trees for details beyond the summaries above.
