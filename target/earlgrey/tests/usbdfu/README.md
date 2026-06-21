# USB DFU Test (`usbdfu`)

This test verifies the USB Device Firmware Upgrade (DFU) functionality on the OpenTitan Earlgrey target. It ensures that the device can enumerate as a DFU device, that the host can perform download (programming) and upload (readback) operations, and that device certificates can be read via alternate interface settings.

## Test Components

The test consists of three main components:

1.  **Firmware (`test_usb.rs`)**: Runs on the Ibex core of the OpenTitan.
    *   Reads the 256-bit Device ID from the Lifecycle Controller (`lc_ctrl`).
    *   Formats the Device ID as a 64-character hex string and prints it to the UART console (`Serial Number: <hex>`).
    *   Initializes the USB DFU device stack, exposing Alt 0 for flash programming, and Alt 1-3 for reading certificates.
    *   On Alt 0:
        *   Receives DFU `DNLOAD` blocks (2048 bytes). Block 0 triggers an erase of 64KB starting at `0xA0000` (Bank 1).
        *   Subsequent blocks are programmed to flash at `0xA0000` using the `flash_server` service.
        *   Supports `UPLOAD` to read back the programmed data.
    *   On Alt 1-3:
        *   Supports `UPLOAD` to read UDS, CDI_0, and CDI_1 certificates. If a certificate is blank (expected for UDS on FPGA), it stalls the control pipe, which is handled gracefully by the host.

2.  **Flash Server (`flash_server.rs`)**: A separate userspace process running on the target.
    *   Acts as a secure intermediary for flash operations.
    *   Receives IPC requests from the DFU process to erase and write flash.
    *   Interacts directly with the hardware flash controller.

3.  **Host Harness (`host_usb_dfu_check.rs`)**: Runs on the host machine.
    *   Resets the target board.
    *   Monitors the UART console to capture the unique Serial Number.
    *   Finds the DFU device on the USB bus matching the captured Serial Number.
    *   Claims the DFU interface and parses the DFU block size (transfer size) from the DFU functional descriptor.
    *   Generates 64KB of test data.
    *   Performs DFU download of the test data.
    *   Sends a Zero-Length Packet (ZLP) to signal the end of download.
    *   Waits for manifestation to complete and verifies the device returns to the `Idle` state.
    *   Performs DFU upload to read back the 64KB and verifies it matches the original data.
    *   Performs DFU upload on Alt 1-3 to read certificates, verifying they can be read (or stalled if blank) without crashing the target.

## Running the Test

### On CW310 (FPGA) Hardware

To run the test on a connected CW310 board:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/usbdfu:usb_hyper310_test
```

To see verbose log output from the host harness and target console during the test:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/usbdfu:usb_hyper310_test --test_arg=--logging=info
```
