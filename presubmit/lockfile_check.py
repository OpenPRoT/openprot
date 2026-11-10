# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Fail if MODULE.bazel.lock is out of date.

Runs `bazel mod deps --lockfile_mode=error`, the same lockfile check the
release build uses (`--config=release`). Bazel's default `update` mode rewrites
a stale lockfile silently, so drift is otherwise only found at release time.
"""

import os
import shutil
import subprocess
import sys

_FIX = "bazelisk mod deps --lockfile_mode=update"


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
        [bazel, "mod", "deps", "--lockfile_mode=error"], cwd=cwd, check=False
    )
    if result.returncode != 0:
        print(
            f"\nMODULE.bazel.lock is out of date. Run `{_FIX}` and commit the "
            "result.",
            file=sys.stderr,
        )
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
