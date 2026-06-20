# USB Device Test (`usbdev`)

This test verifies the USB device functionality on the OpenTitan Earlgrey target. It ensures that the USB device can be enumerated by the host and that it correctly exposes its Device ID as the USB serial number descriptor.

## Test Components

The test consists of two parts:

1.  **Firmware (`test_usb.rs`)**: Runs on the Ibex core of the OpenTitan.
    *   Maps and accesses the Lifecycle Controller (`lc_ctrl`) to read the 256-bit Device ID.
    *   Formats the Device ID as a 64-character hex string and prints it to the console (`Serial Number: <hex>`).
    *   Converts the Device ID bytes into a UTF-16 USB string descriptor.
    *   Initializes the USB device controller and exposes the Device ID as the USB Serial Number descriptor.

2.  **Host Harness (`host_usb_check.rs`)**: Runs on the host machine controlling the test.
    *   Resets the target board to ensure a clean boot and capture all logs.
    *   Monitors the UART console and waits for the `🔄 RUNNING` log.
    *   Uses regex to capture the printed serial number from the console logs: `Serial Number: ([0-9a-fA-F]{64})`.
    *   Enables VBUS on the target board (if supported by the transport).
    *   Polls the host USB bus for the OpenTitan USB device (matching VID/PID).
    *   Once detected, queries the USB device's Serial Number descriptor.
    *   Verifies that the USB Serial Number descriptor matches the serial number captured from the console logs.

## Running the Test

### On CW310 (FPGA) Hardware

To run the test on a connected CW310 board:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/usbdev:usb_hyper310_test
```

The test harness will automatically handle bitstream loading, bootstrapping the firmware, resetting the target, and performing the verification.
