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
        candidate = os.path.join(
            output_base, "external", "pigweed+", "pw_tokenizer", "py"
        )
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
_LOCK_TOUCH_INTERVAL = 10  # seconds between lock touches
_LOCK_STALE_THRESHOLD = 60  # seconds since last touch → lock is stale
_LOCK_ACQUIRE_TIMEOUT = 120  # seconds to wait before giving up


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
    local_user = os.environ.get("USER", "unknown")
    # On noclobber failure, cat outputs the lock file contents so we get
    # owner info from the same SSH call without a second round-trip.
    create = (
        f"set -o noclobber && "
        f"printf '%s\\t%s\\n' '{local_user}' \"$(date +%s)\" > {_LOCK_PATH} || "
        f"{{ cat {_LOCK_PATH} 2>/dev/null; false; }}"
    )
    stale_check = (
        f"mtime=$(stat -c %Y {_LOCK_PATH} 2>/dev/null) && "
        f"now=$(date +%s) && "
        f"[ $(( now - mtime )) -gt {_LOCK_STALE_THRESHOLD} ] && "
        f"rm -f {_LOCK_PATH}"
    )
    last_printed = 0.0
    while time.time() < deadline:
        r = _ssh(host, create, capture_output=True)
        if r.returncode == 0:
            return True
        if not r.stdout.strip():
            if msg := r.stderr.decode(errors="replace").strip():
                print(f"RUNNER: {msg}", file=sys.stderr)
                return False
            continue
        parts = r.stdout.decode(errors="replace").split()
        now = time.time()
        if len(parts) == 2 and now - last_printed >= 30:
            lock_user, acq_ts = parts[0], int(parts[1])
            held_s = int(now) - acq_ts
            mins, secs = divmod(held_s, 60)
            held = f"{mins}m {secs}s" if mins else f"{secs}s"
            stat_r = _ssh(
                host,
                f"now=$(date +%s); mtime=$(stat -c %Y {_LOCK_PATH} 2>/dev/null); echo $((now - mtime))",
                capture_output=True,
            )
            touch_ago = int(stat_r.stdout.decode().strip() or -1)
            if touch_ago > _LOCK_STALE_THRESHOLD:
                print(
                    f"RUNNER: removing stale lock held by {lock_user} {held} ago, last touched {touch_ago}s ago...",
                    file=sys.stderr,
                )
            else:
                print(
                    f"RUNNER: Pi locked by {lock_user} {held} ago, last touched {touch_ago}s ago; waiting...",
                    file=sys.stderr,
                )
            last_printed = now
        _ssh(host, stale_check, capture_output=True)
        time.sleep(2)
    print(
        f"RUNNER: timeout acquiring Pi lock after {timeout}s (Pi may be busy)",
        file=sys.stderr,
    )
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

    def __init__(self, args: argparse.Namespace, *elf_paths: Path) -> None:
        self.args = args
        self.log_file_handle = open(args.log_file, "w") if args.log_file else None
        elf_args = [str(p) for p in elf_paths if p is not None]
        self.detokenizer = (
            Detokenizer(*elf_args) if _PW_TOKENIZER_AVAILABLE and elf_args else None
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
                    os.write(1, text.encode("utf-8", errors="replace"))
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
                    os.write(
                        1,
                        f"\033[32m{decoded_str}\033[0m".encode(
                            "utf-8", errors="replace"
                        ),
                    )
                    self._write_log(decoded_str)
                else:
                    os.write(1, raw_text.encode("utf-8", errors="replace"))
                    self._write_log(raw_text)
            return

        text = raw.decode("utf-8", errors="replace")
        os.write(1, text.encode("utf-8", errors="replace"))
        self._write_log(text)

    def display_stream(self, pipe) -> None:
        """Read raw bytes from pipe and detokenize for display until EOF."""
        while True:
            chunk = pipe.raw.read(4096)
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
    dev_a = config["device_a"]
    baudrate = args.baudrate if args.baudrate else config["baudrate"]

    remote_dir = "/tmp/ast1060_test"

    if not _acquire_lock(host):
        return False
    print("RUNNER: lock acquired, starting test", file=sys.stderr)

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

    remote_slave_fw = None
    if args.slave_firmware:
        slave_firmware_path = Path(args.slave_firmware)
        remote_slave_fw = f"{remote_dir}/{slave_firmware_path.name}"
        subprocess.run(
            ["scp", "-q", str(slave_firmware_path), f"{host}:{remote_slave_fw}"],
            check=True,
        )

    remote_cmd = f"python3 -u {remote_dir}/pi_test_runner.py"
    if not args.parse_only:
        remote_cmd += f" {remote_fw}"
    remote_cmd += (
        f" --srst-pin {dev_a['srst_pin']}"
        f" --fwspick-pin {dev_a['fwspick_pin']}"
        f" --baudrate {baudrate}"
    )
    if uart_device:
        remote_cmd += f" --uart-device {uart_device}"
    if args.verbose_runner:
        remote_cmd += " --verbose-runner"
    if args.parse_only:
        remote_cmd += " --stream-only"
    if remote_slave_fw:
        device_b = config["device_b"]
        remote_cmd += (
            f" --slave-firmware {remote_slave_fw}"
            f" --slave-srst-pin {device_b['srst_pin']}"
            f" --slave-fwspick-pin {device_b['fwspick_pin']}"
        )

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
        help=f"Override baud rate from evb_config.toml (default: {config['baudrate']})",
    )
    parser.add_argument(
        "--log-file",
        help="Write detokenized UART output to this file",
    )
    parser.add_argument(
        "-q",
        "--quiet",
        action="store_true",
        help="Suppress diagnostic messages to stderr",
    )
    parser.add_argument(
        "--parse-only",
        action="store_true",
        help="Skip GPIO and firmware upload; stream and detokenize UART output only",
    )
    parser.add_argument(
        "-v",
        "--verbose-runner",
        action="store_true",
        help=(
            "Dump each port's post-flush buffer during UART discovery. Pass "
            "through Bazel with --test_arg=-v"
        ),
    )
    args, remaining = parser.parse_known_args()

    if os.environ.get("PW_RUNNER_PASSTHROUGH") == "1":
        import subprocess

        res = subprocess.run([args.firmware] + remaining)
        return res.returncode

    # Normally the Pi auto-discovers the UART by resetting the board, so no
    # device is configured. UART_DEVICE lets a caller pin an explicit port;
    # --parse-only can't reset, so it requires one.
    uart_device = os.environ.get("UART_DEVICE")

    if not args.firmware:
        parser.error("firmware is required")

    if args.parse_only and not uart_device:
        parser.error(
            "--parse-only requires UART_DEVICE to be set "
            "(no reset happens, so the port can't be auto-discovered)"
        )

    # system_image emits both .elf and .bin under the same stem; accept either.
    # When invoked via --run_under on a system_image_test, Bazel passes the
    # no-suffix symlink (e.g. threads_test → threads.elf); resolve it first.
    image = Path(args.firmware)

    # When invoked via --run_under the firmware path has no suffix (it is the
    # test-name symlink).  The system_image_test rule places a companion
    # <name>.slave.elf symlink alongside it when slave_image is set; detecting
    # that file is how we enter paired mode without any extra CLI arguments.
    args.slave_firmware = None
    slave_elf_path = None
    if not image.suffix:
        slave_symlink = image.parent / (image.name + ".slave.elf")
        if slave_symlink.exists():
            slave_elf_path = slave_symlink.resolve()
            args.slave_firmware = str(slave_elf_path.with_suffix(".bin"))
        image = image.resolve()
        args.firmware = str(image.with_suffix(".bin"))
    else:
        args.firmware = str(image.with_suffix(".bin"))

    elf_path = image.with_suffix(".elf")
    if not elf_path.exists():
        print(f"Error: ELF not found at {elf_path}", file=sys.stderr)
        return 1

    args.pi_host = os.environ.get(AST1060_EVB_PI_HOST) or args.pi_host

    runner = Path(__file__).parent / "pi_test_runner.py"
    monitor = UartMonitor(args, elf_path, slave_elf_path)

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
