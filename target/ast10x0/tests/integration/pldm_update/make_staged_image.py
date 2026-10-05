# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Writes the image the update agent hands over, for the host to compare
the device's flash against after a run.

This repeats `expected_byte` in the three Rust apps, and the repetition is
the point: the guest cannot read a host file and the host cannot run guest
code, so an independent check needs its own copy of the rule. Change one
and this fails, which is the behaviour wanted. Keep it to one line so
there is nothing else to keep in step.
"""

import sys

SIZE = 1024


def main() -> None:
    with open(sys.argv[1], "wb") as f:
        f.write(bytes(offset % 251 for offset in range(SIZE)))


if __name__ == "__main__":
    main()
