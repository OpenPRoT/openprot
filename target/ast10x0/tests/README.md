# AST10x0 Test Infrastructure

## Overview

Tests for the AST10x0 target are firmware images that run identically under
QEMU or on a physical board. Pass/fail is signalled by writing a sentinel
string to UART:

```
TEST_RESULT:PASS\n
TEST_RESULT:FAIL\n
```

The same `system_image_test` target is used for both execution environments —
no separate hardware-only test targets exist.

## Running Tests

### QEMU (no hardware required)

```
bazel test --config=virt_ast10x0 //target/ast10x0/tests/...
```

### Physical AST1060 EVB via Raspberry Pi SSH fixture

```
AST1060_EVB_PI_HOST=<pi-hostname> bazel test --config=k_ast1060_evb //target/ast10x0/tests/...
```

or inline without modifying the shell environment:

```
bazel test --config=k_ast1060_evb --test_env=AST1060_EVB_PI_HOST=<pi-hostname> //target/ast10x0/tests/...
```

Key-based SSH auth is required (`ssh-copy-id <user>@<pi-host>`). The Pi runs
`pi_test_runner.py`, which handles GPIO reset sequencing, firmware upload over
the UART bootloader, and sentinel detection. UART output is streamed back to
the host for detokenization and display. Tests no longer use cortex_m_semihosting.

### Physical AST1060 EVB wired (not yet implemented)

A wired mode — where the host connects to the Pi fixture over a local serial
port rather than SSH — is not yet implemented. The physical connection type
between the host and the Pi has not been defined (e.g. Pi serial console over
UART, USB serial gadget, or USB networking), so the host-side protocol cannot
be specified. Omitting `AST1060_EVB_PI_HOST` should default to using a wired
connection, but currently logs an unimplemented error.

## Test Results (2026-05-12)

| Test | QEMU | Physical board |
|------|------|----------------|
| `interrupts/kernel:interrupts_test` | PASSED | TIMEOUT — see note below |
| `interrupts/user:interrupts_test` | PASSED | PASSED |
| `ipc/user:ipc_test` | FAILED (hangs at `object_set_peer_user_signal`) | PASSED |
| `threads/kernel:threads_test` | PASSED | PASSED |
| `unittest_runner:unittest_runner` | PASSED | SKIPPED (qemu_only) |
| `usart:usart_test` | PASSED | SKIPPED (qemu_only) |
| `*/no_panics_test` (×5) | SKIPPED (host-only) | SKIPPED |

### `interrupts/kernel:interrupts_test` — times out on physical board

The firmware produces no UART output after upload, indicating a crash before
UART initialisation. The same binary passes in QEMU. The `interrupts/user`
variant (which manages IRQ 42 through the kernel IPC abstraction rather than
raw NVIC manipulation) passes on both.

The suspected cause is the `interrupt_table` entry in `system.json5`: the
codegen for that entry installs an NVIC handler via `early_init()`, which runs
before UART is initialised. If `early_init()` faults (e.g. invalid vector table
layout, bad IRQ number on hardware), the firmware crashes with no UART output
and no way to signal failure. This cannot be verified without a hardware
debugger (GDB via OpenOCD or J-Link) attached to the board.

## How Pass/Fail Signalling Works

Firmware writes the sentinel via `console_backend_write_all`, which calls
`Usart::write_all` directly, bypassing `pw_log` and the tokenizer. This means
the sentinel is always plain ASCII regardless of whether the rest of the log
output is tokenized, and it can be detected without an ELF for detokenization.

### QEMU

`qemu_runner.py` starts QEMU with a PTY for serial I/O and a named pipe for
the raw byte stream. A sentinel watcher thread scans the raw stream; when a
sentinel is found QEMU is killed and the runner exits 0 or 1. A 30-second
watchdog kills QEMU if no sentinel arrives.

### Physical board

`pi_test_runner.py` (running on the Raspberry Pi) sequences the GPIO reset
lines to enter UART bootloader mode, uploads the firmware binary, then streams
raw UART bytes to stdout while scanning for the sentinel. It exits 0 (PASS) or
1 (FAIL/timeout). `test_runner.py` on the host SCP's the script to the Pi,
streams the output back for detokenization and display, and reports the Pi's
exit code to Bazel.

Because the Pi is a shared fixture, `test_runner.py` holds an atomic noclobber
lock file at `/tmp/ast1060_evb.lock` on the Pi for the duration of each test,
preventing multiple users from driving the board over SSH simultaneously. The
lock is touched every 10 seconds by a background thread and considered stale
after 60 seconds of inactivity (e.g. after a crash). If the lock cannot be
acquired within 120 seconds the run is aborted.

## Semihosting Migration

This infrastructure previously used ARM semihosting to signal pass/fail. On
real hardware with no attached debugger, a semihosting trap causes a HardFault,
so hardware testing was impossible. Replacing semihosting with UART sentinels
removed that constraint and enabled the Pi SSH test fixture.

## `uart_upload_test` Targets (removed)

The five `*_uart_upload_test` targets that previously existed in these BUILD
files have been removed. They used an earlier harness (`uart_upload_test.bzl`)
that predated the `run_under` approach. The `system_image_test` targets cover
both QEMU and hardware execution; no separate hardware-only test rule is needed.
