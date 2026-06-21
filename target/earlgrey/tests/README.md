# OpenTitan Earlgrey Integration Tests

This directory contains integration tests for the OpenTitan Earlgrey target, running on the Pigweed Maize microkernel.

## Test Directory Structure

Each subdirectory here represents a test suite or a specific test target:

*   **[`drivers/gpio`](drivers/gpio)**: Tests the GPIO driver functionality, verifying that pins can be configured, read, written, and toggled.
*   **[`eflash`](eflash)**: Tests the embedded flash driver operations (erase, program, read) using a secure `flash_server` userspace process.
*   **[`ipc/user`](ipc/user)**: Tests userspace IPC channel communication between different processes on the microkernel.
*   **[`logging`](logging)**: Tests the microkernel logging subsystems and `pw_log` routing.
*   **[`threads/kernel`](threads/kernel)**: Tests kernel-level threading, context switching, and scheduling invariants.
*   **[`uart`](uart)**: Tests UART driver initialization and loopback communication (using interrupt-driven RX/TX).
*   **[`unittest_runner`](unittest_runner)**: Runs the upstream Pigweed kernel unit and integration test suite on the target hardware.
*   **[`usbdev`](usbdev)**: Tests basic USB device controller initialization and host-side enumeration.
*   **[`usbdfu`](usbdfu)**: Tests USB Device Firmware Upgrade (DFU) protocol, including loopback download/upload verification and certificate reading.
*   **[`usbserial`](usbserial)**: Tests USB CDC-ACM (Virtual Serial Port) bidirectional data transmission using an echo loop.

---

## Guide for Writing New Tests

When writing new integration tests, you can choose between two main patterns depending on whether the test requires host-side interaction (like USB or JTAG) or is self-contained on the target.

### 1. Self-Contained Tests

Self-contained tests run entirely on the Ibex core and report their results via the UART console. The Bazel test runner (`opentitan_test` macro) monitors the console output to determine success or failure.

*   **Success Criteria**: By default, the test runner looks for the string `PASS\n` in the console output. Ensure your firmware prints this (e.g., `pw_log::info!("✅ TEST PASS")` or similar) when all assertions succeed.
*   **Failure Criteria**: By default, the test runner matches the regex `FAIL: .+\n` for failures. If an assertion fails or a panic occurs, ensure the output matches this pattern.
*   **Customization**: You can override these defaults in the `opentitan_test` rule using `exit_success` and `exit_failure` attributes.

Example firmware structure:
```rust
#[entry]
fn entry() -> Result<(), Error> {
    pw_log::info!("🔄 MY_TEST START");
    let result = run_my_test_logic();
    match result {
        Ok(()) => {
            pw_log::info!("PASS"); // Triggers Bazel test success
            Ok(())
        }
        Err(e) => {
            pw_log::error!("FAIL: {:?}", e); // Triggers Bazel test failure
            Err(Error::Unknown)
        }
    }
}
```

### 2. Tests with Host-Based Test Harnesses

If your test involves external interfaces (e.g., verifying USB enumeration, DFU transfers, or JTAG operations), you must write a host-based test harness that runs on the host machine and coordinates with the target.

*   **Harness Definition**: Define the host harness as a Rust binary using `opentitan_rust_binary` (which applies the necessary transition for `opentitanlib` compatibility) and link it via the `test_harness` attribute of `opentitan_test`.
*   **Target Reset**: The host harness should start by resetting the target (e.g., `transport.reset(UartRx::Clear)?`) to ensure a clean boot and capture early boot logs.
*   **Device Correlation (Best Practice)**: On systems with multiple connected devices, it is critical to ensure the host harness talks to the correct target.
    *   **Firmware**: Read the unique 256-bit Device ID from the Lifecycle Controller (`lc_ctrl`) and print it to the console (e.g., `Serial Number: <hex>`) early in the boot sequence. If using USB, also use this Device ID to populate the USB Serial Number string descriptor.
    *   **Harness**: Monitor the UART console first, capture the printed serial number using regex, and then use that serial number to look up and open the correct USB/JTAG device (e.g., `device_by_id_with_timeout(vid, pid, Some(serial), timeout)`).
*   **Console Monitoring**: Use `UartConsole::wait_for` to synchronize host actions with target states (e.g., waiting for `🔄 RUNNING` before starting USB transfers).
