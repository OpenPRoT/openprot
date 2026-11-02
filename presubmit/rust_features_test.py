# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the rust_features presubmit check."""

import unittest

from presubmit.rust_features import find_violations


class FindViolationsTest(unittest.TestCase):
    def test_flags_features_on_rust_rules(self):
        src = (
            'rust_test(\n    name = "a",\n    features = ["test-support"],\n)\n'
            'rust_library(name = "b", features = ["x"])\n'
        )
        self.assertEqual(
            find_violations(src), [(1, "rust_test"), (5, "rust_library")]
        )

    def test_allows_crate_features(self):
        src = 'rust_test(name = "a", crate_features = ["test-support"])\n'
        self.assertEqual(find_violations(src), [])

    def test_ignores_non_rust_rules(self):
        src = 'cc_library(name = "a", features = ["-use_header_modules"])\n'
        self.assertEqual(find_violations(src), [])

    def test_flags_nested_call(self):
        src = 'if True:\n    rust_binary(name = "a", features = [])\n'
        self.assertEqual(find_violations(src), [(2, "rust_binary")])


if __name__ == "__main__":
    unittest.main()
