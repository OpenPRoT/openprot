# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Fail if a target's `system.json5` files waste or overrun memory.

pw_kernel's system generator stacks apps after the kernel regions and does not
check how much memory that leaves unused. This models its placement
(`populate_addresses` in `system_generator/lib.rs`) and reports, per config:

  overlap   kernel flash inside the vector table, or (for targets whose flash
            shares memory with RAM) flash running into RAM
  overflow  RAM (or flash) running past the target's `ram_limit` (`flash_limit`)
  pow2      process RAM that is not a power of two (ARMv7-M MPU)
  gap       unused memory between the end of flash and the start of kernel RAM
            (only for targets with `flash_before_ram`)
  tail      unused memory between the end of RAM and `ram_limit`
  padding   alignment padding between a process and the previous allocation

Targets are declared in `TARGETS`. To onboard one, add a `Target` entry (its
config directory, memory limits and an initially empty allowlist) and list its
existing findings in the allowlist. Existing findings are tolerated per config
and per kind; prefer fixing the layout.
"""

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

_FLASH_ALIGNMENT = 4
_MIN_RAM_ALIGNMENT = 32


@dataclass(frozen=True)
class Target:
    """Memory model and policy for one target's `system.json5` files."""

    name: str
    # Repo-relative directory searched for `*system.json5`.
    root: Path
    # Exclusive end of the usable RAM. Findings: `tail`, `overflow`.
    ram_limit: int | None = None
    # Exclusive end of the usable flash. Finding: `overflow`.
    flash_limit: int | None = None
    # True when flash and RAM share one memory and flash must end before RAM
    # starts (non-XIP). Findings: `gap`, `overlap`.
    flash_before_ram: bool = False
    # Repo-relative config path -> finding kinds tolerated for it.
    allowlist: dict[str, frozenset[str]] = field(default_factory=dict)


_AST10X0_ALLOWLIST: dict[str, frozenset[str]] = {
    "target/ast10x0/tests/flash_service/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/interrupts/user/system.json5": frozenset(("tail",)),
    "target/ast10x0/tests/mctp/control/qemu_system.json5": frozenset(("tail",)),
    "target/ast10x0/tests/mctp/control/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/mctp/ipc_client/system.json5": frozenset(("padding",)),
    "target/ast10x0/tests/mctp/multi_protocol/system.json5": frozenset(("overflow",)),
    "target/ast10x0/tests/mctp/server/peer_system.json5": frozenset(
        (
            "gap",
            "overflow",
        )
    ),
    "target/ast10x0/tests/mctp/server/system.json5": frozenset(
        (
            "gap",
            "overflow",
        )
    ),
    "target/ast10x0/tests/orchestrator/runtime/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/orchestrator/timer/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/peripherals/gpio/gpio_irq/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/master_system.json5": frozenset(
        ("tail",)
    ),
    "target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_burst/system.json5": frozenset(
        ("tail",)
    ),
    "target/ast10x0/tests/peripherals/i2c/i2c_slave_rx_ipc/system.json5": frozenset(
        ("tail",)
    ),
    "target/ast10x0/tests/peripherals/sgpiom/sgpiom_irq/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
    "target/ast10x0/tests/spdm/auth/peer_system.json5": frozenset(("overflow",)),
    "target/ast10x0/tests/spdm/auth/system.json5": frozenset(("overflow",)),
    "target/ast10x0/tests/spdm/responder/system.json5": frozenset(("padding",)),
    "target/ast10x0/tests/spdm/vca/peer_system.json5": frozenset(("overflow",)),
    "target/ast10x0/tests/spdm/vca/system.json5": frozenset(("overflow",)),
    "target/ast10x0/tests/util_ipc/async_transaction/system.json5": frozenset(
        ("tail",)
    ),
    "target/ast10x0/tests/util_service/deferred_single_channel/system.json5": frozenset(
        (
            "gap",
            "tail",
        )
    ),
}

TARGETS = (
    # The AST10x0 is non-XIP: kernel and app "flash" and RAM all live in the
    # cached SRAM window, which ends where `RAM_NC` starts (see
    # `target/ast10x0/target.ld.tmpl`).
    Target(
        name="ast10x0",
        root=Path("target/ast10x0"),
        ram_limit=0xA0000,
        flash_before_ram=True,
        allowlist=_AST10X0_ALLOWLIST,
    ),
)

# JSON5 tokens that need care: strings must survive comment stripping.
_TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/', re.DOTALL)


def parse_json5(text: str) -> dict:
    """Parses the JSON5 subset used by `system.json5` (no dependencies)."""
    text = _TOKEN.sub(lambda m: m.group(0) if m.group(0)[0] == '"' else "", text)
    # Quote bare keys, outside of strings.
    parts = re.split(r'("(?:\\.|[^"\\])*")', text)
    for i in range(0, len(parts), 2):
        s = re.sub(r"([A-Za-z_]\w*)\s*:", r'"\1":', parts[i])
        s = re.sub(r"\b0[xX][0-9a-fA-F]+\b", lambda m: str(int(m.group(0), 16)), s)
        parts[i] = s
    text = "".join(parts)
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", text))


def _align(value: int, alignment: int) -> int:
    return (value + alignment - 1) & ~(alignment - 1)


def analyze(config: dict, target: Target) -> list[tuple[str, str]]:
    """Returns (kind, message) findings for one parsed `system.json5`."""
    arch, kernel = config["arch"], config["kernel"]
    armv7m = arch.get("type") == "armv7m"
    findings = []

    if "vector_table_start_address" in arch:
        vector_end = (
            arch["vector_table_start_address"] + arch["vector_table_size_bytes"]
        )
        if kernel["flash_start_address"] < vector_end:
            findings.append(
                (
                    "overlap",
                    f"kernel flash {kernel['flash_start_address']:#x} is inside the vector table",
                )
            )

    ram_start = kernel["ram_start_address"]
    flash_end = _align(
        kernel["flash_start_address"] + kernel["flash_size_bytes"], _FLASH_ALIGNMENT
    )
    ram_end = _align(ram_start + kernel["ram_size_bytes"], _MIN_RAM_ALIGNMENT)
    padding = 0
    for app in config.get("apps", []):
        flash_end = _align(flash_end + app["flash_size_bytes"], _FLASH_ALIGNMENT)
        cursor = ram_end
        for process in app["processes"]:
            size = process["ram_size_bytes"]
            # ARMv7-M MPU regions are power-of-two sized and size-aligned.
            if armv7m:
                if size & (size - 1):
                    findings.append(
                        (
                            "pow2",
                            f"process {process['name']} RAM {size} is not a power of two",
                        )
                    )
                    continue
                start = _align(cursor, size)
            else:
                start = _align(cursor, _MIN_RAM_ALIGNMENT)
            padding += start - cursor
            cursor = start + size
        ram_end = _align(cursor, _MIN_RAM_ALIGNMENT)

    if target.flash_before_ram:
        if flash_end > ram_start:
            findings.append(
                (
                    "overlap",
                    f"flash ends at {flash_end:#x}, past RAM start {ram_start:#x}",
                )
            )
        elif flash_end < ram_start:
            findings.append(
                (
                    "gap",
                    f"{ram_start - flash_end} bytes unused between {flash_end:#x} and {ram_start:#x}",
                )
            )
    if target.flash_limit is not None and flash_end > target.flash_limit:
        findings.append(
            (
                "overflow",
                f"flash ends at {flash_end:#x}, past the limit {target.flash_limit:#x}",
            )
        )
    if target.ram_limit is not None:
        if ram_end > target.ram_limit:
            findings.append(
                (
                    "overflow",
                    f"RAM ends at {ram_end:#x}, past the limit {target.ram_limit:#x}",
                )
            )
        elif ram_end < target.ram_limit:
            findings.append(
                (
                    "tail",
                    f"{target.ram_limit - ram_end} bytes unused between {ram_end:#x} and {target.ram_limit:#x}",
                )
            )
    if padding:
        findings.append(
            ("padding", f"{padding} bytes of process RAM alignment padding")
        )
    return findings


def check(root: Path, target: Target) -> tuple[int, list[str]]:
    """Returns (configs checked, error lines) for one target's system configs."""
    errors, checked = [], 0
    seen = set()
    for path in sorted((root / target.root).rglob("*system.json5")):
        rel = path.relative_to(root).as_posix()
        config = parse_json5(path.read_text())
        if "kernel" not in config:
            continue
        checked += 1
        seen.add(rel)
        allowed = target.allowlist.get(rel, frozenset())
        found = set()
        for kind, message in analyze(config, target):
            found.add(kind)
            if kind not in allowed:
                errors.append(f"{rel}: {kind}: {message}")
        for kind in sorted(allowed - found):
            errors.append(f"{rel}: stale allowlist entry for `{kind}`; remove it")
    for rel in sorted(target.allowlist.keys() - seen):
        errors.append(f"{rel}: allowlisted config not found; remove it")
    return checked, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--target",
        action="append",
        choices=[t.name for t in TARGETS],
        help="target to check (default: all)",
    )
    parser.add_argument(
        "root", type=Path, nargs="?", help="repository root (default: workspace)"
    )
    args = parser.parse_args()
    root = args.root or Path(
        os.environ.get("BUILD_WORKSPACE_DIRECTORY")
        or os.environ.get("BUILD_WORKING_DIRECTORY", ".")
    )

    failed = False
    for target in TARGETS:
        if args.target and target.name not in args.target:
            continue
        checked, errors = check(root, target)
        for line in errors:
            print(line, file=sys.stderr)
        print(
            f"{target.name}: checked {checked} system configs, {len(errors)} findings"
        )
        failed |= bool(errors)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
