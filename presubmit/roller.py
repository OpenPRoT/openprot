#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

import argparse
import re
import subprocess
import logging
import sys


def get_roll_directives(module):
    """Parses MODULE.bazel content to find roll directives.

    Roll directives are comments in the format `# roll:dependency_name`
    following a commit hash in quotes.

    Args:
        module: The content of the MODULE.bazel file as a string.

    Returns:
        A dictionary mapping dependency names to their current commit hashes.
    """
    roll = {}
    for match in re.finditer(r'"([0-9A-Fa-f]+)",?\s+#\s+roll:(\w+)', module):
        roll[match.group(2)] = match.group(1)
    return roll


def git_ls_remote(url):
    """Retrieves the latest commit hash of a remote Git repository.

    Supports specifying a ref in the URL using the format 'url{ref}'.
    If no ref is specified, it defaults to 'HEAD'.

    Args:
        url: The URL of the remote Git repository, optionally followed by
          '{ref}'.

    Returns:
        The latest commit hash for the specified ref as a string.

    Raises:
        subprocess.CalledProcessError: If the git ls-remote command fails.
    """
    ref = "HEAD"
    if url.endswith("}"):
        url, ref = url.split("{")
        ref = ref[:-1]

    result = subprocess.run(
        [
            "git",
            "ls-remote",
            url,
            ref,
        ],
        capture_output=True,
        check=True,
    )
    data = result.stdout.decode("utf-8")
    hash, *_ = data.split()
    return hash


def roll(module, roll, replacements, dry_run=False):
    """Performs the rolling of dependencies in the module content.

    Iterates through the identified roll directives and replaces the old
    version (hash) with the new one from replacements.

    Args:
        module: The content of the MODULE.bazel file as a string.
        roll: A dictionary mapping dependency names to their current hashes.
        replacements: A dictionary mapping dependency names to their new
          hashes. This dictionary is modified in-place.
        dry_run: If True, only log what would be updated without modifying the
          module content.

    Returns:
        A tuple containing:
        - module (str): The updated module content.
        - err (bool): True if there were errors (missing or unused
        replacements), False otherwise.
    """
    err = False
    for name, oldver in roll.items():
        if newver := replacements.pop(name, None):
            if dry_run:
                logging.info("DRY-RUN: updating %s from %s to %s", name, oldver, newver)
            else:
                logging.info("Updating %s from %s to %s", name, oldver, newver)
                module = module.replace(oldver, newver)
        else:
            logging.error("No replacement for %s", name)
            err = True

    for name in replacements:
        logging.error("No roll directive for %s", name)
        err = True
    return (module, err)


def main():
    """Main entry point for the dependency roller script.

    Parses command-line arguments, reads the module file, fetches remote
    hashes, performs the roll, and writes the updated content back to the
    file (unless dry-run is enabled).

    Returns:
        An integer: 0 if successful, 1 if there were errors during the roll.
    """
    parser = argparse.ArgumentParser(description="Visualize ELF memory layout.")
    parser.add_argument(
        "--logging",
        default="info",
        choices=["error", "warning", "info", "debug"],
        help="Logging level",
    )
    parser.add_argument(
        "--dry-run",
        action=argparse.BooleanOptionalAction,
        help="Dry-run: don't update dependencies",
    )
    parser.add_argument(
        "--remote",
        action="append",
        default=[],
        help="Remote repositorie URLs to check as name=url or name=url{ref}",
    )
    parser.add_argument("module", help="Path to MODULE.bazel")

    args = parser.parse_args()
    logging.basicConfig(level=args.logging.upper())

    replacements = {}
    for r in args.remote:
        name, url = r.split("=", 1)
        replacements[name] = git_ls_remote(url)
        logging.info("Remote %s = %s", name, replacements[name])

    with open(args.module, "rt") as f:
        module = f.read()

    rolls = get_roll_directives(module)
    module, err = roll(module, rolls, replacements, args.dry_run)

    if args.dry_run:
        logging.info("DRY-RUN: would rewrite %s", args.module)
    else:
        with open(args.module, "wt") as f:
            f.write(module)

    return int(err)


if __name__ == "__main__":
    sys.exit(main())
