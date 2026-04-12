#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Unit tests for uprev.py pure-helper functions.

Run via:
  python3 -m unittest third_party.caliptra.uprev_test
or from repo root:
  python3 third_party/caliptra/uprev_test.py
"""

import pathlib
import sys
import unittest

# Ensure repo root is on the path so `import third_party.caliptra.uprev` works
# both when run as a module and when invoked directly.
_HERE = pathlib.Path(__file__).parent
_REPO_ROOT = _HERE.parent.parent
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

from third_party.caliptra.uprev import (  # noqa: E402
    extract_commit_from_module_bazel,
    extract_rev_from_cargo_lock,
    extract_rev_from_cargo_toml,
    extract_sha_from_versions_bzl,
    replace_commit_in_module_bazel,
    replace_release_tag_in_versions_bzl,
    replace_rev_in_cargo_toml,
    replace_sha_in_versions_bzl,
)

_TESTDATA = _HERE / "testdata"


def _read(name: str) -> str:
    return (_TESTDATA / name).read_text()


class TestExtractRevFromCargoLock(unittest.TestCase):
    def setUp(self):
        self.lock = _read("sample_cargo.lock")

    def test_extract_rev_from_cargo_lock_caliptra_sw(self):
        sha = extract_rev_from_cargo_lock(self.lock, "caliptra-sw")
        self.assertEqual(sha, "a" * 40)

    def test_extract_rev_from_cargo_lock_nonexistent_repo(self):
        sha = extract_rev_from_cargo_lock(self.lock, "no-such-repo")
        self.assertIsNone(sha)

    def test_extract_rev_from_cargo_lock_partial_match_guard(self):
        # caliptra-sw must NOT match caliptra-sw-foo
        sha = extract_rev_from_cargo_lock(self.lock, "caliptra-sw")
        # caliptra-sw-foo uses "f" * 40; we must get "a" * 40 (caliptra-sw)
        self.assertEqual(sha, "a" * 40)
        self.assertNotEqual(sha, "f" * 40)
        # Directly verify caliptra-sw-foo is not confused with caliptra-sw
        sha_foo = extract_rev_from_cargo_lock(self.lock, "caliptra-sw-foo")
        self.assertEqual(sha_foo, "f" * 40)


class TestExtractRevFromCargoToml(unittest.TestCase):
    def setUp(self):
        self.toml = _read("sample_cargo_embedded.toml")

    def test_extract_rev_from_cargo_toml_default_features_false(self):
        # caliptra-api has default-features = false
        sha = extract_rev_from_cargo_toml(self.toml, "caliptra-api")
        self.assertEqual(sha, "a" * 40)

    def test_extract_rev_from_cargo_toml_multiline_table(self):
        # caliptra-error uses multi-line table form
        sha = extract_rev_from_cargo_toml(self.toml, "caliptra-error")
        self.assertEqual(sha, "a" * 40)

    def test_extract_rev_from_cargo_toml_package_rename(self):
        # caliptra-dpe-platform uses package = "platform"
        sha = extract_rev_from_cargo_toml(self.toml, "caliptra-dpe-platform")
        self.assertEqual(sha, "d" * 40)

    def test_extract_rev_from_cargo_toml_nonexistent(self):
        sha = extract_rev_from_cargo_toml(self.toml, "no-such-dep")
        self.assertIsNone(sha)


class TestReplaceRevInCargoToml(unittest.TestCase):
    def setUp(self):
        self.toml = _read("sample_cargo_embedded.toml")

    def test_replace_rev_in_cargo_toml_preserves_everything_else(self):
        old = "c" * 40
        new = "1" * 40
        result = replace_rev_in_cargo_toml(self.toml, "caliptra-cfi-lib-git", old, new)
        # New SHA is present for the target dep
        self.assertIn(f'rev = "{new}"', result)
        # Old SHA still present for other deps (caliptra-cfi-derive-git not changed)
        self.assertIn(f'rev = "{old}"', result)
        # caliptra-api line is unchanged
        self.assertIn(f'"a" * 40' if False else f'rev = "{"a" * 40}"', result)
        # DPE lines are unchanged
        self.assertIn(f'rev = "{"d" * 40}"', result)

    def test_replace_rev_in_cargo_toml_raises_on_missing_dep(self):
        with self.assertRaises(ValueError):
            replace_rev_in_cargo_toml(self.toml, "no-such-dep", "a" * 40, "b" * 40)

    def test_replace_rev_in_cargo_toml_raises_on_wrong_old_sha(self):
        with self.assertRaises(ValueError):
            replace_rev_in_cargo_toml(
                self.toml, "caliptra-cfi-lib-git", "9" * 40, "1" * 40
            )


class TestExtractShaFromVersionsBzl(unittest.TestCase):
    def setUp(self):
        self.versions = _read("sample_versions.bzl")

    def test_extract_sha_from_versions_bzl_all_keys(self):
        expected = {
            "caliptra_mcu_sw": "a" * 40,
            "caliptra_sw": "b" * 40,
            "caliptra_cfi": "b" * 40,
            "caliptra_dpe": "d" * 40,
            "ureg": "e" * 40,
        }
        for key, sha in expected.items():
            with self.subTest(key=key):
                self.assertEqual(extract_sha_from_versions_bzl(self.versions, key), sha)

    def test_extract_sha_from_versions_bzl_release_tag(self):
        # release_tag is an empty string — not a 40-char hex — so returns None
        result = extract_sha_from_versions_bzl(self.versions, "release_tag")
        self.assertIsNone(result)

    def test_extract_sha_from_versions_bzl_unknown_key(self):
        result = extract_sha_from_versions_bzl(self.versions, "no_such_key")
        self.assertIsNone(result)


class TestReplaceShaInVersionsBzl(unittest.TestCase):
    def setUp(self):
        self.versions = _read("sample_versions.bzl")

    def test_replace_sha_in_versions_bzl_no_cross_contamination(self):
        # caliptra_sw and caliptra_cfi share "b" * 40.
        # Bumping caliptra_sw must NOT change caliptra_cfi.
        old = "b" * 40
        new = "1" * 40
        result = replace_sha_in_versions_bzl(self.versions, "caliptra_sw", old, new)
        # caliptra_sw now has new SHA
        self.assertEqual(extract_sha_from_versions_bzl(result, "caliptra_sw"), new)
        # caliptra_cfi still has old SHA
        self.assertEqual(extract_sha_from_versions_bzl(result, "caliptra_cfi"), old)

    def test_replace_sha_in_versions_bzl_raises_on_missing(self):
        with self.assertRaises(ValueError):
            replace_sha_in_versions_bzl(
                self.versions, "caliptra_mcu_sw", "9" * 40, "1" * 40
            )


class TestReplaceReleaseTagInVersionsBzl(unittest.TestCase):
    def setUp(self):
        self.versions = _read("sample_versions.bzl")

    def test_replace_release_tag_empty_to_populated(self):
        result = replace_release_tag_in_versions_bzl(self.versions, "v2026-03")
        self.assertIn('"release_tag": "v2026-03"', result)

    def test_replace_release_tag_populated_to_empty(self):
        intermediate = replace_release_tag_in_versions_bzl(self.versions, "v2026-03")
        result = replace_release_tag_in_versions_bzl(intermediate, "")
        self.assertIn('"release_tag": ""', result)

    def test_replace_release_tag_other_keys_unchanged(self):
        result = replace_release_tag_in_versions_bzl(self.versions, "v2026-03")
        self.assertEqual(
            extract_sha_from_versions_bzl(result, "caliptra_mcu_sw"), "a" * 40
        )


class TestExtractCommitFromModuleBazel(unittest.TestCase):
    def setUp(self):
        self.module = _read("sample_module.bazel")

    def test_extract_commit_from_module_bazel_caliptra_repos(self):
        sha = extract_commit_from_module_bazel(
            self.module, "caliptra_mcu_sw", "caliptra_repos"
        )
        self.assertEqual(sha, "a" * 40)

    def test_extract_commit_from_module_bazel_caliptra_sw(self):
        sha = extract_commit_from_module_bazel(
            self.module, "caliptra_sw", "caliptra_repos"
        )
        self.assertEqual(sha, "a" * 40)

    def test_extract_commit_from_module_bazel_git_override(self):
        sha = extract_commit_from_module_bazel(self.module, "ureg", "git_override")
        self.assertEqual(sha, "e" * 40)

    def test_extract_commit_from_module_bazel_nonexistent(self):
        sha = extract_commit_from_module_bazel(
            self.module, "no_such_repo", "caliptra_repos"
        )
        self.assertIsNone(sha)


class TestReplaceCommitInModuleBazel(unittest.TestCase):
    def setUp(self):
        self.module = _read("sample_module.bazel")

    def test_replace_commit_in_module_bazel_two_blocks_same_sha(self):
        # caliptra_mcu_sw and caliptra_sw both have "a" * 40.
        # Bumping caliptra_mcu_sw must NOT change caliptra_sw.
        old = "a" * 40
        new = "1" * 40
        result = replace_commit_in_module_bazel(
            self.module, "caliptra_mcu_sw", "caliptra_repos", old, new
        )
        self.assertEqual(
            extract_commit_from_module_bazel(
                result, "caliptra_mcu_sw", "caliptra_repos"
            ),
            new,
        )
        self.assertEqual(
            extract_commit_from_module_bazel(result, "caliptra_sw", "caliptra_repos"),
            old,
        )

    def test_replace_commit_in_module_bazel_git_override(self):
        old = "e" * 40
        new = "2" * 40
        result = replace_commit_in_module_bazel(
            self.module, "ureg", "git_override", old, new
        )
        self.assertEqual(
            extract_commit_from_module_bazel(result, "ureg", "git_override"), new
        )

    def test_replace_commit_in_module_bazel_raises_on_missing(self):
        with self.assertRaises(ValueError):
            replace_commit_in_module_bazel(
                self.module, "no_such_repo", "caliptra_repos", "a" * 40, "b" * 40
            )

    def test_replace_commit_in_module_bazel_raises_on_wrong_old_sha(self):
        with self.assertRaises(ValueError):
            replace_commit_in_module_bazel(
                self.module, "caliptra_mcu_sw", "caliptra_repos", "9" * 40, "1" * 40
            )


if __name__ == "__main__":
    unittest.main()
