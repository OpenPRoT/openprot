# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Utility to invoke the caliptra emulator and pipe the output through the detokenizer."""

import argparse
import logging
import subprocess
import sys
import tempfile
import threading
import time
import pathlib

from pathlib import Path
from pw_tokenizer import detokenize

_LOG = logging.getLogger(__name__)
_LOG.setLevel(logging.INFO)

try:

    import caliptra.emulator_cptra_rom  # type: ignore
    import caliptra.emulator_cptra_firmware  # type: ignore
    import caliptra.emulator_mcu_rom  # type: ignore
    import caliptra.emulator_exe  # type: ignore
    from python.runfiles import runfiles  # type: ignore

    r = runfiles.Create()
    _CPTRA_ROM = r.Rlocation(*caliptra.emulator_cptra_rom.RLOCATION)
    _CPTRA_FIRMWARE = r.Rlocation(*caliptra.emulator_cptra_firmware.RLOCATION)
    _MCU_ROM = r.Rlocation(*caliptra.emulator_mcu_rom.RLOCATION)
    _EMULATOR = r.Rlocation(*caliptra.emulator_exe.RLOCATION)
except ImportError as e:
    _LOG.fatal("runfiles could not open resources: %r", e)


def _parse_args():
    """Parse and return command line arguments."""

    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--interface",
        type=str,
        help="interface type",
    )
    parser.add_argument(
        "--elf",
        type=pathlib.Path,
        help="elf file ",
    )
    parser.add_argument(
        "--bin",
        type=pathlib.Path,
        help="bin file",
    )
    parser.add_argument(
        "--manifest",
        type=pathlib.Path,
        help="authorization manifest",
    )
    parser.add_argument(
        "--vendor-pk-hash",
        type=str,
        help="SHA384 of public keys",
    )

    return parser.parse_args()


def _detokenizer(image: Path, tokenized_file: Path, finished: threading.Event):
    try:
        detokenizer = detokenize.Detokenizer(image)
        line_buffer = ""
        with open(tokenized_file, "r", buffering=1) as f:
            while not finished.is_set():
                try:
                    chunk = f.readline()
                    if chunk:
                        # qemu may not write a complete line, so buffer
                        # the chunks until there is a complete line to
                        # pass to the detokenizer.
                        line_buffer += chunk

                        # Use a while loop, as there could also potentially
                        # be multiple lines printed in-between iterations.
                        while "\n" in line_buffer:
                            newline_pos = line_buffer.find("\n") + 1
                            complete_line = line_buffer[:newline_pos]
                            if not complete_line.endswith("\r\n"):
                                complete_line = complete_line.replace("\n", "\r\n")
                            detokenizer.detokenize_text_to_file(
                                complete_line, sys.stdout.buffer
                            )
                            sys.stdout.flush()

                            line_buffer = line_buffer[newline_pos:]
                except BlockingIOError:
                    # If writing to stdout too fast, it's sometimes possible
                    # to get BlockingIOError due to the stdout buffer being
                    # full, so sleep and try again.
                    time.sleep(0.1)

            # detokenize any remaining data in the buffer.
            if line_buffer:
                detokenizer.detokenize_text_to_file(complete_line, sys.stdout.buffer)
                sys.stdout.flush()
    except OSError as e:
        print(f"Exception opening file {e}", file=sys.stderr)


def load_and_run(
    image: Path,
    interface: str,
    manifest: str,
    vendor_pk_hash: str,
) -> list[str]:
    """Prepare arguments to load an image into a board and spawn a console."""
    if interface == "emulator":
        cmd = [
            _EMULATOR,
            f"--rom={_MCU_ROM}",
            f"--firmware={image}",
            f"--caliptra-rom={_CPTRA_ROM}",
            f"--caliptra-firmware={_CPTRA_FIRMWARE}",
            "--i3c-port=65534",
            "--rom-offset=0x80000000",
            "--rom-size=0x8000",
            "--dccm-offset=0x50000000",
            "--dccm-size=0x4000",
            "--sram-offset=0x40000000",
            "--sram-size=0x80000",
            "--pic-offset=0x60000000",
            "--i3c-offset=0x20004000",
            "--i3c-size=0x1000",
            "--mci-offset=0x21000000",
            "--mci-size=0xe00000",
            "--mbox-offset=0x30020000",
            "--mbox-size=0x28",
            "--soc-offset=0x30030000",
            "--soc-size=0x5e0",
            "--otp-offset=0x70000000",
            "--otp-size=0x140",
            "--lc-offset=0x70000400",
            "--lc-size=0x8c",
        ]
        if manifest and str(manifest) != "None":
            cmd.append(f"--soc-manifest={manifest}")
        if vendor_pk_hash and str(vendor_pk_hash) != "None":
            cmd.append(f"--vendor-pk-hash={vendor_pk_hash}")
        return cmd
    else:
        raise Exception("unknown mechanism", mechanism)


def simple_console(cmd: list[str]):
    """Invoke for a simple (non-tokenized) console."""
    _LOG.info("Invoking mcu emulator: %s", cmd)
    return subprocess.run(cmd, check=False).returncode


def tokenized_console(cmd: list[str]):
    """Invoke for a tokenized console."""
    _LOG.info("Invoking mcu emulator: %s", cmd)
    with tempfile.NamedTemporaryFile() as f:
        with subprocess.Popen(
            args=cmd,
            stdout=f,
        ) as proc:
            # Capturing the sub process stdout or stderr and then writing to
            # stdout can cause deadlocks (see
            # https://docs.python.org/3/library/subprocess.html#subprocess.Popen.stderr)
            # due to a write buffer (child process) filling up the pipe
            # buffer before the parent process can consume it.
            # To work around this, write to a temp file, and have the
            # detokenizer poll and detokenize the temp file.
            finished_event = threading.Event()
            stdout_thread = threading.Thread(
                target=_detokenizer,
                args=(Path(args.elf), Path(f.name), finished_event),
                daemon=True,
            )
            stdout_thread.start()
            out, err = proc.communicate()
            finished_event.set()
            if out:
                print(out)
            if err:
                print(err)
            return_code = proc.returncode
    stdout_thread.join()
    return return_code


def _main(args) -> int:
    cmd = load_and_run(
        args.bin,
        args.interface,
        args.manifest,
        args.vendor_pk_hash,
    )
    # TODO(cfrantz): add support for the tokenized console.
    return_code = simple_console(cmd)
    sys.exit(return_code)


if __name__ == "__main__":
    logging.basicConfig()
    _main(_parse_args())
