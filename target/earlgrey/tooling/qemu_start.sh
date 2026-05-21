#!/usr/bin/env bash
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
#
# Launch qemu-system-riscv32 for the ot-earlgrey machine and daemonize it.
# The CPU starts paused (-S); the runner must send `cont` to $QEMU_MONITOR
# before any firmware output appears.
#
# Ported from opentitan/hw/top_earlgrey/sw/util/qemu.sh.
# Stripped for v1: JTAG sockets, GPIO socket, I2C, USB, extra UARTs.
# UART0 uses a Unix socket (not PTY) for Bazel sandbox hermeticity.
#
# Required environment variables:
#   QEMU_BIN          path to qemu-system-riscv32
#   QEMU_CONFIG       path to QEMU readconfig INI (from cfggen.py)
#   QEMU_ROM          path to test ROM ELF
#   QEMU_OTP          path to mutable raw OTP image (from otptool.py)
#   QEMU_FLASH        path to mutable flash image (from flashgen.py)
#   QEMU_SPIFLASH     path to mutable 32 MiB SPI-flash backing store
#   QEMU_PIDFILE      path where QEMU writes its PID after daemonizing
#   QEMU_LOG          path for QEMU's -D log output
#   QEMU_ICOUNT       icount shift value (default: 6)
#   QEMU_MONITOR      path for the QEMU monitor Unix socket (HMP mode)
#   QEMU_UART_SOCKET  path for the UART0 Unix socket

set -e

_fail() {
    echo "qemu_start.sh: $1" >&2
    exit 1
}

# Mandatory checks — all must be non-empty strings pointing at real files.
[ -n "$QEMU_BIN"         ] || _fail "QEMU_BIN is unset"
[ -f "$QEMU_BIN"         ] || _fail "QEMU_BIN not found: $QEMU_BIN"
[ -n "$QEMU_CONFIG"      ] || _fail "QEMU_CONFIG is unset"
[ -f "$QEMU_CONFIG"      ] || _fail "QEMU_CONFIG not found: $QEMU_CONFIG"
[ -n "$QEMU_ROM"         ] || _fail "QEMU_ROM is unset"
[ -f "$QEMU_ROM"         ] || _fail "QEMU_ROM not found: $QEMU_ROM"
[ -n "$QEMU_OTP"         ] || _fail "QEMU_OTP is unset"
[ -f "$QEMU_OTP"         ] || _fail "QEMU_OTP not found: $QEMU_OTP"
[ -n "$QEMU_FLASH"       ] || _fail "QEMU_FLASH is unset"
[ -f "$QEMU_FLASH"       ] || _fail "QEMU_FLASH not found: $QEMU_FLASH"
[ -n "$QEMU_SPIFLASH"    ] || _fail "QEMU_SPIFLASH is unset"
[ -f "$QEMU_SPIFLASH"    ] || _fail "QEMU_SPIFLASH not found: $QEMU_SPIFLASH"
[ -n "$QEMU_PIDFILE"     ] || _fail "QEMU_PIDFILE is unset"
[ -n "$QEMU_LOG"         ] || _fail "QEMU_LOG is unset"
[ -n "$QEMU_MONITOR"     ] || _fail "QEMU_MONITOR is unset"
[ -n "$QEMU_UART_SOCKET" ] || _fail "QEMU_UART_SOCKET is unset"

QEMU_ICOUNT="${QEMU_ICOUNT:-6}"

qemu_args=(
    # No GUI.
    "-display" "none"

    # Earlgrey 1.0.0 machine.
    "-M" "ot-earlgrey"

    # RTL constants from cfggen.
    "-readconfig" "$QEMU_CONFIG"

    # Daemonize after initialization; write PID to file.
    "-daemonize"
    "-pidfile" "$QEMU_PIDFILE"

    # Start CPU paused — runner sends `cont` via monitor before tailing UART.
    "-S"

    # Log guest errors and unimplemented peripheral access (invaluable on failures).
    "-D" "$QEMU_LOG"
    "-d" "guest_errors,unimp"

    # ROM image.
    "-object" "ot-rom_img,id=rom,file=${QEMU_ROM}"

    # OTP backing store (pflash).
    "-drive" "if=pflash,file=${QEMU_OTP},format=raw"

    # Firmware flash (mtd bus 2 = eflash).
    "-drive" "if=mtd,id=eflash,bus=2,file=${QEMU_FLASH},format=raw"

    # SPI Host 0 backing store — ot-earlgrey machine requires this drive even
    # when the test does not exercise SPI.  W25Q256 = 32 MiB, matches the dd
    # image created by the runner.
    "-global" "ot-earlgrey-board.spiflash0=w25q256"
    "-drive" "if=mtd,file=${QEMU_SPIFLASH},format=raw,bus=0"

    # Virtual-time pacing: 1 GHz >> icount ns/insn ≈ wall-clock alignment.
    "-icount" "shift=${QEMU_ICOUNT},align=on,sleep=on"

    # Disable fatal-reset so tests that trigger a reset don't kill QEMU.
    "-global" "ot-rstmgr.fatal_reset=0"

    # Disable keymgr flash-seed check (info pages not spliced; would error).
    "-global" "ot-keymgr.disable-flash-seed-check=true"

    # Suppress test-status-register exit so multiple resets are possible.
    "-global" "ot-ibex_wrapper.dv-sim-status-exit=off"

    # Monitor socket in HMP (readline) mode — runner writes plain `cont\n`.
    "-chardev" "socket,id=monitor,path=${QEMU_MONITOR},server=on,wait=off"
    "-mon" "chardev=monitor,mode=readline"

    # UART0 as a Unix socket (not PTY) for hermeticity inside Bazel sandboxes.
    "-chardev" "socket,path=${QEMU_UART_SOCKET},server=on,wait=off,id=uart0"
    "-serial" "chardev:uart0"

    # UART1 — no external endpoint needed for software-loopback tests.
    "-serial" "null"
)

echo "qemu_start.sh: launching ${QEMU_BIN} with ${#qemu_args[@]} args" >&2
exec "$QEMU_BIN" "${qemu_args[@]}"
