# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
import unittest

from caliptra_runner import scan_output_for_result


class ScanOutputForResultTest(unittest.TestCase):
    def test_no_sentinel_yet(self):
        self.assertIsNone(scan_output_for_result(["booting...", "init i3c"]))

    def test_pass_sentinel(self):
        self.assertEqual(0, scan_output_for_result(["running tests", "PASS"]))

    def test_fail_sentinel(self):
        self.assertEqual(1, scan_output_for_result(["running tests", "FAIL: 1"]))

    def test_first_sentinel_wins(self):
        self.assertEqual(
            0, scan_output_for_result(["PASS", "unrelated trailing noise"])
        )


if __name__ == "__main__":
    unittest.main()
