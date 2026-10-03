# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Tests for the flash seeding in qemu_runner."""

import tempfile
import unittest

from pathlib import Path

from qemu_runner import _seed_flash_image


class SeedFlashImageTest(unittest.TestCase):
    """A seeded image is always exactly flash_size bytes."""

    def setUp(self) -> None:
        self._dir = tempfile.TemporaryDirectory()
        self.addCleanup(self._dir.cleanup)
        self.image = str(Path(self._dir.name) / "cs1.img")

    def _contents_file(self, data: bytes) -> str:
        path = Path(self._dir.name) / "contents.bin"
        path.write_bytes(data)
        return str(path)

    def test_without_contents_the_image_is_all_fill(self) -> None:
        _seed_flash_image(self.image, 32, fill=0xFF)
        self.assertEqual(Path(self.image).read_bytes(), b"\xff" * 32)

    def test_fill_is_honoured(self) -> None:
        _seed_flash_image(self.image, 8, fill=0x00)
        self.assertEqual(Path(self.image).read_bytes(), b"\x00" * 8)

    def test_contents_land_at_offset_zero_and_the_rest_is_erased(self) -> None:
        _seed_flash_image(self.image, 16, contents=self._contents_file(b"openprot"))
        self.assertEqual(Path(self.image).read_bytes(), b"openprot" + b"\xff" * 8)

    def test_contents_override_fill(self) -> None:
        """Fill describes the erased remainder, not the seeded bytes."""
        _seed_flash_image(self.image, 4, fill=0x00, contents=self._contents_file(b"ab"))
        self.assertEqual(Path(self.image).read_bytes(), b"ab\xff\xff")

    def test_contents_may_fill_the_whole_image(self) -> None:
        _seed_flash_image(self.image, 4, contents=self._contents_file(b"abcd"))
        self.assertEqual(Path(self.image).read_bytes(), b"abcd")

    def test_contents_larger_than_the_flash_is_an_error(self) -> None:
        """Truncating would hand the guest an image that fails for the
        wrong reason."""
        with self.assertRaises(ValueError):
            _seed_flash_image(self.image, 4, contents=self._contents_file(b"abcde"))


if __name__ == "__main__":
    unittest.main()
