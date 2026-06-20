# OpenPRoT Earlgrey Test Environments

This package defines the test environments available for the OpenTitan Earlgrey target. These environments allow running firmware tests on different platforms (QEMU simulation, Verilator simulation, FPGA boards, and physical silicon) using a unified test runner.

## Overview

Historically, environment-specific logic (like bitstreams, ROMs, and commands) was hardcoded in the main test runner rule (`opentitan_runner.bzl`). This package separates the environment configuration from the runner logic.

Test environments are defined using specific rules in [environments.bzl](target/earlgrey/env/environments.bzl) and instantiated as targets in [BUILD.bazel](target/earlgrey/env/BUILD.bazel). They communicate configuration details to the runner via the `TestEnvironmentInfo` provider.

## `TestEnvironmentInfo` Provider

The `TestEnvironmentInfo` provider contains the following fields:

*   **`interface`**: (String) The interface type (e.g., `"qemu"`, `"verilator"`, `"hyper310"`, `"hyper340"`, `"teacup"`).
*   **`runfiles`**: (Depset) Runtime dependencies needed by this environment (e.g., simulator binaries, ROMs, bitstreams).
*   **`setup_cmds`**: (List of strings) Commands to run during the board setup phase (wrapped in `--exec` by the runner).
*   **`boot_cmd`**: (String) Template command to boot the firmware (e.g., `"bootstrap {boot_image}"`).
*   **`rom_ext`**: (File) Optional ROM_EXT binary to assemble with the firmware.
*   **`use_custom_runner`**: (Bool) True if this environment uses a custom runner script (like QEMU) instead of `opentitantool` directly.
*   **`runner`**: (File) The custom runner executable.
*   **`runner_args`**: (List of strings) Arguments for the custom runner.
*   **`test_args`**: (List of strings) Extra arguments for the test command.
*   **`need_flash`**: (Bool) True if the environment requires a flash image to be generated from the firmware.
*   **`prepare`**: (Function) Callback to perform build-time preparation (see below).

## Build-time Preparation (`prepare` callback)

Each environment rule attaches a `prepare` function to the `TestEnvironmentInfo` provider. The runner calls this function during the analysis phase to perform platform-specific preparation:

```python
prep = env.prepare(ctx, env, firmware_bin, tools)
```

The `prepare` function returns a struct with:
*   `boot_image`: The prepared image to boot (e.g., assembled boot image or flash image).
*   `extra_runfiles`: List of files generated during preparation that must be included in runfiles.
*   `output_groups`: Dictionary of output groups to expose.

For example, the QEMU environment's `prepare` function generates a flash image using `flashgen`, while the FPGA environment's `prepare` function performs `image assemble` using `opentitantool` if `rom_ext` is present.

## Available Environments

The following environment targets are defined in [BUILD.bazel](target/earlgrey/env/BUILD.bazel):

### QEMU (`:qemu`)
*   **Type**: `qemu_environment`
*   **Description**: QEMU emulator simulation.
*   **Configuration**: Uses a custom python runner (`qemu_runner`) to launch QEMU with a generated flash image, OTP image, and configuration.
*   **Constraints**: Marked `testonly = True` because it depends on test-only tooling.

### Verilator (`:verilator`)
*   **Type**: `verilator_environment`
*   **Description**: Verilator RTL simulation.
*   **Configuration**: Uses `opentitantool` with verilator-specific arguments (ROM, OTP, flash).

### FPGA CW310 (`:hyper310`)
*   **Type**: `fpga_environment`
*   **Description**: CW310 FPGA board using HyperDebug.
*   **Configuration**: Includes the CW310 bitstream and the signed `rom_ext` binary. Performs build-time image assembly.

### FPGA CW340 (`:hyper340`)
*   **Type**: `fpga_environment`
*   **Description**: CW340 FPGA board using HyperDebug.
*   **Configuration**: Includes the CW340 bitstream and the signed `rom_ext` binary. Performs build-time image assembly.

### Silicon Teacup (`:teacup`)
*   **Type**: `silicon_environment`
*   **Description**: Physical silicon target (Teacup board).
*   **Configuration**: Performs transport initialization and uses rescue firmware boot.

## Usage in Tests

To use an environment in a test, specify it in the `environment` attribute of the `opentitan_test` (or `opentitan_runner`) rule. The `interface` attribute must match the environment's interface.

```python
load("//target/earlgrey/tooling:opentitan_runner.bzl", "opentitan_test")

opentitan_test(
    name = "my_test",
    interface = "hyper310",
    environment = "//target/earlgrey/env:hyper310",
    target = ":my_firmware_image",
)
```
