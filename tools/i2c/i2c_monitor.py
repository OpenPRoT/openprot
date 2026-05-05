#!/usr/bin/env python3
# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""
I2C/SMBus Monitor for TotalPhase Beagle Bus Analyzer with MCTP/SPDM Support
Automatically connects to the first available device and listens for I2C transactions.
Supports MCTP packet assembly and SPDM message parsing.
"""

import sys
import os
import argparse
import threading
import shutil
import curses
from typing import List, Optional, Dict
from collections import defaultdict
import textwrap

# Add the Beagle API to the path
# Assumes script runs at the same level as beagle-api-linux-x86_64-v6.00 directory
script_dir = os.path.dirname(os.path.abspath(__file__))
beagle_lib_path = os.path.join(script_dir, "beagle-api-linux-x86_64-v6.00", "python")
sys.path.insert(0, beagle_lib_path)
from beagle_py import *


# Global state for display
class DisplayState:
    def __init__(self):
        self.lines = []
        self.lock = threading.Lock()
        self.stdscr = None
        self.max_lines = 10000  # Keep last N lines for saving
        self.scroll_offset = 0  # 0 = at bottom, positive = scrolled back
        self.auto_scroll = True  # Auto-scroll when at bottom


display_state = DisplayState()


# Global state for binary data capture
class BinaryDataCapture:
    def __init__(self):
        self.lock = threading.Lock()
        self.transactions = []  # List of raw transaction data with metadata
        self.max_transactions = 10000  # Keep last N transactions

    def add_transaction(
        self, timestamp, raw_i2c_data, i2c_addr, i2c_rw, payload_data, status
    ):
        """Add a raw I2C transaction with metadata for replay"""
        with self.lock:
            self.transactions.append(
                {
                    "timestamp": timestamp,
                    "data": raw_i2c_data,  # Full transaction including address
                    "addr": i2c_addr,
                    "rw": i2c_rw,
                    "payload": payload_data,  # Just the data bytes (no address)
                    "status": status,
                }
            )
            if len(self.transactions) > self.max_transactions:
                self.transactions.pop(0)

    def clear(self):
        """Clear all captured data"""
        with self.lock:
            self.transactions.clear()


binary_capture = BinaryDataCapture()


# Global state for statistics
class Statistics:
    def __init__(self):
        self.lock = threading.Lock()
        self.commands_sent = 0
        self.acks_received = 0
        self.naks_received = 0
        self.total_bytes = 0

    def reset(self):
        with self.lock:
            self.commands_sent = 0
            self.acks_received = 0
            self.naks_received = 0
            self.total_bytes = 0


statistics = Statistics()


# Global state for device management
class DeviceState:
    def __init__(self):
        self.lock = threading.Lock()
        self.available_devices = []  # List of (port, unique_id) tuples
        self.current_device_index = None
        self.beagle_handle = None
        self.samplerate_khz = 10000
        self.timeout_ms = 500
        self.latency_ms = 200

    def update_device_list(self):
        """Refresh the list of available devices"""
        with self.lock:
            (num, ports, unique_ids) = bg_find_devices_ext(16, 16)
            self.available_devices = []
            for i in range(num):
                port = ports[i]
                unique_id = unique_ids[i]
                in_use = bool(port & BG_PORT_NOT_FREE)
                actual_port = port & ~BG_PORT_NOT_FREE
                self.available_devices.append(
                    {
                        "port": actual_port,
                        "unique_id": unique_id,
                        "in_use": in_use,
                        "raw_port": port,
                    }
                )


device_state = DeviceState()


# Global state for display mode
class DisplayMode:
    def __init__(self):
        self.lock = threading.Lock()
        self.mode = "raw"  # 'raw', 'smbus', 'mctp', 'spdm'
        self.validate_pec = False

    def set_mode(self, mode):
        """Set the display mode"""
        with self.lock:
            self.mode = mode

    def get_mode(self):
        """Get the current display mode"""
        with self.lock:
            return self.mode


display_mode = DisplayMode()

# About text
ABOUT_TEXT = [
    "I2C Monitor TUI for TotalPhase Beagle, © 2026 AMD, Inc.",
    "Made available under the Apache 2 License as part of the OpenPRoT Project.",
]

# Command definitions with help text (alphabetically ordered)
COMMANDS = {
    "about": {
        "description": "Display program information",
        "usage": "about",
        "args": [],
    },
    "clear": {
        "description": "Clear the screen and reset statistics",
        "usage": "clear",
        "args": [],
    },
    "device": {
        "description": "Show or change the active device",
        "usage": "device [list|use <index|id>]",
        "args": [
            ("list", "Show all available devices"),
            ("use <index>", "Switch to device by index from list"),
            ("use <id>", "Switch to device by serial number"),
        ],
    },
    "display": {
        "description": "Change display mode and replay buffer",
        "usage": "display [raw|smbus|mctp|spdm]",
        "args": [
            ("raw", "Show raw I2C transactions"),
            ("smbus", "Show SMBus packets"),
            ("mctp", "Show MCTP packets (hides non-MCTP traffic)"),
            ("spdm", "Show SPDM messages (hides non-SPDM traffic)"),
        ],
    },
    "exit": {"description": "Exit the analyzer", "usage": "exit", "args": []},
    "help": {
        "description": "Show available commands or help for a specific command",
        "usage": "help [command]",
        "args": [("command", "Optional. The command to get help for")],
    },
    "quit": {"description": "Exit the analyzer", "usage": "quit", "args": []},
    "save": {
        "description": "Save captured data to a file",
        "usage": "save [-b] <filename>",
        "args": [
            ("-b", "Save binary I2C data instead of text output"),
            ("filename", "Path to output file"),
        ],
    },
    "settings": {
        "description": "Show or modify device settings",
        "usage": "settings [samplerate=KHZ] [timeout=MS] [latency=MS]",
        "args": [
            ("samplerate", "Sample rate in kHz (e.g., samplerate=10000)"),
            ("timeout", "Idle timeout in milliseconds (e.g., timeout=500)"),
            ("latency", "Latency in milliseconds (e.g., latency=200)"),
        ],
    },
}


# ==========================================================================
# COMMAND INTERFACE
# ==========================================================================


def add_display_line(text):
    """Add a line to the display buffer"""
    with display_state.lock:
        display_state.lines.append(text)
        if len(display_state.lines) > display_state.max_lines:
            display_state.lines.pop(0)
        # Reset scroll if auto-scrolling
        if display_state.auto_scroll:
            display_state.scroll_offset = 0


def print_help(command=None):
    """Print help information"""
    if command:
        if command in COMMANDS:
            cmd_info = COMMANDS[command]
            add_display_line("")
            add_display_line(f"Command: {command}")
            add_display_line(f"Description: {cmd_info['description']}")
            add_display_line(f"Usage: {cmd_info['usage']}")
            if cmd_info["args"]:
                add_display_line("")
                add_display_line("Arguments:")
                for arg_name, arg_desc in cmd_info["args"]:
                    add_display_line(f"  {arg_name}: {arg_desc}")
        else:
            add_display_line("")
            add_display_line(f"Unknown command: {command}")
    else:
        add_display_line("")
        add_display_line("Available commands:")
        for cmd, info in COMMANDS.items():
            add_display_line(f"  {cmd:15} - {info['description']}")
        add_display_line("")
        add_display_line(
            "Type 'help <command>' for more information on a specific command"
        )
        add_display_line("")
        add_display_line("Keyboard shortcuts:")
        add_display_line("  Up/Down       - Navigate command history")
        add_display_line("  Page Up/Down  - Scroll through capture buffer")
        add_display_line("  Home/End      - Jump to top/bottom of buffer")
        add_display_line("  Ctrl+C        - Exit the analyzer")


def handle_settings_command(args):
    """Handle settings command"""
    if not args:
        # Show current settings
        with device_state.lock:
            add_display_line("")
            add_display_line("Current Device Settings:")
            add_display_line(f"  Sample Rate: {device_state.samplerate_khz} kHz")
            add_display_line(f"  Timeout:     {device_state.timeout_ms} ms")
            add_display_line(f"  Latency:     {device_state.latency_ms} ms")
        return

    # Parse and apply settings
    for arg in args:
        if "=" not in arg:
            add_display_line(f"Invalid setting format: {arg} (use key=value)")
            continue

        key, value = arg.split("=", 1)
        key = key.lower()

        try:
            value_int = int(value)
        except ValueError:
            add_display_line(f"Invalid value for {key}: {value} (must be integer)")
            continue

        if key == "samplerate":
            if value_int <= 0:
                add_display_line(f"Sample rate must be positive")
                continue
            with device_state.lock:
                if device_state.beagle_handle:
                    result = bg_samplerate(device_state.beagle_handle, value_int)
                    if result < 0:
                        add_display_line(
                            f"Error setting sample rate: {bg_status_string(result)}"
                        )
                    else:
                        device_state.samplerate_khz = result
                        add_display_line(f"Sample rate set to {result} kHz")
                else:
                    device_state.samplerate_khz = value_int
                    add_display_line(
                        f"Sample rate will be set to {value_int} kHz on next device connection"
                    )

        elif key == "timeout":
            if value_int < 0:
                add_display_line(f"Timeout must be non-negative")
                continue
            with device_state.lock:
                if device_state.beagle_handle:
                    bg_timeout(device_state.beagle_handle, value_int)
                device_state.timeout_ms = value_int
                add_display_line(f"Timeout set to {value_int} ms")

        elif key == "latency":
            if value_int < 0:
                add_display_line(f"Latency must be non-negative")
                continue
            with device_state.lock:
                if device_state.beagle_handle:
                    bg_latency(device_state.beagle_handle, value_int)
                device_state.latency_ms = value_int
                add_display_line(f"Latency set to {value_int} ms")

        else:
            add_display_line(f"Unknown setting: {key}")


def handle_device_command(args, stop_event):
    """Handle device command"""
    if not args:
        # Show current device
        with device_state.lock:
            add_display_line("")
            if (
                device_state.current_device_index is not None
                and device_state.current_device_index
                < len(device_state.available_devices)
            ):
                dev = device_state.available_devices[device_state.current_device_index]
                add_display_line(f"Current Device:")
                add_display_line(f"  Port: {dev['port']}")
                add_display_line(
                    f"  S/N:  {dev['unique_id']:04d}-{dev['unique_id'] % 1000000:06d}"
                )
            else:
                add_display_line("No device currently selected")
        return

    subcmd = args[0].lower()

    if subcmd == "list":
        # Show all available devices
        device_state.update_device_list()
        with device_state.lock:
            add_display_line("")
            add_display_line("Available Devices:")
            if not device_state.available_devices:
                add_display_line("  No devices found")
            else:
                for i, dev in enumerate(device_state.available_devices):
                    marker = ">" if i == device_state.current_device_index else " "
                    status = (
                        "(in use)"
                        if dev["in_use"] and i != device_state.current_device_index
                        else ""
                    )
                    add_display_line(
                        f"{marker} {i}: Port {dev['port']}, S/N {dev['unique_id']:04d}-{dev['unique_id'] % 1000000:06d} {status}"
                    )

    elif subcmd == "use":
        if len(args) < 2:
            add_display_line("Usage: device use <index|serial>")
            return

        identifier = args[1]
        device_state.update_device_list()

        new_index = None
        with device_state.lock:
            # Try as index first
            try:
                idx = int(identifier)
                if 0 <= idx < len(device_state.available_devices):
                    new_index = idx
            except ValueError:
                # Try as serial number
                for i, dev in enumerate(device_state.available_devices):
                    sn_full = f"{dev['unique_id']:04d}-{dev['unique_id'] % 1000000:06d}"
                    if str(dev["unique_id"]) == identifier or sn_full == identifier:
                        new_index = i
                        break

        if new_index is None:
            add_display_line(f"Device not found: {identifier}")
            return

        # Check if device is in use
        with device_state.lock:
            if (
                device_state.available_devices[new_index]["in_use"]
                and new_index != device_state.current_device_index
            ):
                add_display_line(
                    f"Device {new_index} is already in use by another process"
                )
                return

        add_display_line(f"Switching to device {new_index}...")
        add_display_line(
            "Note: Device switching requires restart - feature not yet fully implemented"
        )
        # TODO: Implement device switching by stopping monitor thread, closing device, opening new one

    else:
        add_display_line(f"Unknown device subcommand: {subcmd}")
        add_display_line("Usage: device [list|use <index|id>]")


def handle_save_command(args):
    """Handle save command"""
    if not args:
        add_display_line("Usage: save [-b] <filename>")
        return

    # Check for -b flag
    binary_mode = False
    filename = None

    for arg in args:
        if arg == "-b":
            binary_mode = True
        elif not filename:
            filename = arg

    if not filename:
        add_display_line("Error: filename required")
        add_display_line("Usage: save [-b] <filename>")
        return

    try:
        if binary_mode:
            # Save raw binary I2C data
            with binary_capture.lock:
                if not binary_capture.transactions:
                    add_display_line("No binary data captured")
                    return

                total_bytes = 0
                with open(filename, "wb") as f:
                    for trans in binary_capture.transactions:
                        # Write raw I2C data only
                        f.write(bytes(trans["data"]))
                        total_bytes += len(trans["data"])

                add_display_line(
                    f"Saved {len(binary_capture.transactions)} transactions ({total_bytes} bytes) to {filename}"
                )
        else:
            # Save text output
            with display_state.lock:
                if not display_state.lines:
                    add_display_line("No text data to save")
                    return

                with open(filename, "w") as f:
                    for line in display_state.lines:
                        f.write(line + "\n")

                add_display_line(
                    f"Saved {len(display_state.lines)} lines to {filename}"
                )

    except IOError as e:
        add_display_line(f"Error saving file: {e}")
    except Exception as e:
        add_display_line(f"Unexpected error: {e}")


def replay_buffer(mode):
    """Replay captured data in specified display mode"""
    with binary_capture.lock:
        if not binary_capture.transactions:
            add_display_line("No data to replay")
            return

        # Clear display and reset scroll
        with display_state.lock:
            display_state.lines.clear()
            display_state.scroll_offset = 0
            display_state.auto_scroll = True

        add_display_line("")
        add_display_line("=" * 80)
        add_display_line(
            f"{mode.upper()} Replay - {len(binary_capture.transactions)} transactions"
        )
        add_display_line("=" * 80)
        add_display_line("")

        # Create new MCTP assembler for replay
        mctp_assembler = MCTPAssembler()

        # Replay each transaction
        for trans in binary_capture.transactions:
            timestamp = trans["timestamp"]
            addr = trans["addr"]
            rw = trans["rw"]
            raw_data = trans["payload"]
            status = trans["status"]

            if not raw_data:
                continue

            # Process based on mode
            if mode == "raw":
                print_raw_i2c(timestamp, addr, rw, raw_data, status)

            elif mode in ["smbus", "mctp", "spdm"]:
                # Try to parse as SMBus with PEC
                smbus_result = parse_smbus_header(raw_data, display_mode.validate_pec)

                if smbus_result and mode in ["mctp", "spdm"]:
                    try:
                        # Create synthetic MCTP packet
                        synthetic_mctp = [
                            smbus_result["dest_eid"],
                            (smbus_result["mctp_hdr_version"] << 4)
                            | smbus_result["mctp_reserved"],
                            smbus_result["src_eid"],
                        ] + smbus_result["mctp_data"]

                        mctp_parsed = MCTPParser.parse(synthetic_mctp)

                        # Assemble fragments
                        complete_msg = mctp_assembler.add_fragment(mctp_parsed)

                        if complete_msg is not None:
                            # Complete message assembled
                            if mode == "spdm" and "spdm" in complete_msg:
                                print_spdm_message(timestamp, addr, rw, complete_msg)
                            elif mode == "mctp":
                                print_mctp_message(
                                    timestamp, addr, rw, smbus_result, complete_msg
                                )

                    except Exception as e:
                        add_display_line(f"[{timestamp:12d} ns] MCTP Parse Error: {e}")

                elif smbus_result and mode == "smbus":
                    print_smbus_message(timestamp, addr, rw, smbus_result, raw_data)

        add_display_line("")
        add_display_line("=" * 80)
        add_display_line("Replay Complete")
        add_display_line("=" * 80)


def handle_display_command(args):
    """Handle display mode command"""
    if not args:
        # Show current mode
        mode = display_mode.get_mode()
        add_display_line("")
        add_display_line(f"Current display mode: {mode}")
        add_display_line("Available modes: raw, smbus, mctp, spdm")
        return

    new_mode = args[0].lower()
    if new_mode not in ["raw", "smbus", "mctp", "spdm"]:
        add_display_line(f"Unknown mode: {new_mode}")
        add_display_line("Available modes: raw, smbus, mctp, spdm")
        return

    # Set the new mode
    display_mode.set_mode(new_mode)
    add_display_line(f"Display mode changed to: {new_mode}")

    # Replay the buffer in the new mode
    replay_buffer(new_mode)


def handle_command(cmd_line, stop_event):
    """Handle a command input. Returns True to continue, False to stop."""
    parts = cmd_line.strip().split()
    if not parts:
        return True

    cmd = parts[0].lower()
    args = parts[1:]

    if cmd == "about":
        add_display_line("")
        for line in ABOUT_TEXT:
            add_display_line(line)
        add_display_line("")
    elif cmd == "clear":
        with display_state.lock:
            display_state.lines.clear()
        binary_capture.clear()
        statistics.reset()
    elif cmd == "help":
        if args:
            print_help(args[0])
        else:
            print_help()
    elif cmd == "settings":
        handle_settings_command(args)
    elif cmd == "device":
        handle_device_command(args, stop_event)
    elif cmd == "save":
        handle_save_command(args)
    elif cmd == "display":
        handle_display_command(args)
    elif cmd == "quit" or cmd == "exit":
        add_display_line("")
        add_display_line("Stopping monitoring...")
        stop_event.set()
        return False
    else:
        add_display_line("")
        add_display_line(f"Unknown command: {cmd}. Type 'help' for available commands.")

    return True


def draw_screen(stdscr, input_buffer, last_line_count, force_redraw=False):
    """Draw the screen with monitoring output, command prompt, and status bar at bottom"""
    height, width = stdscr.getmaxyx()

    # Calculate areas (3 lines for prompt + 1 status bar at bottom = 4 lines)
    monitor_height = height - 4

    # Check if we need to redraw
    current_line_count = len(display_state.lines)
    needs_redraw = force_redraw or (current_line_count != last_line_count)

    if needs_redraw:
        # Only erase, don't clear (reduces flicker)
        stdscr.erase()

        # Draw monitoring output
        with display_state.lock:
            # Calculate which lines to show based on scroll offset
            total_lines = len(display_state.lines)
            # scroll_offset = 0 means show the last monitor_height lines
            # scroll_offset > 0 means scroll back that many lines
            end_idx = total_lines - display_state.scroll_offset
            start_idx = max(0, end_idx - monitor_height)

            # Ensure we don't go past the end
            end_idx = min(end_idx, total_lines)

            visible_lines = display_state.lines[start_idx:end_idx]

            for i, line in enumerate(visible_lines):
                if i >= monitor_height:
                    break
                try:
                    # Wrap line if too long
                    if len(line) > width:
                        wrapped = textwrap.wrap(line, width)
                        for j, wrapped_line in enumerate(wrapped):
                            if i + j < monitor_height:
                                stdscr.addstr(i + j, 0, wrapped_line[: width - 1])
                    else:
                        stdscr.addstr(i, 0, line[: width - 1])
                except curses.error:
                    pass

    # Always redraw prompt area (minimal flicker since it's small)
    prompt_y = height - 4
    try:
        # Top line of prompt
        stdscr.addstr(prompt_y, 0, "-" * (width - 1))
        # Prompt line with >
        prompt_line = "> " + input_buffer
        # Clear the rest of the line
        stdscr.addstr(prompt_y + 1, 0, (prompt_line + " " * width)[: width - 1])
        # Bottom line of prompt
        stdscr.addstr(prompt_y + 2, 0, "-" * (width - 1))
        # Position cursor after prompt
        cursor_x = min(2 + len(input_buffer), width - 1)
        stdscr.move(prompt_y + 1, cursor_x)
    except curses.error:
        pass

    # Draw status bar (always redraw) - below the prompt
    status_y = height - 1
    try:
        with statistics.lock:
            cmds = statistics.commands_sent
            acks = statistics.acks_received
            naks = statistics.naks_received
            bytes_total = statistics.total_bytes

        # Get current display mode
        mode = display_mode.get_mode()

        # Build status bar with color highlighting
        x_pos = 0

        # Clear the line first
        stdscr.addstr(status_y, 0, " " * (width - 1), curses.A_REVERSE)

        # "Cmds: X | "
        text = f"Cmds: {cmds} | "
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE)
        x_pos += len(text)

        # "ACKs: " (normal)
        text = "ACKs: "
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE)
        x_pos += len(text)

        # ACK number (green)
        text = str(acks)
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE | curses.color_pair(1))
        x_pos += len(text)

        # " | "
        text = " | "
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE)
        x_pos += len(text)

        # "NAKs: " (normal)
        text = "NAKs: "
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE)
        x_pos += len(text)

        # NAK number (red)
        text = str(naks)
        stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE | curses.color_pair(2))
        x_pos += len(text)

        # " | Bytes: X"
        text = f" | Bytes: {bytes_total}"
        if x_pos + len(text) < width:
            stdscr.addstr(status_y, x_pos, text, curses.A_REVERSE)
            x_pos += len(text)

        # Mode (right-justified)
        mode_text = mode
        mode_x = width - len(mode_text) - 1
        if mode_x > x_pos + 1:  # Make sure there's space
            stdscr.addstr(status_y, mode_x, mode_text, curses.A_REVERSE)

    except curses.error:
        pass

    stdscr.refresh()
    return current_line_count


def command_loop_curses(stdscr, stop_event):
    """Command input loop with curses UI"""
    display_state.stdscr = stdscr

    # Configure curses
    curses.curs_set(1)  # Show cursor
    stdscr.nodelay(True)  # Non-blocking input
    stdscr.timeout(100)  # 100ms timeout for getch
    stdscr.keypad(True)  # Enable keypad mode for special keys

    # Enable mouse support
    try:
        curses.mousemask(curses.ALL_MOUSE_EVENTS | curses.REPORT_MOUSE_POSITION)
    except:
        pass  # Mouse support not available

    # Initialize colors
    if curses.has_colors():
        curses.start_color()
        curses.use_default_colors()
        # Color pair 1: Green for ACKs
        curses.init_pair(1, curses.COLOR_GREEN, -1)
        # Color pair 2: Red for NAKs
        curses.init_pair(2, curses.COLOR_RED, -1)

    input_buffer = ""
    last_line_count = 0
    force_redraw = True

    # Command history
    command_history = []
    history_index = -1  # -1 = not browsing history
    saved_input = ""  # Save current input when browsing history

    # For handling escape sequences
    escape_sequence = ""
    in_escape = False

    try:
        while not stop_event.is_set():
            # Draw the screen (only redraws if content changed or forced)
            last_line_count = draw_screen(
                stdscr, input_buffer, last_line_count, force_redraw
            )
            force_redraw = False

            # Get input
            try:
                ch = stdscr.getch()

                if ch == -1:  # No input
                    continue

                # Handle mouse events
                if ch == curses.KEY_MOUSE:
                    try:
                        _, mx, my, _, bstate = curses.getmouse()
                        height, width = stdscr.getmaxyx()
                        monitor_height = height - 4

                        # Mouse wheel up (scroll up in buffer)
                        if bstate & curses.BUTTON4_PRESSED:
                            with display_state.lock:
                                max_scroll = max(
                                    0, len(display_state.lines) - monitor_height
                                )
                                display_state.scroll_offset = min(
                                    display_state.scroll_offset + 3, max_scroll
                                )
                                if display_state.scroll_offset > 0:
                                    display_state.auto_scroll = False
                            force_redraw = True
                            continue

                        # Mouse wheel down (scroll down in buffer)
                        elif bstate & curses.BUTTON5_PRESSED:
                            with display_state.lock:
                                display_state.scroll_offset = max(
                                    0, display_state.scroll_offset - 3
                                )
                                if display_state.scroll_offset == 0:
                                    display_state.auto_scroll = True
                            force_redraw = True
                            continue

                    except curses.error:
                        pass
                    continue

                # Handle escape sequences for arrow keys (if curses keys don't work)
                if ch == 27:  # ESC
                    in_escape = True
                    escape_sequence = ""
                    continue
                elif in_escape:
                    escape_sequence += chr(ch) if ch < 256 else ""

                    # Check if we have a complete escape sequence
                    if escape_sequence == "[A":  # Up arrow
                        ch = curses.KEY_UP
                        in_escape = False
                    elif escape_sequence == "[B":  # Down arrow
                        ch = curses.KEY_DOWN
                        in_escape = False
                    elif escape_sequence == "[5~":  # Page Up
                        ch = curses.KEY_PPAGE
                        in_escape = False
                    elif escape_sequence == "[6~":  # Page Down
                        ch = curses.KEY_NPAGE
                        in_escape = False
                    elif escape_sequence == "[H":  # Home
                        ch = curses.KEY_HOME
                        in_escape = False
                    elif escape_sequence == "[F":  # End
                        ch = curses.KEY_END
                        in_escape = False
                    elif len(escape_sequence) >= 3:  # Unknown sequence, give up
                        in_escape = False
                        continue
                    else:
                        continue  # Need more characters

                if ch == ord("\n"):  # Enter
                    if input_buffer.strip():
                        # Add to history if not duplicate of last command
                        if not command_history or command_history[-1] != input_buffer:
                            command_history.append(input_buffer)
                            # Limit history to 100 commands
                            if len(command_history) > 100:
                                command_history.pop(0)

                        if not handle_command(input_buffer, stop_event):
                            break
                    input_buffer = ""
                    history_index = -1
                    saved_input = ""
                    force_redraw = True  # Redraw after command

                elif ch == curses.KEY_BACKSPACE or ch == 127 or ch == 8:  # Backspace
                    if input_buffer:
                        input_buffer = input_buffer[:-1]
                    history_index = -1  # Exit history browsing

                elif ch == curses.KEY_UP:  # Up arrow - previous command
                    if command_history:
                        if history_index == -1:
                            # Start browsing history, save current input
                            saved_input = input_buffer
                            history_index = len(command_history) - 1
                        elif history_index > 0:
                            history_index -= 1

                        if history_index >= 0:
                            input_buffer = command_history[history_index]

                elif ch == curses.KEY_DOWN:  # Down arrow - next command
                    if history_index != -1:
                        if history_index < len(command_history) - 1:
                            history_index += 1
                            input_buffer = command_history[history_index]
                        else:
                            # Reached end of history, restore saved input
                            history_index = -1
                            input_buffer = saved_input

                elif ch == curses.KEY_PPAGE:  # Page Up - scroll up
                    height, width = stdscr.getmaxyx()
                    monitor_height = height - 4
                    with display_state.lock:
                        # Scroll up one page
                        max_scroll = max(0, len(display_state.lines) - monitor_height)
                        display_state.scroll_offset = min(
                            display_state.scroll_offset + monitor_height, max_scroll
                        )
                        # Disable auto-scroll when manually scrolling
                        if display_state.scroll_offset > 0:
                            display_state.auto_scroll = False
                    force_redraw = True

                elif ch == curses.KEY_NPAGE:  # Page Down - scroll down
                    with display_state.lock:
                        display_state.scroll_offset = max(
                            0, display_state.scroll_offset - (height - 4)
                        )
                        # Re-enable auto-scroll when scrolled to bottom
                        if display_state.scroll_offset == 0:
                            display_state.auto_scroll = True
                    force_redraw = True

                elif ch == curses.KEY_HOME:  # Home - scroll to top
                    height, width = stdscr.getmaxyx()
                    monitor_height = height - 4
                    with display_state.lock:
                        display_state.scroll_offset = max(
                            0, len(display_state.lines) - monitor_height
                        )
                        display_state.auto_scroll = False
                    force_redraw = True

                elif ch == curses.KEY_END:  # End - scroll to bottom
                    with display_state.lock:
                        display_state.scroll_offset = 0
                        display_state.auto_scroll = True
                    force_redraw = True

                elif ch == 3:  # Ctrl+C
                    stop_event.set()
                    break
                elif ch == curses.KEY_RESIZE:  # Terminal resized
                    force_redraw = True
                elif 32 <= ch <= 126:  # Printable characters
                    input_buffer += chr(ch)
                    history_index = -1  # Exit history browsing on new input
            except curses.error:
                pass

    except KeyboardInterrupt:
        stop_event.set()
    finally:
        display_state.stdscr = None


# ==========================================================================
# MCTP/SPDM PARSING CODE (from mctp-parse)
# ==========================================================================


class SPDMParser:
    """Parser for SPDM messages (DSP0274)"""

    # Request codes (DSP0274 Table 4)
    REQUEST_CODES = {
        0x81: "GET_DIGESTS",
        0x82: "GET_CERTIFICATE",
        0x83: "CHALLENGE",
        0x84: "GET_VERSION",
        0x85: "CHUNK_SEND",
        0x86: "CHUNK_GET",
        0x87: "GET_ENDPOINT_INFO",
        0xE0: "GET_MEASUREMENTS",
        0xE1: "GET_CAPABILITIES",
        0xE2: "GET_SUPPORTED_EVENT_TYPES",
        0xE3: "NEGOTIATE_ALGORITHMS",
        0xE4: "KEY_EXCHANGE",
        0xE5: "FINISH",
        0xE6: "PSK_EXCHANGE",
        0xE7: "PSK_FINISH",
        0xE8: "HEARTBEAT",
        0xE9: "KEY_UPDATE",
        0xEA: "GET_ENCAPSULATED_REQUEST",
        0xEB: "DELIVER_ENCAPSULATED_RESPONSE",
        0xEC: "END_SESSION",
        0xED: "GET_CSR",
        0xEE: "SET_CERTIFICATE",
        0xEF: "GET_MEASUREMENT_EXTENSION_LOG",
        0xF0: "SUBSCRIBE_EVENT_TYPES",
        0xF1: "SEND_EVENT",
        0xFC: "GET_KEY_PAIR_INFO",
        0xFD: "SET_KEY_PAIR_INFO",
        0xFE: "VENDOR_DEFINED_REQUEST",
        0xFF: "RESPOND_IF_READY",
    }

    # Response codes (DSP0274 Table 5)
    RESPONSE_CODES = {
        0x01: "DIGESTS",
        0x02: "CERTIFICATE",
        0x03: "CHALLENGE_AUTH",
        0x04: "VERSION",
        0x05: "CHUNK_SEND_ACK",
        0x06: "CHUNK_RESPONSE",
        0x07: "ENDPOINT_INFO",
        0x60: "MEASUREMENTS",
        0x61: "CAPABILITIES",
        0x62: "SUPPORTED_EVENT_TYPES",
        0x63: "ALGORITHMS",
        0x64: "KEY_EXCHANGE_RSP",
        0x65: "FINISH_RSP",
        0x66: "PSK_EXCHANGE_RSP",
        0x67: "PSK_FINISH_RSP",
        0x68: "HEARTBEAT_ACK",
        0x69: "KEY_UPDATE_ACK",
        0x6A: "ENCAPSULATED_REQUEST",
        0x6B: "ENCAPSULATED_RESPONSE_ACK",
        0x6C: "END_SESSION_ACK",
        0x6D: "CSR",
        0x6E: "SET_CERTIFICATE_RSP",
        0x6F: "MEASUREMENT_EXTENSION_LOG",
        0x70: "SUBSCRIBE_EVENT_TYPES_ACK",
        0x71: "EVENT_ACK",
        0x7C: "KEY_PAIR_INFO",
        0x7D: "SET_KEY_PAIR_INFO_ACK",
        0x7E: "VENDOR_DEFINED_RESPONSE",
        0x7F: "ERROR",
    }

    @staticmethod
    def parse(payload: List[int]) -> Optional[Dict]:
        """Parse SPDM message from payload"""
        if len(payload) < 2:
            return None

        result = {}

        # Byte 0: SPDM version (major [7:4], minor [3:0])
        version_byte = payload[0]
        result["version_major"] = (version_byte >> 4) & 0x0F
        result["version_minor"] = version_byte & 0x0F
        result["version"] = f"{result['version_major']}.{result['version_minor']}"

        # Byte 1: Request/Response code
        code = payload[1]
        result["code"] = code

        # Determine if request or response
        if code in SPDMParser.REQUEST_CODES:
            result["msg_direction"] = "Request"
            result["msg_name"] = SPDMParser.REQUEST_CODES[code]
        elif code in SPDMParser.RESPONSE_CODES:
            result["msg_direction"] = "Response"
            result["msg_name"] = SPDMParser.RESPONSE_CODES[code]
        else:
            result["msg_direction"] = "Unknown"
            result["msg_name"] = f"Unknown (0x{code:02X})"

        # Byte 2: Param1 (if present)
        if len(payload) > 2:
            result["param1"] = payload[2]

        # Byte 3: Param2 (if present)
        if len(payload) > 3:
            result["param2"] = payload[3]

        # Remaining data
        if len(payload) > 4:
            result["data"] = payload[4:]
            result["data_len"] = len(result["data"])
        else:
            result["data"] = []
            result["data_len"] = 0

        return result


class MCTPParser:
    """Parser for MCTP packets"""

    # Message type definitions (DSP0236 Table 3)
    MESSAGE_TYPES = {
        0x00: "MCTP Control Message",
        0x01: "PLDM",
        0x02: "NC-SI over MCTP",
        0x03: "Ethernet over MCTP",
        0x04: "NVMe-MI over MCTP",
        0x05: "SPDM over MCTP",
        0x06: "SECDED over MCTP",
        0x07: "CXL FM API over MCTP",
        0x08: "CXL CCI over MCTP",
    }

    @staticmethod
    def parse(data: List[int]) -> dict:
        """Parse MCTP packet and return structured data"""
        if len(data) < 4:
            raise ValueError(
                "Packet too short - minimum 4 bytes required for MCTP header"
            )

        result = {}

        # Byte 0: Destination EID
        result["dest_eid"] = data[0]

        # Byte 1: Header Version [7:4], Reserved [3:0]
        byte1 = data[1]
        result["header_version"] = (byte1 >> 4) & 0x0F
        result["rsvd"] = byte1 & 0x0F

        # Byte 2: Source EID
        result["src_eid"] = data[2]

        # Byte 3: SOM, EOM, Pkt_Seq, TO, Msg_Tag
        byte3 = data[3]
        result["som"] = bool(byte3 & 0x80)  # Start of Message
        result["eom"] = bool(byte3 & 0x40)  # End of Message
        result["pkt_seq"] = (byte3 >> 4) & 0x03  # Packet sequence number
        result["to"] = bool(byte3 & 0x08)  # Tag Owner
        result["msg_tag"] = byte3 & 0x07  # Message Tag

        # Message body starts at byte 4
        if len(data) > 4:
            # Byte 4: IC [7], Message Type [6:0]
            byte4 = data[4]
            result["ic"] = bool(byte4 & 0x80)  # Integrity Check
            result["msg_type"] = byte4 & 0x7F
            result["msg_type_name"] = MCTPParser.MESSAGE_TYPES.get(
                result["msg_type"],
                (
                    f"Vendor Defined (0x{result['msg_type']:02X})"
                    if result["msg_type"] >= 0x7E
                    else f"Reserved (0x{result['msg_type']:02X})"
                ),
            )

            # Payload starts at byte 5
            if len(data) > 5:
                result["payload"] = data[5:]
                result["payload_len"] = len(result["payload"])

                # Parse SPDM if message type is 0x05
                if result["msg_type"] == 0x05 and result["payload_len"] > 0:
                    result["spdm"] = SPDMParser.parse(result["payload"])
            else:
                result["payload"] = []
                result["payload_len"] = 0

        return result


def calculate_pec(data: List[int]) -> int:
    """
    Calculate SMBus PEC (Packet Error Code) using CRC-8.

    SMBus uses CRC-8 with polynomial 0x07 (x^8 + x^2 + x + 1).
    Initial value is 0x00.
    """
    crc = 0x00
    polynomial = 0x07

    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 0x80:
                crc = (crc << 1) ^ polynomial
            else:
                crc = crc << 1
            crc &= 0xFF

    return crc


def parse_smbus_header(data: List[int], expect_pec: bool = False) -> Optional[Dict]:
    """
    Parse SMBus/I2C transport binding header (DSP0237).
    Returns parsed transport info or None if not valid SMBus format.
    """
    if len(data) < 3:
        return None

    # Check for MCTP over SMBus command code (0x0F)
    if data[0] != 0x0F:
        return None

    result = {}
    result["cmd_code"] = data[0]
    result["byte_count"] = data[1]

    expected_total = 2 + result["byte_count"]
    if expect_pec:
        expected_total += 1

    if len(data) < expected_total:
        return None

    if expect_pec and len(data) >= expected_total:
        result["pec_received"] = data[expected_total - 1]
        # Calculate expected PEC
        pec_data = data[: expected_total - 1]
        result["pec_calculated"] = calculate_pec(pec_data)
        result["pec_valid"] = result["pec_received"] == result["pec_calculated"]
        data_end = expected_total - 1
    else:
        result["pec_received"] = None
        result["pec_calculated"] = None
        result["pec_valid"] = None
        data_end = min(len(data), expected_total)

    # SMBus-specific headers start at byte 2
    if result["byte_count"] < 5:
        return None

    offset = 2
    result["source_slave_addr"] = data[offset]
    offset += 1

    # MCTP reserved + header version
    byte_hdr = data[offset]
    result["mctp_reserved"] = (byte_hdr >> 4) & 0x0F
    result["mctp_hdr_version"] = byte_hdr & 0x0F
    offset += 1

    # Destination and Source EIDs
    result["dest_eid"] = data[offset]
    offset += 1
    result["src_eid"] = data[offset]
    offset += 1

    # MCTP packet starts here (SOM/EOM byte)
    result["mctp_offset"] = offset
    result["mctp_data"] = data[offset:data_end]

    return result


# ==========================================================================
# MCTP MESSAGE ASSEMBLER
# ==========================================================================


class MCTPAssembler:
    """Assembles fragmented MCTP messages"""

    def __init__(self):
        # Key: (src_eid, dest_eid, msg_tag)
        # Value: {'fragments': [], 'expected_seq': int, 'complete': bool}
        self.sessions = {}

    def add_fragment(self, mctp_data: Dict) -> Optional[Dict]:
        """
        Add a fragment and return complete message if EOM is reached.
        Returns None if message is incomplete.
        """
        key = (mctp_data["src_eid"], mctp_data["dest_eid"], mctp_data["msg_tag"])

        # SOM - start new session
        if mctp_data["som"]:
            self.sessions[key] = {
                "fragments": [mctp_data],
                "expected_seq": (mctp_data["pkt_seq"] + 1) % 4,
                "src_eid": mctp_data["src_eid"],
                "dest_eid": mctp_data["dest_eid"],
                "msg_tag": mctp_data["msg_tag"],
                "msg_type": mctp_data.get("msg_type"),
                "msg_type_name": mctp_data.get("msg_type_name"),
            }

            # Single packet message (SOM+EOM)
            if mctp_data["eom"]:
                session = self.sessions.pop(key)
                return self._assemble_session(session)

            return None

        # Middle or end fragment
        if key not in self.sessions:
            # Fragment without SOM - ignore or could be error
            return None

        session = self.sessions[key]

        # Check sequence number
        if mctp_data["pkt_seq"] != session["expected_seq"]:
            # Sequence error - drop session
            del self.sessions[key]
            return None

        session["fragments"].append(mctp_data)
        session["expected_seq"] = (mctp_data["pkt_seq"] + 1) % 4

        # EOM - assemble complete message
        if mctp_data["eom"]:
            complete_session = self.sessions.pop(key)
            return self._assemble_session(complete_session)

        return None

    def _assemble_session(self, session: Dict) -> Dict:
        """Assemble fragments into complete message"""
        # Combine all payloads
        complete_payload = []
        for frag in session["fragments"]:
            if "payload" in frag and frag["payload"]:
                complete_payload.extend(frag["payload"])

        result = {
            "src_eid": session["src_eid"],
            "dest_eid": session["dest_eid"],
            "msg_tag": session["msg_tag"],
            "msg_type": session["msg_type"],
            "msg_type_name": session["msg_type_name"],
            "payload": complete_payload,
            "payload_len": len(complete_payload),
            "fragment_count": len(session["fragments"]),
        }

        # Parse SPDM if applicable
        if session["msg_type"] == 0x05 and complete_payload:
            result["spdm"] = SPDMParser.parse(complete_payload)

        return result


# ==========================================================================
# BEAGLE DEVICE FUNCTIONS
# ==========================================================================


def find_and_connect():
    """Find and connect to the first available Beagle device."""
    print("Searching for Beagle devices...")

    # Update device list
    device_state.update_device_list()

    with device_state.lock:
        if not device_state.available_devices:
            print("Error: No Beagle devices found!")
            sys.exit(1)

        print(f"Found {len(device_state.available_devices)} device(s)")

        # Find the first available (not in-use) device
        device_idx = None
        for i, dev in enumerate(device_state.available_devices):
            if not dev["in_use"]:
                device_idx = i
                print(
                    f"Connecting to device on port {dev['port']} (S/N: {dev['unique_id']:04d}-{dev['unique_id'] % 1000000:06d})"
                )
                break
            else:
                print(f"Port {dev['port']} is in use")

        if device_idx is None:
            print("Error: All devices are in use!")
            sys.exit(1)

        # Open the device
        device_port = device_state.available_devices[device_idx]["port"]
        beagle = bg_open(device_port)
        if beagle <= 0:
            print(f"Error: Unable to open Beagle device on port {device_port}")
            print(f"Error code = {beagle}")
            sys.exit(1)

        print(f"Successfully opened Beagle device on port {device_port}")

        # Store device state
        device_state.current_device_index = device_idx
        device_state.beagle_handle = beagle

    return beagle


def configure_device(beagle, samplerate_khz=10000, timeout_ms=500, latency_ms=200):
    """Configure the Beagle device for I2C monitoring."""
    # Store settings in device_state
    with device_state.lock:
        device_state.samplerate_khz = samplerate_khz
        device_state.timeout_ms = timeout_ms
        device_state.latency_ms = latency_ms

    # Set sampling rate
    samplerate = bg_samplerate(beagle, samplerate_khz)
    if samplerate < 0:
        print(f"Error setting sample rate: {bg_status_string(samplerate)}")
        sys.exit(1)
    print(f"Sample rate set to {samplerate} kHz")

    with device_state.lock:
        device_state.samplerate_khz = samplerate

    # Set idle timeout
    bg_timeout(beagle, timeout_ms)
    print(f"Idle timeout set to {timeout_ms} ms")

    # Set latency
    bg_latency(beagle, latency_ms)
    print(f"Latency set to {latency_ms} ms")

    # Disable pullups and target power (passive monitoring)
    bg_i2c_pullup(beagle, BG_I2C_PULLUP_OFF)
    bg_target_power(beagle, BG_TARGET_POWER_OFF)

    # Get host interface speed
    if bg_host_ifce_speed(beagle):
        print("Host interface: high speed")
    else:
        print("Host interface: full speed")


# ==========================================================================
# MONITORING FUNCTIONS
# ==========================================================================


def monitor_i2c(beagle, args, stop_event, max_packet_len=256):
    """Monitor and print I2C transactions with optional MCTP/SPDM parsing."""
    # Calculate timing size
    timing_size = bg_bit_timing_size(BG_PROTOCOL_I2C, max_packet_len)

    # Get sample rate for timestamp conversion
    samplerate_khz = bg_samplerate(beagle, 0)

    # Enable I2C capture
    if bg_enable(beagle, BG_PROTOCOL_I2C) != BG_OK:
        add_display_line("Error: Could not enable I2C capture!")
        sys.exit(1)

    # Display about text
    add_display_line("")
    for line in ABOUT_TEXT:
        add_display_line(line)
    add_display_line("")

    add_display_line("=" * 80)
    add_display_line("I2C Monitoring Started - Type 'help' for commands")
    add_display_line("=" * 80)

    mode = display_mode.get_mode()
    if mode == "spdm":
        add_display_line("Mode: SPDM")
        add_display_line("      Hides non-SPDM traffic")
    elif mode == "mctp":
        add_display_line("Mode: MCTP")
        add_display_line("      Hides non-MCTP traffic")
    elif mode == "smbus":
        add_display_line("Mode: SMBus")
        if display_mode.validate_pec:
            add_display_line("      PEC validation enabled")
    else:
        add_display_line("Mode: Raw I2C")

    add_display_line("=" * 80)
    add_display_line("")

    # Allocate buffers
    data_in = array_u16(max_packet_len)
    timing = array_u32(timing_size)

    packet_count = 0
    mctp_assembler = MCTPAssembler()

    try:
        while not stop_event.is_set():
            # Read I2C transaction
            (
                count,
                status,
                time_sop,
                time_duration,
                time_dataoffset,
                data_in,
                timing,
            ) = bg_i2c_read_bit_timing(beagle, data_in, timing)

            # Convert timestamp to nanoseconds
            time_sop_ns = (time_sop * 1000) // (samplerate_khz // 1000)

            # Skip if no data
            if count <= 0:
                if count < 0:
                    add_display_line(f"Error reading I2C data: {count}")
                    break
                continue

            packet_count += 1

            # Update statistics - count this as a command
            with statistics.lock:
                statistics.commands_sent += 1

            # Extract raw data bytes (strip NACK bits)
            i2c_addr = None
            i2c_rw = None
            offset = 0
            has_nack = False

            # Get address if present
            if not (status & BG_READ_ERR_MIDDLE_OF_PACKET) and count >= 1:
                nack = data_in[0] & BG_I2C_MONITOR_NACK
                has_nack = bool(nack)
                if count == 1 or (data_in[0] & 0xF9) != 0xF0 or nack:
                    # 7-bit address
                    i2c_addr = int(data_in[0] & 0xFF) >> 1
                    i2c_rw = "R" if (data_in[0] & 0x01) else "W"
                    offset = 1
                else:
                    # 10-bit address
                    i2c_addr = ((data_in[0] << 7) & 0x300) | (data_in[1] & 0xFF)
                    i2c_rw = "R" if (data_in[0] & 0x01) else "W"
                    offset = 2

            # Count ACKs/NAKs in data bytes
            for i in range(offset, count):
                if data_in[i] & BG_I2C_MONITOR_NACK:
                    has_nack = True

            # Update ACK/NAK statistics
            with statistics.lock:
                if has_nack:
                    statistics.naks_received += 1
                else:
                    statistics.acks_received += 1

            # Extract payload bytes
            raw_data = [int(data_in[i] & 0xFF) for i in range(offset, count)]

            # Update byte count
            with statistics.lock:
                statistics.total_bytes += len(raw_data)

            # Capture raw binary data with metadata for replay
            full_transaction = [int(data_in[i] & 0xFF) for i in range(count)]
            binary_capture.add_transaction(
                time_sop_ns, full_transaction, i2c_addr, i2c_rw, raw_data, status
            )

            # Get current display mode
            mode = display_mode.get_mode()

            # Process based on mode
            if mode in ["smbus", "mctp", "spdm"] and raw_data:
                smbus_result = parse_smbus_header(raw_data, display_mode.validate_pec)

                if smbus_result and mode in ["mctp", "spdm"]:
                    # Parse MCTP
                    try:
                        # Create synthetic MCTP packet
                        synthetic_mctp = [
                            smbus_result["dest_eid"],
                            (smbus_result["mctp_hdr_version"] << 4)
                            | smbus_result["mctp_reserved"],
                            smbus_result["src_eid"],
                        ] + smbus_result["mctp_data"]

                        mctp_parsed = MCTPParser.parse(synthetic_mctp)

                        # Handle MCTP assembly
                        complete_msg = mctp_assembler.add_fragment(mctp_parsed)

                        if complete_msg is None:
                            # Incomplete fragment - hide in mctp/spdm mode
                            continue

                        # Complete message assembled
                        mctp_parsed = complete_msg

                        # Display based on mode
                        if mode == "spdm" and "spdm" in mctp_parsed:
                            print_spdm_message(
                                time_sop_ns, i2c_addr, i2c_rw, mctp_parsed
                            )
                        elif mode == "mctp":
                            print_mctp_message(
                                time_sop_ns, i2c_addr, i2c_rw, smbus_result, mctp_parsed
                            )

                    except Exception as e:
                        add_display_line(
                            f"[{time_sop_ns:12d} ns] MCTP Parse Error: {e}"
                        )

                elif smbus_result and mode == "smbus":
                    # SMBus mode - show all SMBus traffic
                    print_smbus_message(
                        time_sop_ns, i2c_addr, i2c_rw, smbus_result, raw_data
                    )

                # In mctp/spdm mode, hide non-MCTP traffic

            elif mode == "raw":
                # Raw I2C mode
                print_raw_i2c(time_sop_ns, i2c_addr, i2c_rw, raw_data, status)

    except KeyboardInterrupt:
        stop_event.set()
        add_display_line("")
        add_display_line("Capture stopped by user")
        add_display_line(f"Total packets captured: {packet_count}")

    finally:
        # Disable capture
        bg_disable(beagle)
        add_display_line(f"Total packets captured: {packet_count}")


def print_raw_i2c(timestamp, addr, rw, data, status):
    """Print raw I2C transaction"""
    line = f"[{timestamp:12d} ns] "

    if addr is not None:
        line += f"[S] <0x{addr:02X}:{rw}> "

    if data:
        hex_str = " ".join(f"0x{b:02X}" for b in data)
        line += hex_str + " "

    if not (status & BG_READ_I2C_NO_STOP):
        line += "[P]"

    add_display_line(line.rstrip())


def print_smbus_message(timestamp, addr, rw, smbus_info, raw_data):
    """Print SMBus message with PEC info"""
    # Add X prefix if PEC validation is enabled and invalid
    prefix = ""
    if smbus_info["pec_valid"] is not None and not smbus_info["pec_valid"]:
        prefix = "X "

    line = f"[{timestamp:12d} ns] {prefix}SMBus: "
    line += f"Cmd=0x{smbus_info['cmd_code']:02X} Len={smbus_info['byte_count']} "

    if smbus_info["pec_valid"] is not None:
        if smbus_info["pec_valid"]:
            line += f"PEC=✓ "
        else:
            line += f"PEC=✗ "

    # Show data
    hex_str = " ".join(f"{b:02X}" for b in raw_data[:16])
    if len(raw_data) > 16:
        hex_str += "..."
    line += f"[{hex_str}]"

    add_display_line(line)


def print_mctp_message(timestamp, addr, rw, smbus_info, mctp_parsed):
    """Print MCTP message with highlighting"""
    som_marker = "🟢 SOM" if mctp_parsed.get("som") else ""
    eom_marker = "🔴 EOM" if mctp_parsed.get("eom") else ""
    markers = f"{som_marker} {eom_marker}".strip()

    line = f"[{timestamp:12d} ns] MCTP: "
    if markers:
        line += f"{markers} "

    line += f"EID {mctp_parsed['src_eid']:02X}→{mctp_parsed['dest_eid']:02X} "
    line += (
        f"Seq={mctp_parsed.get('pkt_seq', '?')} Tag={mctp_parsed.get('msg_tag', '?')} "
    )
    line += f"Type=0x{mctp_parsed.get('msg_type', 0):02X} ({mctp_parsed.get('msg_type_name', 'Unknown')})"

    # Show fragment count if assembled
    if "fragment_count" in mctp_parsed and mctp_parsed["fragment_count"] > 1:
        line += f" [{mctp_parsed['fragment_count']} fragments]"

    add_display_line(line)


def print_spdm_message(timestamp, addr, rw, mctp_parsed):
    """Print SPDM message with request/response highlighting"""
    spdm = mctp_parsed.get("spdm")
    if not spdm:
        return

    # Highlight request vs response
    if spdm["msg_direction"] == "Request":
        direction_marker = "📤 REQ"
    elif spdm["msg_direction"] == "Response":
        direction_marker = "📥 RSP"
    else:
        direction_marker = "❓"

    line = f"[{timestamp:12d} ns] SPDM: {direction_marker} "

    line += f"EID {mctp_parsed['src_eid']:02X}→{mctp_parsed['dest_eid']:02X} "
    if "fragment_count" in mctp_parsed and mctp_parsed["fragment_count"] > 1:
        line += f"[{mctp_parsed['fragment_count']} frags] "

    line += f"v{spdm['version']} {spdm['msg_name']} "

    if "param1" in spdm:
        line += f"P1=0x{spdm['param1']:02X} "
    if "param2" in spdm:
        line += f"P2=0x{spdm['param2']:02X} "

    if spdm.get("data_len", 0) > 0:
        line += f"+{spdm['data_len']}B"

    add_display_line(line.rstrip())


# ==========================================================================
# MAIN
# ==========================================================================


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="I2C/SMBus Monitor with MCTP/SPDM Support",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                                    # Raw I2C monitoring
  %(prog)s --smbus --validate-pec             # SMBus with PEC validation
  %(prog)s --mctp                             # MCTP packet monitoring
  %(prog)s --spdm                             # SPDM message monitoring
  %(prog)s --samplerate 5000 --timeout 1000   # Custom sample rate and timeout
        """,
    )

    # Protocol mode arguments
    parser.add_argument(
        "--smbus", action="store_true", help="Treat all packets as having SMBus header"
    )
    parser.add_argument(
        "--validate-pec",
        action="store_true",
        help="Validate PEC and mark invalid packets with X (requires --smbus)",
    )
    parser.add_argument(
        "--mctp",
        action="store_true",
        help="MCTP mode (implies --smbus --validate-pec, hides non-MCTP traffic)",
    )
    parser.add_argument(
        "--mctp-no-partial",
        action="store_true",
        help="Hide MCTP fragments until EOM (requires --mctp or --spdm)",
    )
    parser.add_argument(
        "--spdm",
        action="store_true",
        help="SPDM mode (implies --mctp, hides non-SPDM traffic)",
    )

    # Device configuration arguments
    parser.add_argument(
        "--samplerate",
        type=int,
        default=10000,
        metavar="KHZ",
        help="Sample rate in kHz (default: 10000)",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=500,
        metavar="MS",
        help="Idle timeout in milliseconds (default: 500)",
    )
    parser.add_argument(
        "--latency",
        type=int,
        default=200,
        metavar="MS",
        help="Latency in milliseconds (default: 200)",
    )

    args = parser.parse_args()

    # Handle argument implications and set initial display mode
    if args.spdm:
        args.mctp = True
        args.mctp_no_partial = True  # SPDM mode always waits for complete messages
        display_mode.set_mode("spdm")

    if args.mctp:
        args.smbus = True
        args.validate_pec = True
        if not args.spdm:
            display_mode.set_mode("mctp")

    if args.smbus and not args.mctp:
        display_mode.set_mode("smbus")

    # Store validate_pec in display_mode
    display_mode.validate_pec = args.validate_pec

    # Validate argument combinations
    if args.validate_pec and not args.smbus:
        parser.error("--validate-pec requires --smbus")

    if args.mctp_no_partial and not (args.mctp or args.spdm):
        parser.error("--mctp-no-partial requires --mctp or --spdm")

    # Validate configuration arguments
    if args.samplerate <= 0:
        parser.error("Sample rate must be positive")
    if args.timeout < 0:
        parser.error("Timeout must be non-negative")
    if args.latency < 0:
        parser.error("Latency must be non-negative")

    # Print about text to console before starting TUI
    print()
    for line in ABOUT_TEXT:
        print(line)
    print()

    # Find and connect to device
    beagle = find_and_connect()

    # Create stop event for coordinating threads
    stop_event = threading.Event()

    try:
        # Configure device
        print()
        configure_device(beagle, args.samplerate, args.timeout, args.latency)

        # Start monitoring in a separate thread
        monitor_thread = threading.Thread(
            target=monitor_i2c, args=(beagle, args, stop_event), daemon=True
        )
        monitor_thread.start()

        # Run command loop in main thread with curses
        curses.wrapper(command_loop_curses, stop_event)

        # Wait for monitoring thread to finish
        monitor_thread.join(timeout=2.0)

    finally:
        # Clean up
        stop_event.set()
        bg_close(beagle)
        print("Beagle device closed")


if __name__ == "__main__":
    main()
