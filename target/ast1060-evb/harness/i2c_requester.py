#!/usr/bin/env python3
"""
Aardvark I2C Multi-Controller Transceiver
Sends pre-fabricated I2C payloads as controller and receives responses as target.
Operates in multi-controller mode.
"""

import sys
import os
import time
import argparse
from typing import List, Tuple

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
    Configure the Aardvark for I2C multi-controller operation.

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

    print("\nConfiguration complete. Device ready for multi-controller operation.")
    print("  Controller: Can send to other devices")
    print("  Target:     Can receive from other controllers\n")


#==========================================================================
# I2C OPERATIONS
#==========================================================================

def send_payload(handle, remote_addr, payload: List[int], description=""):
    """
    Send a payload as I2C controller.

    Args:
        handle: Aardvark device handle
        remote_addr: Remote device address (7-bit)
        payload: List of bytes to send
        description: Optional description of the payload

    Returns:
        Tuple of (success, num_bytes_written)
    """
    if description:
        print(f">>> Sending: {description}")

    print(f"    Remote: 0x{remote_addr:02X}")
    print(f"    Length: {len(payload)} bytes")
    print(f"    Data:   {' '.join(f'{b:02X}' for b in payload)}")

    # Convert to array
    data_out = array('B', payload)

    # Write to remote device
    num_written = aa_i2c_write(handle, remote_addr, AA_I2C_NO_FLAGS, data_out)

    if num_written < 0:
        print(f"    Status: ERROR - {aa_status_string(num_written)}")
        return False, 0
    elif num_written != len(payload):
        print(f"    Status: PARTIAL - Wrote {num_written}/{len(payload)} bytes")
        return False, num_written
    else:
        print(f"    Status: SUCCESS - {num_written} bytes written")
        return True, num_written


def poll_for_response(handle, timeout_ms=1000, max_buffer=256):
    """
    Poll for incoming I2C target data (response from another controller).

    Args:
        handle: Aardvark device handle
        timeout_ms: Timeout in milliseconds
        max_buffer: Maximum buffer size for receiving data

    Returns:
        Tuple of (has_data, addr, data_bytes)
    """
    # Poll for async events
    result = aa_async_poll(handle, timeout_ms)

    if result == AA_ASYNC_NO_DATA:
        return False, None, []

    if result == AA_ASYNC_I2C_READ:
        # Data was written to us (we're the target)
        (num_bytes, addr, data_in) = aa_i2c_slave_read(handle, max_buffer)

        if num_bytes < 0:
            print(f"    Error reading data: {aa_status_string(num_bytes)}")
            return False, None, []

        # Convert to list
        data_list = [data_in[i] for i in range(num_bytes)]
        return True, addr, data_list

    elif result == AA_ASYNC_I2C_WRITE:
        # Data was read from us (controller read from our target)
        num_bytes = aa_i2c_slave_write_stats(handle)
        print(f"<<< Controller read {num_bytes} bytes from us")
        return False, None, []

    else:
        print(f"    Unexpected async event: {result}")
        return False, None, []


def wait_for_response(handle, timeout_ms=1000, description=""):
    """
    Wait for and display a response from another controller.

    Args:
        handle: Aardvark device handle
        timeout_ms: Timeout in milliseconds
        description: Optional description

    Returns:
        Tuple of (success, data_bytes)
    """
    if description:
        print(f"<<< Waiting for response: {description}")
    else:
        print(f"<<< Waiting for response...")

    print(f"    Timeout: {timeout_ms} ms")

    has_data, addr, data = poll_for_response(handle, timeout_ms)

    if not has_data:
        print(f"    Status: NO DATA (timeout)")
        return False, []

    print(f"    From:   0x{addr:02X}")
    print(f"    Length: {len(data)} bytes")
    print(f"    Data:   {' '.join(f'{b:02X}' for b in data)}")
    print(f"    Status: SUCCESS")

    return True, data


def set_response(handle, response_data: List[int]):
    """
    Set the data to send when a controller reads from us.

    Args:
        handle: Aardvark device handle
        response_data: List of bytes to respond with
    """
    data_array = array('B', response_data)
    result = aa_i2c_slave_set_response(handle, data_array)
    if result < 0:
        print(f"Error setting response: {aa_status_string(result)}")
    else:
        print(f"Response buffer set ({len(response_data)} bytes)")


#==========================================================================
# TRANSACTION SEQUENCES
#==========================================================================

def execute_transaction(handle, remote_addr, payload, wait_response=True,
                        response_timeout=1000, description=""):
    """
    Execute a complete transaction: send payload and optionally wait for response.

    Args:
        handle: Aardvark device handle
        remote_addr: Remote device address
        payload: Payload to send
        wait_response: Whether to wait for a response
        response_timeout: Response timeout in ms
        description: Transaction description

    Returns:
        Tuple of (send_success, response_data)
    """
    print("=" * 80)
    if description:
        print(f"Transaction: {description}")
    print("=" * 80)

    # Send the payload
    success, _ = send_payload(handle, remote_addr, payload, "Request")

    response_data = []
    if success and wait_response:
        print()
        # Wait a bit for the device to process
        time.sleep(0.05)

        # Wait for response
        _, response_data = wait_for_response(handle, response_timeout, "Response")

    print("=" * 80)
    print()

    return success, response_data


#==========================================================================
# MAIN
#==========================================================================

def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description='Aardvark I2C Multi-Controller Transceiver',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
This tool operates in multi-controller mode:
  - Sends payloads as I2C controller to remote devices
  - Receives responses as I2C target from other controllers

Default configuration:
  - Our I2C address: 0x10
  - Remote I2C address: 0x13
  - Bus speed: 100 kHz
  - Bus timeout: 150 ms
        '''
    )

    parser.add_argument('--own-addr', type=lambda x: int(x, 0), default=0x10,
                        metavar='ADDR', help='Our I2C address (default: 0x10)')
    parser.add_argument('--own-eid', type=lambda x: int(x, 0), default=0x08,
                        metavar='EID', help='Our MCTP endpoint ID (default: 0x08)')
    parser.add_argument('--remote-addr', type=lambda x: int(x, 0), default=0x13,
                        metavar='ADDR', help='Remote device address (default: 0x13)')
    parser.add_argument('--remote-eid', type=lambda x: int(x, 0), default=0x42,
                        metavar='EID', help='Remote MCTP endpoint ID (default: 0x42)')
    parser.add_argument('--bitrate', type=int, default=100, metavar='KHZ',
                        help='I2C bus speed in kHz (default: 100)')
    parser.add_argument('--bus-timeout', type=int, default=150, metavar='MS',
                        help='Bus lock timeout in ms (default: 150)')
    parser.add_argument('--response-timeout', type=int, default=1000, metavar='MS',
                        help='Response timeout in ms (default: 1000)')
    parser.add_argument('--no-wait-response', action='store_true',
                        help='Do not wait for response after sending')
    parser.add_argument('--interactive', action='store_true',
                        help='Interactive mode - prompt between transactions')

    args = parser.parse_args()

    # Find and connect to device
    handle = find_and_connect()

    try:
        # Configure device
        configure_device(handle, args.own_addr, args.bitrate, args.bus_timeout)

        # Display configuration
        print()
        print("MCTP Requester Configuration:")
        print(f"  Our I2C Address:    0x{args.own_addr:02X}")
        print(f"  Our EID:            0x{args.own_eid:02X}")
        print(f"  Remote I2C Address: 0x{args.remote_addr:02X}")
        print(f"  Remote EID:         0x{args.remote_eid:02X}")
        print()

        # Build MCTP SPDM GET_VERSION payload using configured addresses
        # Format: [CMD][ByteCount][SrcI2C][MCTP_Ver][DestEID][SrcEID][Flags][MsgType][SPDM][PEC]
        src_i2c_with_read = (args.own_addr << 1) | 1  # Source I2C with read bit

        # Build the frame (without I2C destination address prefix)
        frame = [
            0x0F,                    # Command code (MCTP)
            0x0A,                    # Byte count (10 bytes)
            src_i2c_with_read,       # Source I2C address with read bit
            0x01,                    # MCTP version
            args.remote_eid,         # Destination EID (responder)
            args.own_eid,            # Source EID (us)
            0xC8,                    # Flags (SOM=1, EOM=1, Seq=0, TO=1, Tag=0)
            0x05,                    # Message type (SPDM)
            0x10,                    # SPDM GET_VERSION
            0x84,                    # SPDM request code
            0x00,                    # Param1
            0x00,                    # Param2
        ]

        # Calculate PEC (includes destination I2C address)
        # For PEC calculation, prepend the destination I2C write address
        dest_i2c_write = args.remote_addr << 1  # Destination I2C write address
        frame_with_dest = [dest_i2c_write] + frame

        # Calculate CRC-8 PEC
        crc = 0
        for byte in frame_with_dest:
            crc ^= byte
            for _ in range(8):
                if crc & 0x80:
                    crc = (crc << 1) ^ 0x07
                else:
                    crc <<= 1
                crc &= 0xFF

        frame.append(crc)  # Append PEC to payload

        print(f"Generated GET_VERSION payload:")
        print(f"  Source I2C:    0x{args.own_addr:02X} (frame: 0x{src_i2c_with_read:02X})")
        print(f"  Source EID:    0x{args.own_eid:02X}")
        print(f"  Dest I2C:      0x{args.remote_addr:02X}")
        print(f"  Dest EID:      0x{args.remote_eid:02X}")
        print(f"  PEC:           0x{crc:02X}")
        print()

        # Define pre-fabricated payloads
        payloads = [
            {
                'name': 'MCTP SPDM GET_VERSION',
                'data': frame,
                'description': 'MCTP over SMBus: SPDM GET_VERSION request'
            },
            # Add more payloads here as needed
        ]

        # Execute transactions
        for idx, payload_info in enumerate(payloads):
            if args.interactive and idx > 0:
                input("\nPress Enter to send next transaction...")

            execute_transaction(
                handle,
                args.remote_addr,
                payload_info['data'],
                wait_response=not args.no_wait_response,
                response_timeout=args.response_timeout,
                description=payload_info['name']
            )

        print("All transactions complete.")

    except KeyboardInterrupt:
        print("\n\nInterrupted by user")

    finally:
        # Disable target mode and close device
        aa_i2c_slave_disable(handle)
        aa_close(handle)
        print("Aardvark device closed")


if __name__ == "__main__":
    main()
