#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Tool to generate a system.json5 configuration variant with overridden properties."""

import argparse
import re
import sys


def main():
    parser = argparse.ArgumentParser(description="Override system.json5 properties")
    parser.add_argument("--input", required=True, help="Input system.json5 file")
    parser.add_argument("--output", required=True, help="Output system.json5 file")
    parser.add_argument(
        "--flash-start-address",
        help="Override kernel.flash_start_address (e.g. 0xA0016000)",
    )
    args = parser.parse_args()

    with open(args.input, "r", encoding="utf-8") as f:
        content = f.read()

    if args.flash_start_address:
        content, count = re.subn(
            r"(flash_start_address\s*:\s*)(0x[0-9a-fA-F]+|\d+)",
            r"\g<1>" + args.flash_start_address,
            content,
            count=1,
        )
        if count == 0:
            sys.exit(f"Error: flash_start_address not found in {args.input}")

    with open(args.output, "w", encoding="utf-8") as f:
        f.write(content)


if __name__ == "__main__":
    main()
