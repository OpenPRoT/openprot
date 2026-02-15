#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
"""
AST1060 UART Test Execution Script

Provides GPIO control for SRST/FWSPICK pins and automated firmware upload
with test execution monitoring via pyserial.

This script is designed to be invoked by Bazel test rules or used standalone.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional, Tuple

try:
    import serial
except ImportError:
    print("Error: pyserial not installed. Install with: pip install pyserial")
    sys.exit(1)


class UartTestExecutor:
    """Handles AST1060 UART test execution with GPIO control."""

    # Default success/failure patterns for test monitoring
    SUCCESS_PATTERNS = ["COMPLETE", "TEST PASSED", "All tests passed"]
    FAILURE_PATTERNS = ["panic", "FAIL", "ERROR", "abort"]

    def __init__(self, args):
        self.args = args
        self.serial_port: Optional[serial.Serial] = None
        self.log_file = getattr(args, "log_file", None) or f"uart-test-{os.getpid()}.log"
        self.log_file_handle = None

    def log(self, message: str):
        """Print message unless in quiet mode."""
        if not self.args.quiet:
            print(message, flush=True)

    def run_command(self, cmd: list, check: bool = True) -> Tuple[int, str, str]:
        """Run command and return (returncode, stdout, stderr)."""
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=False)
            if check and result.returncode != 0:
                raise subprocess.CalledProcessError(
                    result.returncode, cmd, result.stdout, result.stderr
                )
            return result.returncode, result.stdout, result.stderr
        except FileNotFoundError:
            raise RuntimeError(f"Command not found: {cmd[0]}")

    def gpio_set(self, pin: int, state: str):
        """Set GPIO pin state using pinctrl."""
        cmd = ["pinctrl", "set", str(pin), "op", state]
        self.log(f"GPIO {pin}: {state}")

        if self.args.dry_run:
            self.log(f"DRY RUN: {' '.join(cmd)}")
            return

        try:
            self.run_command(cmd)
        except Exception as e:
            raise RuntimeError(f"Failed to set GPIO {pin} to {state}: {e}")

    def toggle_srst(self, state: str):
        """Toggle SRST pin (dl=low, dh=high)."""
        self.gpio_set(self.args.srst_pin, state)

    def toggle_fwspick(self, state: str):
        """Toggle FWSPICK pin (dh=high, dl=low)."""
        if state == "dh":
            cmd_state = "pn dh"
        else:
            cmd_state = state

        cmd = ["pinctrl", "set", str(self.args.fwspick_pin), "op"] + cmd_state.split()
        self.log(f"GPIO {self.args.fwspick_pin} (FWSPICK): {state}")

        if self.args.dry_run:
            self.log(f"DRY RUN: {' '.join(cmd)}")
            return

        try:
            self.run_command(cmd)
        except Exception as e:
            raise RuntimeError(
                f"Failed to set FWSPICK {self.args.fwspick_pin} to {state}: {e}"
            )

    def sequence_to_fwspick_mode(self):
        """Execute sequence to enter FWSPICK mode."""
        self.log("Entering FWSPICK mode sequence...")
        self.toggle_srst("dl")  # SRST low
        time.sleep(0.1)
        self.toggle_fwspick("dh")  # FWSPICK high
        time.sleep(1)
        self.toggle_srst("dh")  # SRST high
        time.sleep(1)
        self.log("FWSPICK mode sequence complete")

    def sequence_to_normal_mode(self):
        """Execute sequence to enter normal boot mode."""
        self.log("Entering normal boot mode sequence...")
        self.toggle_fwspick("dl")  # FWSPICK low
        time.sleep(0.1)
        self.toggle_srst("dl")  # SRST low
        time.sleep(0.5)
        self.toggle_srst("dh")  # SRST high
        time.sleep(2)
        self.log("Normal boot mode sequence complete")

    def open_serial(self) -> bool:
        """Open serial port connection."""
        if self.args.skip_uart:
            self.log("Skipping UART setup")
            return True

        if self.args.dry_run:
            self.log("DRY RUN: Would open serial port")
            return True

        try:
            self.serial_port = serial.Serial(
                port=self.args.uart_device,
                baudrate=self.args.baudrate,
                timeout=1.0,
                write_timeout=1.0,
            )

            # Open log file
            self.log_file_handle = open(self.log_file, "w")

            self.log(
                f"Serial port opened: {self.args.uart_device} @ {self.args.baudrate} baud"
            )
            self.log(f"Logging to: {self.log_file}")
            return True

        except Exception as e:
            raise RuntimeError(f"Failed to open serial port: {e}")

    def close_serial(self):
        """Close serial port connection."""
        if self.serial_port:
            self.serial_port.close()
            self.serial_port = None

        if self.log_file_handle:
            self.log_file_handle.close()
            self.log_file_handle = None

    def read_serial_data(self, timeout_seconds: float = 1.0) -> str:
        """Read available data from serial port."""
        if not self.serial_port or self.args.skip_uart:
            return ""

        if self.args.dry_run:
            return ""

        try:
            self.serial_port.timeout = timeout_seconds
            data = self.serial_port.read(1024)

            if data:
                decoded = data.decode("utf-8", errors="ignore")

                # Log to file
                if self.log_file_handle:
                    self.log_file_handle.write(decoded)
                    self.log_file_handle.flush()

                return decoded

            return ""

        except Exception as e:
            self.log(f"Serial read error: {e}")
            return ""

    def write_serial_data(self, data: bytes) -> bool:
        """Write data to serial port."""
        if not self.serial_port or self.args.skip_uart:
            return True

        if self.args.dry_run:
            self.log(f"DRY RUN: Would write {len(data)} bytes to serial")
            return True

        try:
            bytes_written = self.serial_port.write(data)
            self.serial_port.flush()
            return bytes_written == len(data)

        except Exception as e:
            self.log(f"Serial write error: {e}")
            return False

    def wait_for_uart_ready(self, timeout: int = 30) -> bool:
        """Wait for 'U' character indicating UART bootloader ready."""
        if self.args.skip_uart:
            self.log("Skipping UART ready check")
            return True

        self.log(f"Waiting for UART ready signal ('U') with {timeout}s timeout...")

        if self.args.dry_run:
            self.log("DRY RUN: Would wait for UART ready")
            return True

        start_time = time.time()
        buffer = ""

        while time.time() - start_time < timeout:
            data = self.read_serial_data(0.1)
            if data:
                buffer += data
                if not self.args.quiet:
                    print(data, end="", flush=True)

                # Look for 'U' character
                if "U" in buffer:
                    self.log("\nUART bootloader ready detected!")
                    return True

        self.log("\nTimeout waiting for UART ready signal")
        return False

    def upload_firmware(self) -> bool:
        """Upload firmware via serial port."""
        if self.args.skip_uart or not self.args.firmware:
            self.log("Skipping firmware upload")
            return True

        firmware_path = Path(self.args.firmware)
        if not firmware_path.exists():
            raise RuntimeError(f"Firmware file not found: {firmware_path}")

        self.log(f"Uploading firmware: {firmware_path}")

        if self.args.dry_run:
            self.log("DRY RUN: Would upload firmware")
            return True

        try:
            with open(firmware_path, "rb") as f:
                firmware_data = f.read()

            self.log(f"Uploading {len(firmware_data)} bytes...")

            # Upload firmware in chunks
            chunk_size = 1024
            bytes_sent = 0

            for i in range(0, len(firmware_data), chunk_size):
                chunk = firmware_data[i : i + chunk_size]

                if not self.write_serial_data(chunk):
                    self.log("Failed to write firmware chunk")
                    return False

                bytes_sent += len(chunk)
                time.sleep(0.01)

                # Progress indicator
                if not self.args.quiet and bytes_sent % (chunk_size * 10) == 0:
                    progress = (bytes_sent * 100) // len(firmware_data)
                    print(f"\rProgress: {progress}%", end="", flush=True)

            if not self.args.quiet:
                print()

            self.log("Firmware upload completed")
            return True

        except Exception as e:
            self.log(f"Failed to upload firmware: {e}")
            return False

    def monitor_test_execution(self, timeout: int = 600) -> bool:
        """Monitor test execution via serial port."""
        if self.args.skip_uart:
            self.log("Skipping test monitoring")
            return True

        actual_timeout = getattr(self.args, "test_timeout", timeout)
        self.log(f"Monitoring test execution with {actual_timeout}s timeout...")

        if self.args.dry_run:
            self.log("DRY RUN: Would monitor test execution")
            return True

        start_time = time.time()
        buffer = ""
        test_results = {"passed": 0, "failed": 0, "skipped": 0}

        while time.time() - start_time < actual_timeout:
            data = self.read_serial_data(0.5)
            if data:
                buffer += data
                if not self.args.quiet:
                    print(data, end="", flush=True)

                lines = buffer.split("\n")
                for line in lines:
                    if "PASS" in line:
                        test_results["passed"] += 1
                    elif "FAIL" in line:
                        test_results["failed"] += 1
                    elif "SKIP" in line:
                        test_results["skipped"] += 1

                    # Check for completion
                    for pattern in self.SUCCESS_PATTERNS:
                        if pattern in line:
                            self.log(f"\nTest execution completed!")
                            self.log(f"Results: {test_results}")
                            return test_results["failed"] == 0

                    # Check for failure
                    for pattern in self.FAILURE_PATTERNS:
                        if pattern.lower() in line.lower():
                            self.log(f"\nFailure detected: {pattern}")
                            return False

                buffer = "\n".join(lines[-10:])

        self.log(f"\nTest monitoring timeout. Results so far: {test_results}")
        return test_results["failed"] == 0

    def cleanup(self):
        """Clean up resources."""
        self.close_serial()

    def run_full_test_sequence(self) -> bool:
        """Execute the complete test sequence."""
        try:
            if not self.open_serial():
                return False

            self.sequence_to_fwspick_mode()

            if not self.wait_for_uart_ready():
                return False

            if not self.upload_firmware():
                return False

            return self.monitor_test_execution()

        finally:
            self.cleanup()


def main():
    parser = argparse.ArgumentParser(
        description="AST1060 UART Test Execution Script",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Full test sequence
  ./uart_test_exec.py /dev/ttyUSB0 firmware.bin

  # Manual GPIO control
  ./uart_test_exec.py --manual-srst low
  ./uart_test_exec.py --manual-fwspick high

  # Sequence control
  ./uart_test_exec.py --sequence fwspick-mode
  ./uart_test_exec.py --sequence normal-mode

  # Upload-only (no GPIO, no monitoring)
  ./uart_test_exec.py --upload-only /dev/ttyUSB0 firmware.bin

  # Bazel test mode (exit code indicates pass/fail)
  ./uart_test_exec.py --bazel-test /dev/ttyUSB0 firmware.bin
        """,
    )

    # Positional arguments
    parser.add_argument(
        "uart_device", nargs="?", help="UART device path (e.g., /dev/ttyUSB0)"
    )
    parser.add_argument("firmware", nargs="?", help="Firmware binary file path")

    # GPIO control
    parser.add_argument(
        "--srst-pin", type=int, default=23, help="SRST GPIO pin number (default: 23)"
    )
    parser.add_argument(
        "--fwspick-pin",
        type=int,
        default=18,
        help="FWSPICK GPIO pin number (default: 18)",
    )

    # Manual GPIO operations
    parser.add_argument(
        "--manual-srst",
        choices=["low", "high", "dl", "dh"],
        help="Manually toggle SRST pin",
    )
    parser.add_argument(
        "--manual-fwspick",
        choices=["low", "high", "dl", "dh"],
        help="Manually toggle FWSPICK pin",
    )

    # Sequence operations
    parser.add_argument(
        "--sequence",
        choices=["fwspick-mode", "normal-mode"],
        help="Run GPIO sequence",
    )

    # UART settings
    parser.add_argument(
        "-b", "--baudrate", type=int, default=115200, help="UART baud rate (default: 115200)"
    )
    parser.add_argument(
        "--test-timeout",
        type=int,
        default=600,
        help="Test execution monitoring timeout in seconds (default: 600)",
    )
    parser.add_argument(
        "--log-file", help="Log file path (auto-generated if not specified)"
    )

    # Control flags
    parser.add_argument(
        "--skip-uart", action="store_true", help="Skip all UART operations"
    )
    parser.add_argument(
        "-q", "--quiet", action="store_true", help="Run silently (no output)"
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Show what would be done without executing"
    )
    parser.add_argument(
        "--upload-only",
        action="store_true",
        help="Skip GPIO commands, upload firmware only",
    )
    parser.add_argument(
        "--bazel-test",
        action="store_true",
        help="Run in Bazel test mode (structured output)",
    )
    parser.add_argument(
        "--skip-gpio",
        action="store_true",
        help="Skip GPIO operations but still monitor tests",
    )

    args = parser.parse_args()

    # Validate arguments
    if args.upload_only:
        if not args.uart_device or not args.firmware:
            parser.error("--upload-only requires UART device and firmware file")
    elif not any(
        [args.manual_srst, args.manual_fwspick, args.sequence, args.uart_device]
    ):
        parser.error("Must specify manual GPIO control, sequence, or UART device")

    if (
        args.uart_device
        and not args.skip_uart
        and not args.dry_run
        and not Path(args.uart_device).exists()
    ):
        parser.error(f"UART device not found: {args.uart_device}")

    executor = UartTestExecutor(args)

    try:
        # Handle manual GPIO operations
        if args.manual_srst:
            state = "dl" if args.manual_srst in ["low", "dl"] else "dh"
            executor.toggle_srst(state)
            return 0

        if args.manual_fwspick:
            state = "dh" if args.manual_fwspick in ["high", "dh"] else "dl"
            executor.toggle_fwspick(state)
            return 0

        # Upload-only mode
        if args.upload_only:
            if not executor.open_serial():
                return 1
            ok = executor.upload_firmware()
            executor.cleanup()
            return 0 if ok else 1

        # Handle sequence operations
        if args.sequence == "fwspick-mode":
            executor.sequence_to_fwspick_mode()
            return 0
        elif args.sequence == "normal-mode":
            executor.sequence_to_normal_mode()
            return 0

        # Run full test sequence (or skip GPIO if requested)
        if args.skip_gpio:
            # Open serial, upload, and monitor without GPIO
            try:
                if not executor.open_serial():
                    return 1
                if not executor.upload_firmware():
                    return 1
                if executor.monitor_test_execution():
                    executor.log("Test execution completed successfully!")
                    return 0
                else:
                    executor.log("Test execution failed!")
                    return 1
            finally:
                executor.cleanup()
        else:
            if executor.run_full_test_sequence():
                executor.log("Test execution completed successfully!")
                return 0
            else:
                executor.log("Test execution failed!")
                return 1

    except KeyboardInterrupt:
        executor.log("\nInterrupted by user")
        return 130
    except Exception as e:
        executor.log(f"Error: {e}")
        return 1
    finally:
        executor.cleanup()


if __name__ == "__main__":
    sys.exit(main())
