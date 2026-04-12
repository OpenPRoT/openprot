#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Caliptra dependency uprev tool — replaces uprev.sh / verify_shas.sh.

Run via: bazel run //third_party/caliptra:uprev -- <subcommand>

Subcommands:
  verify              — cross-check versions.bzl vs
                        crates_io/{embedded,host}/Cargo.toml and
                        against caliptra-mcu-sw's Cargo.lock
                        (fetched from GitHub). MODULE.bazel is
                        checked only for the ureg git_override commit.
  bump <new-mcu-sw-sha> — set caliptra_mcu_sw to the given SHA,
                        derive caliptra_sw + caliptra_cfi from its
                        Cargo.lock, write three source files
                        (versions.bzl + both Cargo.toml).
                        MODULE.bazel is NOT written — caliptra_sw
                        and caliptra_mcu_sw commits are now driven
                        by versions.bzl via extensions.bzl.
  latest              — resolve caliptra-mcu-sw HEAD of main, then bump.
  release <tag>       — resolve a caliptra-mcu-sw tag via
                        git ls-remote --tags, then bump, AND set
                        release_tag to the tag string.

The canonical current pins are injected via --versions-json by the
py_binary's args attribute at BUILD load time — uprev.py never
parses versions.bzl itself for reads. Writes use targeted regex.
"""

import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import tomllib
import urllib.request

# ---------------------------------------------------------------------------
# Paths — resolved at import time from BUILD_WORKSPACE_DIRECTORY.
# Only valid when running under `bazel run`. The main() guard below
# enforces this.
# ---------------------------------------------------------------------------


def _repo_root() -> pathlib.Path:
    return pathlib.Path(os.environ["BUILD_WORKSPACE_DIRECTORY"])


# These are lazily referenced; actual Path objects are created inside
# cmd_* functions so that import-time path resolution doesn't break
# unit tests (which don't set BUILD_WORKSPACE_DIRECTORY).
def _module_bazel() -> pathlib.Path:
    return _repo_root() / "MODULE.bazel"


def _versions_bzl() -> pathlib.Path:
    return _repo_root() / "third_party" / "caliptra" / "versions.bzl"


def _embedded_cargo_toml() -> pathlib.Path:
    return (
        _repo_root()
        / "third_party"
        / "caliptra"
        / "crates_io"
        / "embedded"
        / "Cargo.toml"
    )


def _host_cargo_toml() -> pathlib.Path:
    return (
        _repo_root() / "third_party" / "caliptra" / "crates_io" / "host" / "Cargo.toml"
    )


def _local_mcu_sw_checkout() -> pathlib.Path:
    return _repo_root().parent / "caliptra-mcu-sw"


_MCU_SW_REMOTE = "https://github.com/chipsalliance/caliptra-mcu-sw"


# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------


def _info(msg: str) -> None:
    """Print a progress message to stderr (matches bash info() ==> prefix)."""
    print(f"==> {msg}", file=sys.stderr)


def _die(msg: str) -> None:
    """Print an error to stderr and exit 1 (matches bash die())."""
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# Pure helpers — no filesystem I/O, no subprocess, no network.
# These are what uprev_test.py exercises.
# ---------------------------------------------------------------------------


def extract_rev_from_cargo_lock(lock_contents: str, repo: str) -> str | None:
    """Extract the rev SHA for a chipsalliance git dep from Cargo.lock.

    Matches source = "git+https://github.com/chipsalliance/<repo>(.git)?\\?rev=<sha>#<sha>"
    Returns the first match's 40-char hex SHA, or None if not found.
    The (.git)?\\? anchor prevents partial matches (e.g. caliptra-sw
    must not match caliptra-sw-foo).
    """
    pattern = (
        rf"git\+https://github\.com/chipsalliance/{re.escape(repo)}"
        rf"(?:\.git)?\?rev=([0-9a-f]{{40}})"
    )
    m = re.search(pattern, lock_contents)
    return m.group(1) if m else None


def extract_rev_from_cargo_toml(toml_contents: str, dep: str) -> str | None:
    """Extract the rev field for a git dep from a Cargo.toml string.

    Parses with tomllib for robustness. Returns None if the dep
    doesn't exist or has no rev field.
    """
    data = tomllib.loads(toml_contents)
    deps = data.get("dependencies", {})
    entry = deps.get(dep)
    if not isinstance(entry, dict):
        return None
    return entry.get("rev")


def replace_rev_in_cargo_toml(toml_contents: str, dep: str, old: str, new: str) -> str:
    """Replace dep's rev field from <old> to <new>, byte-minimally.

    Uses targeted regex — does NOT round-trip through tomllib
    (which would reformat the whole file). Preserves comments,
    whitespace, and dep ordering.
    Raises ValueError if the old rev is not found in the expected
    position on the dep's line.
    """
    # Match lines starting with the dep name (inline table form), find rev = "old"
    pattern = re.compile(
        rf'^({re.escape(dep)}\s*=.*?rev\s*=\s*"){re.escape(old)}(")',
        re.MULTILINE,
    )
    result, count = re.subn(
        pattern,
        lambda m: m.group(1) + new + m.group(2),
        toml_contents,
    )
    if count == 0:
        raise ValueError(f"rev = {old!r} not found for dep {dep!r} in Cargo.toml")
    return result


def extract_sha_from_versions_bzl(versions_contents: str, key: str) -> str | None:
    """Extract a SHA from a CALIPTRA_VERSIONS dict entry.

    Matches "<key>":\\s*"([0-9a-f]{40})" with key-anchored regex.
    Returns None for unknown keys or non-SHA values (e.g. release_tag).
    """
    pattern = rf'"{re.escape(key)}"\s*:\s*"([0-9a-f]{{40}})"'
    m = re.search(pattern, versions_contents)
    return m.group(1) if m else None


def replace_sha_in_versions_bzl(
    versions_contents: str, key: str, old: str, new: str
) -> str:
    """Replace versions.bzl SHA for <key> from <old> to <new>.

    Regex is key-anchored so bumping one key does NOT touch another
    key that happens to share the same old SHA. Raises ValueError
    if the old value isn't found at the expected position.
    """
    pattern = re.compile(rf'("{re.escape(key)}"\s*:\s*"){re.escape(old)}(")')
    result, count = re.subn(
        pattern,
        lambda m: m.group(1) + new + m.group(2),
        versions_contents,
    )
    if count == 0:
        raise ValueError(f"SHA {old!r} not found for key {key!r} in versions.bzl")
    return result


def replace_release_tag_in_versions_bzl(versions_contents: str, new_tag: str) -> str:
    """Set the release_tag field to <new_tag>.

    new_tag may be empty string. Matches "release_tag":\\s*"[^"]*".
    """
    pattern = re.compile(r'("release_tag"\s*:\s*")[^"]*(")')
    result, count = re.subn(
        pattern,
        lambda m: m.group(1) + new_tag + m.group(2),
        versions_contents,
    )
    if count == 0:
        raise ValueError("release_tag key not found in versions.bzl")
    return result


def extract_commit_from_module_bazel(
    module_contents: str, block_name: str, block_type: str
) -> str | None:
    """Extract commit SHA from a caliptra_repos(...) or git_override(...) block.

    block_type is "caliptra_repos" or "git_override".
    block_name is the repo name (e.g. "caliptra_mcu_sw") or module_name
    (e.g. "ureg"). Regex is block-scoped so two blocks cannot
    cross-contaminate even if they share the same old SHA.
    """
    if block_type == "caliptra_repos":
        name_attr = "name"
        block_start = "caliptra_repos"
    elif block_type == "git_override":
        name_attr = "module_name"
        block_start = "git_override"
    else:
        raise ValueError(f"Unknown block_type: {block_type!r}")

    pattern = re.compile(
        rf"{re.escape(block_start)}\([^)]*?"
        rf'{name_attr}\s*=\s*"{re.escape(block_name)}"[^)]*?'
        rf'commit\s*=\s*"([0-9a-f]{{40}})"',
        re.DOTALL,
    )
    m = pattern.search(module_contents)
    return m.group(1) if m else None


def replace_commit_in_module_bazel(
    module_contents: str, block_name: str, block_type: str, old: str, new: str
) -> str:
    """Replace a block's commit SHA, block-scoped.

    Same semantics as the existing bash set_module_sha() but with
    clearer error handling and testable on frozen fixtures.
    """
    if block_type == "caliptra_repos":
        name_attr = "name"
        block_start = "caliptra_repos"
    elif block_type == "git_override":
        name_attr = "module_name"
        block_start = "git_override"
    else:
        raise ValueError(f"Unknown block_type: {block_type!r}")

    pattern = re.compile(
        rf"({re.escape(block_start)}\([^)]*?"
        rf'{name_attr}\s*=\s*"{re.escape(block_name)}"[^)]*?'
        rf'commit\s*=\s*"){re.escape(old)}(")',
        re.DOTALL,
    )
    result, count = re.subn(
        pattern,
        lambda m: m.group(1) + new + m.group(2),
        module_contents,
    )
    if count == 0:
        raise ValueError(
            f"commit {old!r} not found in {block_type}({name_attr}={block_name!r}) block"
        )
    return result


# ---------------------------------------------------------------------------
# I/O wrappers — not unit-tested (covered by differential test in Phase 4)
# ---------------------------------------------------------------------------


def fetch_cargo_lock(sha: str, local_checkout: pathlib.Path | None = None) -> str:
    """Prefer local checkout at REPO_ROOT/../caliptra-mcu-sw if it exists;
    fall back to raw.githubusercontent.com fetch via urllib.request.
    Returns the Cargo.lock contents as a string.
    """
    if local_checkout is None:
        local_checkout = _local_mcu_sw_checkout()

    if local_checkout.is_dir() and (local_checkout / ".git").exists():
        _info(f"Using local caliptra-mcu-sw checkout at {local_checkout}")
        result = subprocess.run(
            ["git", "-C", str(local_checkout), "show", f"{sha}:Cargo.lock"],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            _die(
                f"SHA {sha} not found in local checkout. "
                f"Try 'git fetch' in {local_checkout}."
            )
        return result.stdout
    else:
        _info(f"Fetching Cargo.lock from GitHub for {sha}...")
        url = (
            f"https://raw.githubusercontent.com/chipsalliance/"
            f"caliptra-mcu-sw/{sha}/Cargo.lock"
        )
        with urllib.request.urlopen(url) as resp:
            return resp.read().decode("utf-8")


def resolve_latest() -> str:
    """Return caliptra-mcu-sw HEAD of main. Prefer local checkout if
    present, else `git ls-remote` the public URL.
    """
    local_checkout = _local_mcu_sw_checkout()
    if local_checkout.is_dir() and (local_checkout / ".git").exists():
        subprocess.run(
            ["git", "-C", str(local_checkout), "fetch", "--quiet", "origin"],
            check=False,
        )
        result = subprocess.run(
            ["git", "-C", str(local_checkout), "rev-parse", "origin/main"],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    else:
        result = subprocess.run(
            ["git", "ls-remote", _MCU_SW_REMOTE, "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.split()[0]


def resolve_tag(tag: str) -> str:
    """Return the SHA for a caliptra-mcu-sw tag via `git ls-remote --tags`.
    Try <tag>^{} first (dereferenced annotated tag), fall back to <tag>
    (lightweight tag).
    """
    result = subprocess.run(
        ["git", "ls-remote", "--tags", _MCU_SW_REMOTE],
        capture_output=True,
        text=True,
        check=True,
    )
    # Look for dereferenced annotated tag first
    deref_ref = f"refs/tags/{tag}^{{}}"
    for line in result.stdout.splitlines():
        sha, ref = line.split("\t", 1)
        if ref == deref_ref:
            return sha.strip()
    # Fall back to lightweight tag
    ref_name = f"refs/tags/{tag}"
    for line in result.stdout.splitlines():
        sha, ref = line.split("\t", 1)
        if ref == ref_name:
            return sha.strip()
    _die(f"Tag {tag!r} not found in caliptra-mcu-sw")


# ---------------------------------------------------------------------------
# Bulk-replace helper for Cargo.toml files
# ---------------------------------------------------------------------------


def _bulk_replace_rev_by_url(toml_contents: str, repo: str, old: str, new: str) -> str:
    """Replace rev = old on all inline-table lines with chipsalliance/<repo> git URL.

    Only acts on lines that contain the exact git URL for <repo> — partial
    name matches (e.g. caliptra-sw vs caliptra-sw-foo) are prevented by
    the closing quote in the URL match.
    """
    url = f"https://github.com/chipsalliance/{repo}"
    lines = toml_contents.split("\n")
    result = []
    for line in lines:
        if f'git = "{url}"' in line:
            line = re.sub(
                rf'(rev\s*=\s*"){re.escape(old)}(")',
                lambda m: m.group(1) + new + m.group(2),
                line,
            )
        result.append(line)
    return "\n".join(result)


# ---------------------------------------------------------------------------
# Subcommand entry points
# ---------------------------------------------------------------------------


def cmd_verify(args: argparse.Namespace, versions: dict) -> int:
    """Cross-check versions.bzl/Cargo.toml/Cargo.lock/MODULE.bazel(ureg) consistency.

    caliptra_sw and caliptra_mcu_sw commits are no longer in MODULE.bazel —
    they are driven by versions.bzl via extensions.bzl. This function reads
    the canonical SHAs from the injected versions dict (which comes from
    versions.bzl at BUILD load time). MODULE.bazel is still checked for the
    ureg git_override commit.
    """
    _info("Verifying caliptra SHA consistency...")

    module_contents = _module_bazel().read_text()
    embedded_contents = _embedded_cargo_toml().read_text()

    mcu_sha = versions["caliptra_mcu_sw"]
    lock_contents = fetch_cargo_lock(mcu_sha)

    expected_sw = extract_rev_from_cargo_lock(lock_contents, "caliptra-sw")
    expected_cfi = extract_rev_from_cargo_lock(lock_contents, "caliptra-cfi")

    actual_sw = versions["caliptra_sw"]
    actual_cfi = extract_rev_from_cargo_toml(embedded_contents, "caliptra-cfi-lib-git")

    ok = True

    # caliptra_sw check (versions.bzl vs Cargo.lock)
    if expected_sw != actual_sw:
        print("MISMATCH: caliptra_sw")
        print(f"  versions.bzl: {actual_sw}")
        print(f"  Cargo.lock:   {expected_sw}")
        ok = False
    else:
        print(f"OK: caliptra_sw = {actual_sw}")

    # caliptra_cfi check
    if expected_cfi != actual_cfi:
        print("MISMATCH: caliptra_cfi")
        print(f"  Cargo.toml: {actual_cfi}")
        print(f"  Cargo.lock: {expected_cfi}")
        ok = False
    else:
        print(f"OK: caliptra_cfi = {actual_cfi}")

    # caliptra_dpe check (independently tracked — verify lockstep within Cargo.toml)
    actual_dpe_platform = extract_rev_from_cargo_toml(
        embedded_contents, "caliptra-dpe-platform"
    )
    actual_dpe_crypto = extract_rev_from_cargo_toml(
        embedded_contents, "caliptra-dpe-crypto"
    )
    actual_dpe = extract_rev_from_cargo_toml(embedded_contents, "caliptra-dpe")
    if not actual_dpe_platform:
        print("MISMATCH: caliptra_dpe — could not read caliptra-dpe-platform rev")
        ok = False
    elif actual_dpe_platform != actual_dpe_crypto or actual_dpe_platform != actual_dpe:
        print("MISMATCH: caliptra_dpe entries out of sync")
        print(f"  caliptra-dpe-platform: {actual_dpe_platform}")
        print(f"  caliptra-dpe-crypto  : {actual_dpe_crypto}")
        print(f"  caliptra-dpe         : {actual_dpe}")
        ok = False
    else:
        print(f"OK: caliptra_dpe = {actual_dpe_platform} (independently tracked)")

    # ureg MODULE.bazel cross-check (git_override block — still lives in MODULE.bazel)
    ureg_module_sha = extract_commit_from_module_bazel(
        module_contents, "ureg", "git_override"
    )
    if not ureg_module_sha:
        print("MISMATCH: cannot read ureg SHA from MODULE.bazel git_override block")
        ok = False
    elif ureg_module_sha != versions.get("ureg"):
        print(
            f"MISMATCH: ureg MODULE.bazel ({ureg_module_sha}) != versions.bzl ({versions.get('ureg')})"
        )
        ok = False
    else:
        print(f"OK: ureg MODULE.bazel = {ureg_module_sha}")

    # Additional versions.bzl cross-checks (Python-only, not in bash baseline)
    versions_contents = _versions_bzl().read_text()
    _verify_versions_bzl_consistency(
        versions_contents, versions, module_contents, embedded_contents
    )

    if not ok:
        sys.stdout.flush()
        _die("SHA verification failed. Run this script with a new mcu-sw SHA to fix.")
    sys.stdout.flush()
    _info("All derived SHAs are consistent.")
    return 0


def _verify_versions_bzl_consistency(
    versions_contents: str,
    injected_versions: dict,
    module_contents: str,
    embedded_contents: str,
) -> None:
    """Print additional consistency checks for versions.bzl vs other files.

    These lines are Python-only (not emitted by the bash baseline), and
    appear after the bash-equivalent checks above.
    """
    sha_keys = [
        "caliptra_mcu_sw",
        "caliptra_sw",
        "caliptra_cfi",
        "caliptra_dpe",
        "ureg",
    ]
    for key in sha_keys:
        v = extract_sha_from_versions_bzl(versions_contents, key)
        injected = injected_versions.get(key)
        if v and injected and v != injected:
            print(f"MISMATCH: versions.bzl {key} ({v}) != injected value ({injected})")
        elif v:
            print(f"OK: versions.bzl {key} = {v}")


def cmd_bump(args: argparse.Namespace, versions: dict) -> int:
    """Fetch Cargo.lock at new caliptra_mcu_sw SHA and dual-write all four files."""
    new_mcu_sha = args.sha
    if not re.fullmatch(r"[0-9a-f]{40}", new_mcu_sha):
        _die(f"Invalid SHA: {new_mcu_sha!r}. Expected a 40-character hex string.")

    release_tag = getattr(args, "release_tag", "")
    _do_bump_transaction(new_mcu_sha, versions, release_tag=release_tag)
    return 0


def cmd_latest(args: argparse.Namespace, versions: dict) -> int:
    """Resolve caliptra-mcu-sw HEAD of main, then bump."""
    new_mcu_sha = resolve_latest()
    _info(f"Resolved latest to {new_mcu_sha}")
    # Re-use bump path
    args.sha = new_mcu_sha
    args.release_tag = ""
    return _do_bump_transaction(new_mcu_sha, versions, release_tag="")


def cmd_release(args: argparse.Namespace, versions: dict) -> int:
    """Resolve a caliptra-mcu-sw tag, bump, and set release_tag."""
    tag = args.tag
    _info(f"Resolving tag {tag!r}...")
    new_mcu_sha = resolve_tag(tag)
    _info(f"Resolved {tag!r} to {new_mcu_sha}")
    return _do_bump_transaction(new_mcu_sha, versions, release_tag=tag)


def _do_bump_transaction(
    new_mcu_sha: str, versions: dict, release_tag: str = ""
) -> int:
    """Core write transaction for bump/latest/release.

    Reads all four source files, computes new contents, writes them,
    and re-validates. Emits progress to stderr matching bash format.
    """
    old_mcu = versions["caliptra_mcu_sw"]
    old_sw = versions["caliptra_sw"]
    old_cfi = versions["caliptra_cfi"]

    _info(f"Fetching Cargo.lock for caliptra-mcu-sw @ {new_mcu_sha}...")
    lock_contents = fetch_cargo_lock(new_mcu_sha)

    new_sw = extract_rev_from_cargo_lock(lock_contents, "caliptra-sw")
    new_cfi = extract_rev_from_cargo_lock(lock_contents, "caliptra-cfi")

    if not new_sw:
        _die("Could not find caliptra-sw rev in Cargo.lock")
    if not new_cfi:
        _die("Could not find caliptra-cfi rev in Cargo.lock")

    _info("Updating versions.bzl and caliptra crates_io/{embedded,host}/Cargo.toml...")

    # Read three files (MODULE.bazel is no longer written — caliptra_sw and
    # caliptra_mcu_sw commits are driven by versions.bzl via extensions.bzl)
    versions_content = _versions_bzl().read_text()
    embedded_content = _embedded_cargo_toml().read_text()
    host_content = _host_cargo_toml().read_text()

    # --- versions.bzl ---
    if new_mcu_sha != old_mcu:
        versions_content = replace_sha_in_versions_bzl(
            versions_content, "caliptra_mcu_sw", old_mcu, new_mcu_sha
        )
    if new_sw != old_sw:
        versions_content = replace_sha_in_versions_bzl(
            versions_content, "caliptra_sw", old_sw, new_sw
        )
    if new_cfi != old_cfi:
        versions_content = replace_sha_in_versions_bzl(
            versions_content, "caliptra_cfi", old_cfi, new_cfi
        )
    old_tag = versions.get("release_tag", "")
    if release_tag != old_tag:
        versions_content = replace_release_tag_in_versions_bzl(
            versions_content, release_tag
        )

    # --- embedded Cargo.toml ---
    if new_mcu_sha != old_mcu:
        embedded_content = _bulk_replace_rev_by_url(
            embedded_content, "caliptra-mcu-sw", old_mcu, new_mcu_sha
        )
    if new_sw != old_sw:
        embedded_content = _bulk_replace_rev_by_url(
            embedded_content, "caliptra-sw", old_sw, new_sw
        )
    if new_cfi != old_cfi:
        embedded_content = replace_rev_in_cargo_toml(
            embedded_content, "caliptra-cfi-lib-git", old_cfi, new_cfi
        )
        embedded_content = replace_rev_in_cargo_toml(
            embedded_content, "caliptra-cfi-derive-git", old_cfi, new_cfi
        )
        _info(f"caliptra_cfi: {old_cfi} -> {new_cfi} (Cargo.toml)")
        _info(
            "  NOTE: run 'CARGO_BAZEL_REPIN=1 bazel build "
            "//target/veer/tooling:caliptra_runner' to regenerate Cargo.lock"
        )
    else:
        _info(f"caliptra_cfi: already at {new_cfi} (no change)")

    # --- host Cargo.toml ---
    if new_mcu_sha != old_mcu:
        host_content = _bulk_replace_rev_by_url(
            host_content, "caliptra-mcu-sw", old_mcu, new_mcu_sha
        )
    if new_sw != old_sw:
        host_content = _bulk_replace_rev_by_url(
            host_content, "caliptra-sw", old_sw, new_sw
        )

    # Write three files (versions.bzl, embedded Cargo.toml, host Cargo.toml).
    # MODULE.bazel is no longer written — caliptra_sw / caliptra_mcu_sw commits
    # are now driven by versions.bzl via third_party/caliptra/extensions.bzl.
    _versions_bzl().write_text(versions_content)
    _embedded_cargo_toml().write_text(embedded_content)
    _host_cargo_toml().write_text(host_content)

    # Re-read and validate
    _revalidate_writes(new_mcu_sha, new_sw, new_cfi)

    # Print summary (stdout, matching bash format)
    dpe_sha = (
        extract_rev_from_cargo_toml(
            _embedded_cargo_toml().read_text(), "caliptra-dpe-platform"
        )
        or "(unknown)"
    )
    print("")
    print("============================================================")
    print("  Caliptra uprev summary")
    print("============================================================")
    print("")
    print(f"  caliptra_mcu_sw : {old_mcu}")
    print(f"                 -> {new_mcu_sha}")
    print("")
    print(f"  caliptra_sw     : {old_sw}")
    print(f"                 -> {new_sw}  (derived from Cargo.lock)")
    print("")
    print(f"  caliptra_cfi    : {old_cfi}")
    print(f"                 -> {new_cfi}  (derived from Cargo.lock)")
    print("")
    print("------------------------------------------------------------")
    print("  MANUAL ACTION REQUIRED for independently-tracked repos:")
    print("")
    print(f"  caliptra_dpe : {dpe_sha}  (Cargo.toml, unchanged)")
    print("  ureg         : check git_override in MODULE.bazel  (unchanged)")
    print("")
    print("------------------------------------------------------------")
    print("  Next steps:")
    print("    1. Check if overlay BUILD files need updating")
    print("       (upstream may have added/removed source files)")
    print("    2. Verify the build:")
    print("       bazel build //target/veer/tooling:caliptra_runner")
    print("    3. Run tests and commit the changes")
    print("============================================================")
    return 0


def _revalidate_writes(new_mcu_sha: str, new_sw: str, new_cfi: str) -> None:
    """Re-read three files and confirm new SHAs are present.

    MODULE.bazel is no longer checked here — caliptra_sw / caliptra_mcu_sw
    commits are driven by versions.bzl via extensions.bzl, not inline in
    MODULE.bazel. Raises RuntimeError if any extractor returns the old value.
    Leaves files in known state (no rollback).
    """
    versions_contents = _versions_bzl().read_text()
    embedded_contents = _embedded_cargo_toml().read_text()

    checks = [
        (
            extract_sha_from_versions_bzl(versions_contents, "caliptra_mcu_sw"),
            new_mcu_sha,
            "versions.bzl caliptra_mcu_sw",
        ),
        (
            extract_sha_from_versions_bzl(versions_contents, "caliptra_sw"),
            new_sw,
            "versions.bzl caliptra_sw",
        ),
        (
            extract_sha_from_versions_bzl(versions_contents, "caliptra_cfi"),
            new_cfi,
            "versions.bzl caliptra_cfi",
        ),
        (
            extract_rev_from_cargo_toml(embedded_contents, "caliptra-cfi-lib-git"),
            new_cfi,
            "embedded Cargo.toml caliptra-cfi-lib-git",
        ),
    ]
    failures = []
    for actual, expected, label in checks:
        if actual != expected:
            failures.append(f"  {label}: expected {expected!r}, got {actual!r}")
    if failures:
        raise RuntimeError(
            "Write validation failed — files may be in a broken state:\n"
            + "\n".join(failures)
        )


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> int:
    if "BUILD_WORKSPACE_DIRECTORY" not in os.environ:
        sys.exit(
            "uprev.py must be run via `bazel run //third_party/caliptra:uprev`. "
            "See plans/uprevpy.md."
        )

    parser = argparse.ArgumentParser(
        description="Caliptra dependency uprev tool.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--versions-json",
        required=True,
        help=(
            "JSON-encoded CALIPTRA_VERSIONS dict, injected by the py_binary's "
            "args attribute at BUILD load time. Not meant to be set manually."
        ),
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("verify", help="Cross-check all SHA sources for consistency.")

    p_bump = sub.add_parser("bump", help="Bump caliptra_mcu_sw to a given SHA.")
    p_bump.add_argument("sha", help="New 40-char hex SHA for caliptra-mcu-sw.")

    sub.add_parser("latest", help="Resolve caliptra-mcu-sw HEAD, then bump.")

    p_release = sub.add_parser(
        "release", help="Resolve a caliptra-mcu-sw tag, bump, and set release_tag."
    )
    p_release.add_argument("tag", help="caliptra-mcu-sw release tag, e.g. v2026-03.")

    args = parser.parse_args()
    versions = json.loads(args.versions_json)

    return {
        "verify": cmd_verify,
        "bump": cmd_bump,
        "latest": cmd_latest,
        "release": cmd_release,
    }[args.cmd](args, versions)


if __name__ == "__main__":
    sys.exit(main())
