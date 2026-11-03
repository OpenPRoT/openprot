# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Reject Bazel's `features` attribute on Rust targets.

`features` is a Bazel-wide attribute (toolchain features), not a Cargo feature.
On a rules_rust target it is silently accepted and never sets a Rust feature,
so `#[cfg(feature = "...")]` code is compiled out and a test can pass without
running anything. Cargo features are set with `crate_features`.
"""

import ast
import logging

from pw_presubmit.presubmit import PresubmitContext, PresubmitFailure, filter_paths

_LOG = logging.getLogger("rust_features")

_RUST_RULES = frozenset(
    (
        # Pigweed wrapper that forwards **kwargs to rust_binary.
        "rust_app",
        "rust_binary",
        "rust_library",
        "rust_proc_macro",
        "rust_shared_library",
        "rust_static_library",
        "rust_test",
    )
)


def find_violations(source: str) -> list[tuple[int, str]]:
    """Returns (line, rule) for each Rust rule call that sets `features`.

    BUILD files are parsed as Python, which is a superset of the Starlark
    subset they use. Raises SyntaxError if the source cannot be parsed.
    """
    violations = []
    for node in ast.walk(ast.parse(source)):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        name = func.id if isinstance(func, ast.Name) else None
        if name not in _RUST_RULES:
            continue
        if any(kw.arg == "features" for kw in node.keywords):
            violations.append((node.lineno, name))
    return sorted(violations)


@filter_paths(endswith=(".bazel", ".bzl"), exclude=(r"\bthird_party/",))
def rust_features_check(ctx: PresubmitContext):
    """Checks that Rust targets use `crate_features`, not `features`."""
    failed = False
    for path in ctx.paths:
        try:
            violations = find_violations(path.read_text())
        except (SyntaxError, UnicodeDecodeError) as exc:
            raise PresubmitFailure(f"failed to parse {path}: {exc}") from exc
        for line, rule in violations:
            failed = True
            _LOG.warning(
                "%s:%d: `features` on %s is Bazel's toolchain attribute and "
                "does not enable Rust features; use `crate_features`",
                path,
                line,
                rule,
            )
    if failed:
        raise PresubmitFailure
