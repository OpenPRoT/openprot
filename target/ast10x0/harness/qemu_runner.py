# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""AST10x0 QEMU test runner.

Runs a firmware image under QEMU. Pass/fail is determined by whichever signal
arrives first: a TEST_RESULT:PASS/FAIL sentinel in UART output, or QEMU's own
exit code from a semihosting exit() call. Semihosting is always enabled in
QEMU (harmless when unused), so both signalling mechanisms work transparently.
"""

import argparse
import logging
import subprocess
import sys
import tempfile
import threading
import time

from pathlib import Path
from pw_tokenizer import detokenize

_LOG = logging.getLogger(__name__)
_LOG.setLevel(logging.INFO)

try:
    # qemu-system-arm-runfiles is a pw_py_importable_runfile target from the
    # qemu repo (canonical: @@pigweed++_repo_rules5+qemu). If this import
    # breaks after a pigweed upgrade, run:
    #   ls $(bazel info output_base)/external/ | grep qemu
    import qemu.qemu_system_arm  # type: ignore
    from python.runfiles import runfiles  # type: ignore

    r = runfiles.Create()
    assert r is not None
    _QEMU_ARM = r.Rlocation(*qemu.qemu_system_arm.RLOCATION)
except ImportError as e:
    print(f"Fatal: runfiles could not find qemu: {e}", file=sys.stderr)
    sys.exit(1)

assert _QEMU_ARM is not None

PASS_SENTINEL = b"TEST_RESULT:PASS"
FAIL_SENTINEL = b"TEST_RESULT:FAIL"
TIMEOUT_SECONDS = 30


def _parse_args():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--machine", type=str, help="qemu machine type")
    parser.add_argument("--cpu", type=str, help="qemu cpu type")
    parser.add_argument("--image", type=str, help="image file to run")
    parser.add_argument(
        "--qemu-args", nargs="*", help="Extra arguments to pass to qemu"
    )
    parser.add_argument(
        "--no-timeout",
        action="store_true",
        help="Run indefinitely; for stress tests that never emit PASS",
    )
    return parser.parse_args()


def _detokenizer(image: Path, tokenized_file: Path, qemu_finished: threading.Event):
    try:
        detokenizer = detokenize.Detokenizer(image)
        line_buffer = ""
        with open(tokenized_file, "r", buffering=1) as f:
            while not qemu_finished.is_set():
                try:
                    chunk = f.readline()
                    if chunk:
                        line_buffer += chunk
                        while "\n" in line_buffer:
                            newline_pos = line_buffer.find("\n") + 1
                            complete_line = line_buffer[:newline_pos]
                            detokenizer.detokenize_text_to_file(
                                complete_line, sys.stdout.buffer
                            )
                            sys.stdout.flush()
                            line_buffer = line_buffer[newline_pos:]
                except BlockingIOError:
                    time.sleep(0.1)
        if line_buffer:
            detokenizer.detokenize_text_to_file(line_buffer, sys.stdout.buffer)
            sys.stdout.flush()
    except OSError as e:
        print(f"Exception opening file {e}", file=sys.stderr)


def _sentinel_watcher(
    tokenized_file: Path,
    result: list,
    qemu_finished: threading.Event,
    proc: subprocess.Popen,
):
    buf = b""
    try:
        with open(tokenized_file, "rb") as f:
            while not qemu_finished.is_set():
                chunk = f.read(256)
                if chunk:
                    buf += chunk
                    if PASS_SENTINEL in buf:
                        result[0] = 0
                        proc.kill()
                        return
                    if FAIL_SENTINEL in buf:
                        result[0] = 1
                        proc.kill()
                        return
                else:
                    time.sleep(0.01)
    except OSError as e:
        print(f"Exception watching sentinel: {e}", file=sys.stderr)


def _main(args) -> None:
    qemu_args = [
        _QEMU_ARM,
        "-machine",
        args.machine,
        "-cpu",
        args.cpu,
        "-bios",
        "none",
        "-nographic",
        "-serial",
        "mon:stdio",
        "-semihosting-config",
        "enable=on,target=native",
        "-kernel",
        args.image,
    ]

    if args.qemu_args:
        qemu_args.extend(args.qemu_args)

    _LOG.info("Invoking QEMU: %s", qemu_args)

    result = [None]  # 0 = pass, 1 = fail, None = no sentinel found

    with tempfile.NamedTemporaryFile() as f:
        with subprocess.Popen(args=qemu_args, stdout=f, stdin=subprocess.DEVNULL) as proc:
            qemu_finished = threading.Event()
            sentinel_thread = threading.Thread(
                target=_sentinel_watcher,
                args=(Path(f.name), result, qemu_finished, proc),
                daemon=True,
            )
            stdout_thread = threading.Thread(
                target=_detokenizer,
                args=(Path(args.image), Path(f.name), qemu_finished),
                daemon=True,
            )
            sentinel_thread.start()
            stdout_thread.start()

            try:
                if args.no_timeout:
                    proc.wait()
                else:
                    proc.wait(timeout=TIMEOUT_SECONDS)
            except KeyboardInterrupt:
                proc.kill()
                proc.wait()
            except subprocess.TimeoutExpired:
                _LOG.error(
                    "Test timed out after %ds — no sentinel detected",
                    TIMEOUT_SECONDS,
                )
                proc.kill()
                proc.wait()

            qemu_finished.set()

        stdout_thread.join(timeout=5)

    if result[0] is None:
        # No UART sentinel — check if QEMU exited naturally via semihosting.
        # Processes killed by timeout have a negative returncode (SIGKILL = -9).
        if proc.returncode >= 0:
            sys.exit(0 if proc.returncode == 0 else 1)
        _LOG.error("No TEST_RESULT sentinel found in UART output")
        sys.exit(1)

    sys.exit(result[0])


if __name__ == "__main__":
    _main(_parse_args())
