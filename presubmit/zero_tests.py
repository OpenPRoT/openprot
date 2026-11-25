# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Fail if a `rust_test` target ran zero tests.

`rust_test` reports PASSED when the test binary contains no tests, so a target
whose sources are all stripped by `cfg` looks the same as one that ran.

Reads a Bazel Build Event Protocol JSON file (`--build_event_json_file`) from a
`bazel test` run. BEP names every test's `test.log` whether the result was
cached or not, so this does not depend on the console stream or on where the
`bazel-testlogs` symlink points.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse

# libtest prints one summary per test binary (unit tests, each doctest run).
_RESULT = re.compile(r"^test result: \w+\. (\d+) passed;", re.MULTILINE)

_RUST_TEST_KIND = "rust_test rule"

# workflows.json sets this (via `--run_under=env`) to the BEP file the `ci_tests`
# build writes under its `${BUILD_ROOT}`; it may be relative to the invoking
# directory.
_BEP_ENV = "ZERO_TESTS_BEP"

# Targets that intentionally have no tests yet. Prefer deleting the target.
_ALLOWLIST = frozenset(
    (
        "//services/pldm:pldm_service_test",
        "//services/spdm/transport-mctp:spdm_transport_mctp_test",
    )
)


def passed_count(log: str) -> int:
    """Total tests passed across every libtest summary line in `log`."""
    return sum(int(n) for n in _RESULT.findall(log))


def find_zero_test_targets(bep_lines, allowlist=_ALLOWLIST) -> tuple[int, list[str]]:
    """Returns (rust_test results audited, labels that ran zero tests).

    Raises ValueError if the stream has no `buildFinished` event, i.e. it is
    truncated because the run died part-way.
    """
    rust_tests = set()
    logs: dict[str, str] = {}
    finished = False
    for line in bep_lines:
        event = json.loads(line)
        event_id = event.get("id", {})
        if "buildFinished" in event_id:
            finished = True
        elif "targetConfigured" in event_id:
            if event["configured"].get("targetKind") == _RUST_TEST_KIND:
                rust_tests.add(event_id["targetConfigured"]["label"])
        elif "testResult" in event_id:
            # A failed target already fails the bazel invocation; judging its
            # passed-count would misreport "ran no tests".
            if event["testResult"].get("status") != "PASSED":
                continue
            for output in event["testResult"].get("testActionOutput", []):
                if output["name"] == "test.log":
                    logs[event_id["testResult"]["label"]] = output["uri"]

    if not finished:
        raise ValueError("BEP stream is incomplete: the test run did not finish")

    audited, zero = 0, []
    for label in sorted(rust_tests & logs.keys()):
        audited += 1
        if label in allowlist:
            continue
        path = Path(unquote(urlparse(logs[label]).path))
        if passed_count(path.read_text(errors="replace")) == 0:
            zero.append(label)
    return audited, zero


def resolve_bep_path(arg: Path | None, env=os.environ) -> Path | None:
    """Picks the BEP file: explicit argument, then $ZERO_TESTS_BEP, else None.

    `bazel run` executes from the runfiles tree, so a relative path is taken
    relative to BUILD_WORKING_DIRECTORY, where the user (or pw) invoked it.
    """
    if arg is not None:
        return arg
    if _BEP_ENV in env:
        return Path(env.get("BUILD_WORKING_DIRECTORY", ".")) / env[_BEP_ENV]
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "bep",
        type=Path,
        nargs="?",
        help=f"BEP JSON file from bazel test (default: ${_BEP_ENV})",
    )
    args = parser.parse_args()
    args.bep = resolve_bep_path(args.bep)
    if args.bep is None:
        parser.error(f"no BEP file: pass a path or set {_BEP_ENV}")

    try:
        with args.bep.open() as f:
            audited, zero = find_zero_test_targets(f)
    except (OSError, ValueError) as exc:
        print(f"cannot read {args.bep}: {exc}", file=sys.stderr)
        return 2

    for label in zero:
        print(f"ZERO TESTS: {label} ran no tests", file=sys.stderr)
    print(f"audited {audited} rust_test targets, {len(zero)} ran zero tests")
    return 1 if zero else 0


if __name__ == "__main__":
    sys.exit(main())
