# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

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
    import opentitan.opentitantool  # type: ignore
    import opentitan.verilator  # type: ignore
    import opentitan.test_rom  # type: ignore
    import opentitan.test_otp  # type: ignore
    import opentitan.rom_ext_cw310  # type: ignore
    import opentitan.rom_ext_cw340  # type: ignore
    import opentitan.bitstream_hyper310  # type: ignore
    import opentitan.bitstream_hyper340  # type: ignore
    import opentitan.fake_keys.app_prod_ecdsa  # type: ignore
    from python.runfiles import runfiles  # type: ignore

    r = runfiles.Create()
    _OTTO = r.Rlocation(*opentitan.opentitantool.RLOCATION)
    _VERILATOR = r.Rlocation(*opentitan.verilator.RLOCATION)
    _TEST_ROM = r.Rlocation(*opentitan.test_rom.RLOCATION)
    _TEST_OTP = r.Rlocation(*opentitan.test_otp.RLOCATION)
    _PRODKEY = r.Rlocation(*opentitan.fake_keys.app_prod_ecdsa.RLOCATION)
    _ROM_EXT = {
        "hyper310": r.Rlocation(*opentitan.rom_ext_cw310.RLOCATION),
        "hyper340": r.Rlocation(*opentitan.rom_ext_cw340.RLOCATION),
    }
    _BITSTREAM = {
        "hyper310": r.Rlocation(*opentitan.bitstream_hyper310.RLOCATION),
        "hyper340": r.Rlocation(*opentitan.bitstream_hyper340.RLOCATION),
    }
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
        help="opentitantool interface type",
    )
    parser.add_argument(
        "--load-bitstream",
        type=bool,
        action=argparse.BooleanOptionalAction,
        help="load a bitstream into the FPGA board",
    )
    parser.add_argument(
        "--mechanism",
        type=str,
        default="bootstrap",
        choices=["bootstrap", "rescue"],
        help="mechanism used to load the image",
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
        "--exit-success",
        type=str,
        default=None,
        help="regex to determine success",
    )
    parser.add_argument(
        "--exit-failure",
        type=str,
        default=None,
        help="regex to determine failure",
    )

    return parser.parse_args()


def _detokenizer(image: Path, tokenized_file: Path, otto_finished: threading.Event):
    try:
        detokenizer = detokenize.Detokenizer(image)
        line_buffer = ""
        with open(tokenized_file, "r", buffering=1) as f:
            while not otto_finished.is_set():
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


def transport_init(interface: str):
    subprocess.run(
        [
            _OTTO,
            f"--interface={interface}",
            "transport",
            "init",
        ],
        check=True,
    )


def load_bitstream(interface: str):
    """Load an appropriate bitstream for the given board type."""
    try:
        bitstream = _BITSTREAM[interface]
    except KeyError:
        _LOG.error("No bitstream for board %s", interface)
        sys.exit(1)

    _LOG.info("Loading bitstream: %s", bitstream)
    subprocess.run(
        [
            _OTTO,
            f"--interface={interface}",
            "fpga",
            "load-bitstream",
            bitstream,
        ],
        check=True,
    )


def load_and_run(
    image: Path, interface: str, mechanism: str, exit_success: str, exit_failure: str
) -> list[str]:
    """Prepare opentitantool arguments to load an image into a board and spawn a console."""
    if interface == "verilator":
        # When the interface is verilator, we don't use any of the normal image
        # loading mechanisms.  Instead, the verilator test bench loads the image
        # directly into flash memory before starting the simulation.
        load_command = [
            f"--verilator-bin={_VERILATOR}",
            f"--verilator-rom={_TEST_ROM}",
            f"--verilator-otp={_TEST_OTP}",
            f"--verilator-flash={image}",
        ]
    elif mechanism == "bootstrap":
        try:
            rom_ext = _ROM_EXT[interface]
        except KeyError:
            _LOG.error("No ROM_EXT for board %s", interface)
            sys.exit(1)

        # To load via bootstrap, we need to assemble a complete flash image that
        # includes the ROM_EXT and the application image.  Once assembled, we
        # load that image into the device.
        boot_img = image.with_suffix(".img")
        load_command = [
            "--exec",
            f"image assemble --mirror=false --output={boot_img} {rom_ext}@0 {image}@0x10000",
            "--exec",
            f"bootstrap --clear-uart=true {boot_img}",
        ]
    elif mechanism == "rescue":
        # To load via rescue, we assume that the device already has a functional
        # ROM_EXT and we trigger the serial rescue protocol to load the image.
        load_command = ["--exec", f"rescue firmware {image}"]
    else:
        raise Exception("unknown mechanism", mechanism)

    console_command = ["console", "--timestamp"]
    if exit_success:
        console_command.append(f"--exit-success={exit_success}")
    if exit_failure:
        console_command.append(f"--exit-failure={exit_failure}")

    if exit_success and exit_failure:
        console_command.append("--non-interactive")

    return (
        [
            _OTTO,
            f"--interface={interface}",
        ]
        + load_command
        + console_command
    )


def simple_console(opentitantool_cmd: list[str]):
    """Invoke opentitantool for a simple (non-tokenized) console."""
    _LOG.info("Invoking opentitantool: %s", opentitantool_cmd)
    process = subprocess.run(opentitantool_cmd, check=False)
    return process.returncode


def tokenized_console(opentitantool_cmd: list[str]):
    """Invoke opentitantool for a tokenized console."""
    _LOG.info("Invoking opentitantool: %s", opentitantool_cmd)
    with tempfile.NamedTemporaryFile() as f:
        with subprocess.Popen(
            args=otto_args,
            stdout=f,
        ) as proc:
            # Capturing the sub process stdout or stderr and then writing to
            # stdout can cause deadlocks (see
            # https://docs.python.org/3/library/subprocess.html#subprocess.Popen.stderr)
            # due to a write buffer (child process) filling up the pipe
            # buffer before the parent process can consume it.
            # To work around this, write to a temp file, and have the
            # detokenizer poll and detokenize the temp file.
            otto_finished_event = threading.Event()
            stdout_thread = threading.Thread(
                target=_detokenizer,
                args=(Path(args.elf), Path(f.name), otto_finished_event),
                daemon=True,
            )
            stdout_thread.start()
            out, err = proc.communicate()
            otto_finished_event.set()
            if out:
                print(out)
            if err:
                print(err)
            return_code = proc.returncode
    stdout_thread.join()
    return return_code


def _main(args) -> int:
    if args.interface == "verilator":
        # Verilator loads the unsigned binary. The build process for the
        # kernel will detect when verilator is the target and will place
        # a ROM_EXT header that will jump directly into the kernel.
        pass
    else:
        transport_init(args.interface)
        if args.load_bitstream:
            load_bitstream(args.interface)

    cmd = load_and_run(
        args.bin, args.interface, args.mechanism, args.exit_success, args.exit_failure
    )
    # TODO(cfrantz): add support for the tokenized console.
    return_code = simple_console(cmd)
    sys.exit(return_code)


if __name__ == "__main__":
    logging.basicConfig()
    _main(_parse_args())
