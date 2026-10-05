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
import os
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
        "--flash-image",
        type=str,
        help="Path to a raw SPI-NOR image to attach as the FMC CS0 flash "
        "(if=mtd). Re-seeded to an erased (0xFF) state of --flash-size bytes "
        "on every run so tests start from a known device state.",
    )
    parser.add_argument(
        "--flash-size",
        type=int,
        default=8 * 1024 * 1024,
        help="Size in bytes of the --flash-image backing store (default: 8 MiB).",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=30,
        help="Seconds to wait for a test result sentinel (0 = no timeout, default: 30)",
    )
    return parser.parse_known_args()


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


def _seed_flash_image(
    path: str, size: int, fill: int = 0xFF, contents: str = ""
) -> None:
    """Create/overwrite `path` with `size` bytes.

    Without `contents` the image is `size` bytes of `fill` (0xFF = erased).
    With it, the file is copied in at offset 0 and the remainder is left
    erased, so a test can hand the device a real image to read.

    A `contents` file longer than `size` is an error rather than a truncation:
    a half-written image still parses far enough to fail verification, which
    would make a verify-failure scenario pass for the wrong reason.
    """
    if not contents:
        with open(path, "wb") as f:
            f.write(bytes([fill & 0xFF]) * size)
        return

    data = Path(contents).read_bytes()
    if len(data) > size:
        raise ValueError(
            f"{contents} is {len(data)} bytes, larger than the {size}-byte "
            f"flash it seeds"
        )
    with open(path, "wb") as f:
        f.write(data)
        f.write(b"\xff" * (size - len(data)))


def _check_flash_contents(drives) -> bool:
    """Compare a backing image against what the guest said it wrote.

    AST10X0_CS1_EXPECT names a file, AST10X0_CS1_EXPECT_OFFSET where in the
    CS1 image it should appear. The guest's own verdict is a log line it
    produced about its own memory; this is the host reading the device's
    flash afterwards, which is the only check the guest cannot talk its way
    past. Returns True when there is nothing to check.
    """
    expect = os.environ.get("AST10X0_CS1_EXPECT")
    if not expect:
        return True

    cs1 = [path for index, path, _, _, _ in drives if index == 1]
    if not cs1:
        _LOG.error("AST10X0_CS1_EXPECT is set but no CS1 image was attached")
        return False

    want = Path(expect).read_bytes()
    offset = int(os.environ.get("AST10X0_CS1_EXPECT_OFFSET", "0"))
    with open(cs1[0], "rb") as f:
        f.seek(offset)
        got = f.read(len(want))

    if got == want:
        _LOG.info("CS1 holds the expected %d bytes at %#x", len(want), offset)
        return True

    if len(got) != len(want):
        _LOG.error(
            "CS1 image ends at %#x, short of the expected %d bytes",
            offset + len(got),
            len(want),
        )
        return False

    first = next(i for i, (a, b) in enumerate(zip(got, want)) if a != b)
    _LOG.error(
        "CS1 differs from the expected image at %#x: got %#04x, want %#04x",
        offset + first,
        got[first],
        want[first],
    )
    return False


def _resolve_flash_drives(args):
    """Return a list of (index, path, size, fill, contents) FMC backing images.

    index 0 -> FMC CS0, index 1 -> FMC CS1. Each image is re-seeded on every
    run so tests start from a known device state. An explicit --flash-image
    (manual runs) attaches at CS1. A flash_system_image_test sets
    AST10X0_CS0_IMAGE / AST10X0_CS1_IMAGE (basenames) plus AST10X0_FLASH_SIZE
    and per-CS AST10X0_CS0_FILL / AST10X0_CS1_FILL, resolved against
    $TEST_TMPDIR so each run gets private, freshly-seeded images.

    AST10X0_CS0_CONTENTS / AST10X0_CS1_CONTENTS name a file to copy in at
    offset 0 instead of filling, for a test that needs the device to already
    hold an image. The path is a runfile, resolved by the test rule.
    """
    base = os.environ.get("TEST_TMPDIR", tempfile.gettempdir())
    size = int(os.environ.get("AST10X0_FLASH_SIZE", str(args.flash_size)))
    drives = []
    if args.flash_image:
        drives.append((1, args.flash_image, size, 0xFF, ""))
    cs0 = os.environ.get("AST10X0_CS0_IMAGE")
    if cs0:
        fill = int(os.environ.get("AST10X0_CS0_FILL", "255"))
        contents = os.environ.get("AST10X0_CS0_CONTENTS", "")
        drives.append((0, os.path.join(base, cs0), size, fill, contents))
    cs1 = os.environ.get("AST10X0_CS1_IMAGE")
    if cs1:
        fill = int(os.environ.get("AST10X0_CS1_FILL", "255"))
        contents = os.environ.get("AST10X0_CS1_CONTENTS", "")
        drives.append((1, os.path.join(base, cs1), size, fill, contents))
    return drives


def _main(args) -> None:
    drives = _resolve_flash_drives(args)

    machine = args.machine
    if drives:
        # ast1030-evb models one flash type for the whole FMC controller.
        # w25q80bl matches internal flash on evb CS0, but is smaller than evb CS1.
        # Both CS share this model — only the attached backing images differ.
        model = os.environ.get("AST10X0_FMC_MODEL", "w25q80bl")
        machine = f"{machine},fmc-model={model}"

    qemu_args = [
        _QEMU_ARM,
        "-machine",
        machine,
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

    for index, path, size, fill, contents in drives:
        _seed_flash_image(path, size, fill, contents)
        qemu_args += [
            "-drive",
            f"file={path},format=raw,if=mtd,index={index}",
        ]

    if args.qemu_args:
        qemu_args.extend(args.qemu_args)

    _LOG.info("Invoking QEMU: %s", qemu_args)

    result = [None]  # 0 = pass, 1 = fail, None = no sentinel found

    with tempfile.NamedTemporaryFile() as f:
        with subprocess.Popen(
            args=qemu_args, stdout=f, stdin=subprocess.DEVNULL
        ) as proc:
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
                proc.wait(timeout=args.timeout if args.timeout > 0 else None)
            except KeyboardInterrupt:
                proc.kill()
                proc.wait()
            except subprocess.TimeoutExpired:
                _LOG.error(
                    "Test timed out after %ds — no sentinel detected",
                    args.timeout,
                )
                proc.kill()
                proc.wait()

            qemu_finished.set()

        stdout_thread.join(timeout=5)

    # Checked whatever the guest said, so a run that claims to have written
    # an image and did not fails here rather than passing.
    flash_ok = _check_flash_contents(drives)

    if result[0] is None:
        # No UART sentinel — check if QEMU exited naturally via semihosting.
        # Processes killed by timeout have a negative returncode (SIGKILL = -9).
        if proc.returncode >= 0:
            sys.exit(0 if proc.returncode == 0 and flash_ok else 1)
        _LOG.error("No TEST_RESULT sentinel found in UART output")
        sys.exit(1)

    sys.exit(result[0] if flash_ok else 1)


if __name__ == "__main__":
    known_args, remaining_args = _parse_args()
    if os.environ.get("PW_RUNNER_PASSTHROUGH") == "1":
        _LOG.info("Bypassing QEMU: %s", known_args.image)
        res = subprocess.run([known_args.image] + remaining_args)
        sys.exit(res.returncode)

    _main(known_args)
