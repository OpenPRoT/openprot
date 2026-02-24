# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Check license headers."""

import re
import logging
from pathlib import Path
from typing import Iterable, Sequence, TextIO

from pw_cli.plural import plural
from pw_presubmit.presubmit_context import PresubmitContext, PresubmitFailure
from pw_presubmit.presubmit import filter_paths

_LOG = logging.getLogger("license")

_EXCLUDE_FROM_LICENSE_CHECK: Sequence[str] = (
    # Configuration
    # keep-sorted: start
    r"MODULE.bazel.lock",
    r"\bCargo.lock$",
    r"\bDoxyfile$",
    r"\bconstraint.list$",
    r"\bconstraint_hashes_darwin.list$",
    r"\bconstraint_hashes_linux.list$",
    r"\bconstraint_hashes_windows.list$",
    r"\bpython_base_requirements.txt$",
    r"\bupstream_requirements_darwin_lock.txt$",
    r"\bupstream_requirements_linux_lock.txt$",
    r"\bupstream_requirements_windows_lock.txt$",
    r"^(?:.+/)?\.bazelignore$",
    r"^(?:.+/)?\.bazelversion$",
    r"^(?:.+/)?\.gitignore$",
    r"^\.github/.*",
    # keep-sorted: end
    # Metadata
    # keep-sorted: start
    r"\b.*OWNERS.*$",
    r"\bAUTHORS$",
    r"\bLICENSE$",
    r"\b\.vscodeignore$",
    r"\bgo.(mod|sum)$",
    r"\bpackage-lock.json$",
    r"\bpackage.json$",
    r"\bpnpm-lock.yaml$",
    r"\brequirements.txt$",
    r"\byarn.lock$",
    r"^docker/tag$",
    r"^patches.json$",
    # keep-sorted: end
    # Data files
    # keep-sorted: start
    r"\.bin$",
    r"\.csv$",
    r"\.der$",
    r"\.elf$",
    r"\.gif$",
    r"\.ico$",
    r"\.jpg$",
    r"\.json$",
    r"\.pem$",
    r"\.png$",
    r"\.svg$",
    r"\.vsix$",
    r"\.woff2",
    r"\.xml$",
    # keep-sorted: end
    # Documentation
    # keep-sorted: start
    r"\.expected$",
    r"\.md$",
    r"\.rst$",
    # keep-sorted: end
    # Generated protobuf files
    # keep-sorted: start
    r"\.pb\.c$",
    r"\.pb\.h$",
    r"\_pb2.pyi?$",
    # keep-sorted: end
    # Generated files
    # keep-sorted: start
    r"\btarget/earlgrey/registers/.*",
    # keep-sorted: end
    # Generated third-party files
    # keep-sorted: start
    r"\bdocs/src/third_party/.*",
    r"\bthird_party/.*\.bazelrc$",
    # keep-sorted: end
    # Diff/Patch files
    # keep-sorted: start
    r"\.diff$",
    r"\.patch$",
    # keep-sorted: end
)


_LICENSE = re.compile(
    r"""(#|//|::| \*|)( ?)Licensed under the Apache-2.0 license
\1\2SPDX-License-Identifier: Apache-2.0
""",
    re.MULTILINE,
)

_SKIP_LINE_PREFIXES = (
    "#!",
    "#autoload",
    "#compdef",
    "@echo off",
    ":<<",
    "/*",
    " * @jest-environment jsdom",
    " */",
    "{#",  # Jinja comment block
    "# -*- coding: utf-8 -*-",
    "<!--",
)


def _read_license_lines(file: TextIO) -> Iterable[str]:
    lines = iter(file)
    try:
        # Read until the first line of the copyright notice.
        line = next(lines)
        while line.isspace() or line.startswith(_SKIP_LINE_PREFIXES):
            line = next(lines)

        yield line

        for _ in range(1):  # The notice is 2 lines; read the remaining 1.
            yield next(lines)
    except StopIteration:
        pass


@filter_paths(exclude=_EXCLUDE_FROM_LICENSE_CHECK)
def license_check(ctx: PresubmitContext):
    """Checks that the license is present."""
    errors = []

    for path in ctx.paths:
        if path.stat().st_size == 0:
            continue  # Skip empty files

        try:
            with path.open() as file:
                if not _LICENSE.match("".join(_read_license_lines(file))):
                    errors.append(path)
        except UnicodeDecodeError as exc:
            raise PresubmitFailure(f"failed to read {path}") from exc

    if errors:
        _LOG.warning(
            "%s with a missing or incorrect license:\n%s",
            plural(errors, "file"),
            "\n".join(str(e) for e in errors),
        )
        raise PresubmitFailure
