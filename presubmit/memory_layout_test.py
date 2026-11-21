# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the memory_layout presubmit check."""

import tempfile
import unittest
from pathlib import Path

from presubmit import memory_layout as layout

_TARGET = layout.Target(
    name="t", root=Path("target/t"), ram_limit=0xA0000, flash_before_ram=True
)
_ARCH = "arch: { type: \"armv7m\", vector_table_start_address: 0x0, vector_table_size_bytes: 1280 },"


def _config(kernel: str, apps: str = "") -> dict:
    return layout.parse_json5(f"{{ {_ARCH} kernel: {{ {kernel} }}, apps: [{apps}] }}")


def _kinds(config: dict, target=_TARGET) -> set[str]:
    return {kind for kind, _ in layout.analyze(config, target)}


_TIGHT = (
    "flash_start_address: 0x500, flash_size_bytes: 262144,"
    "ram_start_address: 0x40500, ram_size_bytes: 391936,"
)


class ParseTest(unittest.TestCase):
    def test_comments_hex_and_trailing_commas(self):
        parsed = layout.parse_json5(
            '{ a: 0x10, // c\n b: "x // not a comment", /* c */ c: [1,], }'
        )
        self.assertEqual(parsed, {"a": 16, "b": "x // not a comment", "c": [1]})


class AnalyzeTest(unittest.TestCase):
    def test_tightly_packed_is_clean(self):
        self.assertEqual(_kinds(_config(_TIGHT)), set())

    def test_gap_and_tail(self):
        kernel = (
            "flash_start_address: 0x500, flash_size_bytes: 4096,"
            "ram_start_address: 0x40000, ram_size_bytes: 4096,"
        )
        self.assertEqual(_kinds(_config(kernel)), {"gap", "tail"})

    def test_overflow_past_cached_window(self):
        kernel = (
            "flash_start_address: 0x500, flash_size_bytes: 262144,"
            "ram_start_address: 0x40500, ram_size_bytes: 524288,"
        )
        self.assertEqual(_kinds(_config(kernel)), {"overflow"})

    def test_flash_overlapping_ram(self):
        kernel = (
            "flash_start_address: 0x500, flash_size_bytes: 262144,"
            "ram_start_address: 0x20000, ram_size_bytes: 0x80000,"
        )
        self.assertIn("overlap", _kinds(_config(kernel)))

    def test_kernel_flash_inside_vector_table(self):
        kernel = (
            "flash_start_address: 0x100, flash_size_bytes: 262144,"
            "ram_start_address: 0x40100, ram_size_bytes: 0x5FF00,"
        )
        self.assertIn("overlap", _kinds(_config(kernel)))

    def test_process_padding_and_pow2(self):
        kernel = (
            "flash_start_address: 0x500, flash_size_bytes: 0x5FB00,"
            "ram_start_address: 0x60000, ram_size_bytes: 0x14000,"
        )
        # The 32 KB process is aligned to 32 KB, so 0x74000 -> 0x78000 is wasted.
        app = (
            "{ flash_size_bytes: 0, processes: [{ name: 'p', ram_size_bytes: 32768 }] }"
        ).replace("'", '"')
        self.assertIn("padding", _kinds(_config(kernel, app)))
        bad = app.replace("32768", "30000")
        self.assertIn("pow2", _kinds(_config(kernel, bad)))


class TargetPolicyTest(unittest.TestCase):
    _KERNEL = (
        "flash_start_address: 0x500, flash_size_bytes: 4096,"
        "ram_start_address: 0x40000, ram_size_bytes: 4096,"
    )

    def test_separate_flash_has_no_gap_or_overlap(self):
        target = layout.Target(name="t", root=Path("t"), ram_limit=0xA0000)
        self.assertEqual(_kinds(_config(self._KERNEL), target), {"tail"})

    def test_no_limits_checks_nothing_by_default(self):
        target = layout.Target(name="t", root=Path("t"))
        self.assertEqual(_kinds(_config(self._KERNEL), target), set())

    def test_flash_limit(self):
        target = layout.Target(name="t", root=Path("t"), flash_limit=0x1000)
        self.assertEqual(_kinds(_config(self._KERNEL), target), {"overflow"})

    def test_non_armv7m_skips_pow2(self):
        config = layout.parse_json5(
            "{ arch: { type: 'riscv' }, kernel: { flash_start_address: 0,"
            "flash_size_bytes: 4096, ram_start_address: 0x40000, ram_size_bytes: 4096 },"
            "apps: [{ flash_size_bytes: 0, processes: [{ name: 'p', ram_size_bytes: 30000 }] }] }".replace("'", '"')
        )
        self.assertNotIn("pow2", _kinds(config, layout.Target(name="t", root=Path("t"))))


def _with(allowlist: dict) -> layout.Target:
    return layout.Target(
        name="t",
        root=Path("target/t"),
        ram_limit=0xA0000,
        flash_before_ram=True,
        allowlist=allowlist,
    )


class CheckTest(unittest.TestCase):
    def _root(self, tmp: str, kernel: str) -> Path:
        root = Path(tmp)
        path = root / "target/t/x/system.json5"
        path.parent.mkdir(parents=True)
        path.write_text(f"{{ {_ARCH} kernel: {{ {kernel} }} }}")
        return root

    def test_clean_layout_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            checked, errors = layout.check(self._root(tmp, _TIGHT), _with({}))
        self.assertEqual((checked, errors), (1, []))

    def test_finding_fails_unless_allowlisted(self):
        kernel = _TIGHT.replace("391936", "262144")
        rel = "target/t/x/system.json5"
        with tempfile.TemporaryDirectory() as tmp:
            root = self._root(tmp, kernel)
            _, errors = layout.check(root, _with({}))
            self.assertEqual(len(errors), 1)
            self.assertIn("tail", errors[0])
            _, errors = layout.check(root, _with({rel: frozenset({"tail"})}))
            self.assertEqual(errors, [])

    def test_stale_allowlist_entries_fail(self):
        rel = "target/t/x/system.json5"
        with tempfile.TemporaryDirectory() as tmp:
            root = self._root(tmp, _TIGHT)
            _, errors = layout.check(root, _with({rel: frozenset({"tail"})}))
            self.assertIn("stale", errors[0])
            _, errors = layout.check(root, _with({"target/t/gone.json5": frozenset()}))
            self.assertIn("not found", errors[0])


class RepoTest(unittest.TestCase):
    def test_checked_in_configs_match_allowlists(self):
        root = Path(__file__).resolve().parent.parent
        for target in layout.TARGETS:
            if not (root / target.root).is_dir():
                self.skipTest("sources are not in the runfiles tree")
            with self.subTest(target=target.name):
                checked, errors = layout.check(root, target)
                self.assertGreater(checked, 0)
                self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
