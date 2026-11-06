# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the module_pins presubmit check."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from pw_presubmit.presubmit import PresubmitFailure
from pw_presubmit.presubmit_context import PresubmitContext

from presubmit.module_pins import find_mismatches, find_pins, module_pins_check

_PW = 'git_override(module_name = "pigweed", commit = "{}", remote = "r")\n'
_CIPD = 'pw_rust.toolchain(cipd_tag = "{}")\n'


class FindPinsTest(unittest.TestCase):
    def test_reads_git_override_commit(self):
        self.assertEqual(find_pins(_PW.format("abc")), {"git_override pigweed": "abc"})

    def test_reads_cipd_tag(self):
        self.assertEqual(find_pins(_CIPD.format("t1")), {"pw_rust cipd_tag": "t1"})

    def test_ignores_other_overrides(self):
        src = 'local_path_override(module_name = "x", path = "p")\n'
        self.assertEqual(find_pins(src), {})

    def test_ignores_git_override_without_commit(self):
        src = 'git_override(module_name = "x", branch = "main", remote = "r")\n'
        self.assertEqual(find_pins(src), {})


class FindMismatchesTest(unittest.TestCase):
    def test_equal_pins_pass(self):
        sources = {
            "MODULE.bazel": _PW.format("abc"),
            "n/MODULE.bazel": _PW.format("abc"),
        }
        self.assertEqual(find_mismatches(sources), [])

    def test_different_commit_is_reported(self):
        sources = {
            "MODULE.bazel": _PW.format("abc"),
            "n/MODULE.bazel": _PW.format("def"),
        }
        (message,) = find_mismatches(sources)
        self.assertIn("git_override pigweed", message)
        self.assertIn("MODULE.bazel=abc", message)
        self.assertIn("n/MODULE.bazel=def", message)

    def test_different_cipd_tag_is_reported(self):
        sources = {"a": _CIPD.format("1"), "b": _CIPD.format("2")}
        self.assertEqual(len(find_mismatches(sources)), 1)

    def test_pin_in_only_one_file_is_not_compared(self):
        sources = {"a": _PW.format("abc"), "b": "module(name = 'b')\n"}
        self.assertEqual(find_mismatches(sources), [])

    def test_unrelated_modules_are_independent(self):
        sources = {
            "a": 'git_override(module_name = "x", commit = "1", remote = "r")\n',
            "b": 'git_override(module_name = "y", commit = "2", remote = "r")\n',
        }
        self.assertEqual(find_mismatches(sources), [])


class ModuleFilesTest(unittest.TestCase):
    def _run(self, files: dict[str, str]) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, source in files.items():
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_text(source)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "add", "-A"], cwd=root, check=True)
            ctx = PresubmitContext.create_for_testing(
                root=root, paths=(root / "MODULE.bazel",)
            )
            # Run the wrapped function directly; the Check object applies the
            # path filter separately (see FilterTest).
            module_pins_check.substeps()[0](ctx)

    def test_fails_when_nested_module_drifts(self):
        with self.assertRaises(PresubmitFailure):
            self._run(
                {
                    "MODULE.bazel": _PW.format("abc"),
                    "target/soc/MODULE.bazel": _PW.format("def"),
                }
            )

    def test_passes_when_in_sync(self):
        self._run(
            {
                "MODULE.bazel": _PW.format("abc"),
                "target/soc/MODULE.bazel": _PW.format("abc"),
            }
        )

    def test_fails_on_unparseable_file(self):
        with self.assertRaises(PresubmitFailure):
            self._run({"MODULE.bazel": "git_override(\n"})


class FilterTest(unittest.TestCase):
    def test_selects_module_files(self):
        matches = module_pins_check.filter.matches
        self.assertTrue(matches("MODULE.bazel"))
        self.assertTrue(matches("target/ast10x0/MODULE.bazel"))
        self.assertFalse(matches("BUILD.bazel"))


if __name__ == "__main__":
    unittest.main()
