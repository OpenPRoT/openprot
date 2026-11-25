# VeeR tests on the VCK190 FPGA

Tools for running `//target/veer` tests on a Caliptra Subsystem VCK190 FPGA
board, from your workstation, with `bazel test`.

The same test targets also run on the Caliptra emulator. Each test is defined
once and generates a `<name>_emulator` and a `<name>_fpga` target.

## How it works

Every test is a host-side Rust test binary that drives an MCU firmware image
through [`caliptra_mcu_hw_model`]:

| | `<name>_emulator` | `<name>_fpga` |
|---|---|---|
| Test binary runs on | your workstation (x86_64) | the VCK190's ARM cores (aarch64) |
| Hardware model | `ModelEmulated` | `ModelFpgaRealtime`, over UIO |
| MCU image memory map | emulator | FPGA |
| Caliptra ROM/FW, MCU ROM | emulator builds | FPGA builds |

`bazel test --config=fpga` cross-compiles the test binaries and FPGA images on
your workstation, then runs each test on the board through
[`run_remote.sh`](run_remote.sh) (set as `--run_under` in `//.bazelrc`):

1. `run_remote.sh` copies the test's runfiles tree to
   `~/openprot-fpga/runfiles/` on the board with `rsync`. The tree contains the
   test binary, the signed MCU image, the boot ROMs and firmware, the bitstream,
   and [`run_on_board.sh`](run_on_board.sh). Files that haven't changed since the
   last test aren't copied again.
2. It runs `run_on_board.sh` on the board as root, over one SSH connection that
   is reused across tests. `run_on_board.sh`:
   1. Takes `/run/lock/openprot-fpga.lock`, so only one test uses the board at
      a time, even across separate `bazel test` invocations.
   2. Loads the bitstream into the FPGA, unless it's already loaded. It
      compares against `/lib/firmware/caliptra-bitstream.pdi` and
      `/dev/shm/fpga-config`, the same files upstream `xtask fpga bootstrap`
      uses.
   3. Runs the test binary. Output streams back to Bazel, and the test's exit
      code becomes the Bazel result.

Infrastructure failures (SSH, missing bitstream, board busy) exit with code 2.

## Board setup

You need:

- A VCK190 running the **Caliptra FPGA image** from caliptra-sw (hostname
  `caliptra-fpga`), built by
  [this](https://github.com/chipsalliance/caliptra-sw/actions/workflows/fpga-image.yml)
  workflow. It provides the kernel modules and runtime bitstream loading. Stock
  Ubuntu images are not supported.
- **Key-based SSH** to the board. An alias in `~/.ssh/config` works.
  `run_remote.sh` uses `BatchMode=yes`, so it fails instead of prompting.
- **Passwordless `sudo`** on the board, for UIO access and bitstream loading.
- `rsync` on both your workstation and the board.

Then tell Bazel which board to use. Add this to `.bazelrc-site` at the repo
root (it's gitignored):

```
test:fpga --test_env=VCK190_FPGA_HOST=<ssh-host-or-alias>
```

Or pass `--test_env=VCK190_FPGA_HOST=<host>` on each command.

## Running tests

```sh
# All FPGA tests.
bazel test --config=fpga //target/veer/...

# One test, with live output.
bazel test --config=fpga --test_output=streamed \
    //target/veer/unittest_runner:host_smoke_test_fpga

# Filter Rust #[test] functions inside one target.
bazel test --config=fpga --test_arg=probe_mci_and_boot \
    //target/veer/unittest_runner:host_smoke_test_fpga

# Build everything for the FPGA without a board
bazel build --config=fpga //target/veer/...

# The same tests on the emulator. FPGA tests are skipped by default.
bazel test //target/veer/...
```

`--config=fpga` selects tests tagged `fpga`, runs them one at a time, and gives
each a 600s timeout. Without it, `fpga` tests are filtered out.

## Writing tests

Tests live under `//target/veer/tests/<area>/`, next to the system image they
boot. Load the macros from `//target/veer/tooling:caliptra_test.bzl`.

### 1. Define the image

Use `veer_system_image` from `//target/veer:defs.bzl` instead of a plain
`system_image`. It also defines signed `<name>_image_emulator` and
`<name>_image_fpga` targets that the test macros pick up:

```starlark
veer_system_image(
    name = "my_feature",
    kernel = ":target",
    platform = "//target/veer",
)
```

### 2a. On-device test: the firmware decides pass/fail

If the firmware exits with PASS or FAIL on its own, use
`caliptra_on_device_test`. It boots the image and waits for the exit status;
there's no Rust to write:

```starlark
caliptra_on_device_test(
    name = "my_feature_test",
    mcu_image = ":my_feature",
)
# -> :my_feature_test_emulator, :my_feature_test_fpga
```

### 2b. Host-driven test: your Rust drives the firmware

If the host has to do something (poke registers, act as the BMC, send
mailbox commands), use `caliptra_test` with your own sources:

```starlark
caliptra_test(
    name = "my_host_test",
    srcs = ["my_host_test.rs"],
    mcu_image = ":my_feature",
)
```

```rust
use caliptra_mcu_hw_model::McuHwModel;
use test_boot_files::BootFiles;

#[test]
fn my_host_test() {
    // ROMs, firmware, MCU image, SoC manifest and vendor PK hash for
    // `mcu_image`, found through runfiles.
    let files = BootFiles::from_env();
    let mut hw = caliptra_mcu_hw_model::new(files.init_params()).unwrap();
    // ... drive `hw` ...
    hw.step_until_exit_success().unwrap();
}
```

Use `caliptra_mcu_hw_model::new_unbooted` if you need to do something before
boot. See [`host_smoke_test.rs`](../../tests/unittest_runner/host_smoke_test.rs)
for an example.

### Options

Both macros take:

- `interfaces`: defaults to `["emulator", "fpga"]`. Use `["emulator"]` for a
  test that depends on emulator-only peripherals.
- `tags`, `env`, `data`, `deps`, `timeout`, and other `rust_test` arguments.

They add automatically, so don't set these yourself: the `emulator` / `fpga`
tag, `exclusive` on FPGA tests, `RUST_TEST_THREADS=1`, `CALIPTRA_BOOT_ENV`,
the `caliptra_mcu_hw_model` and `test_boot_files` deps, and (for FPGA) the
board-side files.

## Updating the bitstream

The bitstream is the `@caliptra_fpga_bitstream` `http_file` in
`//MODULE.bazel`. It has to match the `caliptra_mcu_sw` pin: take the URL and
sha256 from `hw/fpga/bitstream_manifests/subsystem.toml` at the pinned commit,
and bump them together with the pin.

## Troubleshooting

| Symptom | Fix |
|---|---|
| `VCK190_FPGA_HOST: is not set` | Set it in `.bazelrc-site` (see [Board setup](#board-setup)). |
| `SSH connection to <host> failed` | Check that `ssh -o BatchMode=yes <host> true` works without a prompt. |
| `board is busy` | Another test held the board lock for over 120s. Wait, or find the stuck process on the board. |
| `unsupported board image` | Flash the Caliptra FPGA image; see [Board setup](#board-setup). |
| Tests fail after someone used `xtask` on the board | `run_on_board.sh` reloads our bitstream if it doesn't match, but `xtask` users will need to bootstrap again afterwards. To force a reload: `ssh <host> sudo rm /dev/shm/fpga-config`. |

## Known gaps

- `//target/veer/tests/interrupts/kernel` is emulator-only: it uses an
  emulator-only flash controller to raise an interrupt.
- `//target/veer/tests/threads/kernel:threads_test` is tagged `manual`.
- `//target/veer/syscall_latency:measure_syscall_latency_test` is tagged
  `disabled` (kernel exception on process exit).

[`caliptra_mcu_hw_model`]: https://github.com/chipsalliance/caliptra-mcu-sw/tree/main/hw/model
