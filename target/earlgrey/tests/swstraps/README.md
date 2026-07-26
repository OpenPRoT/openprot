# Software Straps Test (`swstraps`)

This integration test verifies software straps pinmux configuration, GPIO reading, and value reassembly on the OpenTitan Earlgrey target. It ensures that the firmware can correctly read the three two-bit `SW_STRAPn` GPIO pins (`SW_STRAP0`, `SW_STRAP1`, and `SW_STRAP2`) and reassemble the 6-bit strap value (`0x00` to `0x3f`).

## Test Components

The test consists of two main components:

1.  **Firmware (`test_swstraps.rs`)**: Runs on the Pigweed Maize microkernel on the OpenTitan Earlgrey target.
    *   Configures the GPIO and pinmux using `SwStraps::configure` from the `earlgrey_pinout` crate.
    *   Logs the running banner `🔄 RUNNING SWSTRAPS TEST` to the console.
    *   In an infinite loop, reads the strap value using `SwStraps::read_straps(&mut gpio)`.
    *   When a change from the last reported value is detected, enters a 10ms debounce and stabilization loop (`sleep_until(SystemClock::now() + Duration::from_millis(10))`).
    *   Once two consecutive 10ms readings match, prints the stable hex value `SW_STRAP = 0xXX` to the UART console via `pw_log::info!`.

2.  **Host Harness (`host_swstraps_check.rs`)**: Runs on the host machine controlling the test across hardware and simulator environments.
    *   Resets the target board and waits for the initial `RUNNING` banner and initial stable strap value (`SW_STRAP = 0x00`) left by board initialization.
    *   Iterates through all test strapping patterns (`1..64`, followed by `0x00` at the end):
        *   **Verilator / Silicon (`teacup`)**: Tests all 64 possible 2-bit strap combinations (strong zero, weak zero, weak one, and strong one).
        *   **FPGA (`hyper310` / `hyper340`)**: FPGAs do not support weak strapping values. The harness dynamically filters out any pattern containing weak bits (`1` or `2`) and tests only the 8 strong-only patterns (`0x03`, `0x0c`, `0x0f`, `0x30`, `0x33`, `0x3c`, `0x3f`, and `0x00`).
    *   Drives the target's strap pins using `PinMode::PushPull` (for strong 0/1) or `PinMode::Input` with pull-up/pull-down (to simulate weak 0/1 on Verilator).
    *   Monitors the UART console using `UartConsole::wait_for` until the target detects the transition, stabilizes, and logs the expected `SW_STRAP = 0xXX` pattern.
    *   Resets the strap pins back to all zeros (`0x00`) after the test completes.

## Strap Bit Encoding & Pin Mapping

Each of the three software straps (`SW_STRAP0`, `SW_STRAP1`, `SW_STRAP2`) encodes a 2-bit value determined by high and low pull-direction testing:

| Value | Meaning | Description |
| :---: | :--- | :--- |
| `0` (`00`) | **Strong Zero** | Actively driven low (`PushPull`, low) |
| `1` (`01`) | **Weak Zero** | Pulled low via pull-down resistor (`Input`, pull-down) |
| `2` (`10`) | **Weak One** | Pulled high via pull-up resistor (`Input`, pull-up) |
| `3` (`11`) | **Strong One** | Actively driven high (`PushPull`, high) |

*   **Silicon (`teacup`)**: Uses dedicated HyperDebug strong/weak GPIO pin pairs (`SW_STRAP0`/`SW_STRAP0_WEAK`, etc.).
*   **Verilator & FPGA**: Directly drives `IOC0` (`SW_STRAP0`), `IOC1` (`SW_STRAP1`), and `IOC2` (`SW_STRAP2`).

## Running the Test

### On CW310 (FPGA) Hardware

To run the test on a connected CW310 board:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/swstraps:swstraps_hyper310_test
```

### On CW340 (FPGA) Hardware

To run the test on a connected CW340 board:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/swstraps:swstraps_hyper340_test
```

### On Verilator Simulator

To run the full 64-pattern test suite in Verilator simulation:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/swstraps:swstraps_verilator_test
```

### On Silicon (`teacup`) Hardware

To run the full 64-pattern test suite on Teacup silicon:

```bash
bazelisk test --test_output=all --cache_test_results=no //target/earlgrey/tests/swstraps:swstraps_silicon_test
```
