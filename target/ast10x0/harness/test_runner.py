#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""
AST1060 EVB test orchestration layer.

Loads evb_config.toml, resolves the ELF for detokenization, then either
spawns pi_test_runner.py locally (USB-attached board) or SCP+SSHes it to a
Raspberry Pi fixture. Raw UART bytes come back over a pipe or SSH stdout;
this script detokenizes them and pattern-matches for PASS/FAIL sentinels.
Diagnostics go to stderr; nothing is written to the Pi filesystem except the
firmware binary and pi_test_runner.py itself.
"""

import argparse
import base64
import binascii
import os
import signal
import subprocess
import sys
import threading
import time
import tomllib
from pathlib import Path

AST1060_EVB_PI_HOST = "AST1060_EVB_PI_HOST"

# ── pw_tokenizer discovery ────────────────────────────────────────────────────
# When run as a Bazel py_binary, pw_tokenizer is already on sys.path via deps.
# When run outside Bazel, try PW_TOK_ROOT or the Bazel output base as fallbacks.

def _extend_path_for_pw_tokenizer() -> None:
    pw_tok_root = os.environ.get("PW_TOK_ROOT")
    if pw_tok_root:
        sys.path.insert(0, os.path.join(pw_tok_root, "pw_tokenizer", "py"))
        return
    try:
        output_base = subprocess.check_output(
            ["bazel", "info", "output_base"], text=True, stderr=subprocess.DEVNULL
        ).strip()
        candidate = os.path.join(output_base, "external", "pigweed+", "pw_tokenizer", "py")
        if os.path.isdir(candidate):
            sys.path.insert(0, candidate)
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass


_extend_path_for_pw_tokenizer()

try:
    from pw_tokenizer import Detokenizer
    from pw_tokenizer.detokenize import NestedMessageParser
    _PW_TOKENIZER_AVAILABLE = True
except ImportError:
    _PW_TOKENIZER_AVAILABLE = False

# ── Remote lock constants ─────────────────────────────────────────────────────
# The Pi is a single-board computer; only one test session should hold the
# UART device and GPIO lines at a time. We use an atomic noclobber lock file
# on the Pi and touch it every _LOCK_TOUCH_INTERVAL seconds from a background
# thread so the stale-lock detector knows we're still alive.

_LOCK_PATH = "/tmp/ast1060_evb.lock"
_LOCK_TOUCH_INTERVAL = 10       # seconds between lock touches
_LOCK_STALE_THRESHOLD = 60      # seconds since last touch → lock is stale
_LOCK_ACQUIRE_TIMEOUT = 120     # seconds to wait before giving up


# ── SSH helpers ───────────────────────────────────────────────────────────────

def _ssh(host: str, cmd: str, **kwargs) -> subprocess.CompletedProcess:
    """Run a single SSH command synchronously."""
    return subprocess.run(
        ["ssh", "-o", "BatchMode=yes", host, cmd],
        **kwargs,
    )


def _ssh_stream(host: str, cmd: str) -> subprocess.Popen:
    """Open a streaming SSH session; stdout is a pipe the caller reads."""
    return subprocess.Popen(
        ["ssh", "-o", "BatchMode=yes", "-o", "ServerAliveInterval=5", host, cmd],
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
    )


def _acquire_lock(host: str, timeout: int = _LOCK_ACQUIRE_TIMEOUT) -> bool:
    """Atomically create lock file on Pi; retry until timeout."""
    deadline = time.time() + timeout
    create = f"set -o noclobber && echo $$ > {_LOCK_PATH}"
    stale_check = (
        f"mtime=$(stat -c %Y {_LOCK_PATH} 2>/dev/null) && "
        f"now=$(date +%s) && "
        f"[ $(( now - mtime )) -gt {_LOCK_STALE_THRESHOLD} ] && "
        f"rm -f {_LOCK_PATH}"
    )
    while time.time() < deadline:
        r = _ssh(host, create, capture_output=True)
        if r.returncode == 0:
            return True
        _ssh(host, stale_check, capture_output=True)
        time.sleep(2)
    print(f"Timeout acquiring Pi lock after {timeout}s", file=sys.stderr)
    return False


def _release_lock(host: str) -> None:
    _ssh(host, f"rm -f {_LOCK_PATH}", capture_output=True)


def _touch_lock_forever(host: str, stop: threading.Event) -> None:
    """Background thread: touch the lock file every _LOCK_TOUCH_INTERVAL seconds."""
    while not stop.wait(_LOCK_TOUCH_INTERVAL):
        _ssh(host, f"touch {_LOCK_PATH}", capture_output=True)


# ── UART monitor ──────────────────────────────────────────────────────────────


class UartMonitor:
    """Detokenizes and displays raw UART bytes. Pass/fail is determined by pi_test_runner.py exit code."""

    def __init__(self, args: argparse.Namespace, elf_path: Path) -> None:
        # elf_path is always derived on the host from firmware path; caller
        # validates existence before constructing this object.
        self.args = args
        self.log_file_handle = open(args.log_file, "w") if args.log_file else None
        self.detokenizer = (
            Detokenizer(str(elf_path)) if _PW_TOKENIZER_AVAILABLE else None
        )
        self._token_parser = NestedMessageParser() if _PW_TOKENIZER_AVAILABLE else None

    def _write_log(self, text: str) -> None:
        if self.log_file_handle:
            self.log_file_handle.write(text)
            self.log_file_handle.flush()

    def print_uart_data(self, raw: bytes) -> None:
        """Detokenize raw UART bytes and print them.

        pw_tokenizer embeds $<base64> token frames in the byte stream.
        NestedMessageParser preserves state across calls so tokens split
        across successive reads are reassembled correctly.
        """
        if self.detokenizer and self._token_parser:
            for is_token, span in self._token_parser.read_messages(raw):
                if not is_token:
                    text = span.decode("utf-8", errors="replace")
                    print(text, end="", flush=True)
                    self._write_log(text)
                    continue

                # span is b'$<base64chars>' — strip '$' and add padding.
                raw_text = span.decode("utf-8", errors="replace")
                try:
                    b64 = span[1:]
                    b64 += b"=" * (-len(b64) % 4)
                    encoded = base64.b64decode(b64, validate=True)
                    result = self.detokenizer.detokenize(encoded)
                except (binascii.Error, ValueError):
                    result = None

                if result is not None and result.ok():
                    decoded_str = str(result)
                    print(f"\033[32m{decoded_str}\033[0m", end="", flush=True)
                    self._write_log(decoded_str)
                else:
                    print(raw_text, end="", flush=True)
                    self._write_log(raw_text)
            return

        text = raw.decode("utf-8", errors="replace")
        print(text, end="", flush=True)
        self._write_log(text)

    def display_stream(self, pipe) -> None:
        """Read raw bytes from pipe and detokenize for display until EOF."""
        while True:
            chunk = pipe.read(4096)
            if not chunk:
                break
            self.print_uart_data(chunk)

    def cleanup(self) -> None:
        if self.log_file_handle:
            self.log_file_handle.close()
            self.log_file_handle = None


# ── Local execution ───────────────────────────────────────────────────────────

def _run_local(
    args: argparse.Namespace,
    config: dict,
    runner: Path,
    monitor: UartMonitor,
    uart_device: str,
) -> bool:
    """Wired (non-SSH) connection to the Pi fixture — not yet implemented."""
    print(
        "Error: wired Pi mode is not yet implemented. "
        f"Set ${AST1060_EVB_PI_HOST} or pass --pi-host to use SSH.",
        file=sys.stderr,
    )
    return False


# ── Remote execution ──────────────────────────────────────────────────────────

def _run_remote(
    args: argparse.Namespace,
    config: dict,
    runner: Path,
    monitor: UartMonitor,
    uart_device: str,
) -> bool:
    """SCP firmware and runner to Pi; stream UART back over SSH stdout."""
    host = args.pi_host
    gpio = config["gpio"]
    uart = config["uart"]
    baudrate = args.baudrate if args.baudrate else uart["baudrate"]

    remote_dir = "/tmp/ast1060_test"

    if not _acquire_lock(host):
        return False

    stop_touch = threading.Event()
    touch_thread = threading.Thread(
        target=_touch_lock_forever, args=(host, stop_touch), daemon=True
    )
    touch_thread.start()

    _ssh(host, f"rm -rf {remote_dir} && mkdir -p {remote_dir}", check=True)

    subprocess.run(
        ["scp", "-q", str(runner), f"{host}:{remote_dir}/pi_test_runner.py"],
        check=True,
    )

    if not args.parse_only:
        firmware_path = Path(args.firmware)
        remote_fw = f"{remote_dir}/{firmware_path.name}"
        subprocess.run(
            ["scp", "-q", str(firmware_path), f"{host}:{remote_fw}"],
            check=True,
        )
    else:
        remote_fw = None

    remote_cmd = f"python3 -u {remote_dir}/pi_test_runner.py {uart_device}"
    if not args.parse_only:
        remote_cmd += f" {remote_fw}"
    remote_cmd += (
        f" --srst-pin {gpio['srst_pin']}"
        f" --fwspick-pin {gpio['fwspick_pin']}"
        f" --baudrate {baudrate}"
        f" --timeout {args.timeout}"
    )
    if args.parse_only:
        remote_cmd += " --stream-only"

    proc = _ssh_stream(host, remote_cmd)
    try:
        monitor.display_stream(proc.stdout)
        proc.wait()
    finally:
        if proc.returncode is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
        stop_touch.set()
        _release_lock(host)

    return proc.returncode == 0


# ── Entry point ───────────────────────────────────────────────────────────────

def main() -> int:
    with (Path(__file__).parent / "evb_config.toml").open("rb") as f:
        config = tomllib.load(f)

    parser = argparse.ArgumentParser(
        description="AST1060 EVB test orchestration: detokenize UART, match sentinels"
    )
    parser.add_argument(
        "firmware",
        nargs="?",
        help=(
            "Firmware image (.elf or .bin). system_image emits both under the "
            "same stem; pass either and the other is derived automatically."
        ),
    )
    parser.add_argument(
        "--pi-host",
        default=None,
        help=f"Raspberry Pi hostname/IP. Falls back to ${AST1060_EVB_PI_HOST} env var. Omit both to use a locally attached board",
    )
    parser.add_argument(
        "--baudrate",
        type=int,
        default=None,
        help=f"Override baud rate from evb_config.toml (default: {config['uart']['baudrate']})",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=600,
        help="Seconds to wait for a test result sentinel (0 = no timeout, default: 600)",
    )
    parser.add_argument(
        "--log-file",
        help="Write detokenized UART output to this file",
    )
    parser.add_argument(
        "-q", "--quiet",
        action="store_true",
        help="Suppress diagnostic messages to stderr",
    )
    parser.add_argument(
        "--parse-only",
        action="store_true",
        help="Skip GPIO and firmware upload; stream and detokenize UART output only",
    )
    args = parser.parse_args()

    uart_device = os.environ.get("UART_DEVICE") or config["uart"]["serial_port"]

    if not args.firmware:
        parser.error("firmware is required")

    # system_image emits both .elf and .bin under the same stem; accept either.
    # When invoked via --run_under on a system_image_test, Bazel passes the
    # no-suffix symlink (e.g. threads_test → threads.elf); resolve it first.
    image = Path(args.firmware)
    if not image.suffix:
        image = image.resolve()
    elf_path = image.with_suffix(".elf")
    args.firmware = str(image.with_suffix(".bin"))
    if not elf_path.exists():
        print(f"Error: ELF not found at {elf_path}", file=sys.stderr)
        return 1

    args.pi_host = os.environ.get(AST1060_EVB_PI_HOST) or args.pi_host

    runner = Path(__file__).parent / "pi_test_runner.py"
    monitor = UartMonitor(args, elf_path)

    signal.signal(signal.SIGINT, lambda s, f: sys.exit(130))

    try:
        if args.pi_host:
            ok = _run_remote(args, config, runner, monitor, uart_device)
        else:
            ok = _run_local(args, config, runner, monitor, uart_device)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    finally:
        monitor.cleanup()

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
