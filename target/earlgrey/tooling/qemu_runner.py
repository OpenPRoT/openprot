# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""QEMU runner for the ot-earlgrey machine.

Launches firmware under the QEMU ot-earlgrey machine, pipes UART output through
pw_tokenizer detokenization, and exits with a structured code:

  0  success-regex matched
  1  failure-regex matched
  2  timeout (no match before --timeout-seconds elapsed)
  3  QEMU process exited unexpectedly before any match
"""

import atexit
import argparse
import io
import logging
import os
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

from pw_tokenizer import detokenize

_LOG = logging.getLogger(__name__)

_SPIFLASH_SIZE = 32 * 1024 * 1024  # W25Q256 = 32 MiB
_SCRIPT_DIR = Path(__file__).resolve().parent

EXIT_SUCCESS = 0
EXIT_FAILURE = 1
EXIT_TIMEOUT = 2
EXIT_QEMU_CRASH = 3

# Shared state for signal handlers — mutable list used as a ref cell.
_active_pid: list[int] = []


def _parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "--qemu-start",
        type=Path,
        default=_SCRIPT_DIR / "qemu_start.sh",
        help="path to qemu_start.sh (default: sibling of this script)",
    )
    p.add_argument(
        "--qemu-bin", required=True, type=Path, help="path to qemu-system-riscv32"
    )
    p.add_argument(
        "--qemu-config",
        required=True,
        type=Path,
        help="QEMU readconfig INI from qemu_cfg rule",
    )
    p.add_argument("--qemu-rom", required=True, type=Path, help="test ROM ELF")
    p.add_argument(
        "--qemu-otp",
        required=True,
        type=Path,
        help="read-only OTP image; will be copied to a writable temp path",
    )
    p.add_argument(
        "--qemu-flash",
        required=True,
        type=Path,
        help="read-only flash image; will be copied to a writable temp path",
    )
    p.add_argument(
        "--firmware-elf",
        required=True,
        type=Path,
        help="firmware ELF for pw_tokenizer detokenization",
    )
    p.add_argument(
        "--exit-success",
        default=None,
        type=str,
        help="regex matched against detokenized UART output to signal pass",
    )
    p.add_argument(
        "--exit-failure",
        default=None,
        type=str,
        help="regex matched against detokenized UART output to signal fail",
    )
    p.add_argument(
        "--timeout-seconds",
        default=120,
        type=int,
        help="seconds before EXIT_TIMEOUT (0 = no timeout)",
    )
    p.add_argument(
        "--icount", default=6, type=int, help="QEMU icount shift value (default 6)"
    )
    return p.parse_args()


def _wait_for_pid(pidfile: Path, deadline: float) -> int:
    while time.monotonic() < deadline:
        try:
            text = pidfile.read_text().strip()
            pid = int(text)
            if pid > 0:
                return pid
        except (FileNotFoundError, ValueError):
            pass
        time.sleep(0.05)
    raise TimeoutError(f"QEMU PID file never appeared: {pidfile}")


def _connect_unix(path: Path, deadline: float) -> socket.socket:
    while True:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            sock.connect(str(path))
            return sock
        except OSError:
            sock.close()
            if time.monotonic() >= deadline:
                raise TimeoutError(f"timed out connecting to {path}")
            time.sleep(0.05)


def _alive(pid: int) -> bool:
    return Path(f"/proc/{pid}").exists()


def _kill_qemu(pid: int) -> None:
    """SIGTERM → wait 5 s → SIGKILL."""
    if not _alive(pid):
        return
    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + 5.0
    while _alive(pid) and time.monotonic() < deadline:
        time.sleep(0.1)
    if _alive(pid):
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def _signal_handler(signum: int, _frame) -> None:
    # sys.exit raises SystemExit, unwinding try/finally and TemporaryDirectory.
    if _active_pid:
        _kill_qemu(_active_pid[0])
    sys.exit(EXIT_QEMU_CRASH)


class _Watcher:
    """Thread-safe regex watcher operating on detokenized text."""

    def __init__(self, success: str | None, failure: str | None):
        self._success = re.compile(success) if success else None
        self._failure = re.compile(failure) if failure else None
        self.result: int | None = None
        self._lock = threading.Lock()

    def feed(self, text: str) -> bool:
        """Return True and record result on first pattern match."""
        with self._lock:
            if self.result is not None:
                return True
            if self._success and self._success.search(text):
                self.result = EXIT_SUCCESS
                return True
            if self._failure and self._failure.search(text):
                self.result = EXIT_FAILURE
                return True
        return False


def _emit_line(det: detokenize.Detokenizer | None, line: str) -> str:
    """Detokenize one line, write to stdout, return the decoded text."""
    if det is None:
        sys.stdout.write(line)
        sys.stdout.flush()
        return line
    try:
        buf = io.BytesIO()
        det.detokenize_text_to_file(line, buf)
        decoded = buf.getvalue().decode("utf-8", errors="replace")
        sys.stdout.write(decoded)
        sys.stdout.flush()
        return decoded
    except Exception as e:
        _LOG.debug("detokenize error: %s", e)
        sys.stdout.write(line)
        sys.stdout.flush()
        return line


def _uart_thread(
    sock: socket.socket,
    watcher: _Watcher,
    elf: Path,
    done: threading.Event,
) -> None:
    try:
        det: detokenize.Detokenizer | None = detokenize.Detokenizer(elf)
    except Exception as e:
        _LOG.warning("detokenizer init failed (%s); raw output only", e)
        det = None

    line_buf = b""
    try:
        while not done.is_set():
            try:
                chunk = sock.recv(4096)
            except OSError:
                break
            if not chunk:
                break

            line_buf += chunk
            while b"\n" in line_buf:
                idx = line_buf.index(b"\n") + 1
                line = line_buf[:idx]
                line_buf = line_buf[idx:]
                decoded = _emit_line(det, line.decode("utf-8", errors="replace"))
                if watcher.feed(decoded):
                    done.set()
                    return

        # Flush any partial line after the connection closes.
        if line_buf:
            decoded = _emit_line(det, line_buf.decode("utf-8", errors="replace"))
            watcher.feed(decoded)
    except Exception as e:
        _LOG.warning("UART reader error: %s", e)
    done.set()


def _run(args: argparse.Namespace) -> int:
    # Validate regex patterns before touching the filesystem.
    for flag, val in (
        ("--exit-success", args.exit_success),
        ("--exit-failure", args.exit_failure),
    ):
        if val is not None:
            try:
                re.compile(val)
            except re.error as e:
                _LOG.error("invalid %s pattern: %s", flag, e)
                return EXIT_FAILURE

    signal.signal(signal.SIGINT, _signal_handler)
    signal.signal(signal.SIGTERM, _signal_handler)

    with tempfile.TemporaryDirectory() as _tmpdir:
        tmp = Path(_tmpdir)

        # Step 1: writable copies of read-only Bazel-built backing files.
        # QEMU pflash/mtd drivers write back to the backing file on close.
        # Use copyfile (content-only) not copy2, so Bazel's read-only mode bits
        # are not propagated to the writable temp copy.
        otp_rw = tmp / "otp.raw"
        flash_rw = tmp / "flash.qemu_bin"
        shutil.copyfile(args.qemu_otp, otp_rw)
        shutil.copyfile(args.qemu_flash, flash_rw)

        # Step 2: 32 MiB SPI flash backing store (sparse — W25Q256).
        spiflash = tmp / "spiflash.bin"
        with open(spiflash, "wb") as fh:
            fh.seek(_SPIFLASH_SIZE - 1)
            fh.write(b"\x00")

        pidfile = tmp / "qemu.pid"
        logfile = tmp / "qemu.log"
        monitor_path = tmp / "monitor.sock"
        uart_path = tmp / "uart0.sock"

        # Step 3: launch qemu_start.sh.  -daemonize causes QEMU to fork to
        # background; the script (and thus subprocess.run) returns once the
        # child has written its PID file.
        env = dict(os.environ)
        env.update(
            {
                "QEMU_BIN": str(args.qemu_bin),
                "QEMU_CONFIG": str(args.qemu_config),
                "QEMU_ROM": str(args.qemu_rom),
                "QEMU_OTP": str(otp_rw),
                "QEMU_FLASH": str(flash_rw),
                "QEMU_SPIFLASH": str(spiflash),
                "QEMU_PIDFILE": str(pidfile),
                "QEMU_LOG": str(logfile),
                "QEMU_ICOUNT": str(args.icount),
                "QEMU_MONITOR": str(monitor_path),
                "QEMU_UART_SOCKET": str(uart_path),
            }
        )

        cmd = [str(args.qemu_start)]
        _LOG.info("starting QEMU via %s", args.qemu_start)
        launch = subprocess.run(cmd, env=env, check=False)
        if launch.returncode != 0:
            _LOG.error("qemu_start.sh exited %d", launch.returncode)
            return EXIT_QEMU_CRASH

        deadline = (
            time.monotonic() + args.timeout_seconds
            if args.timeout_seconds > 0
            else float("inf")
        )

        # Step 4a: poll for QEMU's PID file.
        try:
            qemu_pid = _wait_for_pid(pidfile, deadline)
        except TimeoutError as e:
            _LOG.error("%s", e)
            return EXIT_TIMEOUT
        _LOG.info("QEMU daemonized, pid=%d", qemu_pid)

        # Register PID for signal-handler cleanup.
        _active_pid.clear()
        _active_pid.append(qemu_pid)
        atexit.register(_kill_qemu, qemu_pid)

        # Step 4b: connect to HMP monitor socket and resume the CPU.
        # The -S flag holds the CPU paused; without `cont` the UART is silent.
        try:
            mon = _connect_unix(monitor_path, deadline)
            try:
                # Wait for the HMP `(qemu) ` prompt before sending cont.
                mon.settimeout(5.0)
                banner = b""
                try:
                    while b"(qemu)" not in banner:
                        chunk = mon.recv(64)
                        if not chunk:
                            break
                        banner += chunk
                except socket.timeout:
                    _LOG.warning(
                        "monitor: no (qemu) prompt within 5s, sending cont anyway"
                    )
                mon.settimeout(None)
                mon.sendall(b"cont\n")
            finally:
                mon.close()
        except (TimeoutError, OSError) as e:
            _LOG.error("monitor: %s", e)
            _kill_qemu(qemu_pid)
            return EXIT_QEMU_CRASH

        # Step 5: connect to UART0 socket and watch for exit patterns.
        try:
            uart_sock = _connect_unix(uart_path, deadline)
        except TimeoutError as e:
            _LOG.error("%s", e)
            _kill_qemu(qemu_pid)
            return EXIT_QEMU_CRASH

        watcher = _Watcher(args.exit_success, args.exit_failure)
        done = threading.Event()
        reader = threading.Thread(
            target=_uart_thread,
            args=(uart_sock, watcher, args.firmware_elf, done),
            daemon=True,
        )
        reader.start()

        timed_out = False
        crashed = False
        try:
            while not done.is_set():
                if args.timeout_seconds > 0 and time.monotonic() >= deadline:
                    timed_out = True
                    _LOG.error("test timed out after %ds", args.timeout_seconds)
                    break
                if not _alive(qemu_pid):
                    crashed = True
                    _LOG.error("QEMU pid=%d exited unexpectedly", qemu_pid)
                    if logfile.exists():
                        tail = logfile.read_text(errors="replace")[-2000:]
                        _LOG.info("QEMU log tail:\n%s", tail)
                    break
                done.wait(timeout=0.1)
        finally:
            # Step 6: cleanup on all exit paths.
            done.set()
            try:
                uart_sock.close()
            except OSError:
                pass
            reader.join(timeout=2)
            _kill_qemu(qemu_pid)
            _active_pid.clear()

        if watcher.result is not None:
            return watcher.result
        if timed_out:
            return EXIT_TIMEOUT
        if crashed:
            return EXIT_QEMU_CRASH
        # Reader set done without a watcher match (clean EOF, no sentinel).
        return EXIT_TIMEOUT


if __name__ == "__main__":
    logging.basicConfig(format="%(levelname)s: %(message)s")
    sys.exit(_run(_parse_args()))
