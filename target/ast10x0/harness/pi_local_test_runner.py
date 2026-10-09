#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""
Runs the paired PLDM firmware update demo from the Pi, with no Bazel involved.

test_runner.py does the same thing from a workstation, but it builds the images
and SCPs them over first. Here the images and ELFs are already sitting beside
this script, so the pins and devices from evb_config.toml are inlined and
pi_test_runner.py is called directly.

This terminal shows both boards interleaved. Each board's lines are also written
to logs/rot.log and logs/bmc.log, so a second and third SSH session can `tail -F`
one board each and watch the two sides separately. That is done for you in one
tmux window by default (`--no-tmux` opts out), and the session outlives the SSH connection -- a dropped link loses
nothing, since `tmux attach -t demo` gets back in front of it.
"""

import argparse
import os
import shlex
import subprocess
import sys
import time
from pathlib import Path

import pi_test_runner

_HERE = Path(__file__).resolve().parent
_LOG_DIR = _HERE / "logs"

SRST_PIN = 23
FWSPICK_PIN = 18
RESET_PASSTHROUGH_PIN = 8
UART_DEVICE = "/dev/ttyUSB0"
BAUDRATE = 115200

SLAVE_SRST_PIN = 25
SLAVE_FWSPICK_PIN = 24
SLAVE_UART_DEVICE = "/dev/ttyUSB1"

# Kernel bring-up chatter, printed by every board on every one of the four
# boots this demo performs. It comes from pigweed's own pw_kernel, which has no
# log-level knob, so dropping it on the way to the terminal is the only lever.
# The per-board .log files are written through a different handle and keep it.
_BOOT_NOISE = (
    b"Hello World!",
    b"ast1060 pigweed fw is running!",
    b"Initializing NVIC",
    b"Welcome to Maize",
    b"Cortex-M early initialization",
    b"CPUID:",
    b"MPU regions:",
    b"Starting monotonic SysTick timer",
    b"Created initial thread; bootstrapping",
    b"Context switching to first thread",
    b"Welcome to the first thread, continuing bootstrap",
    b"Cortex-M initialization",
    b"Allocating non-privileged",
)

# Per-fragment MCTP sender chatter, logged by both boards. Hidden from the
# panes only: the .log files keep it, and the board still emits it either way.
_PANE_NOISE = (
    "Calling fragment_vectored",
    "packet sending to",
    "Encoded packet length",
    "packet sent",
    # tail's own notice when the demo rewrites a log under it.
    "file truncated",
)


def _tail_cmd(log: Path) -> str:
    """tail -F a board log, minus the lines _PANE_NOISE and _BOOT_NOISE name."""
    noise = list(_PANE_NOISE) + [p.decode() for p in _BOOT_NOISE]
    pats = " ".join(f"-e {shlex.quote(p)}" for p in noise)
    return f"tail -F {shlex.quote(str(log))} 2>&1 | grep --line-buffered -vF {pats}"


class _QuietStdout:
    """Stands in for sys.stdout, dropping the _BOOT_NOISE lines.

    pi_test_runner writes board output as bytes through sys.stdout.buffer, so
    this object serves as its own .buffer and filters there. Text writes --
    the runner's own progress prints -- pass straight through.

    With drop_all, no board output reaches this pane at all; under --tmux the
    two tail panes are the board views, so repeating them here is noise.
    """

    def __init__(self, real, drop_all=False):
        self._real = real
        self._drop_all = drop_all
        self.buffer = self

    def write(self, data):
        if isinstance(data, str):
            return self._real.write(data)
        if self._drop_all:
            return 0
        kept = b"".join(
            line + b"\n"
            for line in data.split(b"\n")
            if line and not any(noise in line for noise in _BOOT_NOISE)
        )
        return self._real.buffer.write(kept) if kept else 0

    def flush(self):
        self._real.flush()


_slave_upload_patched = False
_slave_staging_verdict_pending = False


def _boot_slave_from_flash() -> None:
    """Make later runs start the mock BMC from the firmware already in its flash.

    Its FMC CS0 still holds what the first run put there, so the upload --
    fwspick mode, wait for the bootloader, send the image, read the bootstrap's
    staging verdict -- has nothing left to do, and skipping it is what lets the
    version carry on climbing instead of being reset to the one in the image on
    this host. The reset that follows it is the only step that still matters and
    is left alone. The RoT is uploaded every run regardless: its image lives in
    SRAM and does not survive the reset.
    """
    global _slave_upload_patched, _slave_staging_verdict_pending
    _slave_staging_verdict_pending = True
    if _slave_upload_patched:
        return
    _slave_upload_patched = True

    def _is_slave(port) -> bool:
        return getattr(port, "port", None) == SLAVE_UART_DEVICE

    real_fwspick = pi_test_runner._sequence_to_fwspick_mode
    real_ready = pi_test_runner._wait_for_uart_ready
    real_upload = pi_test_runner._upload_firmware
    real_stream = pi_test_runner._stream_uart

    def fwspick(srst_pin, fwspick_pin, port):
        if not _is_slave(port):
            real_fwspick(srst_pin, fwspick_pin, port)

    def ready(port, *a, **kw):
        return True if _is_slave(port) else real_ready(port, *a, **kw)

    def upload(port, *a, **kw):
        if not _is_slave(port):
            real_upload(port, *a, **kw)

    def stream(port, *a, **kw):
        # The staging verdict is the first thing read from the mock BMC, and no
        # bootstrap ran to print one. Everything read after it is the real
        # firmware's and goes through untouched.
        global _slave_staging_verdict_pending
        if _is_slave(port) and _slave_staging_verdict_pending:
            _slave_staging_verdict_pending = False
            return True
        return real_stream(port, *a, **kw)

    pi_test_runner._sequence_to_fwspick_mode = fwspick
    pi_test_runner._wait_for_uart_ready = ready
    pi_test_runner._upload_firmware = upload
    pi_test_runner._stream_uart = stream


def _relaunch_in_tmux(session: str) -> int:
    """Re-runs this script in a tmux session showing the two boards side by side.

    The two board logs take the bulk of the window, with a short full-width pane
    beneath them for the harness itself -- its progress lines, the verdict, and
    the --repeat prompt. Board output is suppressed there, so that pane carries
    only what the log panes above don't already show.

    Living in a tmux session also means a dropped SSH connection doesn't kill
    the run."""
    if (
        subprocess.run(
            ["tmux", "has-session", "-t", session],
            capture_output=True,
        ).returncode
        == 0
    ):
        print(
            f"Session '{session}' already exists. Attach to it with:", file=sys.stderr
        )
        print(f"  tmux attach -t {session}", file=sys.stderr)
        print("Or end it with:", file=sys.stderr)
        print(f"  tmux kill-session -t {session}", file=sys.stderr)
        return 1

    # tmux stays on for the inner run, which reads it to keep board output out
    # of its own pane. TMUX is set inside the session, so the guard
    # in main() stops it relaunching itself.
    inner = [sys.executable, str(Path(__file__).resolve())]
    skip_next = False
    for arg in sys.argv[1:]:
        if skip_next:
            skip_next = False
        elif arg == "--tmux-session":
            skip_next = True
        elif not arg.startswith("--tmux-session="):
            inner.append(arg)

    here = str(_HERE)
    # -F rather than -f: the run truncates both logs as it starts, and only
    # capital F follows a file through truncation. It also waits for a log that
    # does not exist yet, which is the case until the harness pane starts.
    subprocess.run(
        [
            "tmux",
            "new-session",
            "-d",
            "-s",
            session,
            "-n",
            "boards",
            "-c",
            here,
            _tail_cmd(_LOG_DIR / "rot.log"),
        ],
        check=True,
    )
    subprocess.run(
        [
            "tmux",
            "split-window",
            "-h",
            "-t",
            f"{session}:boards",
            "-c",
            here,
            _tail_cmd(_LOG_DIR / "bmc.log"),
        ],
        check=True,
    )
    # -f makes the split span the whole window rather than just the pane it was
    # taken from, so the harness sits under both logs. It drops to a shell
    # afterwards so the verdict stays readable instead of the pane closing the
    # moment the run ends.
    subprocess.run(
        [
            "tmux",
            "split-window",
            "-v",
            "-f",
            "-l",
            "5",
            "-t",
            f"{session}:boards",
            "-c",
            here,
            f"{shlex.join(inner)}; echo; echo '[runner exited]'; exec bash",
        ],
        check=True,
    )

    # The harness pane is the one that reads keystrokes, for --repeat's prompt.
    subprocess.run(["tmux", "select-pane", "-t", f"{session}:boards.2"], check=True)
    return subprocess.run(["tmux", "attach", "-t", f"{session}:boards"]).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--firmware",
        default=str(_HERE / "pldm_fd_image.bin"),
        help="RoT image uploaded to device A",
    )
    parser.add_argument(
        "--slave-firmware",
        default=str(_HERE / "bootstrap_image.bin"),
        help="Mock BMC image uploaded to device B",
    )
    parser.add_argument(
        "--elf",
        action="append",
        default=None,
        help="ELF whose tokens the logs are decoded against; repeatable. "
        "Defaults to every .elf beside this script.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Show the kernel bring-up lines this script normally hides. "
        "The per-board logs always have them either way.",
    )
    parser.add_argument(
        "--no-repeat",
        dest="repeat",
        action="store_false",
        help="Exit after one run instead of prompting to run again. The prompt "
        "is in the bottom pane, not the board panes.",
    )
    parser.add_argument(
        "--no-tmux",
        dest="tmux",
        action="store_false",
        help="Run in this terminal instead of a tmux window with each board's "
        "log in its own pane.",
    )
    parser.add_argument(
        "--tmux-session",
        default="demo",
        help="tmux session name",
    )
    args = parser.parse_args()

    if args.tmux and not os.environ.get("TMUX"):
        return _relaunch_in_tmux(args.tmux_session)

    elfs = args.elf if args.elf else sorted(str(p) for p in _HERE.glob("*.elf"))
    if not elfs:
        print(
            f"Error: no .elf found in {_HERE}; logs will print as $base64",
            file=sys.stderr,
        )
        return 1

    _LOG_DIR.mkdir(parents=True, exist_ok=True)
    print(
        f"Per-board logs: tail -F {_LOG_DIR}/rot.log   (and bmc.log)", file=sys.stderr
    )
    if args.repeat and args.tmux:
        print("Repeat prompt is in the bottom pane.", file=sys.stderr)

    argv = [
        UART_DEVICE,
        args.firmware,
        "--srst-pin",
        str(SRST_PIN),
        "--fwspick-pin",
        str(FWSPICK_PIN),
        "--baudrate",
        str(BAUDRATE),
        "--slave-firmware",
        args.slave_firmware,
        "--slave-uart-device",
        SLAVE_UART_DEVICE,
        "--slave-srst-pin",
        str(SLAVE_SRST_PIN),
        "--slave-fwspick-pin",
        str(SLAVE_FWSPICK_PIN),
        "--slave-stages-to-flash",
        "--reset-passthrough-pin",
        str(RESET_PASSTHROUGH_PIN),
        "--log-dir",
        str(_LOG_DIR),
    ]
    for elf in elfs:
        argv += ["--elf", elf]

    if args.tmux:
        sys.stdout = _QuietStdout(sys.stdout, drop_all=True)
    elif not args.verbose:
        sys.stdout = _QuietStdout(sys.stdout)

    first_run = True
    while True:
        # Truncated per run so a `tail -F` pane shows the current run only.
        for label in ("rot", "bmc"):
            (_LOG_DIR / f"{label}.log").write_bytes(b"")
        if not first_run:
            _boot_slave_from_flash()
        # Set once at import, so without this every run's stamps carry on from
        # where the last one stopped.
        pi_test_runner._T0 = time.monotonic()
        sys.argv = [sys.argv[0]] + argv
        rc = pi_test_runner.main()
        first_run = False

        if not args.repeat:
            return rc
        # Parked until the next run's reset wakes it, so it cannot start an
        # update against a RoT that has already reported and shut down. Only
        # safe here: the mirror thread drives this same line, and main() has
        # stopped it by the time it returns.
        pi_test_runner._gpio_set(SLAVE_SRST_PIN, "dl")
        print(f"\n[run exited {rc}]", file=sys.stderr)
        try:
            answer = input("Enter to run again, q to quit: ")
        except EOFError:
            return rc
        if answer.strip().lower().startswith("q"):
            return rc


if __name__ == "__main__":
    sys.exit(main())
