#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""SGPIOM JSON tooling for validate/merge/generate/check/report workflows."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

JsonMap = Dict[str, Any]


class ValidationError(Exception):
    """Raised when manifest validation fails."""


@dataclass
class ValidationResult:
    errors: List[str]

    @property
    def ok(self) -> bool:
        return not self.errors


def load_json(path: Path) -> JsonMap:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ValidationError(f"input not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValidationError(f"invalid JSON in {path}: {exc}") from exc

    if not isinstance(data, dict):
        raise ValidationError(f"top-level JSON must be an object: {path}")
    return data


def deep_merge(a: JsonMap, b: JsonMap) -> JsonMap:
    """Merge b into a recursively for plain objects (non-list values)."""
    out = copy.deepcopy(a)
    for key, b_val in b.items():
        a_val = out.get(key)
        if isinstance(a_val, dict) and isinstance(b_val, dict):
            out[key] = deep_merge(a_val, b_val)
        else:
            out[key] = copy.deepcopy(b_val)
    return out


def merge_named_list(
    base: List[JsonMap], override: List[JsonMap], key_name: str
) -> List[JsonMap]:
    """Deterministically merge list entries by key, preserving base order first."""
    result: List[JsonMap] = []
    index: Dict[str, int] = {}

    for item in base:
        name = item.get(key_name)
        if isinstance(name, str) and name not in index:
            index[name] = len(result)
            result.append(copy.deepcopy(item))

    for item in override:
        name = item.get(key_name)
        if not isinstance(name, str):
            # Validation handles this; keep deterministic behavior by append.
            result.append(copy.deepcopy(item))
            continue
        if name in index:
            old_item = result[index[name]]
            if isinstance(old_item, dict) and isinstance(item, dict):
                result[index[name]] = deep_merge(old_item, item)
            else:
                result[index[name]] = copy.deepcopy(item)
        else:
            index[name] = len(result)
            result.append(copy.deepcopy(item))

    return result


def merge_manifests(inputs: Iterable[JsonMap]) -> JsonMap:
    merged: JsonMap = {}
    for incoming in inputs:
        previous = copy.deepcopy(merged)
        merged = deep_merge(merged, incoming)

        prev_banks = previous.get("banks")
        in_banks = incoming.get("banks")
        if isinstance(prev_banks, list) and isinstance(in_banks, list):
            merged["banks"] = merge_named_list(prev_banks, in_banks, "name")

        prev_signals = previous.get("signals")
        in_signals = incoming.get("signals")
        if isinstance(prev_signals, list) and isinstance(in_signals, list):
            merged["signals"] = merge_named_list(
                prev_signals, in_signals, "logical_name"
            )

    return merged


def _is_nonneg_int(value: Any) -> bool:
    # `bool` is a subclass of `int`; exclude it so `true`/`false` are rejected.
    return type(value) is int and value >= 0


def _is_bool(value: Any) -> bool:
    return isinstance(value, bool)


def _is_hex_string(value: Any) -> bool:
    # Lowercase `0x` prefix only, to match the schema pattern and keep a single
    # canonical form across schema-only and tool validation.
    if not isinstance(value, str) or not value.startswith("0x"):
        return False
    try:
        int(value, 16)
        return True
    except ValueError:
        return False


# Largest value representable by the generated `u32` fields.
_U32_MAX = 0xFFFF_FFFF


def _hex_or_int_value(value: Any) -> int:
    """Numeric value of a hex-string or integer (callers pre-validate the type)."""
    return int(value, 16) if isinstance(value, str) else int(value)


def _rust_str_literal(value: str) -> str:
    """Emit a `value` as an escaped Rust string literal (incl. surrounding quotes)."""
    out = []
    for ch in value:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def validate_manifest(manifest: JsonMap) -> ValidationResult:
    errors: List[str] = []

    board = manifest.get("board")
    if not isinstance(board, str) or not board:
        errors.append("board: required non-empty string")

    controller = manifest.get("controller")
    if not isinstance(controller, dict):
        errors.append("controller: required object")
        return ValidationResult(errors)

    required_controller = [
        "name",
        "base_addr",
        "bus_frequency_hz",
        "ngpios",
        "enabled",
    ]
    for key in required_controller:
        if key not in controller:
            errors.append(f"controller.{key}: required")

    if "name" in controller and not isinstance(controller["name"], str):
        errors.append("controller.name: must be string")

    if "base_addr" in controller:
        base_addr = controller["base_addr"]
        if not (_is_hex_string(base_addr) or _is_nonneg_int(base_addr)):
            errors.append("controller.base_addr: must be hex string or integer")
        elif _hex_or_int_value(base_addr) > _U32_MAX:
            errors.append("controller.base_addr: exceeds u32 range (> 0xffffffff)")

    if "bus_frequency_hz" in controller:
        bus_freq = controller["bus_frequency_hz"]
        if not _is_nonneg_int(bus_freq):
            errors.append("controller.bus_frequency_hz: must be non-negative integer")
        elif bus_freq > _U32_MAX:
            errors.append(
                "controller.bus_frequency_hz: exceeds u32 range (> 0xffffffff)"
            )

    if "ngpios" in controller and (
        not _is_nonneg_int(controller["ngpios"])
        or controller["ngpios"] == 0
        or controller["ngpios"] > 128
    ):
        # Four 32-pin banks (A-P) => 128 max (matches Zephyr AST10x0 DTS).
        errors.append("controller.ngpios: must be a positive integer <= 128")

    if "enabled" in controller and not _is_bool(controller["enabled"]):
        errors.append("controller.enabled: must be bool")

    banks = manifest.get("banks")
    if not isinstance(banks, list) or not banks:
        errors.append("banks: required non-empty list")
        return ValidationResult(errors)

    bank_by_name: Dict[str, JsonMap] = {}
    seen_bank_names: set[str] = set()
    seen_pin_offsets: set[int] = set()
    total_bank_gpios = 0
    controller_ngpios = controller.get("ngpios")

    for i, bank in enumerate(banks):
        prefix = f"banks[{i}]"
        if not isinstance(bank, dict):
            errors.append(f"{prefix}: must be object")
            continue

        for field in ["name", "pin_offset", "ngpios", "reserved_pins"]:
            if field not in bank:
                errors.append(f"{prefix}.{field}: required")

        name = bank.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"{prefix}.name: must be non-empty string")
        else:
            if name in seen_bank_names:
                errors.append(f"{prefix}.name: duplicate bank name '{name}'")
            seen_bank_names.add(name)
            bank_by_name[name] = bank

        pin_offset = bank.get("pin_offset")
        if not _is_nonneg_int(pin_offset):
            errors.append(f"{prefix}.pin_offset: must be non-negative integer")
        elif pin_offset not in (0, 32, 64, 96):
            # Runtime derives the bank as pin_offset >> 5; only the four 32-pin
            # bank boundaries are valid (Zephyr offsets 0/32/64/96).
            errors.append(
                f"{prefix}.pin_offset: must be one of 0, 32, 64, 96 (got {pin_offset})"
            )
        else:
            if pin_offset in seen_pin_offsets:
                errors.append(f"{prefix}.pin_offset: duplicate pin_offset {pin_offset}")
            seen_pin_offsets.add(pin_offset)

        ngpios = bank.get("ngpios")
        if not _is_nonneg_int(ngpios) or ngpios == 0 or ngpios > 32:
            errors.append(f"{prefix}.ngpios: must be integer in range 1..32")
        else:
            total_bank_gpios += ngpios
            if (
                _is_nonneg_int(pin_offset)
                and isinstance(controller_ngpios, int)
                and pin_offset + ngpios > controller_ngpios
            ):
                errors.append(
                    f"{prefix}: pin_offset + ngpios ({pin_offset} + {ngpios}) "
                    f"exceeds controller.ngpios ({controller_ngpios})"
                )

        reserved = bank.get("reserved_pins")
        if not isinstance(reserved, list):
            errors.append(f"{prefix}.reserved_pins: must be list")
        else:
            for j, pin in enumerate(reserved):
                if not _is_nonneg_int(pin):
                    errors.append(
                        f"{prefix}.reserved_pins[{j}]: must be non-negative integer"
                    )
                elif isinstance(ngpios, int) and pin >= ngpios:
                    errors.append(
                        f"{prefix}.reserved_pins[{j}]: pin {pin} out of range for ngpios {ngpios}"
                    )

    if isinstance(controller_ngpios, int) and total_bank_gpios > controller_ngpios:
        errors.append(
            "controller.ngpios: smaller than sum of bank ngpios "
            f"({controller_ngpios} < {total_bank_gpios})"
        )

    signals = manifest.get("signals")
    if not isinstance(signals, list):
        errors.append("signals: required list")
        return ValidationResult(errors)

    seen_signal_names: set[str] = set()
    seen_pin_ownership: set[Tuple[str, int]] = set()

    for i, signal in enumerate(signals):
        prefix = f"signals[{i}]"
        if not isinstance(signal, dict):
            errors.append(f"{prefix}: must be object")
            continue

        for field in [
            "logical_name",
            "bank",
            "pin",
            "direction",
            "active_level",
            "safe_default",
        ]:
            if field not in signal:
                errors.append(f"{prefix}.{field}: required")

        logical_name = signal.get("logical_name")
        if not isinstance(logical_name, str) or not logical_name:
            errors.append(f"{prefix}.logical_name: must be non-empty string")
        elif logical_name in seen_signal_names:
            errors.append(
                f"{prefix}.logical_name: duplicate logical name '{logical_name}'"
            )
        else:
            seen_signal_names.add(logical_name)

        bank_name = signal.get("bank")
        if not isinstance(bank_name, str) or not bank_name:
            errors.append(f"{prefix}.bank: must be non-empty string")
            continue

        pin = signal.get("pin")
        if not _is_nonneg_int(pin):
            errors.append(f"{prefix}.pin: must be non-negative integer")
            continue

        direction = signal.get("direction")
        if direction not in ("in", "out"):
            errors.append(f"{prefix}.direction: must be 'in' or 'out'")

        active_level = signal.get("active_level")
        if active_level not in ("high", "low"):
            errors.append(f"{prefix}.active_level: must be 'high' or 'low'")

        safe_default = signal.get("safe_default")
        # `bool` is an `int` subclass and true==1/false==0, so exclude it explicitly.
        if not (
            safe_default is None
            or (type(safe_default) is int and safe_default in (0, 1))
        ):
            errors.append(f"{prefix}.safe_default: must be null, 0, or 1")

        bank = bank_by_name.get(bank_name)
        if bank is None:
            errors.append(f"{prefix}.bank: unknown bank '{bank_name}'")
            continue

        bank_ngpios = bank.get("ngpios")
        if isinstance(bank_ngpios, int) and pin >= bank_ngpios:
            errors.append(
                f"{prefix}.pin: pin {pin} out of range for bank '{bank_name}' ngpios {bank_ngpios}"
            )

        reserved_pins = bank.get("reserved_pins", [])
        if isinstance(reserved_pins, list) and pin in reserved_pins:
            errors.append(f"{prefix}.pin: pin {pin} is reserved in bank '{bank_name}'")

        owner_key = (bank_name, pin)
        if owner_key in seen_pin_ownership:
            errors.append(
                f"{prefix}: overlaps existing signal ownership at {bank_name}[{pin}]"
            )
        else:
            seen_pin_ownership.add(owner_key)

        if direction == "in" and safe_default is not None:
            errors.append(f"{prefix}.safe_default: input signal must use null")

    return ValidationResult(errors)


def canonical_json(data: JsonMap) -> str:
    return json.dumps(data, sort_keys=True, separators=(",", ":"))


def rust_ident(name: str) -> str:
    out = []
    for ch in name:
        if ch.isalnum():
            out.append(ch.upper())
        else:
            out.append("_")
    ident = "".join(out).strip("_")
    if not ident:
        ident = "UNNAMED"
    if ident[0].isdigit():
        ident = f"_{ident}"
    return ident


def render_rust(manifest: JsonMap) -> str:
    manifest_hash = hashlib.sha256(canonical_json(manifest).encode("utf-8")).hexdigest()

    controller = manifest["controller"]
    banks = sorted(manifest["banks"], key=lambda b: (b["pin_offset"], b["name"]))
    signals = sorted(manifest["signals"], key=lambda s: s["logical_name"])

    lines: List[str] = []
    lines.append("// Licensed under the Apache-2.0 license")
    lines.append("// SPDX-License-Identifier: Apache-2.0")
    lines.append("")
    lines.append("// @generated by tools/sgpiom/sgpio_json_tool.py; DO NOT EDIT.")
    lines.append(f'pub const SGPIOM_MANIFEST_HASH: &str = "{manifest_hash}";')
    lines.append("")
    lines.append("#[derive(Debug, Copy, Clone, Eq, PartialEq)]")
    lines.append("pub enum Direction { In, Out }")
    lines.append("")
    lines.append("#[derive(Debug, Copy, Clone, Eq, PartialEq)]")
    lines.append("pub enum ActiveLevel { High, Low }")
    lines.append("")
    lines.append("#[derive(Debug, Copy, Clone, Eq, PartialEq)]")
    lines.append("pub struct SgpiomControllerConfig {")
    lines.append("    pub name: &'static str,")
    lines.append("    pub base_addr: u32,")
    lines.append("    pub bus_frequency_hz: u32,")
    lines.append("    pub ngpios: u16,")
    lines.append("    pub enabled: bool,")
    lines.append("}")
    lines.append("")
    lines.append("#[derive(Debug, Copy, Clone, Eq, PartialEq)]")
    lines.append("pub struct SgpiomBankConfig {")
    lines.append("    pub name: &'static str,")
    lines.append("    pub pin_offset: u8,")
    lines.append("    pub ngpios: u8,")
    lines.append("    pub reserved_pins: &'static [u8],")
    lines.append("}")
    lines.append("")
    lines.append("#[derive(Debug, Copy, Clone, Eq, PartialEq)]")
    lines.append("pub struct SgpiomSignalConfig {")
    lines.append("    pub logical_name: &'static str,")
    lines.append("    pub bank: &'static str,")
    lines.append("    pub pin: u8,")
    lines.append("    pub direction: Direction,")
    lines.append("    pub active_level: ActiveLevel,")
    lines.append("    pub safe_default: Option<bool>,")
    lines.append("}")
    lines.append("")

    # Canonicalize to a lowercase `0x` Rust literal regardless of input form.
    base_addr_literal = hex(_hex_or_int_value(controller["base_addr"]))

    lines.append(
        "pub const SGPIOM_CONTROLLER: SgpiomControllerConfig = SgpiomControllerConfig {"
    )
    lines.append(f"    name: {_rust_str_literal(controller['name'])},")
    lines.append(f"    base_addr: {base_addr_literal},")
    lines.append(f"    bus_frequency_hz: {controller['bus_frequency_hz']},")
    lines.append(f"    ngpios: {controller['ngpios']},")
    lines.append(f"    enabled: {'true' if controller['enabled'] else 'false'},")
    lines.append("};")
    lines.append("")

    # Index the const name (not rust_ident(name)) so distinct bank names that
    # sanitize to the same identifier (e.g. "a-b" / "a_b") cannot collide into
    # one RESERVED_PINS_* symbol.
    for i, bank in enumerate(banks):
        reserved = ", ".join(str(pin) for pin in sorted(bank.get("reserved_pins", [])))
        lines.append(
            f"const RESERVED_PINS_{i}: [u8; {len(bank.get('reserved_pins', []))}] = [{reserved}];"
        )
    lines.append("")

    lines.append(f"pub const SGPIOM_BANKS: [SgpiomBankConfig; {len(banks)}] = [")
    for i, bank in enumerate(banks):
        lines.append("    SgpiomBankConfig {")
        lines.append(f"        name: {_rust_str_literal(bank['name'])},")
        lines.append(f"        pin_offset: {bank['pin_offset']},")
        lines.append(f"        ngpios: {bank['ngpios']},")
        lines.append(f"        reserved_pins: &RESERVED_PINS_{i},")
        lines.append("    },")
    lines.append("];\n")

    lines.append(f"pub const SGPIOM_SIGNALS: [SgpiomSignalConfig; {len(signals)}] = [")
    for signal in signals:
        dir_value = "Direction::In" if signal["direction"] == "in" else "Direction::Out"
        lvl_value = (
            "ActiveLevel::High"
            if signal["active_level"] == "high"
            else "ActiveLevel::Low"
        )
        safe_default = signal["safe_default"]
        if safe_default is None:
            safe_default_text = "None"
        else:
            safe_default_text = "Some(true)" if safe_default == 1 else "Some(false)"

        lines.append("    SgpiomSignalConfig {")
        lines.append(
            f"        logical_name: {_rust_str_literal(signal['logical_name'])},"
        )
        lines.append(f"        bank: {_rust_str_literal(signal['bank'])},")
        lines.append(f"        pin: {signal['pin']},")
        lines.append(f"        direction: {dir_value},")
        lines.append(f"        active_level: {lvl_value},")
        lines.append(f"        safe_default: {safe_default_text},")
        lines.append("    },")
    lines.append("];\n")

    return "\n".join(lines)


def load_and_merge(input_paths: List[Path]) -> JsonMap:
    if not input_paths:
        raise ValidationError("at least one --input is required")
    manifests = [load_json(path) for path in input_paths]
    return merge_manifests(manifests)


def cmd_validate(args: argparse.Namespace) -> int:
    merged = load_and_merge(args.input)
    result = validate_manifest(merged)
    if not result.ok:
        for err in result.errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    if args.merged_out:
        args.merged_out.write_text(
            json.dumps(merged, indent=2) + "\n", encoding="utf-8"
        )

    print("validate: OK")
    return 0


def cmd_merge(args: argparse.Namespace) -> int:
    merged = load_and_merge(args.input)
    result = validate_manifest(merged)
    if not result.ok:
        for err in result.errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    args.output.write_text(json.dumps(merged, indent=2) + "\n", encoding="utf-8")
    print(f"merge: wrote {args.output}")
    return 0


def cmd_generate(args: argparse.Namespace) -> int:
    merged = load_and_merge(args.input)
    result = validate_manifest(merged)
    if not result.ok:
        for err in result.errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    rust_text = render_rust(merged)
    args.output.write_text(rust_text, encoding="utf-8")
    print(f"generate: wrote {args.output}")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    merged = load_and_merge(args.input)
    result = validate_manifest(merged)
    if not result.ok:
        for err in result.errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    expected = render_rust(merged)
    try:
        actual = args.output.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"ERROR: output file missing: {args.output}", file=sys.stderr)
        return 1

    if actual != expected:
        print("ERROR: generated output is out of date", file=sys.stderr)
        return 1

    print("check: OK")
    return 0


def cmd_report(args: argparse.Namespace) -> int:
    merged = load_and_merge(args.input)
    result = validate_manifest(merged)
    if not result.ok:
        for err in result.errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    controller = merged["controller"]
    banks = sorted(merged["banks"], key=lambda b: (b["pin_offset"], b["name"]))
    signals = merged["signals"]

    bank_counts: Dict[str, int] = {bank["name"]: 0 for bank in banks}
    out_count = 0
    in_count = 0
    for signal in signals:
        bank_counts[signal["bank"]] = bank_counts.get(signal["bank"], 0) + 1
        if signal["direction"] == "out":
            out_count += 1
        else:
            in_count += 1

    print(f"board: {merged['board']}")
    print(
        "controller: "
        f"{controller['name']} base={controller['base_addr']} "
        f"freq={controller['bus_frequency_hz']}Hz ngpios={controller['ngpios']} "
        f"enabled={controller['enabled']}"
    )
    print(
        f"banks: {len(banks)}  signals: {len(signals)} (in={in_count}, out={out_count})"
    )
    print("bank usage:")
    for bank in banks:
        name = bank["name"]
        print(
            f"  - {name}: offset={bank['pin_offset']} ngpios={bank['ngpios']} "
            f"reserved={len(bank['reserved_pins'])} signals={bank_counts.get(name, 0)}"
        )

    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="SGPIOM JSON tooling")
    subparsers = parser.add_subparsers(dest="command", required=True)

    def add_inputs(sub: argparse.ArgumentParser) -> None:
        sub.add_argument(
            "--input",
            type=Path,
            action="append",
            required=True,
            help="Input manifest path (repeatable, merge order is declaration order)",
        )

    validate_parser = subparsers.add_parser("validate", help="Validate merged manifest")
    add_inputs(validate_parser)
    validate_parser.add_argument("--merged-out", type=Path)
    validate_parser.set_defaults(func=cmd_validate)

    merge_parser = subparsers.add_parser("merge", help="Merge manifests and emit JSON")
    add_inputs(merge_parser)
    merge_parser.add_argument("--output", type=Path, required=True)
    merge_parser.set_defaults(func=cmd_merge)

    generate_parser = subparsers.add_parser(
        "generate", help="Generate Rust static configuration module"
    )
    add_inputs(generate_parser)
    generate_parser.add_argument("--output", type=Path, required=True)
    generate_parser.set_defaults(func=cmd_generate)

    check_parser = subparsers.add_parser(
        "check", help="Verify generated Rust output is up to date"
    )
    add_inputs(check_parser)
    check_parser.add_argument("--output", type=Path, required=True)
    check_parser.set_defaults(func=cmd_check)

    report_parser = subparsers.add_parser(
        "report", help="Print manifest summary report"
    )
    add_inputs(report_parser)
    report_parser.set_defaults(func=cmd_report)

    return parser


def main(argv: List[str]) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except ValidationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
