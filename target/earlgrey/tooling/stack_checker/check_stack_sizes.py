#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Tool to statically check Rust function stack sizes and cumulative call-stack depth against process allocations."""

import argparse
from collections import defaultdict
import json
import os
import re
import shutil
import subprocess
import sys


def clean_json5(content: str) -> str:
    """Remove comments, unquoted keys, hex literals, and trailing commas to parse JSON5 as standard JSON."""
    # Remove single-line comments // ...
    content = re.sub(r"//.*", "", content)
    # Remove multi-line comments /* ... */
    content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
    # Quote unquoted keys (e.g. key: -> "key":)
    content = re.sub(r"([{\s,])([a-zA-Z_][a-zA-Z0-9_]*)\s*:", r'\1"\2":', content)
    # Convert hex numbers (e.g. : 0x10000 -> : 65536)
    content = re.sub(
        r":\s*(0x[0-9a-fA-F]+)", lambda m: f": {int(m.group(1), 16)}", content
    )
    # Remove trailing commas before } or ]
    content = re.sub(r",\s*([\}\]])", r"\1", content)
    return content


def extract_process_budgets(config_path: str, app_name: str, headroom: int):
    """Extract process allocations from system.json5."""
    with open(config_path, "r", encoding="utf-8") as f:
        raw_text = f.read()

    cleaned = clean_json5(raw_text)
    try:
        config = json.loads(cleaned)
    except json.JSONDecodeError as e:
        print(f"Error parsing {config_path}: {e}", file=sys.stderr)
        sys.exit(1)

    target_app = None
    for app in config.get("apps", []):
        if app.get("name") == app_name:
            target_app = app
            break

    # If not found in apps, check if processes are at the root level
    processes = []
    if target_app:
        processes = target_app.get("processes", [])
    elif "processes" in config:
        processes = config["processes"]
    else:
        print(f"Error: App '{app_name}' not found in {config_path}.", file=sys.stderr)
        sys.exit(1)

    budgets = {}
    for proc in processes:
        name = proc.get("name")
        ram_bytes = (
            proc.get("ram_size_bytes")
            or proc.get("stack_bytes")
            or proc.get("stack_size")
        )
        if not name or ram_bytes is None:
            continue
        budget = max(0, ram_bytes - headroom)
        budgets[name] = {
            "ram_size_bytes": ram_bytes,
            "headroom": headroom,
            "budget": budget,
        }

    if not budgets:
        print(
            f"Error: No processes with memory allocations found for app '{app_name}'.",
            file=sys.stderr,
        )
        sys.exit(1)

    return budgets


def find_binary_tool(tool_name: str, override_path: str = None) -> str:
    """Find a tool executable (e.g. llvm-readobj or llvm-objdump)."""
    if (
        override_path
        and os.path.isfile(override_path)
        and os.access(override_path, os.X_OK)
    ):
        return override_path

    which_path = shutil.which(tool_name)
    if which_path:
        return which_path

    # Common fallback locations
    for candidate in [
        f"/usr/bin/{tool_name}",
        f"/usr/local/bin/{tool_name}",
    ]:
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate

    print(f"Error: {tool_name} executable not found.", file=sys.stderr)
    sys.exit(1)


def parse_stack_sizes(llvm_readobj_bin: str, elf_path: str):
    """Run llvm-readobj --stack-sizes and parse function stack sizes."""
    if not os.path.isfile(elf_path):
        print(f"Error: ELF file '{elf_path}' does not exist.", file=sys.stderr)
        sys.exit(1)

    cmd = [llvm_readobj_bin, "--stack-sizes", elf_path]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Error running {' '.join(cmd)}: {e.stderr}", file=sys.stderr)
        sys.exit(1)

    stack_sizes = {}
    in_entry = False
    curr_funcs = []
    curr_size = None

    for line in proc.stdout.splitlines():
        line = line.strip()
        if line.startswith("Entry {"):
            in_entry = True
            curr_funcs = []
            curr_size = None
        elif in_entry:
            if line.startswith("Functions:"):
                # Extract function names: Functions: [func1, func2]
                match = re.search(r"Functions:\s*\[(.*?)\]", line)
                if match:
                    curr_funcs = [
                        fn.strip() for fn in match.group(1).split(",") if fn.strip()
                    ]
            elif line.startswith("Size:"):
                val_str = line.split(":", 1)[1].strip()
                try:
                    curr_size = int(val_str, 0)
                except ValueError:
                    curr_size = None
            elif line == "}":
                if curr_size is not None and curr_funcs:
                    for fn in curr_funcs:
                        stack_sizes[fn] = curr_size
                in_entry = False

    return stack_sizes


def build_call_graph(llvm_objdump_bin: str, elf_path: str):
    """Disassemble the binary using llvm-objdump and extract the function call graph and indirect jumps."""
    cmd = [llvm_objdump_bin, "-d", elf_path]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Error running {' '.join(cmd)}: {e.stderr}", file=sys.stderr)
        sys.exit(1)

    call_graph = defaultdict(set)
    indirect_jumps = defaultdict(list)
    curr_fn = None

    for line in proc.stdout.splitlines():
        line = line.strip()
        # Match function symbol header: 00010000 <func_name>:
        m_head = re.match(r"^[0-9a-fA-F]+\s+<(.*?)>:", line)
        if m_head:
            curr_fn = m_head.group(1)
            continue
        if curr_fn:
            # Match call instructions with target symbol: <target_name> or <target_name+0x...>
            m_call = re.findall(r"<([a-zA-Z0-9_\$.\<\>@]+)(?:\+[0-9a-fA-Fx]+)?>", line)
            for target in m_call:
                if target != curr_fn:
                    call_graph[curr_fn].add(target)

            # Detect unresolved dynamic indirect calls / jumps (excluding standard returns)
            if re.search(r"\b(jalr|jr)\b", line):
                has_symbol = bool(re.search(r"<.*?>", line))
                is_ret = bool(
                    re.search(
                        r"\b(ret|jr\s+(ra|x1)|jalr\s+(zero|x0),\s*0\((ra|x1)\))\b",
                        line,
                    )
                )
                if not has_symbol and not is_ret:
                    inst_part = line.split("\t")[-1] if "\t" in line else line
                    indirect_jumps[curr_fn].append(inst_part.strip())

    return call_graph, indirect_jumps


def get_reachable_functions(entry_candidates: list, call_graph: dict) -> set:
    """Find all functions transitively reachable from candidate entry points."""
    reachable = set()
    stack = list(entry_candidates)
    while stack:
        fn = stack.pop()
        if fn not in reachable:
            reachable.add(fn)
            for callee in call_graph.get(fn, []):
                if callee not in reachable:
                    stack.append(callee)
    return reachable


def find_deepest_call_path(
    entry_fn: str,
    call_graph: dict,
    stack_sizes: dict,
    current_path: list = None,
    memo: dict = None,
    detected_cycles: set = None,
):
    """Compute the maximum cumulative stack depth via longest path in the DAG."""
    if current_path is None:
        current_path = []
    if memo is None:
        memo = {}
    if detected_cycles is None:
        detected_cycles = set()

    if entry_fn in current_path:
        # Cycle / recursion detected; record cycle and break to prevent infinite loop
        cycle_start_idx = current_path.index(entry_fn)
        cycle_chain = current_path[cycle_start_idx:] + [entry_fn]
        detected_cycles.add(" -> ".join(cycle_chain))
        return stack_sizes.get(entry_fn, 0), [entry_fn]

    if entry_fn in memo:
        return memo[entry_fn]

    new_path = current_path + [entry_fn]
    my_size = stack_sizes.get(entry_fn, 0)
    max_child_size = 0
    best_child_path = []

    for callee in call_graph.get(entry_fn, []):
        child_size, child_path = find_deepest_call_path(
            callee, call_graph, stack_sizes, new_path, memo, detected_cycles
        )
        if child_size > max_child_size:
            max_child_size = child_size
            best_child_path = child_path

    total_size = my_size + max_child_size
    path = [entry_fn] + best_child_path
    memo[entry_fn] = (total_size, path)
    return total_size, path


def attribute_symbol_to_process(symbol: str, process_names: set) -> str:
    """Attribute a symbol to a specific process or classify it as 'shared'."""
    # Check legacy mangling: _ZN<len><proc_name>...
    for proc in process_names:
        prefix = f"_ZN{len(proc)}{proc}"
        if symbol.startswith(prefix):
            return proc
        if (
            f"::{proc}::" in symbol
            or symbol.startswith(f"{proc}::")
            or symbol == f"_entry_{proc}"
        ):
            return proc

    return "shared"


def find_process_entry_points(proc_name: str, all_symbols: list) -> list:
    """Find candidate entry point symbols for a given process."""
    candidates = []

    # 1. Direct entry wrapper symbol: _entry_<proc_name>
    exact_entry = f"_entry_{proc_name}"
    if exact_entry in all_symbols:
        candidates.append(exact_entry)

    # 2. Main inner loop function: _ZN<len><proc_name>..._inner_... or main
    prefix = f"_ZN{len(proc_name)}{proc_name}"
    for sym in all_symbols:
        if sym.startswith(prefix) and ("_inner_" in sym or sym.endswith("4main")):
            if sym not in candidates:
                candidates.append(sym)

    return candidates


def main():
    parser = argparse.ArgumentParser(
        description="Static stack size and cumulative call depth verification tool."
    )
    parser.add_argument("--binary", required=True, help="Path to ELF binary.")
    parser.add_argument("--config", required=True, help="Path to system.json5.")
    parser.add_argument(
        "--app-name", required=True, help="Application name inside system.json5."
    )
    parser.add_argument(
        "--headroom",
        type=int,
        default=0,
        help="Safety headroom buffer in bytes (default: 0).",
    )
    parser.add_argument(
        "--llvm-readobj", default="", help="Path to llvm-readobj executable."
    )
    parser.add_argument(
        "--llvm-objdump", default="", help="Path to llvm-objdump executable."
    )
    parser.add_argument("--report", default="", help="Optional output report path.")

    args = parser.parse_args()

    budgets = extract_process_budgets(args.config, args.app_name, args.headroom)
    min_budget = min(b["budget"] for b in budgets.values())

    llvm_readobj_bin = find_binary_tool("llvm-readobj", args.llvm_readobj)
    llvm_objdump_bin = find_binary_tool("llvm-objdump", args.llvm_objdump)

    stack_sizes = parse_stack_sizes(llvm_readobj_bin, args.binary)
    if not stack_sizes:
        print(
            f"Error: No stack size data found in '{args.binary}'. Ensure -Zemit-stack-sizes was enabled.",
            file=sys.stderr,
        )
        sys.exit(1)

    call_graph, indirect_jumps = build_call_graph(llvm_objdump_bin, args.binary)

    process_names = set(budgets.keys())
    per_process_funcs = {proc: [] for proc in process_names}
    shared_funcs = []

    # 1. Group individual function frames
    for symbol, size in stack_sizes.items():
        owner = attribute_symbol_to_process(symbol, process_names)
        if owner in per_process_funcs:
            per_process_funcs[owner].append((symbol, size))
        else:
            shared_funcs.append((symbol, size))

    # 2. Compute worst-case cumulative call-stack paths for each process
    process_call_chains = {}
    violations = []
    detected_cycles = set()
    process_indirect_jumps = {}

    all_symbols = list(stack_sizes.keys())
    for proc in sorted(process_names):
        entry_candidates = find_process_entry_points(proc, all_symbols)
        best_total_stack = 0
        best_path = []

        memo = {}
        for entry_fn in entry_candidates:
            tot_sz, path = find_deepest_call_path(
                entry_fn, call_graph, stack_sizes, [], memo, detected_cycles
            )
            if tot_sz > best_total_stack:
                best_total_stack = tot_sz
                best_path = path

        # If no explicit entry wrapper was identified, fallback to the largest function frame
        top_sym = None
        if not best_path and per_process_funcs[proc]:
            top_sym, top_sz = max(per_process_funcs[proc], key=lambda x: x[1])
            best_total_stack, best_path = find_deepest_call_path(
                top_sym, call_graph, stack_sizes, [], memo, detected_cycles
            )

        process_call_chains[proc] = {
            "peak_cumulative_stack": best_total_stack,
            "path": best_path,
        }

        # Check for unresolved indirect jumps in reachable functions
        reachable_roots = (
            entry_candidates if entry_candidates else ([top_sym] if top_sym else [])
        )
        reachable_fns = get_reachable_functions(reachable_roots, call_graph)
        proc_ij = {}
        for fn in reachable_fns:
            if fn in indirect_jumps:
                proc_ij[fn] = indirect_jumps[fn]
        if proc_ij:
            process_indirect_jumps[proc] = proc_ij

        # Check cumulative stack against allocated budget
        allowed = budgets[proc]["budget"]
        if best_total_stack > allowed:
            violations.append(
                {
                    "type": "CUMULATIVE_STACK",
                    "owner": proc,
                    "size": best_total_stack,
                    "allowed": allowed,
                    "ram_size_bytes": budgets[proc]["ram_size_bytes"],
                    "path": best_path,
                }
            )

    # Also check individual shared function frames against the minimum process budget
    for symbol, size in shared_funcs:
        if size > min_budget:
            violations.append(
                {
                    "type": "INDIVIDUAL_FRAME",
                    "owner": f"shared (min process budget: {min_budget} B)",
                    "size": size,
                    "allowed": min_budget,
                    "ram_size_bytes": min_budget + args.headroom,
                    "symbol": symbol,
                }
            )

    # 3. Format detailed report
    report_lines = []
    report_lines.append(
        "================================================================================"
    )
    report_lines.append(
        f"  STATIC STACK SIZE & CALL GRAPH VERIFICATION REPORT: App '{args.app_name}'"
    )
    report_lines.append(
        "================================================================================"
    )
    report_lines.append(f"Binary: {args.binary}")
    report_lines.append(f"Config: {args.config}")
    report_lines.append(f"Headroom buffer: {args.headroom} bytes\n")

    if detected_cycles:
        report_lines.append(
            "WARNING: Recursive call cycle(s) detected during call-graph traversal:"
        )
        for c in sorted(detected_cycles):
            report_lines.append(f"  - Cycle: {c}")
        report_lines.append(
            "  Note: Traversal bounded these recursive cycles to 1 iteration."
        )
        report_lines.append(
            "        Worst-case runtime stack depth may exceed static bounds if dynamic recursion occurs.\n"
        )

    if process_indirect_jumps:
        report_lines.append(
            "WARNING: Unresolved indirect jump(s) detected in reachable call paths:"
        )
        for proc, fns in sorted(process_indirect_jumps.items()):
            report_lines.append(f"  ┌─ Process: {proc}")
            for fn, jumps in sorted(fns.items()):
                report_lines.append(
                    f"  ├── Function: {fn} ({len(jumps)} indirect jump(s): {', '.join(jumps[:3])})"
                )
        report_lines.append(
            "  Note: Dynamic dispatch (e.g. &dyn Trait, fn() pointers) cannot be statically traced."
        )
        report_lines.append(
            "        Callee frames beyond these calls are not included in cumulative stack depth.\n"
        )

    report_lines.append("Process Allocation Budgets:")
    for proc, data in sorted(budgets.items()):
        report_lines.append(
            f"  - {proc:15s}: RAM = {data['ram_size_bytes']:6d} B | Max Allowed Stack = {data['budget']:6d} B"
        )
    report_lines.append(
        f"  - {'[shared/common]':15s}: Min Process Budget = {min_budget:6d} B\n"
    )

    report_lines.append("Worst-Case Cumulative Call-Stack Chain by Process:")
    for proc in sorted(process_names):
        chain_info = process_call_chains.get(proc, {})
        peak_stack = chain_info.get("peak_cumulative_stack", 0)
        path = chain_info.get("path", [])
        margin = budgets[proc]["budget"] - peak_stack

        report_lines.append(
            f"  ┌─ Process: {proc} (Peak Stack: {peak_stack:5d} B | Margin: {margin:5d} B / {budgets[proc]['budget']} B)"
        )
        if path:
            for idx, fn in enumerate(path):
                fn_sz = stack_sizes.get(fn, 0)
                prefix = "  └── " if idx == len(path) - 1 else "  ├── "
                report_lines.append(f"{prefix}{fn} ({fn_sz} B)")
        else:
            report_lines.append("  └── (No call path found)")
        report_lines.append("")

    report_lines.append("Peak Individual Function Frame by Process:")
    for proc in sorted(process_names):
        funcs = per_process_funcs[proc]
        if funcs:
            funcs.sort(key=lambda x: x[1], reverse=True)
            top_sym, top_sz = funcs[0]
            margin = budgets[proc]["budget"] - top_sz
            report_lines.append(
                f"  - {proc:15s}: Peak = {top_sz:5d} B (Margin: {margin:5d} B) | Function: {top_sym}"
            )
        else:
            report_lines.append(f"  - {proc:15s}: (No functions recorded)")

    if shared_funcs:
        shared_funcs.sort(key=lambda x: x[1], reverse=True)
        top_sym, top_sz = shared_funcs[0]
        margin = min_budget - top_sz
        report_lines.append(
            f"  - {'[shared/common]':15s}: Peak = {top_sz:5d} B (Margin: {margin:5d} B) | Function: {top_sym}"
        )

    report_lines.append("\n" + "-" * 80)
    if violations:
        report_lines.append(f"FAILED: {len(violations)} stack violation(s) detected!\n")
        for v in violations:
            if v["type"] == "CUMULATIVE_STACK":
                report_lines.append(
                    f"  * CUMULATIVE CALL-STACK VIOLATION in {v['owner']}:\n"
                    f"      Accumulated Stack: {v['size']} B > Allowed Budget: {v['allowed']} B (RAM: {v['ram_size_bytes']} B)\n"
                    f"      Deepest Call Path:"
                )
                for fn in v.get("path", []):
                    report_lines.append(f"        -> {fn} ({stack_sizes.get(fn, 0)} B)")
            else:
                report_lines.append(
                    f"  * INDIVIDUAL FRAME VIOLATION in {v['owner']}:\n"
                    f"      Function: {v.get('symbol', 'unknown')}\n"
                    f"      Frame Size: {v['size']} B > Allowed Budget: {v['allowed']} B (RAM: {v['ram_size_bytes']} B)"
                )
        report_lines.append("=" * 80)
        report_text = "\n".join(report_lines)
        print(report_text, file=sys.stderr)
        if args.report:
            with open(args.report, "w", encoding="utf-8") as f:
                f.write(report_text + "\n")
        sys.exit(1)
    else:
        report_lines.append(
            "PASSED: All cumulative call stacks and function frames conform to allocated process budgets."
        )
        report_lines.append("=" * 80)
        report_text = "\n".join(report_lines)
        print(report_text)
        if args.report:
            with open(args.report, "w", encoding="utf-8") as f:
                f.write(report_text + "\n")
        sys.exit(0)


if __name__ == "__main__":
    main()
