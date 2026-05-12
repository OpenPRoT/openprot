#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""
AST1060 EVB hardware interaction layer.

Handles GPIO reset sequences, firmware upload via UART bootloader, and raw
UART byte streaming. All configuration is received as CLI arguments from
test_runner.py. Raw UART bytes are written to stdout; diagnostics go to
stderr. Runs locally or is SCP'd to the Pi for remote test execution.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

try:
    import serial
except ImportError:
    print("Error: pyserial not installed. Install with: pip install pyserial", file=sys.stderr)
    sys.exit(1)


def _gpio_set(pin: int, state: str) -> None:
    subprocess.run(["pinctrl", "set", str(pin), "op"] + state.split(), check=True)


def _sequence_to_fwspick_mode(srst_pin: int, fwspick_pin: int, port: serial.Serial) -> None:
    _gpio_set(srst_pin, "dl")
    time.sleep(0.1)
    port.timeout = 0.1
    port.read(4096)
    _gpio_set(fwspick_pin, "pn dh")
    time.sleep(1)
    _gpio_set(srst_pin, "dh")
    time.sleep(1)


def _wait_for_uart_ready(port: serial.Serial, timeout: int = 30) -> bool:
    deadline = time.time() + timeout
    buf = b""
    port.timeout = 0.1
    while time.time() < deadline:
        data = port.read(1024)
        if data:
            buf += data
            sys.stderr.buffer.write(data)
            sys.stderr.buffer.flush()
            if b"U" in buf:
                return True
    print("Timeout waiting for UART bootloader ready signal", file=sys.stderr)
    return False


def _upload_firmware(port: serial.Serial, firmware_path: Path) -> None:
    data = firmware_path.read_bytes()
    size = len(data)
    aligned = (size + 3) & ~3
    port.write(aligned.to_bytes(4, "little"))
    chunk_size = 1024
    for i in range(0, size, chunk_size):
        port.write(data[i : i + chunk_size])
        port.flush()
        time.sleep(0.01)
    padding = aligned - size
    if padding:
        port.write(bytes(padding))
    print(f"Uploaded {size} bytes ({padding} bytes padding)", file=sys.stderr)


_SUCCESS_SENTINEL = b"TEST_RESULT:PASS"
_FAILURE_SENTINELS = [b"TEST_RESULT:FAIL", b"panic"]


def _stream_uart(port: serial.Serial, timeout: int) -> bool:
    port.timeout = 1.0
    deadline = time.time() + timeout if timeout else None
    buf = b""
    while True:
        if deadline and time.time() >= deadline:
            print("Timeout waiting for test result sentinel", file=sys.stderr)
            return False
        data = port.read(1024)
        if data:
            try:
                sys.stdout.buffer.write(data)
                sys.stdout.buffer.flush()
            except (BrokenPipeError, OSError):
                return False
            buf += data
            if _SUCCESS_SENTINEL in buf:
                return True
            for s in _FAILURE_SENTINELS:
                if s in buf:
                    return False
            buf = buf[-256:]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="AST1060 EVB hardware layer: GPIO, firmware upload, UART stream"
    )
    parser.add_argument(
        "uart_device",
        help="Serial port device path (e.g. /dev/ttyUSB0)",
    )
    parser.add_argument(
        "firmware",
        nargs="?",
        help="Firmware binary to upload. Not required with --stream-only",
    )
    parser.add_argument(
        "--srst-pin",
        type=int,
        required=True,
        help="BCM GPIO pin connected to the AST1060 SRST line",
    )
    parser.add_argument(
        "--fwspick-pin",
        type=int,
        required=True,
        help="BCM GPIO pin connected to the AST1060 FWSPICK line",
    )
    parser.add_argument(
        "--baudrate",
        type=int,
        required=True,
        help="Serial port baud rate, must match firmware UART initialisation",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=600,
        help="Seconds to wait for a result sentinel (0 = no timeout, default: 600)",
    )
    parser.add_argument(
        "--stream-only",
        action="store_true",
        help="Skip GPIO sequences and firmware upload; stream raw UART bytes only",
    )
    args = parser.parse_args()

    if not args.stream_only:
        if not args.firmware:
            parser.error("firmware is required unless --stream-only is set")
        firmware_path = Path(args.firmware)
        if not firmware_path.exists():
            print(f"Error: firmware not found: {firmware_path}", file=sys.stderr)
            return 1
    else:
        firmware_path = None

    try:
        port = serial.Serial(
            args.uart_device,
            baudrate=args.baudrate,
            timeout=1.0,
            write_timeout=1.0,
        )
    except serial.SerialException as e:
        print(f"Error: could not open {args.uart_device}: {e}", file=sys.stderr)
        return 1

    result = False
    try:
        if not args.stream_only:
            _sequence_to_fwspick_mode(args.srst_pin, args.fwspick_pin, port)
            if not _wait_for_uart_ready(port):
                return 1
            _upload_firmware(port, firmware_path)
        result = _stream_uart(port, args.timeout)
    except KeyboardInterrupt:
        pass
    finally:
        port.close()

    return 0 if result else 1


if __name__ == "__main__":
    sys.exit(main())
