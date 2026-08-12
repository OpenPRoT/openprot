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


def _gpio_set(pin: int, state: str) -> None:
    subprocess.run(["pinctrl", "set", str(pin), "op"] + state.split(), check=True)


_DISCOVERY_TIMEOUT = 30  # seconds to wait for a reset board to answer
_FLUSH_DURATION = 0.5  # drain ports this long while reset is held, comfortably
# longer than the FTDI latency timer (~16ms) so every pre-reset byte lands and
# is discarded and nothing new can arrive until reset is released
_COLLECT_WINDOW = 1.0  # once the first bytes arrive, keep reading this long so a
# running board has time to reveal a non-0x55 byte and be ruled out


def _list_uart_ports() -> list[str]:
    return sorted(str(p) for p in Path("/dev").glob("ttyUSB*"))


def _discover_uart_port(
    srst_pin: int, fwspick_pin: int, baudrate: int, exclude=(), verbose=False
):
    """Reset the board on (srst_pin, fwspick_pin) and return the /dev/ttyUSB*
    that emits the bootloader ready signal.

    The GPIO wiring to each board is fixed, but USB enumeration order differs
    between Pi fixtures, so ttyUSB numbering can't be hardcoded. The board we
    just reset comes up in the bootloader and emits only the 'U' (0x55) ready
    signal; a board still running its app emits other bytes. So the target is
    the one port whose post-flush buffer is non-empty and entirely 0x55.
    """
    candidates = [p for p in _list_uart_ports() if p not in exclude]
    if not candidates:
        print("RUNNER: no /dev/ttyUSB* ports found", file=sys.stderr)
        return None

    opened: list[tuple[str, serial.Serial]] = []
    for dev in candidates:
        try:
            opened.append((dev, serial.Serial(dev, baudrate=baudrate, timeout=0.1)))
        except serial.SerialException as e:
            print(f"RUNNER: skipping {dev}: {e}", file=sys.stderr)
    if not opened:
        return None

    try:
        # Assert reset first so every board stops transmitting, then drain each
        # port for longer than the FTDI latency timer. Held in reset, no board
        # can emit anything, so once drained the buffers stay empty until reset
        # is released; no pre-reset byte can leak past the flush.
        _gpio_set(srst_pin, "dl")
        flush_deadline = time.time() + _FLUSH_DURATION
        while time.time() < flush_deadline:
            for _, port in opened:
                port.read(4096)
        _gpio_set(fwspick_pin, "pn dh")
        time.sleep(1)
        _gpio_set(srst_pin, "dh")

        buffers = {dev: bytearray() for dev, _ in opened}
        deadline = time.time() + _DISCOVERY_TIMEOUT
        collect_deadline = None
        while time.time() < deadline:
            for dev, port in opened:
                data = port.read(256)
                if data:
                    buffers[dev].extend(data)
            if collect_deadline is None:
                if any(buffers.values()):
                    collect_deadline = time.time() + _COLLECT_WINDOW
            elif time.time() >= collect_deadline:
                break

        if verbose:
            for dev, buf in buffers.items():
                print(
                    f"RUNNER: {dev} post-flush buffer ({len(buf)} bytes): "
                    f"{bytes(buf[:32])!r}",
                    file=sys.stderr,
                )

        # The freshly reset board emits only the bootloader 'U' (0x55); a board
        # still running its app emits other bytes. So the target is the port
        # whose buffer is non-empty and entirely 0x55; any non-0x55 rules it out.
        candidates = [
            dev for dev, buf in buffers.items() if buf and all(b == 0x55 for b in buf)
        ]
        if len(candidates) == 1:
            return candidates[0]

        if candidates:
            print(
                f"RUNNER: multiple ports look like a bootloader: {candidates}",
                file=sys.stderr,
            )
        else:
            print("RUNNER: no bootloader (U-only) port found", file=sys.stderr)
        if not verbose:
            print(
                "RUNNER: re-run with --test_arg=-v to dump post-flush buffers",
                file=sys.stderr,
            )
        return None
    finally:
        for _, port in opened:
            port.close()


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
    print(f"RUNNER: uploading firmware to {port.port}", file=sys.stderr)
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


_stdout_lock = threading.Lock()

_SUCCESS_SENTINEL = b"TEST_RESULT:PASS"
_FAILURE_SENTINELS = [b"TEST_RESULT:FAIL", b"panic"]


def _stream_uart(port: serial.Serial, lock=None) -> bool:
    port.timeout = 1.0
    buf = b""
    while True:
        data = port.read(1024)
        if data:
            try:
                with lock or nullcontext():
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

    try:
        _sequence_to_fwspick_mode(args.slave_srst_pin, args.slave_fwspick_pin, port_b)
        if not _wait_for_uart_ready(port_b):
            return False
        _upload_firmware(port_b, slave_firmware_path)

        _sequence_to_fwspick_mode(args.srst_pin, args.fwspick_pin, port_a)
        if not _wait_for_uart_ready(port_a):
            return False
        _upload_firmware(port_a, firmware_path)

        results = [None, None]

        def _monitor(idx, port):
            results[idx] = _stream_uart(port, _stdout_lock)

        threads = [
            threading.Thread(target=_monitor, args=(0, port_a)),
            threading.Thread(target=_monitor, args=(1, port_b)),
        ]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        return bool(results[0] and results[1])
    except KeyboardInterrupt:
        return False
    finally:
        port_a.close()
        port_b.close()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="AST1060 EVB hardware layer: GPIO, firmware upload, UART stream"
    )
    parser.add_argument(
        "--uart-device",
        default=None,
        help=(
            "Serial port device path (e.g. /dev/ttyUSB0). If omitted, the port "
            "is auto-discovered by resetting the board on --srst-pin/--fwspick-pin "
            "and listening for the bootloader. Required with --stream-only."
        ),
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
        "-v",
        "--verbose-runner",
        action="store_true",
        help="Dump each port's post-flush buffer during UART discovery",
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

    # Resolve the master UART. An explicit --uart-device is trusted as-is;
    # otherwise reset the board and discover which /dev/ttyUSB* answers, so the
    # harness tolerates USB enumeration order differing between Pi fixtures.
    if args.stream_only:
        if not args.uart_device:
            parser.error("--uart-device is required with --stream-only")
    elif not args.uart_device:
        args.uart_device = _discover_uart_port(
            args.srst_pin,
            args.fwspick_pin,
            args.baudrate,
            verbose=args.verbose_runner,
        )
        if args.uart_device is None:
            print("RUNNER: could not discover master UART", file=sys.stderr)
            return 1
        print(f"RUNNER: discovered master UART at {args.uart_device}", file=sys.stderr)

    if args.slave_firmware:
        missing = [
            name
            for name, val in [
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
        if not args.slave_uart_device:
            args.slave_uart_device = _discover_uart_port(
                args.slave_srst_pin,
                args.slave_fwspick_pin,
                args.baudrate,
                exclude=(args.uart_device,),
                verbose=args.verbose_runner,
            )
            if args.slave_uart_device is None:
                print("Error: could not discover slave UART", file=sys.stderr)
                return 1
            print(
                f"RUNNER: discovered slave UART at {args.slave_uart_device}",
                file=sys.stderr,
            )
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
