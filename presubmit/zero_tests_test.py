# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the zero_tests presubmit audit."""

import json
import tempfile
import unittest
from pathlib import Path

from presubmit import zero_tests


def _bep(
    log_path: Path, label="//a:t", kind="rust_test rule", finished=True
) -> list[str]:
    events = [
        json.dumps(
            {
                "id": {"targetConfigured": {"label": label}},
                "configured": {"targetKind": kind},
            }
        ),
        json.dumps(
            {
                "id": {"testResult": {"label": label}},
                "testResult": {
                    "status": "PASSED",
                    "testActionOutput": [
                        {"name": "test.log", "uri": log_path.as_uri()}
                    ],
                },
            }
        ),
    ]
    if finished:
        events.append(json.dumps({"id": {"buildFinished": {}}}))
    return events


class PassedCountTest(unittest.TestCase):
    def test_sums_every_summary_line(self):
        log = (
            "test result: ok. 0 passed; 0 failed\n"
            "test result: ok. 3 passed; 0 failed\n"
        )
        self.assertEqual(zero_tests.passed_count(log), 3)

    def test_no_summary_is_zero(self):
        self.assertEqual(zero_tests.passed_count("garbage\n"), 0)


class FindZeroTestTargetsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.log = Path(tmp.name) / "test.log"

    def _run(self, log_text, allowlist=frozenset(), **kwargs):
        self.log.write_text(log_text)
        return zero_tests.find_zero_test_targets(_bep(self.log, **kwargs), allowlist)

    def test_flags_zero_tests(self):
        result = self._run("test result: ok. 0 passed; 0 failed\n")
        self.assertEqual(result, (1, ["//a:t"]))

    def test_passes_when_tests_ran(self):
        result = self._run("test result: ok. 9 passed; 0 failed\n")
        self.assertEqual(result, (1, []))

    def test_flags_missing_summary(self):
        self.assertEqual(self._run("no summary\n"), (1, ["//a:t"]))

    def test_ignores_other_rule_kinds(self):
        result = self._run("test result: ok. 0 passed; 0 failed\n", kind="py_test rule")
        self.assertEqual(result, (0, []))

    def test_allowlisted_target_is_not_flagged(self):
        result = self._run(
            "test result: ok. 0 passed; 0 failed\n",
            allowlist=frozenset(["//a:t"]),
        )
        self.assertEqual(result, (1, []))

    def test_target_without_result_is_not_audited(self):
        events = _bep(self.log)
        configured_only = [events[0], events[-1]]
        self.assertEqual(zero_tests.find_zero_test_targets(configured_only), (0, []))

    def test_rejects_incomplete_stream(self):
        self.log.write_text("test result: ok. 9 passed; 0 failed\n")
        truncated = _bep(self.log, finished=False)
        with self.assertRaises(ValueError):
            zero_tests.find_zero_test_targets(truncated)

    def test_failed_target_is_not_reported_as_zero_tests(self):
        self.log.write_text("test result: FAILED. 0 passed; 5 failed\n")
        events = _bep(self.log)
        result_event = json.loads(events[1])
        result_event["testResult"]["status"] = "FAILED"
        events[1] = json.dumps(result_event)
        self.assertEqual(zero_tests.find_zero_test_targets(events), (0, []))


class ResolveBepPathTest(unittest.TestCase):
    def test_argument_wins(self):
        env = {"ZERO_TESTS_BEP": "env.json"}
        self.assertEqual(
            zero_tests.resolve_bep_path(Path("arg.json"), env), Path("arg.json")
        )

    def test_env_is_relative_to_invoking_directory(self):
        env = {"ZERO_TESTS_BEP": "out/bep.json", "BUILD_WORKING_DIRECTORY": "/w"}
        self.assertEqual(
            zero_tests.resolve_bep_path(None, env), Path("/w/out/bep.json")
        )

    def test_absolute_env_path_is_kept(self):
        env = {"ZERO_TESTS_BEP": "/abs/bep.json", "BUILD_WORKING_DIRECTORY": "/w"}
        self.assertEqual(zero_tests.resolve_bep_path(None, env), Path("/abs/bep.json"))

    def test_none_without_argument_or_env(self):
        self.assertIsNone(zero_tests.resolve_bep_path(None, {}))


if __name__ == "__main__":
    unittest.main()
