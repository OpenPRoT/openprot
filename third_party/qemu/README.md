# QEMU

For instructions on setting up QEMU for local development and troubleshooting steps, see the [setup guide](./setup.md).

## Introduction

[QEMU](https://www.qemu.org/) is an open-source project providing fast, functional full-system emulation.
OpenPRoT uses the lowRISC QEMU fork's `ot-earlgrey` machine to run earlgrey firmware tests without requiring Verilator or physical hardware.

The QEMU binary is downloaded from the lowRISC GitHub releases as a pre-built archive:

| Item | Value |
|---|---|
| Tag | `v10.2.0-2026-01-15` |
| Archive | `qemu-ot-earlgrey-v10.2.0-2026-01-15-x86_64-unknown-linux-gnu.tar.gz` |
| SHA-256 | `9e97f93b09912c904e84f06571e7b49023ccb405dd3caa232ad1e82a3f7b381c` |

The Bazel targets `//third_party/qemu:cfggen`, `//third_party/qemu:flashgen`, and `//third_party/qemu:otptool` wrap the Python scripts bundled in the release archive.

## Scope

Only the Earlgrey machine (`ot-earlgrey`) is supported.
Emulation is ongoing and incomplete — many peripherals are not yet fully emulated.
Before relying on QEMU results, verify that the peripherals your test exercises are supported.

## Key limitations

- **Not cycle-accurate.** `mcycle` and hardware timers are approximate.
  OpenPRoT uses `icount shift=6` to pace virtual time to wall-clock time.
- **PMP granularity.** Misaligned PMP regions disable TLB caching, causing slowdowns.
  Fine-grained PMP regions in earlgrey ROM/ROM_EXT can produce significant slowdowns.
- **UART CharDev.** UART0 is wired via a Unix socket chardev; the runner opens it for output.
  UART oversampling is not emulated.
- **QEMU starts paused** (`-S` flag). The runner must send `cont\n` to the monitor socket
  before any firmware output appears. Forgetting this produces silent hangs.

## Files

| File | Purpose |
|---|---|
| `extensions.bzl` | Bazel module extension — fetches the pinned QEMU release archive or builds from a local source override |
| `BUILD.qemu_opentitan.bazel` | BUILD file placed inside the fetched QEMU repo |
| `build_qemu.sh` | Shell script that configures + builds QEMU from a local source checkout |
| `BUILD` | Bazel targets for `cfggen`, `flashgen`, `otptool`, `qemu-system-riscv32` |
| `requirements.in` | Unlocked Python deps for cfggen/flashgen |
| `requirements.txt` | Locked + hashed Python deps (input to `rules_python` pip.parse) |

## Local development override

To iterate on QEMU itself without waiting for a new release archive, pass an `--override_repository`
flag pointing at your local source checkout:

```sh
bazelisk build \
  --override_repository=qemu_opentitan_src=/path/to/your/qemu \
  @qemu_opentitan//:build/qemu-system-riscv32
```

The `extensions.bzl` module extension checks for this override; when set it runs `build_qemu.sh`
against the local tree instead of fetching the pinned archive.
The resulting binary is used by all `//third_party/qemu:qemu-system-riscv32` consumers in the
same build, including `ipc_runner_qemu_test`.

For a full walkthrough (configuring build deps, setting `QEMU_CFLAGS`, etc.) see [setup.md](./setup.md).

## Ported from

These files were ported from `opentitan/third_party/qemu/` at the same upstream revision
as the pinned QEMU tag above. When upgrading the QEMU version, re-port `extensions.bzl`
(update the tag + sha256), update `requirements.txt` if script deps changed, and re-verify
`cfggen`/`flashgen`/`otptool` still work against the new scripts.
