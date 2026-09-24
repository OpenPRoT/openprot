# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Utility to invoke the caliptra emulator and pipe the output through the detokenizer."""

import argparse
import logging
import os
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

# Environment variable naming the SSH host (user@host or host alias) for the
# VCK190 FPGA board used by the "fpga" interface.
FPGA_HOST = "VCK190_FPGA_HOST"

# Timeout (seconds) for the remote fpga run. The VeeR core's exit()
# implementation writes the PASS/FAIL sentinel and then spins forever (there's
# no way for it to fully halt itself back to the host), so this bounds how
# long we wait on a stuck or unreachable board rather than hanging forever.
_FPGA_RUN_TIMEOUT_SECONDS = 300


def scan_output_for_result(lines):
    """Scan detokenized output lines for a PASS/FAIL sentinel.

    Returns 0 on a line containing "PASS", 1 on a line containing "FAIL",
    or None if no sentinel has appeared yet.
    """
    for line in lines:
        if "PASS" in line:
            return 0
        if "FAIL" in line:
            return 1
    return None


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
    elf: Path | None = None,
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
    elif interface == "fpga":
        host = os.environ.get(FPGA_HOST)
        if not host:
            _LOG.fatal("%s is not set; cannot reach the VCK190 board", FPGA_HOST)
            sys.exit(1)

        remote_bin = f"/tmp/{Path(image).name}"
        try:
            subprocess.run(["scp", str(image), f"{host}:{remote_bin}"], check=True)
        except subprocess.CalledProcessError as e:
            _LOG.fatal("Failed to copy %s to %s: %s", image, host, e)
            sys.exit(1)

        # Loads the image into the MCU ROM backdoor SRAM, deasserts
        # cptra_ss_rst_b, and streams the debug FIFO back over stdout.
        # See hw/fpga/README.md's "JTAG debug" section and
        # hw/fpga/kernel-modules/mcu_rom_backdoor.c for the mechanism.
        cmd = [
            "ssh",
            host,
            "sudo",
            "caliptra-mcu-sw/hw/fpga/launch_openocd.sh",
            "load-and-run",
            remote_bin,
        ]
        _LOG.info("Invoking fpga runner: %s", cmd)
        try:
            proc = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
                timeout=_FPGA_RUN_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as e:
            _LOG.fatal(
                "fpga runner timed out after %s seconds; stdout so far: %s; "
                "stderr so far: %s",
                _FPGA_RUN_TIMEOUT_SECONDS,
                e.stdout,
                e.stderr,
            )
            sys.exit(1)

        if proc.stderr:
            _LOG.info("fpga runner stderr: %s", proc.stderr)

        if proc.returncode != 0:
            _LOG.fatal(
                "ssh/remote command failed with exit code %d: %s",
                proc.returncode,
                proc.stderr,
            )
            sys.exit(1)

        # This target's kernel config pins its log backend to
        # log_backend_basic (a plain-text logger; see target/veer/BUILD.bazel's
        # platform rule), so the board's console output is never tokenized and
        # there is nothing to detokenize here, unlike the emulator's tokenized
        # console path (see _detokenizer() above).
        print(proc.stdout)
        result = scan_output_for_result(proc.stdout.splitlines())
        if result is None:
            _LOG.fatal(
                "Device produced no PASS/FAIL sentinel; fpga runner stderr: %s",
                proc.stderr,
            )
            sys.exit(1)
        sys.exit(result)
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
        elf=args.elf,
    )
    # TODO(cfrantz): add support for the tokenized console.
    return_code = simple_console(cmd)
    sys.exit(return_code)


if __name__ == "__main__":
    logging.basicConfig()
    _main(_parse_args())
