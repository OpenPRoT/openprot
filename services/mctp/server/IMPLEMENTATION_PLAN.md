# In-Process SPDM Requester — Implementation Plan

Companion to [DESIGN_IN_PROCESS_REQUESTER.md](DESIGN_IN_PROCESS_REQUESTER.md).
Breaks the design into landable phases, each independently compileable and
testable so nothing goes dark on `main` between merges.

## Status

| Phase | Status | Notes |
|---|---|---|
| 0 — baseline build + assumption check | ✅ done | Required two pre-Phase-1 fixes (see Phase 0 Resolution Log) |
| 1 — feature flag restructuring | ✅ done | `direct-client` is now the shared capability; role features `in-process-responder` / `in-process-requester` pick the FSM. Mutual-exclusion `compile_error!` in `main.rs`. |
| 2 — compileable requester stub | ✅ done | Forced creation of a second system_image package `//target/ast1060-evb/mctp-requester` so the standalone `rust_app` target actually compiles (standalone `rust_app` hits a `pw_kernel/userspace` platform-constraint error without a system image). |
| 3 — FSM happy path | ✅ done | Eight-state VCA FSM, one step per polling-loop iteration. |
| 4 — safety net + counters | ✅ done | `AWAIT_STEP_BUDGET = 10_000`, `await_try!` macro, counters `req_send_ok` / `req_recv_ok` / `req_recv_pending`. |
| 5 — Bazel target + system image | ✅ done (merged into Phase 2) | See Phase 2 note above. |
| 6 — two-board integration test | ⏸ blocked | No on-target hardware available in this environment. See IMPLEMENTATION_PLAN.md §A for measurements to collect when hardware is accessible. |
| 6a — QEMU integration test (virt-ast1060-evb) | ✅ done | QEMU patched with ASPEED I2C emulation; `//target/virt-ast1060-evb/mctp-requester:mctp_requester_test` added. No two-board setup needed. |
| 7 — docs + follow-up TODOs | ✅ done | [INITIALIZATION.md](INITIALIZATION.md) updated with requester glossary + setup steps; follow-up design refs annotated at the FSM in `main.rs`. |

Build verification at end of Phase 7:

```
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp:mctp                                          # responder (hw) — clean
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp-requester:mctp_requester                      # requester (hw) — clean
bazel build --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester             # requester (QEMU) — clean
bazel test  --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester_test        # requester QEMU test
```

Legend:
- **Goal** — what this phase must achieve.
- **Deliverables** — concrete artifacts produced.
- **Files** — edited / added in this phase.
- **Exit criteria** — what proves the phase is done.
- **Risks / rollback** — what can go wrong; how to back out.

---

## How To Build Which Role

Role (requester vs. responder) is selected at **build time**, not at
runtime. Same `src/main.rs` source, two binaries, chosen by which Bazel
**system-image** target you build. Standalone `rust_app` targets do not
build in isolation — `pw_kernel/userspace` requires a `target_codegen` →
`kernel_config` chain that only exists inside a `system_image`.

**Developer commands:**

```bash
# Responder image (default, hardware)
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp:mctp

# Requester image (hardware)
bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp-requester:mctp_requester

# Requester image (QEMU — requires patched QEMU with ASPEED I2C emulation)
bazel build --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester
bazel test  --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester_test
```

Running any test or integration probe uses the same prefix
(`bazel test --config=k_ast1060_evb <target>`).

**Selection chain** — each layer picks the next:

```
 System image                                    →  rust_app label                              →  Cargo features                                           →  #[cfg] in main.rs
 ────────────────────────────────────────────────   ─────────────────────────────────────────────   ───────────────────────────────────────────────────────   ─────────────────────────
 //target/ast1060-evb/mctp:mctp                  →  //services/mctp/server:mctp_server          →  ["i2c-polling", "direct-client", "in-process-responder"] →  responder setup + step
 //target/ast1060-evb/mctp-requester:mctp_requester →  //services/mctp/server:mctp_server_requester  →  ["i2c-polling", "direct-client", "in-process-requester"] →  requester setup + FSM
```

Each `rust_app` has its own `codegen_crate_name` — `app_mctp_server` for
the responder, `app_mctp_server_requester` for the requester — because two
`rust_app` rules in the same Bazel package cannot share the codegen name.
`src/main.rs` picks the right one via a cfg-gated `use`.

**Guards:**

- Both roles at once → rejected by `compile_error!` at the top of `main.rs`.
- Neither role + `i2c-polling` → polling loop compiles with no SPDM in-process
  (still valid; drains I2C only).
- No `i2c-polling` → the notification (WG + IRQ) loop compiles; role features
  are ignored because `in-process-*` code is also gated on `i2c-polling`.
- `direct-client` is listed explicitly in both `crate_features` sets;
  Bazel's `rust_binary` does not resolve Cargo-level feature implications
  (`in-process-*  = ["direct-client"]`), so the transitive dep must be
  named directly.

**What flashes:** each system image binds one of the two `rust_app` labels
via its `apps = [...]` list, together with `i2c_server`. The two images
are mutually exclusive — flash one or the other, not both.

**Direct `cargo build` is not a supported path** for the binary — the
package sets `autobins = false` and `main.rs` is a Pigweed binary built
only through Bazel. The Cargo features still matter because Bazel passes
them through; they're just never consumed by a bare `cargo` invocation
of `main.rs`.

---

## Phase 0 — Baseline & Assumption Verification

**Goal**: confirm the starting source tree builds clean before any new
code is written. Source-level only — no on-target execution is required
or assumed at this stage.

**Deliverables**:
- Successful Bazel build of
  `//target/ast1060-evb/mctp:mctp` (responder system image) at current
  `ocp-emea-demo-spdm-spdm-req-resp-anthony-i2c-polling` HEAD.
- Source-level re-read of the existing polling loop at
  [src/main.rs:322-445](src/main.rs#L322-L445) to confirm the interleaving
  assumptions in design §6.5 still match the code (in particular, that
  `wait_for_messages(..., None)` paired with the `e.is_timeout()` arm at
  [src/main.rs:390](src/main.rs#L390) matches constraint §6.5.3).
- **Skipped:** standalone build of
  `//target/ast1060-evb/spdm-req-resp-test:spdm_requester_app`. The app is
  marked `# DISABLED` in the sibling `system_image` and fails platform
  analysis when built alone. The reference constants in
  [spdm_requester.rs](../../target/ast1060-evb/spdm-req-resp-test/spdm_requester.rs)
  are consumed as source, not as a binary, so skipping this build does
  not block later phases.

**Files**: none modified **initially**. See Phase 0 Resolution Log below.

**Exit criteria**: the responder system image builds clean; the §6.5
cross-check is recorded as "verified against HEAD" in the commit
message of Phase 1.

**Risks / rollback**: none — read-only phase by default; any unblocker
applied here gets its own commit (see log below).

### Phase 0 Resolution Log

- **Blocker discovered**: at branch HEAD, commit `023e75e "Update mctp-lib
  (to hotfix branch)"` changed `mctp_lib::Router::inbound` to return
  `Result<Option<AppCookie>, MctpError>`, but the
  [services/mctp/server/src/server.rs:267-271](src/server.rs#L267-L271)
  wrapper `Server::inbound` still declared `Result<(), MctpError>`. The
  error surfaces as E0308 and fails *every* Bazel target that depends on
  `mctp_server_lib` (responder system image, req-resp test image,
  etc.).
- **Verification that the break is isolated**: a worktree at
  `d4719a0 "Chasing down an MCTP packet issue."` (the commit immediately
  preceding the mctp-lib hotfix) built `//target/ast1060-evb/mctp:mctp`
  clean. So the break is entirely within `023e75e`.
- **Resolution — Fix A** (discard the new cookie):
  ```rust
  pub fn inbound(&mut self, pkt: &[u8]) -> Result<(), MctpError> {
      self.stack
          .inbound(pkt)
          .map(|_| ())
          .map_err(mctp_error_to_server_error)
      }
  ```
  Chosen over Fix B (propagate `Option<AppCookie>` to callers) because:
  no caller reads the return value today, the propagation case is cheap
  to add later if Shape-B from design §9.1 ever lands, and keeping the
  unblocker orthogonal to the requester work eases `git blame`.
  Landed as its own small commit before any Phase 1 work.

- **Second blocker** (surfaced after Fix A unblocked rustc past the first
  error): the same mctp-lib hotfix changed
  `MctpI2cReceiver::decode`'s return type from `(&[u8], u8)` to
  `(&[u8], MctpI2cHeader)` — a struct
  `{ dest: u8, source: u8, byte_count: usize }` — but
  [src/main.rs:350-361](src/main.rs#L350-L361) still treated the second
  tuple element as a bare `u8` source address. `as u32` on a struct
  triggers E0605.
- **Resolution — Fix A′** (match the new type, minimum diff): rename
  the binding from `src_addr` to `hdr` and read the source address
  field explicitly: `hdr.source as u32`. Landed in the same
  "pre-Phase-1 unblocker" commit as Fix A.

**Deferred to on-target bringup**: measuring actual idle-poll rate,
`wait_for_messages` latency, and worst-case fragment count is out of
scope for Phase 0. `AWAIT_STEP_BUDGET` will ship with a generous
compile-time default (§6.4) and be tuned when a runtime becomes
available — see Appendix §A.

---

## Phase 1 — Feature Flag Restructuring

**Goal**: rename the responder role feature and add the requester role
feature, without changing behavior. Pure refactor.

**Deliverables**:
- `services/mctp/server/Cargo.toml`:
  - Add `in-process-responder = ["direct-client"]`.
  - Add `in-process-requester = ["direct-client"]`.
  - Update `default` to `["i2c-polling", "in-process-responder"]`.
  - Keep `direct-client` as-is (library-exposed capability).
- `services/mctp/server/src/main.rs`:
  - Add at top:
    ```rust
    #[cfg(all(feature = "in-process-requester", feature = "in-process-responder"))]
    compile_error!("features `in-process-requester` and `in-process-responder` are mutually exclusive");
    ```
  - Replace every `#[cfg(feature = "direct-client")]` inside the polling
    loop and its setup block with
    `#[cfg(feature = "in-process-responder")]`.
  - Leave the `mod direct_client;` export and `DirectMctpClient` untouched
    — those belong to the `direct-client` feature, which still exists.
- `services/mctp/server/BUILD.bazel`:
  - `mctp_server` target: replace
    `crate_features = ["i2c-polling", "direct-client"]` with
    `["i2c-polling", "in-process-responder"]`.
  - Keep `rust_library` `mctp_server_lib` with `crate_features = ["direct-client"]`.

**Files**: `Cargo.toml`, `src/main.rs`, `BUILD.bazel`.

**Exit criteria**:
- `bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp:mctp`
  succeeds (the standalone `rust_app` target cannot be built in isolation
  because `pw_kernel/userspace` needs a system-image codegen context).
- Built binary is byte-identical in behavior to Phase 0 baseline (responder
  loop still runs; log lines at startup unchanged).
- Attempting to build with both features enabled emits the
  `compile_error!`.

**Risks / rollback**:
- *Risk*: downstream Bazel targets still set `direct-client` directly and
  silently lose the responder. *Mitigation*: grep the repo for
  `direct-client` in `crate_features`; the only consumer is
  `services/mctp/server/BUILD.bazel`.
- *Rollback*: revert the single refactor commit; no logic changed.

---

## Phase 2 — Compileable Requester Stub

**Goal**: add the requester setup skeleton behind
`in-process-requester` as a no-op Phase 2 step. Proves feature gating,
imports, and the init sequence compile — no FSM logic yet.

**Deliverables**:
- `services/mctp/server/src/main.rs`:
  - Add constant `const REMOTE_RESPONDER_EID: u8 = 42;` (module-scope, not
    cfg-gated — a single `#[allow(dead_code)]` keeps it clean when the
    feature is off).
  - Add imports gated on
    `#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]`:
    mock platform types, `MctpSpdmTransport`, `SpdmContext`, VCA
    `generate_*` functions, `DemoPeerCertStore`.
  - Inside `mctp_loop()` under the same cfg, add the full init sequence
    from §7 of the design doc:
    1. `DirectMctpClient::new(&server)`
    2. `MctpSpdmTransport::new_requester(client, REMOTE_RESPONDER_EID)`
    3. `transport.init_sequence()` with error log + return
    4. Mock platform instances (cert_store, hash×3, rng, evidence,
       peer_cert_store)
    5. `CapabilityFlags` / `DeviceCapabilities` matching
       [spdm_requester.rs](../../target/ast1060-evb/spdm-req-resp-test/spdm_requester.rs)
    6. `LocalDeviceAlgorithms` (factor into a helper to avoid duplicating
       the responder's inline block)
    7. `SpdmContext::new(..., Some(&mut peer_cert_store), ...)`
    8. `MessageBuf` on stack buffer
    9. `let mut req_state = ReqState::SendVersion;` — **but** Phase 2
       step body is still just
       `#[allow(unused_variables)] let _ = &mut req_state;` with a log
       `"requester FSM stub: state=..."` every 0xfff loops.
  - Add the `ReqState` enum (full variant set — §6.1) even though only
    one variant is used, so Phase 3 just fills in match arms.

**Files**: `src/main.rs`.

**Exit criteria**:
- `bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp-requester:mctp_requester`
  succeeds.  (Phase 2 verification required pulling Phase 5's
  system-image package forward — a standalone `rust_app` target is not
  buildable on its own in this tree; see `mctp-requester/` for the
  system image created here.)
- The responder build is unaffected.
- Running the requester build on-target logs the init sequence, then
  idle-poll messages with the stub state — no SPDM traffic emitted yet.

**Risks / rollback**:
- *Risk*: Rust borrow checker rejects `SpdmContext::new` arg layout
  because `transport` and `server` share a lifetime. *Mitigation*: mirror
  the responder's exact let-binding order, which is already known to
  compile.
- *Risk*: `DemoPeerCertStore` import path differs from responder
  (responder uses `None`). *Mitigation*: path already resolved in
  `spdm_requester.rs` — copy.
- *Rollback*: revert commit; responder build unaffected.

---

## Phase 3 — FSM Implementation (Happy Path)

**Goal**: wire the six Send/Await states so a full VCA flow completes on
the wire.

**Deliverables**:
- Replace the Phase-2 stub body with the match from §6.2 of the design
  doc. Each Send state:
  1. `msg_buf.reset()`
  2. `generate_<step>(...)`
  3. `ctx.requester_send_request(&mut msg_buf, REMOTE_RESPONDER_EID)`
  4. Advance to matching `Await*` state.
- Each Await state:
  1. `msg_buf.reset()`
  2. `ctx.requester_process_message(&mut msg_buf)`
  3. `Ok(_)` → advance; `Err(_)` → stay (counted, not yet budgeted).
- On entry to `ReqState::Done`, log once:
  `"SPDM VCA completed: version/caps/algs OK"`.
- `Done` and `Failed` states become no-ops in Phase 2 (Phase 1 keeps
  draining I2C unconditionally, so late inbound traffic is still handled).

**Files**: `src/main.rs`.

**Exit criteria**:
- End-to-end test on AST1060-EVB against the existing
  `spdm_responder` app running on a second device (or the same device
  with the responder built separately) produces three successful
  request/response pairs.
- Protocol-analyzer capture (or I2C frame logs already present) shows
  GET_VERSION / GET_CAPABILITIES / NEGOTIATE_ALGORITHMS emitted in order
  with matching response consumption.
- `spdm_err` / equivalent pending-counter stays bounded (not monotonically
  growing past expected poll-while-waiting count).

**Risks / rollback**:
- *Risk*: a protocol error wedges the FSM in an Await state forever
  (design §6.3, option 1 not yet implemented). *Mitigation*: Phase 4
  adds the step budget; until then, a power-cycle is acceptable for
  bringup.
- *Risk*: `include_supported_algorithms = false` / other capability
  fields diverge between the in-process requester and the responder's
  expectations. *Mitigation*: copy verbatim from `spdm_requester.rs`;
  already known to interoperate with `spdm_responder.rs`.
- *Rollback*: revert to Phase 2 stub.

---

## Phase 4 — Safety Net & Observability

**Goal**: bound failure latency and give on-target debugging the same
counter set as the responder.

**Deliverables**:
- `AWAIT_STEP_BUDGET` const + `await_steps` counter per design §6.4,
  reset on every state transition, transitions FSM to `Failed` on
  exhaustion with an error log naming the state that timed out.
- Counters from design §8, each `u32` wrapping, logged on first event and
  every 256th:
  - `req_send_ok`, `req_send_err`, `req_recv_ok`, `req_recv_pending`,
    `req_recv_err_terminal`.
- State-transition log (one line per transition, not rate-limited — low
  volume).
- `Done` transition log augmented with
  `"completed in {N} steps, ~{idle_polls} idle polls"`.

**Files**: `src/main.rs`.

**Exit criteria**:
- Forcing a fault (e.g. responder offline) causes `Failed` transition
  within the budget and emits the named-state error; device does not
  hang.
- Counters visible in logs match observed behavior (send counts == 3 on
  success path; pending spikes and settles at each Await entry).

**Risks / rollback**:
- *Risk*: step budget too tight → spurious failures under a legitimately
  multi-fragment response (§6.5.2). *Mitigation*: ship with a deliberately
  generous default (`AWAIT_STEP_BUDGET = 10_000`) and defer tightening
  until on-target measurements exist (see Appendix §A). Erring high
  costs only wall-clock on genuine failures; erring low wedges the
  happy path.
- *Rollback*: revert budget block, keep counters (useful regardless).

---

## Phase 5 — Bazel Target & System Image

**Goal**: make the requester role selectable as a first-class Bazel build
without touching the responder's target.

**Deliverables**:
- New target in `services/mctp/server/BUILD.bazel`:
  ```starlark
  rust_app(
      name = "mctp_server_requester",
      codegen_crate_name = "app_mctp_server",
      srcs = ["src/main.rs"],
      crate_features = ["i2c-polling", "in-process-requester"],
      edition = "2024",
      system_config = "//target/ast1060-evb/mctp:system_config",
      tags = ["kernel"],
      visibility = ["//visibility:public"],
      deps = [ # identical to mctp_server
          ":mctp_server_lib",
          "//services/i2c/api:i2c_api",
          "//services/i2c/client:i2c_client",
          "//services/mctp/api:mctp_api",
          "//services/mctp/transport-i2c:mctp_transport_i2c",
          "//services/spdm/transport-mctp:spdm_transport_mctp",
          "//target/ast1060-evb/spdm-req-resp-test:mock_platform",
          "@rust_crates//:mctp",
          "@rust_crates//:mctp-lib",
          "@pigweed//pw_kernel/syscall:syscall_user",
          "@pigweed//pw_kernel/userspace",
          "@pigweed//pw_log/rust:pw_log",
          "@pigweed//pw_status/rust:pw_status",
          "@oot_crates_no_std//:spdm-lib",
      ],
  )
  ```
- Choose whether the default system image
  ([target/ast1060-evb/mctp/BUILD.bazel](../../target/ast1060-evb/mctp/BUILD.bazel))
  wires the requester or the responder. Do **not** change the default in
  this phase — add a second system image build instead (e.g.
  `system_requester`) so both can be flashed side by side during
  bringup.

**Files actually landed**:
- `services/mctp/server/BUILD.bazel` — second `rust_app(name = "mctp_server_requester", codegen_crate_name = "app_mctp_server_requester", ...)` target.
- `target/ast1060-evb/mctp-requester/BUILD.bazel` — new sibling package with its own `system_image`, `target_codegen`, `target_linker_script`, `rust_binary(name="target",...)`, and `uart_boot_image`.
- `target/ast1060-evb/mctp-requester/system.json5` — copy of the mctp image's config with the MCTP app renamed to `mctp_server_requester`.
- `target/ast1060-evb/mctp-requester/target.rs` — copy of the kernel entry shim.

**Exit criteria**:
- Both system images build clean from scratch:
  ```
  bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp:mctp
  bazel build --config=k_ast1060_evb //target/ast1060-evb/mctp-requester:mctp_requester
  ```
- Responder system image continues to boot unchanged.
- Requester system image boots and reaches the FSM entry log within
  the expected startup window (deferred to on-target bringup — Phase 6).

**Risks / rollback**:
- *Risk*: `codegen_crate_name` collision when both `rust_app` targets
  live in the same Bazel package. *Resolved*: the two targets use
  distinct codegen crate names (`app_mctp_server`,
  `app_mctp_server_requester`) and `src/main.rs` selects between them
  with a cfg-gated `use`.
- *Risk*: platform-constraint analysis error when building
  `rust_app` standalone. *Resolved*: the `rust_app` target must be
  consumed by a `system_image`, never built directly.
- *Rollback*: delete the new system-image package; responder unaffected.

---

## Phase 6 — Integration Test (two-board, on-target hardware)

**Goal**: a repeatable test that exercises the in-process requester
against the in-process responder on the bench.

**Status**: ⏸ blocked — no on-target hardware available. See §A for
measurements to collect when hardware is accessible.

**Deliverables**:
- Two-board test config: board A flashed with
  `mctp_server_requester` system image, board B flashed with
  `mctp_server` (responder). Both use current `OWN_EID` / `OWN_I2C_ADDR`
  constants, with board A targeting board B at
  `REMOTE_RESPONDER_EID = 42` — board B must be re-configured to EID
  42 (its current `OWN_EID = 0x08`; this requires a new const
  `RESPONDER_OWN_EID` on the responder image, or swapping A↔B roles).
- Documented test recipe in `README.md` (new section) or a dedicated
  `tests/ON_TARGET_README.md`: flash steps, expected log lines, expected
  duration-to-`Done`.
- Automated log-checking script is out of scope; bringup-level manual
  verification is sufficient.

**Files**: docs only, plus any const bump in main.rs if we change
`OWN_EID` on the responder side.

**Exit criteria**:
- Manual test produces a `SPDM VCA completed` line on the requester side
  and three matching processed-request lines on the responder side.

**Risks / rollback**:
- *Risk*: EID mismatch between the two images. *Mitigation*: the spec
  prescribes explicit named constants for both ends; document the
  pairing in the test recipe.
- *Rollback*: none needed — docs only.

---

## Phase 6a — QEMU Integration Test (virt-ast1060-evb)

**Goal**: automated QEMU test that exercises the full I2C + MCTP + SPDM
requester stack without physical hardware, using QEMU's ASPEED I2C
device emulation.

**Prerequisite**: QEMU patched with ASPEED i2c device model (user confirmed
April 2026).

**Deliverables**:
- `target/virt-ast1060-evb/mctp-requester/system.json5` — two-process
  (i2c_server + mctp_server_requester) virt image with ASPEED I2C MMIO
  mappings at the real hardware addresses (emulated by QEMU).
- `target/virt-ast1060-evb/mctp-requester/target.rs` — semihosting
  kernel shim; `shutdown()` calls `exit(EXIT_SUCCESS/FAILURE)` so QEMU
  terminates and the test harness can read the exit code.
- `target/virt-ast1060-evb/mctp-requester/BUILD.bazel` — `system_image`,
  `system_image_test`, `target_codegen`, `target_linker_script`,
  `rust_binary(name="target")` wired to `//target/virt-ast1060-evb`
  platform and linker template.
- `services/mctp/server/BUILD.bazel` — `mctp_server_requester_virt`
  rust_app, same feature set as `mctp_server_requester` but with
  `system_config = "//target/virt-ast1060-evb/mctp-requester:system_config"`
  so codegen (handle constants + linker script) is correct for QEMU.

**Files**:
- `target/virt-ast1060-evb/mctp-requester/system.json5` (new)
- `target/virt-ast1060-evb/mctp-requester/target.rs` (new)
- `target/virt-ast1060-evb/mctp-requester/BUILD.bazel` (new)
- `services/mctp/server/BUILD.bazel` (add `mctp_server_requester_virt`)

**Build / test commands**:

```bash
bazel build --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester
bazel test  --config=k_virt_ast1060_evb //target/virt-ast1060-evb/mctp-requester:mctp_requester_test
```

**Exit criteria**:
- `bazel test` passes: QEMU starts, the FSM reaches `Done`, semihosting
  `exit(EXIT_SUCCESS)` fires, QEMU terminates with code 0, Bazel marks
  the test green.
- Responder hardware image (`//target/ast1060-evb/mctp:mctp`) and
  hardware requester image (`//target/ast1060-evb/mctp-requester:mctp_requester`)
  still build clean.

**Key design notes**:
- `mctp_server_requester_virt` shares `codegen_crate_name =
  "app_mctp_server_requester"` with the hardware target — the handle
  layout is identical (I2C object is second in the process objects list
  in both system configs), so no source changes are needed.
- Memory layout (640KB) mirrors `virt-ast1060-evb/spdm-req-resp-test`;
  vector_table_size_bytes = 1280 (matches multi-process virt convention).
- ASPEED I2C MMIO mappings (`0x7e7b0000` / `0x7e6e2000`) are kept so
  the i2c_server binary can run unmodified against the QEMU device model.

**Risks / rollback**:
- *Risk*: QEMU's ASPEED I2C model doesn't emulate bus-2 slave-mode
  loopback precisely enough for the polling loop. *Mitigation*: QEMU
  patch is user-supplied and already tested; fall back to the two-board
  (Phase 6) path if emulation fidelity is insufficient.
- *Rollback*: delete the new `target/virt-ast1060-evb/mctp-requester/`
  package and remove `mctp_server_requester_virt` from services BUILD.bazel;
  hardware targets are unaffected.

---

## Phase 7 — Cleanup & Follow-Up Tickets

**Goal**: close loose ends the design doc called out and make sure
nothing is left under-documented.

**Deliverables**:
- File follow-up issues (or local TODO comments tagged with a tracking
  tag) for:
  - Design §6.3 option 2 — propagate `TransportError` out of
    `requester_process_message` so TimedOut can be discriminated.
  - Design §9.1 — pumping-`DirectMctpClient` variant (Shape B).
  - Design §9.3 — external trigger mechanism (IPC / GPIO / policy).
  - Design §9.4 — dual-role configuration.
- Update [INITIALIZATION.md](INITIALIZATION.md) with a new "Polling mode
  (requester)" subsection mirroring the responder's, so the glossary
  stays truthful.
- Update the build-modes table in both `main.rs` module docs and
  `INITIALIZATION.md` to list the new requester row.

**Files**: `INITIALIZATION.md`, `src/main.rs` (module-level doc comment
only).

**Exit criteria**: follow-up issues filed; docs reviewed; no
undocumented feature flag.

**Risks / rollback**: none — docs and issue filing.

---

## Phase Dependency Graph

```
Phase 0 ──► Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6 ──► Phase 7
                          │                                    ▲
                          └────────────────────────────────────┘
                    (Phase 5 can start after Phase 2 in parallel
                     with Phase 3/4 if a separate hand is available)
```

Phases 1 and 2 are small (half-day each). Phase 3 is the bulk of the
work — expect a full day including on-target debug. Phases 4–6 are
each half-day. Phase 7 is a loose-ends sweep.

---

## A. Deferred Measurements (on-target bringup)

These values are **not** required to land Phases 0–5. Capture them once
hardware execution is available (Phase 6 onward) and revisit §6.4 of the
design doc if any value contradicts the back-of-envelope `10_000`
default.

| Metric | Measured value | Source |
|---|---|---|
| `idle_polls` increment rate (per wall-clock second) | _deferred_ | responder log, counted over a fixed window |
| `wait_for_messages` average return latency when idle | _deferred_ | derived from idle_polls rate |
| Largest expected VCA message fragment count | _deferred_ | `receiver.decode` SOM/EOM logs |
| Observed peak `await_steps` on happy path | _deferred_ | requester log at `Done` transition |
| Tuned `AWAIT_STEP_BUDGET` (≥ ~3× observed peak) | _deferred_ | computed |

Until these exist, the design ships with the default and relies on the
Phase-4 state-timeout log line to surface a bad choice loudly rather
than silently.
