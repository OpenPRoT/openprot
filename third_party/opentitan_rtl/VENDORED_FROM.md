# Vendored RTL Constants

These constants are used to generate QEMU input files. To avoid bringing in large SystemVerilog (`.sv`) and HJSON files from the OpenTitan repository into the build flow, the necessary constants have been extracted into a single JSON file: `qemu_constants.json`.

This JSON file is a **static copy** of constants extracted from a specific upstream commit of OpenTitan. OpenTitan is not a Bazel dependency of openprot.

## Upstream Source

| Repository | URL |
|---|---|
| lowRISC/opentitan | https://github.com/lowRISC/opentitan |

**Upstream commit:** `76d47da22f2310207ae72224f3835969f5a6d274`

## File Provenance

| File in this directory | Extracted From Upstream Paths | Notes |
|---|---|---|
| `qemu_constants.json` | `hw/ip/otp_ctrl/rtl/otp_ctrl_part_pkg.sv`<br>`hw/ip/lc_ctrl/rtl/lc_ctrl_state_pkg.sv`<br>`hw/top_earlgrey/data/autogen/top_earlgrey.gen.hjson` | Extracted using `qemu_constants_dumper.py` tool. |

> **RMA OTP image:** `img_rma.vmem` is consumed directly from the devbundle as `@opentitan_devbundle//:earlgrey/otp/img_rma.24.vmem`. The devbundle pin in `MODULE.bazel` is the single source of truth.

## Uprev and Regeneration Procedure

The constants in `qemu_constants.json` are silicon-locked. Regenerate them only if the upstream RTL constants have meaningfully changed — typically they won't.

When regeneration is genuinely needed:

1.  Temporarily copy the upstream files verbatim to their historical locations (only needed for the dumper tool to find them, or you can pass their paths explicitly to the dumper tool):
    *   `hw/ip/otp_ctrl/rtl/otp_ctrl_part_pkg.sv`
    *   `hw/ip/lc_ctrl/rtl/lc_ctrl_state_pkg.sv`
    *   `hw/top_earlgrey/data/autogen/top_earlgrey.gen.hjson`
    (You can copy them anywhere, e.g. to a temporary `tmp/` directory).

2.  Run the dumper tool `//target/earlgrey/tooling:qemu_constants_dumper` to generate the new JSON file. Pass the temporary paths explicitly:
    ```bash
    bazel run //target/earlgrey/tooling:qemu_constants_dumper -- \
      --json \
      --out $PWD/third_party/opentitan_rtl/qemu_constants.json \
      --top earlgrey \
      --topcfg /path/to/temporary/top_earlgrey.gen.hjson \
      --otpconst /path/to/temporary/otp_ctrl_part_pkg.sv \
      --lifecycle /path/to/temporary/lc_ctrl_state_pkg.sv \
      /path/to/temporary/opentitan_root_if_needed
    ```
    *(Note: If you stage the files in a mock OpenTitan directory structure, you can omit the explicit paths and just pass the root directory path as positional argument).*

3.  Delete the temporary `.sv` and `.hjson` files. Do **NOT** commit them to the repository.

4.  Update the **Upstream commit** hash in this file (`VENDORED_FROM.md`) to capture the commit from which the new constants were obtained.

5.  Commit the updated `qemu_constants.json` and `VENDORED_FROM.md`.

## Why JSON Constants, Not RTL Files

Previously, `cfggen.py` (from the QEMU release archive) consumed the raw `.sv` and `.hjson` files at build time to generate QEMU readconfig files. This required bringing large RTL files into the repository and parsing them during the build, which introduced a build-time dependency on python's `hjson` module and complicated the sandbox environment.

By extracting these constants into `qemu_constants.json` once, we:
1.  Avoid cluttering the repository with large, unused RTL files.
2.  Simplify the target build flow, which now only needs to run a simple, dependency-free Python script (`qemu_cfg_gen.py`) to generate the QEMU INI file from the JSON.
3.  Remove the build-time dependency on python's `hjson` module for target runs (it is only a tool dependency for the dumper now).
