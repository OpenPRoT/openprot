# USB Serial Test (`usbserial`)

This test verifies the USB CDC-ACM (Virtual Serial Port) functionality on the OpenTitan Earlgrey target. It ensures that the device can enumerate as a USB serial port, that the host can identify it by its unique Device ID (exposed as the USB serial number), and that data can be successfully transmitted bidirectionally.

## Test Components

The test consists of two parts:

1.  **Firmware (`test_usb.rs`)**: Runs on the Ibex core of the OpenTitan.
    *   Maps and accesses the Lifecycle Controller (`lc_ctrl`) to read the 256-bit Device ID.
    *   Formats the Device ID as a 64-character hex string and prints it to the UART console (`Serial Number: <hex>`).
    *   Initializes the USB CDC-ACM device stack, using the raw Device ID bytes to populate the USB Serial Number string descriptor (automatically converted to UTF-16 by the driver).
    *   Enters an echo loop: reads bytes from the USB serial interface and writes them back.

2.  **Host Harness (`host_usb_serial_check.rs`)**: Runs on the host machine controlling the test.
    *   Resets the target board to ensure a clean boot and capture all logs.
    *   Monitors the UART console and waits for the `🔄 RUNNING` log.
    *   Uses regex to capture the printed serial number from the console logs: `Serial Number: ([0-9a-fA-F]{64})`.
    *   Enables VBUS on the target board (if supported by the transport).
    *   Polls the host system's serial ports using the `serialport` crate, looking for a USB serial port (matching OpenTitan VID/PID) that reports the captured serial number.
    *   Once detected, opens the serial port device node (e.g., `/dev/ttyACM2`).
    *   Sends a test string (`"Hello, OpenPRoT USB Serial!"`) to the USB serial port.
    *   Reads back the response and verifies it matches the sent string (echo test).

## Bazel Configuration and Transitions

The host harness (`host_usb_serial_check`) depends on both `opentitanlib` (for target control and UART console monitoring) and the `serialport` crate (for host-side serial communication).

Because `opentitanlib` is compiled under a specific Bazel configuration transition (which configures transport flags), the host harness binary must be built under the same transition to prevent Rust `StableCrateId` collisions between the direct and transitive dependencies on `serialport`.

To hide this complexity, we use the `opentitan_rust_binary` macro in `BUILD.bazel` instead of `rust_binary`. This macro automatically creates the raw target and wraps it in a transition-applying rule, aligning all configurations.

## Running the Test

### On CW310 (FPGA) Hardware

To run the test on a connected CW310 board:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/usbserial:usb_hyper310_test
```

To see verbose log output from the host harness during the test:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/usbserial:usb_hyper310_test --test_arg=--logging=info
```
