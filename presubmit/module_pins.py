# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Keep dependency pins identical across the repo's MODULE.bazel files.

Bazel only honors `git_override` (and the like) and extension settings from the
root module. A nested module such as `target/ast10x0` is a root when built on
its own, so it repeats the pins it needs, e.g. the Pigweed commit. Nothing in
Bazel notices when the copies drift, so check it here.

Compared pins:
  * the `commit` of every `git_override`, keyed by `module_name`
  * the `cipd_tag` of `pw_rust.toolchain(...)`
A pin is only compared if two or more MODULE.bazel files set it.

This is a stop-gap: it detects drift but does not remove the duplication. If
the pins move to a single source (e.g. an in-repo Bazel registry), delete it.
"""

import ast
import logging
import subprocess

from pw_presubmit.presubmit import PresubmitContext, PresubmitFailure, filter_paths

_LOG = logging.getLogger("module_pins")


def _literal(node: ast.expr):
    return node.value if isinstance(node, ast.Constant) else None


def find_pins(source: str) -> dict[str, str]:
    """Returns {pin name: value} for the pins set in one MODULE.bazel.

    MODULE.bazel is parsed as Python, which is a superset of the Starlark subset
    it uses. Raises SyntaxError if the source cannot be parsed.
    """
    pins = {}
    for node in ast.walk(ast.parse(source)):
        if not isinstance(node, ast.Call):
            continue
        kwargs = {kw.arg: _literal(kw.value) for kw in node.keywords}
        func = node.func
        if isinstance(func, ast.Name) and func.id == "git_override":
            if kwargs.get("module_name") and kwargs.get("commit"):
                pins[f"git_override {kwargs['module_name']}"] = kwargs["commit"]
        elif isinstance(func, ast.Attribute) and func.attr == "toolchain":
            if kwargs.get("cipd_tag"):
                pins["pw_rust cipd_tag"] = kwargs["cipd_tag"]
    return pins


def find_mismatches(sources: dict[str, str]) -> list[str]:
    """Returns a message for each pin that differs between files.

    `sources` maps a file path to the contents of that MODULE.bazel.
    """
    by_pin: dict[str, dict[str, str]] = {}
    for path, source in sources.items():
        for pin, value in find_pins(source).items():
            by_pin.setdefault(pin, {})[path] = value

    messages = []
    for pin, values in sorted(by_pin.items()):
        if len(set(values.values())) > 1:
            detail = ", ".join(f"{p}={v}" for p, v in sorted(values.items()))
            messages.append(f"{pin} differs: {detail}")
    return messages


def _module_files(root) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "--", "MODULE.bazel", "**/MODULE.bazel"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return out.split()


@filter_paths(endswith="MODULE.bazel")
def module_pins_check(ctx: PresubmitContext):
    """Checks that pins repeated in several MODULE.bazel files agree."""
    sources = {}
    # Read every MODULE.bazel, not just the changed ones: the other copy of a
    # pin is usually the one that was not touched.
    for name in _module_files(ctx.root):
        path = ctx.root / name
        try:
            sources[name] = path.read_text()
        except UnicodeDecodeError as exc:
            raise PresubmitFailure(f"failed to read {path}: {exc}") from exc
    try:
        messages = find_mismatches(sources)
    except SyntaxError as exc:
        raise PresubmitFailure(f"failed to parse a MODULE.bazel: {exc}") from exc
    for message in messages:
        _LOG.warning("%s", message)
    if messages:
        raise PresubmitFailure
