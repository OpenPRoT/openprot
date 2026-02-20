# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Runs the local presubmit checks for the repository."""

import argparse
import logging
import os
import re
import sys
from typing import Pattern

from pw_cli import log

from pw_presubmit import (
    cli,
    json_check,
    keep_sorted,
)
from presubmit import cpp_include_guard, license
from pw_presubmit.install_hook import install_git_hook
from pw_presubmit.presubmit import Programs

_LOG = logging.getLogger("presubmit")

# Paths to completely exclude from presubmit checks.
_EXCLUDE_PATHS = ("\\bthird_party/.*\\.json$",)

EXCLUDES = tuple(re.compile(path) for path in _EXCLUDE_PATHS)

# Quick lint and format checks.
QUICK = (
    cpp_include_guard.include_guard_check,
    license.license_check,
    json_check.presubmit_check,
    # TODO(cfrantz): Determine how to enable the fix option for keep_sorted.
    # keep_sorted.presubmit_check,
)


def parse_args() -> dict:
    """Creates an argument parser and parses arguments."""

    parser = argparse.ArgumentParser(description=__doc__)
    cli.add_arguments(parser, Programs(quick=QUICK), "quick")
    parser.add_argument(
        "--install",
        action="store_true",
        help="Install the presubmit as a Git pre-push hook and exit.",
    )

    return vars(parser.parse_args())


def run(install: bool, exclude: list[Pattern[str]], **presubmit_args) -> int:
    """Entry point for presubmit."""

    if install:
        install_git_hook(
            "pre-push",
            ["./pw", "presubmit"],
        )
        return 0

    exclude.extend(EXCLUDES)
    return cli.run(exclude=exclude, **presubmit_args)


def main() -> int:
    """Run the presubmit for the repository."""
    # Change to working directory if running from Bazel.
    if "BUILD_WORKING_DIRECTORY" in os.environ:
        os.chdir(os.environ["BUILD_WORKING_DIRECTORY"])

    return run(**parse_args())


if __name__ == "__main__":
    log.install(logging.INFO)
    sys.exit(main())
