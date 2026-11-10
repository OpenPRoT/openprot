# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Fail if the committed MODULE.bazel.lock is out of date.

Bazel's default `--lockfile_mode=update` silently rewrites a stale lockfile, and
`./pw` itself runs Bazel before this check, so `--lockfile_mode=error` would
only ever validate a file that was already regenerated. Instead, regenerate the
lockfile and fail if it differs from what git has.
"""

import os
import shutil
import subprocess
import sys

_LOCKFILE = "MODULE.bazel.lock"


def main() -> int:
    bazel = shutil.which("bazelisk") or shutil.which("bazel")
    if bazel is None:
        print("lockfile check: neither bazelisk nor bazel is on PATH", file=sys.stderr)
        return 2

    # `bazel run` executes from the runfiles tree; run in the user's workspace.
    cwd = os.environ.get("BUILD_WORKSPACE_DIRECTORY") or os.environ.get(
        "BUILD_WORKING_DIRECTORY", "."
    )

    result = subprocess.run(
        [bazel, "mod", "deps", "--lockfile_mode=update"], cwd=cwd, check=False
    )
    if result.returncode != 0:
        return result.returncode

    # Compares the working tree to the index, which is HEAD in CI.
    diff = subprocess.run(
        ["git", "diff", "--exit-code", "--", _LOCKFILE], cwd=cwd, check=False
    )
    if diff.returncode != 0:
        print(
            f"\n{_LOCKFILE} is out of date: it differs from the file regenerated "
            "from the current module inputs.\n"
            "To fix it, run:\n"
            "    bazelisk mod deps --lockfile_mode=update\n"
            f"then commit the updated {_LOCKFILE}.",
            file=sys.stderr,
        )
    return diff.returncode


if __name__ == "__main__":
    sys.exit(main())
