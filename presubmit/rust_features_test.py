# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the rust_features presubmit check."""

import tempfile
import unittest
from pathlib import Path

from pw_presubmit.presubmit import PresubmitFailure
from pw_presubmit.presubmit_context import PresubmitContext

from presubmit.rust_features import find_violations, rust_features_check


class FindViolationsTest(unittest.TestCase):
    def test_flags_features_on_rust_rules(self):
        src = (
            'rust_test(\n    name = "a",\n    features = ["test-support"],\n)\n'
            'rust_library(name = "b", features = ["x"])\n'
        )
        self.assertEqual(find_violations(src), [(1, "rust_test"), (5, "rust_library")])

    def test_allows_crate_features(self):
        src = 'rust_test(name = "a", crate_features = ["test-support"])\n'
        self.assertEqual(find_violations(src), [])

    def test_ignores_non_rust_rules(self):
        src = 'cc_library(name = "a", features = ["-use_header_modules"])\n'
        self.assertEqual(find_violations(src), [])

    def test_flags_features_on_rust_app(self):
        src = 'rust_app(name = "x", features = ["test-support"])\n'
        self.assertEqual(find_violations(src), [(1, "rust_app")])

    def test_flags_nested_call(self):
        src = 'if True:\n    rust_binary(name = "a", features = [])\n'
        self.assertEqual(find_violations(src), [(2, "rust_binary")])


class RustFeaturesCheckTest(unittest.TestCase):
    def _run(self, name: str, source: str) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / name
            path.write_text(source)
            ctx = PresubmitContext.create_for_testing(paths=(path,))
            # Run the wrapped function directly; the Check object applies the
            # path filter separately (see FilterTest).
            rust_features_check.substeps()[0](ctx)

    def test_fails_on_violation(self):
        with self.assertRaises(PresubmitFailure):
            self._run("BUILD.bazel", 'rust_test(name = "a", features = ["x"])\n')

    def test_passes_when_clean(self):
        self._run("BUILD.bazel", 'rust_test(name = "a", crate_features = ["x"])\n')

    def test_fails_on_unparseable_file(self):
        with self.assertRaises(PresubmitFailure):
            self._run("BUILD.bazel", "rust_test(\n")


class FilterTest(unittest.TestCase):
    def test_selects_build_files(self):
        matches = rust_features_check.filter.matches
        self.assertTrue(matches("foo/BUILD.bazel"))
        self.assertTrue(matches("foo/defs.bzl"))
        self.assertFalse(matches("foo/lib.rs"))
        self.assertFalse(matches("third_party/foo/BUILD.bazel"))


if __name__ == "__main__":
    unittest.main()
