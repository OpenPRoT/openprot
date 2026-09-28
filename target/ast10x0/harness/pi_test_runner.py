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
import threading
import time
from contextlib import nullcontext
from pathlib import Path

try:
    import serial
except ImportError:
    print(
        "Error: pyserial not installed. Install with: pip install pyserial",
        file=sys.stderr,
    )
    sys.exit(1)


# All elapsed times reported by this script are relative to process start.
_T0 = time.monotonic()


def _gpio_set(pin: int, state: str) -> None:
    subprocess.run(["pinctrl", "set", str(pin), "op"] + state.split(), check=True)


def _gpio_set_input(pin: int, pull: str) -> None:
    subprocess.run(["pinctrl", "set", str(pin), "ip", pull], check=True)


def _gpio_read(pin: int) -> bool:
    """True if `pin` currently reads high."""
    out = subprocess.run(
        ["pinctrl", "get", str(pin)], check=True, capture_output=True, text=True
    ).stdout
    return "hi" in out


def _sequence_to_fwspick_mode(
    srst_pin: int, fwspick_pin: int, port: serial.Serial
) -> None:
    _gpio_set(srst_pin, "dl")
    time.sleep(0.1)
    port.timeout = 0.1
    port.read(4096)
    _gpio_set(fwspick_pin, "pn dh")
    time.sleep(1)
    _gpio_set(srst_pin, "dh")
    time.sleep(1)


# A deliberate request pulse (fd_main.rs's RESET_PULSE_DURATION) is held high
# for 300ms. A power-on/reset transient on an otherwise-undriven pin is not:
# requiring this many consecutive 100ms-spaced samples to read high before
# acting rejects the transient without meaningfully delaying a real request.
_DEBOUNCE_SAMPLES = 3


def _watch_reset_passthrough(
    pin: int, srst_pin: int, stop: threading.Event, lock: threading.Lock
) -> None:
    """Reset device A whenever device B requests it over the passthrough GPIO.

    Device B can't toggle the Pi's reset lines itself, so it asks by driving
    `pin` high; this only plain-resets device A (srst_pin alone, fwspick_pin
    untouched) so it reboots the firmware already in flash rather than
    entering the bootloader.
    """
    _gpio_set_input(pin, "pd")
    triggered = False
    high_streak = 0
    while not stop.wait(0.1):
        is_high = _gpio_read(pin)
        high_streak = high_streak + 1 if is_high else 0
        if not is_high:
            triggered = False
        elif high_streak >= _DEBOUNCE_SAMPLES and not triggered:
            stamped = (
                b"[%7.2f watch] Reset-passthrough asserted; resetting device A\n"
                % (time.monotonic() - _T0,)
            )
            try:
                with lock:
                    sys.stdout.buffer.write(stamped)
                    sys.stdout.buffer.flush()
            except (BrokenPipeError, OSError):
                pass
            _gpio_set(srst_pin, "dl")
            time.sleep(0.1)
            _gpio_set(srst_pin, "dh")
            time.sleep(0.5)
            triggered = True
            high_streak = 0


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
    print(
        f"[{time.monotonic() - _T0:7.2f}] Uploaded {size} bytes"
        f" ({padding} bytes padding); board boots now",
        file=sys.stderr,
    )


_stdout_lock = threading.Lock()

_SUCCESS_SENTINEL = b"TEST_RESULT:PASS"
_FAILURE_SENTINELS = [b"TEST_RESULT:FAIL", b"panic"]


def _stream_uart(port: serial.Serial, lock=None, label: str = "") -> bool:
    port.timeout = 1.0
    buf = b""
    # Held back so the prefix only ever lands at a line start: a pw_tokenizer
    # $base64 frame is terminated by the newline, never split across one.
    partial = b""
    while True:
        data = port.read(1024)
        if data:
            partial += data
            lines = partial.split(b"\n")
            partial = lines.pop()
            if lines:
                stamped = b"".join(
                    b"[%7.2f %s] %s\n" % (time.monotonic() - _T0, label.encode(), line)
                    for line in lines
                )
                try:
                    with lock or nullcontext():
                        sys.stdout.buffer.write(stamped)
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


def _run_paired(args, firmware_path: Path, slave_firmware_path: Path) -> bool:
    try:
        port_b = serial.Serial(
            args.slave_uart_device,
            baudrate=args.baudrate,
            timeout=1.0,
            write_timeout=1.0,
        )
    except serial.SerialException as e:
        print(f"Error: could not open {args.slave_uart_device}: {e}", file=sys.stderr)
        return False

    try:
        port_a = serial.Serial(
            args.uart_device,
            baudrate=args.baudrate,
            timeout=1.0,
            write_timeout=1.0,
        )
    except serial.SerialException as e:
        port_b.close()
        print(f"Error: could not open {args.uart_device}: {e}", file=sys.stderr)
        return False

    stop_watch = threading.Event()
    watcher = None
    try:
        _sequence_to_fwspick_mode(args.slave_srst_pin, args.slave_fwspick_pin, port_b)
        if not _wait_for_uart_ready(port_b):
            return False
        _upload_firmware(port_b, slave_firmware_path)

        # Started only once card B's own firmware is running: until then the
        # passthrough pin has no defined driver (previous firmware, reset
        # transients, boot-time pinmux defaults), so watching any earlier
        # would react to noise instead of a real request.
        if args.reset_passthrough_pin is not None:
            watcher = threading.Thread(
                target=_watch_reset_passthrough,
                args=(
                    args.reset_passthrough_pin,
                    args.srst_pin,
                    stop_watch,
                    _stdout_lock,
                ),
                daemon=True,
            )
            watcher.start()

        results = [None, None]

        def _monitor(idx, port, label):
            results[idx] = _stream_uart(port, _stdout_lock, label)

        # Started before card A is flashed: the slave boots a full upload
        # earlier, and its output would otherwise sit in the tty buffer and
        # arrive all at once with the wrong timestamps.
        threads = [
            threading.Thread(target=_monitor, args=(1, port_b, "slave"), daemon=True)
        ]
        threads[0].start()

        _sequence_to_fwspick_mode(args.srst_pin, args.fwspick_pin, port_a)
        if not _wait_for_uart_ready(port_a):
            return False
        _upload_firmware(port_a, firmware_path)

        threads.append(
            threading.Thread(target=_monitor, args=(0, port_a, "main"), daemon=True)
        )
        threads[1].start()
        for t in threads:
            t.join()

        return bool(results[0] and results[1])
    except KeyboardInterrupt:
        return False
    finally:
        stop_watch.set()
        if watcher:
            watcher.join(timeout=1)
        port_a.close()
        port_b.close()


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
        "--stream-only",
        action="store_true",
        help="Skip GPIO sequences and firmware upload; stream raw UART bytes only",
    )
    parser.add_argument(
        "--slave-firmware",
        default=None,
        help="Slave firmware binary. When present, enables paired two-device mode.",
    )
    parser.add_argument(
        "--slave-uart-device",
        default=None,
        help="Serial port for device B (e.g. /dev/ttyUSB1)",
    )
    parser.add_argument(
        "--slave-srst-pin",
        type=int,
        default=None,
        help="BCM GPIO pin connected to device B SRST",
    )
    parser.add_argument(
        "--slave-fwspick-pin",
        type=int,
        default=None,
        help="BCM GPIO pin connected to device B FWSPICK",
    )
    parser.add_argument(
        "--reset-passthrough-pin",
        type=int,
        default=None,
        help="BCM GPIO pin device B drives high to request a device A reset",
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

    if args.slave_firmware:
        missing = [
            name
            for name, val in [
                ("--slave-uart-device", args.slave_uart_device),
                ("--slave-srst-pin", args.slave_srst_pin),
                ("--slave-fwspick-pin", args.slave_fwspick_pin),
            ]
            if val is None
        ]
        if missing:
            parser.error(f"paired mode requires: {', '.join(missing)}")
        slave_firmware_path = Path(args.slave_firmware)
        if not slave_firmware_path.exists():
            print(
                f"Error: slave firmware not found: {slave_firmware_path}",
                file=sys.stderr,
            )
            return 1
        return 0 if _run_paired(args, firmware_path, slave_firmware_path) else 1

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
        result = _stream_uart(port)
    except KeyboardInterrupt:
        pass
    finally:
        port.close()

    return 0 if result else 1


if __name__ == "__main__":
    sys.exit(main())
