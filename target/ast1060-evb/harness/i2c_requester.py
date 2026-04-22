#!/usr/bin/env python3
"""
Aardvark I2C Multi-Master Transceiver
Sends pre-fabricated I2C payloads as master and receives responses as slave.
Operates in multi-master mode.
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


def configure_device(handle, slave_addr=0x42, bitrate_khz=100, bus_timeout_ms=150):
    """
    Configure the Aardvark for I2C multi-master operation.

    Args:
        handle: Aardvark device handle
        slave_addr: Our slave address (default: 0x42)
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

    # Enable slave mode with our address
    # Parameters: handle, slave_addr, maxTxBytes, maxRxBytes
    # Use 0 for unlimited buffer sizes
    result = aa_i2c_slave_enable(handle, slave_addr, 0, 0)
    if result < 0:
        print(f"  Error enabling slave mode: {aa_status_string(result)}")
        sys.exit(1)
    print(f"  Slave Address: 0x{slave_addr:02X}")
    print("  Slave Mode: Enabled")

    print("\nConfiguration complete. Device ready for multi-master operation.")
    print("  Master: Can send to other devices")
    print("  Slave:  Can receive from other masters\n")


#==========================================================================
# I2C OPERATIONS
#==========================================================================

def send_payload(handle, target_addr, payload: List[int], description=""):
    """
    Send a payload as I2C master.

    Args:
        handle: Aardvark device handle
        target_addr: Target device address (7-bit)
        payload: List of bytes to send
        description: Optional description of the payload

    Returns:
        Tuple of (success, num_bytes_written)
    """
    if description:
        print(f">>> Sending: {description}")

    print(f"    Target: 0x{target_addr:02X}")
    print(f"    Length: {len(payload)} bytes")
    print(f"    Data:   {' '.join(f'{b:02X}' for b in payload)}")

    # Convert to array
    data_out = array('B', payload)

    # Write to target device
    num_written = aa_i2c_write(handle, target_addr, AA_I2C_NO_FLAGS, data_out)

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
    Poll for incoming I2C slave data (response from another master).

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
        # Data was written to us (we're the slave)
        (num_bytes, addr, data_in) = aa_i2c_slave_read(handle, max_buffer)

        if num_bytes < 0:
            print(f"    Error reading slave data: {aa_status_string(num_bytes)}")
            return False, None, []

        # Convert to list
        data_list = [data_in[i] for i in range(num_bytes)]
        return True, addr, data_list

    elif result == AA_ASYNC_I2C_WRITE:
        # Data was read from us (master read from our slave)
        num_bytes = aa_i2c_slave_write_stats(handle)
        print(f"<<< Master read {num_bytes} bytes from us")
        return False, None, []

    else:
        print(f"    Unexpected async event: {result}")
        return False, None, []


def wait_for_response(handle, timeout_ms=1000, description=""):
    """
    Wait for and display a response from another master.

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


def set_slave_response(handle, response_data: List[int]):
    """
    Set the data to send when a master reads from us.

    Args:
        handle: Aardvark device handle
        response_data: List of bytes to respond with
    """
    data_array = array('B', response_data)
    result = aa_i2c_slave_set_response(handle, data_array)
    if result < 0:
        print(f"Error setting slave response: {aa_status_string(result)}")
    else:
        print(f"Slave response buffer set ({len(response_data)} bytes)")


#==========================================================================
# TRANSACTION SEQUENCES
#==========================================================================

def execute_transaction(handle, target_addr, payload, wait_response=True,
                        response_timeout=1000, description=""):
    """
    Execute a complete transaction: send payload and optionally wait for response.

    Args:
        handle: Aardvark device handle
        target_addr: Target device address
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
    success, _ = send_payload(handle, target_addr, payload, "Request")

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
        description='Aardvark I2C Multi-Master Transceiver',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
This tool operates in multi-master mode:
  - Sends payloads as I2C master to target devices
  - Receives responses as I2C slave from other masters

Default configuration:
  - Slave address: 0x42
  - Bus speed: 100 kHz
  - Bus timeout: 150 ms
        '''
    )

    parser.add_argument('--slave-addr', type=lambda x: int(x, 0), default=0x42,
                        metavar='ADDR', help='Our slave address (default: 0x42)')
    parser.add_argument('--bitrate', type=int, default=100, metavar='KHZ',
                        help='I2C bus speed in kHz (default: 100)')
    parser.add_argument('--bus-timeout', type=int, default=150, metavar='MS',
                        help='Bus lock timeout in ms (default: 150)')
    parser.add_argument('--target', type=lambda x: int(x, 0), default=0x10,
                        metavar='ADDR', help='Target device address (default: 0x10)')
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
        configure_device(handle, args.slave_addr, args.bitrate, args.bus_timeout)

        # Define pre-fabricated payloads
        payloads = [
            {
                'name': 'MCTP SPDM GET_VERSION',
                'data': [0x0F, 0x0A, 0x85, 0x01, 0x08, 0x30, 0xC8, 0x05, 0x10, 0x84, 0x00, 0x00, 0x65],
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
                args.target,
                payload_info['data'],
                wait_response=not args.no_wait_response,
                response_timeout=args.response_timeout,
                description=payload_info['name']
            )

        print("All transactions complete.")

    except KeyboardInterrupt:
        print("\n\nInterrupted by user")

    finally:
        # Disable slave mode and close device
        aa_i2c_slave_disable(handle)
        aa_close(handle)
        print("Aardvark device closed")


if __name__ == "__main__":
    main()
