# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Check C/C++ include guards."""

import os
from pathlib import Path

from pw_presubmit import (
    cpp_checks,
)

PROJECT = "openprot"


def guard_name(path: Path) -> str:
    """Transform the path into the required include guard."""
    # The presubmit tool runs in the root of the project.
    # Compute the path relative to the project root.
    path = path.relative_to(os.getcwd())
    guard = f"{PROJECT}_{path}".replace("/", "_").replace(".", "_")
    return guard.upper()


include_guard_check = cpp_checks.include_guard_check(guard_name)
