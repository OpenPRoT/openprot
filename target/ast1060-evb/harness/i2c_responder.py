#!/usr/bin/env python3
"""
Aardvark I2C Responder
Listens for I2C requests and returns a canned response.
"""

import sys
import os
import argparse
from typing import List

# Add the Aardvark API to the path
# Assumes script runs at the same level as aardvark-api-linux-x86_64-v6.00 directory
script_dir = os.path.dirname(os.path.abspath(__file__))
aardvark_lib_path = os.path.join(script_dir, 'aardvark-api-linux-x86_64-v6.00', 'python')
sys.path.insert(0, aardvark_lib_path)
from aardvark_py import *


#==========================================================================
# DEVICE MANAGEMENT
#==========================================================================

def find_and_connect():
    """Find and connect to the first available Aardvark device."""
    print("Searching for Aardvark devices...")

    # Find all attached devices
    (num, ports, unique_ids) = aa_find_devices_ext(16, 16)

    if num == 0:
        print("Error: No Aardvark devices found!")
        sys.exit(1)

    print(f"Found {num} device(s)")

    # Find the first available (not in-use) device
    device_port = None
    for i in range(num):
        port = ports[i]
        unique_id = unique_ids[i]

        if not (port & AA_PORT_NOT_FREE):
            device_port = port
            print(f"Connecting to device on port {port} (S/N: {unique_id:04d}-{unique_id % 1000000:06d})")
            break
        else:
            print(f"Port {port & ~AA_PORT_NOT_FREE} is in use")

    if device_port is None:
        print("Error: All devices are in use!")
        sys.exit(1)

    # Open the device
    handle = aa_open(device_port)
    if handle <= 0:
        print(f"Error: Unable to open Aardvark device on port {device_port}")
        print(f"Error code = {handle}")
        sys.exit(1)

    print(f"Successfully opened Aardvark device on port {device_port}")
    return handle


def configure_device(handle, own_addr=0x42, bitrate_khz=100, bus_timeout_ms=150):
    """
    Configure the Aardvark for I2C operation.

    Args:
        handle: Aardvark device handle
        own_addr: Our I2C address (default: 0x42)
        bitrate_khz: I2C bus speed in kHz (default: 100)
        bus_timeout_ms: Bus lock timeout in ms (default: 150)
    """
    print("\nConfiguring Aardvark I2C interface...")

    # Configure for I2C mode
    aa_configure(handle, AA_CONFIG_SPI_I2C)
    print("  Mode: I2C")

    # Enable I2C pullup resistors (2.2k)
    aa_i2c_pullup(handle, AA_I2C_PULLUP_BOTH)
    print("  Pullups: Enabled (both lines)")

    # Enable target power
    aa_target_power(handle, AA_TARGET_POWER_BOTH)
    print("  Target Power: Enabled")

    # Set the bitrate
    actual_bitrate = aa_i2c_bitrate(handle, bitrate_khz)
    print(f"  Bitrate: {actual_bitrate} kHz (requested: {bitrate_khz} kHz)")

    # Set the bus lock timeout
    actual_timeout = aa_i2c_bus_timeout(handle, bus_timeout_ms)
    print(f"  Bus Timeout: {actual_timeout} ms")

    # Enable target mode with our address
    # Parameters: handle, own_addr, maxTxBytes, maxRxBytes
    # Use 0 for unlimited buffer sizes
    result = aa_i2c_slave_enable(handle, own_addr, 0, 0)
    if result < 0:
        print(f"  Error enabling target mode: {aa_status_string(result)}")
        sys.exit(1)
    print(f"  Our I2C Address: 0x{own_addr:02X}")
    print("  Target Mode: Enabled")

    print("\nConfiguration complete. Device ready.")
    print(f"  Listening on address: 0x{own_addr:02X}\n")


def set_response(handle, response_data: List[int]):
    """
    Set the data to send when remote device reads from us.

    Args:
        handle: Aardvark device handle
        response_data: List of bytes to respond with
    """
    data_array = array('B', response_data)
    result = aa_i2c_slave_set_response(handle, data_array)
    if result < 0:
        print(f"Error setting response: {aa_status_string(result)}")
        sys.exit(1)

    print(f"Canned response configured ({len(response_data)} bytes):")
    print(f"  Data: {' '.join(f'{b:02X}' for b in response_data)}")


#==========================================================================
# I2C OPERATIONS
#==========================================================================

def listen_for_requests(handle, own_addr, remote_addr, response_data, timeout_ms=500, max_buffer=256, transaction_count=0):
    """
    Listen for incoming I2C requests and handle them.
    Uses MCTP multi-controller behavior: receive, then respond.

    Args:
        handle: Aardvark device handle
        own_addr: Our I2C address (needed for re-enabling target mode)
        remote_addr: Remote I2C address to send response to
        response_data: List of bytes to respond with
        timeout_ms: Timeout in milliseconds between requests
        max_buffer: Maximum buffer size for receiving data
        transaction_count: Max number of transactions (0 = infinite)

    Returns:
        Number of transactions handled
    """
    print("=" * 80)
    print("Listening for I2C requests... (Press Ctrl+C to stop)")
    print("=" * 80)
    print()

    trans_num = 0

    try:
        while True:
            # Check if we've reached transaction limit
            if transaction_count > 0 and trans_num >= transaction_count:
                print(f"\nReached transaction limit ({transaction_count})")
                break

            # Poll for async events
            result = aa_async_poll(handle, timeout_ms)

            if result == AA_ASYNC_NO_DATA:
                # No data, keep waiting
                continue

            if result == AA_ASYNC_I2C_READ:
                # Data was written to us (we're receiving a request)
                (num_bytes, addr, data_in) = aa_i2c_slave_read(handle, max_buffer)

                if num_bytes < 0:
                    print(f"Error reading data: {aa_status_string(num_bytes)}")
                    continue

                # Convert to list
                data_list = [data_in[i] for i in range(num_bytes)]

                # Display the request
                print(f"<<< Transaction #{trans_num + 1}: Request received")
                print(f"    Target Address: 0x{addr:02X} (our address)")
                print(f"    Length: {num_bytes} bytes")
                print(f"    Data:   {' '.join(f'{b:02X}' for b in data_list)}")
                print(f"    Will respond to: 0x{remote_addr:02X} (remote address)")
                print()

                # Delay before responding
                import time
                print(f"    Waiting 1 second before responding...")
                time.sleep(1.0)
                print()

                # MCTP multi-controller behavior: switch to controller and send response
                print(f">>> Switching to controller mode to send response...")

                # Disable target mode
                print(f"    Disabling target mode...")
                result = aa_i2c_slave_disable(handle)
                print(f"    aa_i2c_slave_disable returned: {result}")
                if result < 0:
                    print(f"    ERROR disabling target mode: {aa_status_string(result)}")
                    continue
                print(f"    Target mode disabled successfully")

                # Add delay to allow remote to switch to target mode
                # and bus to settle after the request transaction
                import time
                print(f"    Waiting for bus to settle and remote to switch to target mode...")
                time.sleep(0.05)  # 50ms delay

                # Write response as controller to the remote address
                response_array = array('B', response_data)
                print(f"    Preparing to write {len(response_data)} bytes to 0x{remote_addr:02X}")
                print(f"    Data: {' '.join(f'{b:02X}' for b in response_data)}")

                num_written = aa_i2c_write(handle, remote_addr, AA_I2C_NO_FLAGS, response_array)

                print(f"    aa_i2c_write returned: {num_written}")
                if num_written < 0:
                    print(f"    ERROR writing response: {aa_status_string(num_written)}")
                    print(f"    Error code: {num_written}")
                elif num_written == 0:
                    print(f"    WARNING: 0 bytes written!")
                else:
                    print(f"    SUCCESS: Wrote {num_written} bytes to 0x{remote_addr:02X}")
                print()

                # Re-enable target mode
                print(f"    Re-enabling target mode on address 0x{own_addr:02X}...")
                result = aa_i2c_slave_enable(handle, own_addr, 0, 0)
                print(f"    aa_i2c_slave_enable returned: {result}")
                if result < 0:
                    print(f"    ERROR re-enabling target mode: {aa_status_string(result)}")
                    break

                print(f">>> Switched back to target mode (listening on 0x{own_addr:02X})")
                print()

                trans_num += 1

            elif result == AA_ASYNC_I2C_WRITE:
                # This shouldn't happen in multi-controller mode, but log it if it does
                num_bytes = aa_i2c_slave_write_stats(handle)
                print(f"!!! Unexpected passive write event (num_bytes: {num_bytes})")
                print()

            else:
                print(f"Unexpected async event: {result}")

    except KeyboardInterrupt:
        print("\n\nStopped by user")

    return trans_num


#==========================================================================
# MAIN
#==========================================================================

def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description='Aardvark I2C Responder',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
This tool operates with MCTP multi-controller behavior:
  - Listens as I2C target for incoming write requests
  - Switches to I2C controller to actively send response back to requester
  - Returns to target mode for next request

Default configuration (for in-process-requester mode):
  - Responder (us): EID=0x42, I2C address=0x13
  - Requester (peer): EID=0x08, I2C address=0x10
  - Bus speed: 100 kHz
  - Bus timeout: 150 ms

Default canned response (MCTP SPDM VERSION response):
  26 0F 0A 21 01 42 08 D9 05 10 84 00 00 51
        '''
    )

    parser.add_argument('--own-addr', type=lambda x: int(x, 0), default=0x13,
                        metavar='ADDR', help='Our I2C address (default: 0x13)')
    parser.add_argument('--remote-addr', type=lambda x: int(x, 0), default=0x10,
                        metavar='ADDR', help='Remote I2C address to respond to (default: 0x10)')
    parser.add_argument('--own-eid', type=lambda x: int(x, 0), default=0x42,
                        metavar='EID', help='Our MCTP endpoint ID (default: 0x42)')
    parser.add_argument('--remote-eid', type=lambda x: int(x, 0), default=0x08,
                        metavar='EID', help='Remote MCTP endpoint ID (default: 0x08)')
    parser.add_argument('--bitrate', type=int, default=100, metavar='KHZ',
                        help='I2C bus speed in kHz (default: 100)')
    parser.add_argument('--bus-timeout', type=int, default=150, metavar='MS',
                        help='Bus lock timeout in ms (default: 150)')
    parser.add_argument('--poll-timeout', type=int, default=500, metavar='MS',
                        help='Poll timeout in ms (default: 500)')
    parser.add_argument('--count', type=int, default=0, metavar='N',
                        help='Number of transactions to handle (0=infinite, default: 0)')
    parser.add_argument('--response', type=str, default=None, metavar='HEX',
                        help='Custom response as hex string (e.g., "01 02 03")')

    args = parser.parse_args()

    # Default canned response (MCTP SPDM VERSION response)
    # Captured from actual Rust responder on I2C bus:
    # <0x13:W> 0x26 0x0F 0x0A 0x21 0x01 0x42 0x08 0xD9 0x05 0x10 0x84 0x00 0x00 0x51
    #
    # Format: [SrcI2C][CMD][ByteCount][DestI2C][MCTP_Ver][DestEID][SrcEID][Flags][MsgType][SPDM][PEC]
    # Responder (us): EID=0x42, I2C=0x13
    # Requester (peer): EID=0x08, I2C=0x10
    # Src I2C: 0x26 = 0x13 << 1 (no read bit in this position)
    # Dest I2C: 0x21 = 0x10 << 1 | 1 (read bit set)
    # MCTP: Dest EID=0x42, Source EID=0x08 (note: different from expected!)
    # PEC=0x51 verified from actual capture
    canned_response = [
        0x26, 0x0F, 0x0A, 0x21, 0x01, 0x42, 0x08, 0xD9,
        0x05, 0x10, 0x84, 0x00, 0x00, 0x51,
    ]

    # Parse custom response if provided
    if args.response:
        try:
            hex_bytes = args.response.replace(',', ' ').split()
            canned_response = [int(b, 16) for b in hex_bytes if b.strip()]
            print(f"Using custom response ({len(canned_response)} bytes)")
        except ValueError as e:
            print(f"Error parsing custom response: {e}")
            sys.exit(1)

    # Find and connect to device
    handle = find_and_connect()

    try:
        # Configure device
        configure_device(handle, args.own_addr, args.bitrate, args.bus_timeout)

        # Display configuration
        print()
        print("MCTP Responder Configuration:")
        print(f"  Our I2C Address:    0x{args.own_addr:02X}")
        print(f"  Our EID:            0x{args.own_eid:02X}")
        print(f"  Remote I2C Address: 0x{args.remote_addr:02X}")
        print(f"  Remote EID:         0x{args.remote_eid:02X}")
        print()
        print(f"Active response configured ({len(canned_response)} bytes):")
        print(f"  Data: {' '.join(f'{b:02X}' for b in canned_response)}")
        print(f"  Will be sent as I2C controller after receiving requests")
        print()

        # Listen for requests
        num_transactions = listen_for_requests(
            handle,
            args.own_addr,
            args.remote_addr,
            canned_response,
            args.poll_timeout,
            transaction_count=args.count
        )

        print(f"\nTotal transactions handled: {num_transactions}")

    except KeyboardInterrupt:
        print("\n\nInterrupted by user")

    finally:
        # Disable target mode and close device
        aa_i2c_slave_disable(handle)
        aa_close(handle)
        print("Aardvark device closed")


if __name__ == "__main__":
    main()
