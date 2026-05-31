# Earlgrey HWE Firmware

This directory contains the source code and configuration for the Earlgrey Hardware Enablement (HWE) firmware. It is a multi-process application designed to run on the OpenTitan Earlgrey target, built on top of the Pigweed microkernel (`pw_kernel`).

Currently, the firmware structure is defined, but the individual services are stubs.

## File Structure

*   `BUILD.bazel`: Defines the Bazel build targets for the kernel, userspace processes, the combined system image, and tests.
*   `system.json5`: The system configuration file. It defines the memory layout, processes, IPC channels, interrupts, and device memory mappings.
*   `target.rs`: The entry point for the kernel.
*   `logmgr.rs`: Stub for the Log Manager process.
*   `sysmgr.rs`: Stub for the System Manager process.
*   `platform.rs`: Stub for the Platform Server process.
*   `flash_server.rs`: Stub for the Flash Server process.
*   `usbmgr.rs`: Stub for the USB Manager process.

## System Configuration (`system.json5`)

The system configuration defines a single application `hwe` containing five processes:

1.  **`logmgr`**: Intended to manage logging. It defines several `channel_handler` objects to receive logs from other processes (`logger_flash`, `logger_platform`, `logger_sysmgr`, `logger_usb`). It also maps the `rv_timer` device.
2.  **`sysmgr`**: Intended for system management. It has a channel initiator to `logmgr` and maps `retram`, `rstmgr`, and `lc_ctrl` devices.
3.  **`platform`**: Intended for platform-specific services. It has a channel initiator to `logmgr`.
4.  **`flash_server`**: Intended to provide flash access. It has a channel initiator to `logmgr`, a `flash_service` channel handler, maps the `flash_ctrl_core` device, and handles the `flash_ctrl_op_done` interrupt.
5.  **`usbmgr`**: Intended to manage USB. It has a channel initiator to `logmgr`, maps `usbdev` and `pinmux` devices, and handles various USB interrupts.

## Build and Test

The `BUILD.bazel` file defines several targets:

*   `//target/earlgrey/firmware/hwe:hwe_firmware`: The main system image target.
*   `//target/earlgrey/firmware/hwe:target`: The kernel binary.

### Tests

Several `opentitan_test` targets are defined for different environments:

*   `hwe_verilator_test`: Runs in the Verilator simulator.
*   `hwe_hyper310_test`: Runs on the CW310 FPGA board.
*   `hwe_hyper340_test`: Runs on the CW340 FPGA board.
*   `hwe_silicon_test`: Runs on silicon.

You can run these tests using `bazelisk test`. For example:

```bash
bazelisk test //target/earlgrey/firmware/hwe:hwe_verilator_test
```
