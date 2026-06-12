# Earlgrey HWE Firmware

This directory contains the source code and configuration for the Earlgrey Hardware Enablement (HWE) firmware. It is a multi-process application designed to run on the OpenTitan Earlgrey target, built on top of the Pigweed microkernel (`pw_kernel`).

## File Structure

*   `BUILD.bazel`: Defines the Bazel build targets for the kernel, userspace processes, the combined system image, and tests.
*   `system.json5`: The system configuration file. It defines the memory layout, processes, IPC channels, interrupts, and device memory mappings.
*   `target.rs`: The entry point for the kernel.
*   `logmgr.rs`: The Log Manager process. It owns UART0, runs the IPC log server, and outputs logs to UART0 using a non-blocking, interrupt-driven driver.
*   `usbmgr.rs`: The USB Manager process. It implements a USB CDC-ACM stack and redirects system logs to the USB serial interface.
*   `sysmgr.rs`: The System Manager process (currently a periodic hello-world logger for testing).
*   `platform.rs`: Stub for the Platform Server process.
*   `flash_server.rs`: Stub for the Flash Server process.

## System Configuration (`system.json5`)

The system configuration defines a single application `hwe` containing five processes:

1.  **`logmgr`**: Manages system-wide logging.
    *   Exposes IPC channels (`logger_flash`, `logger_platform`, `logger_sysmgr`, `logger_usb`) to receive serialized zfmt logs.
    *   Maps the `uart0` MMIO region (`0x40000000`, size `0x1000`) and the `Uart0TxDone` PLIC interrupt (IRQ 3, mapped as `uart_interrupts`) to drive interrupt-driven logging output.
    *   Maps the `rv_timer` device.
2.  **`usbmgr`**: Manages USB and provides USB logging.
    *   Queries logs from `logmgr` via a client channel interface.
    *   Maps the `usbdev` and `pinmux` MMIO regions.
    *   Handles USB interrupts to drive the CDC-ACM stack.
3.  **`sysmgr`**: Intended for system management. It has a channel initiator to `logmgr` and maps `retram`, `rstmgr`, and `lc_ctrl` devices.
4.  **`platform`**: Intended for platform-specific services. It has a channel initiator to `logmgr`.
5.  **`flash_server`**: Intended to provide flash access. It has a channel initiator to `logmgr`, a `flash_service` channel handler, maps the `flash_ctrl_core` device, and handles the `flash_ctrl_op_done` interrupt.

## Optimization & Design Constraints

*   **`multi_process_app` Space Optimization**: To optimize code space, the HWE firmware is built using Pigweed's `multi_process_app` macro. This links all userspace processes into a single binary image, allowing them to share the code of common libraries (like Pigweed libraries and `util_zfmt`) instead of having duplicate copies in each process's private memory space.
*   **Logging Channel Constraint**: A consequence of the `multi_process_app` architecture is that we cannot have per-process mutable global state (e.g. `.data` or `.bss` sections) for library configuration. Therefore, it is a strict system requirement that **every userspace process must configure IPC channel `0` as its logging channel** to communicate with `logmgr`. The client-side logging macros in `util_zfmt` are hardcoded to send log payloads over channel `0`.

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
