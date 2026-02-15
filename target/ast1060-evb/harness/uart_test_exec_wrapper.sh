#!/bin/bash
# Licensed under the Apache-2.0 license
#
# Wrapper script for uart_test_exec.py
# Invokes the Python script with system Python3

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_SCRIPT="$SCRIPT_DIR/uart_test_exec.py"

# If running from runfiles, look there
if [[ -f "$PYTHON_SCRIPT" ]]; then
    exec python3 "$PYTHON_SCRIPT" "$@"
fi

# Try relative to workspace root
if [[ -f "tools/uart_test/uart_test_exec.py" ]]; then
    exec python3 "tools/uart_test/uart_test_exec.py" "$@"
fi

# Try runfiles directory
RUNFILES="${BASH_SOURCE[0]}.runfiles"
if [[ -d "$RUNFILES" ]]; then
    SCRIPT="$RUNFILES/_main/tools/uart_test/uart_test_exec.py"
    if [[ -f "$SCRIPT" ]]; then
        exec python3 "$SCRIPT" "$@"
    fi
fi

echo "ERROR: Could not find uart_test_exec.py" >&2
exit 1
