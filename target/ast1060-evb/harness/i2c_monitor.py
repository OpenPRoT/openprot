#!/usr/bin/env python3
"""
I2C/SMBus Monitor for Beagle Bus Analyzer with MCTP/SPDM Support
Automatically connects to the first available device and listens for I2C transactions.
Supports MCTP packet assembly and SPDM message parsing.
"""

import sys
import os
import argparse
from typing import List, Optional, Dict
from collections import defaultdict

# Add the Beagle API to the path
# Assumes script runs at the same level as beagle-api-linux-x86_64-v6.00 directory
script_dir = os.path.dirname(os.path.abspath(__file__))
beagle_lib_path = os.path.join(script_dir, 'beagle-api-linux-x86_64-v6.00', 'python')
sys.path.insert(0, beagle_lib_path)
from beagle_py import *


#==========================================================================
# MCTP/SPDM PARSING CODE (from mctp-parse)
#==========================================================================

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
        result['version_major'] = (version_byte >> 4) & 0x0F
        result['version_minor'] = version_byte & 0x0F
        result['version'] = f"{result['version_major']}.{result['version_minor']}"

        # Byte 1: Request/Response code
        code = payload[1]
        result['code'] = code

        # Determine if request or response
        if code in SPDMParser.REQUEST_CODES:
            result['msg_direction'] = "Request"
            result['msg_name'] = SPDMParser.REQUEST_CODES[code]
        elif code in SPDMParser.RESPONSE_CODES:
            result['msg_direction'] = "Response"
            result['msg_name'] = SPDMParser.RESPONSE_CODES[code]
        else:
            result['msg_direction'] = "Unknown"
            result['msg_name'] = f"Unknown (0x{code:02X})"

        # Byte 2: Param1 (if present)
        if len(payload) > 2:
            result['param1'] = payload[2]

        # Byte 3: Param2 (if present)
        if len(payload) > 3:
            result['param2'] = payload[3]

        # Remaining data
        if len(payload) > 4:
            result['data'] = payload[4:]
            result['data_len'] = len(result['data'])
        else:
            result['data'] = []
            result['data_len'] = 0

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
            raise ValueError("Packet too short - minimum 4 bytes required for MCTP header")

        result = {}

        # Byte 0: Destination EID
        result['dest_eid'] = data[0]

        # Byte 1: Header Version [7:4], Reserved [3:0]
        byte1 = data[1]
        result['header_version'] = (byte1 >> 4) & 0x0F
        result['rsvd'] = byte1 & 0x0F

        # Byte 2: Source EID
        result['src_eid'] = data[2]

        # Byte 3: SOM, EOM, Pkt_Seq, TO, Msg_Tag
        byte3 = data[3]
        result['som'] = bool(byte3 & 0x80)  # Start of Message
        result['eom'] = bool(byte3 & 0x40)  # End of Message
        result['pkt_seq'] = (byte3 >> 4) & 0x03  # Packet sequence number
        result['to'] = bool(byte3 & 0x08)  # Tag Owner
        result['msg_tag'] = byte3 & 0x07  # Message Tag

        # Message body starts at byte 4
        if len(data) > 4:
            # Byte 4: IC [7], Message Type [6:0]
            byte4 = data[4]
            result['ic'] = bool(byte4 & 0x80)  # Integrity Check
            result['msg_type'] = byte4 & 0x7F
            result['msg_type_name'] = MCTPParser.MESSAGE_TYPES.get(
                result['msg_type'],
                f"Vendor Defined (0x{result['msg_type']:02X})" if result['msg_type'] >= 0x7E
                else f"Reserved (0x{result['msg_type']:02X})"
            )

            # Payload starts at byte 5
            if len(data) > 5:
                result['payload'] = data[5:]
                result['payload_len'] = len(result['payload'])

                # Parse SPDM if message type is 0x05
                if result['msg_type'] == 0x05 and result['payload_len'] > 0:
                    result['spdm'] = SPDMParser.parse(result['payload'])
            else:
                result['payload'] = []
                result['payload_len'] = 0

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
    result['cmd_code'] = data[0]
    result['byte_count'] = data[1]

    expected_total = 2 + result['byte_count']
    if expect_pec:
        expected_total += 1

    if len(data) < expected_total:
        return None

    if expect_pec and len(data) >= expected_total:
        result['pec_received'] = data[expected_total - 1]
        # Calculate expected PEC
        pec_data = data[:expected_total - 1]
        result['pec_calculated'] = calculate_pec(pec_data)
        result['pec_valid'] = (result['pec_received'] == result['pec_calculated'])
        data_end = expected_total - 1
    else:
        result['pec_received'] = None
        result['pec_calculated'] = None
        result['pec_valid'] = None
        data_end = min(len(data), expected_total)

    # SMBus-specific headers start at byte 2
    if result['byte_count'] < 5:
        return None

    offset = 2
    result['source_slave_addr'] = data[offset]
    offset += 1

    # MCTP reserved + header version
    byte_hdr = data[offset]
    result['mctp_reserved'] = (byte_hdr >> 4) & 0x0F
    result['mctp_hdr_version'] = byte_hdr & 0x0F
    offset += 1

    # Destination and Source EIDs
    result['dest_eid'] = data[offset]
    offset += 1
    result['src_eid'] = data[offset]
    offset += 1

    # MCTP packet starts here (SOM/EOM byte)
    result['mctp_offset'] = offset
    result['mctp_data'] = data[offset:data_end]

    return result


#==========================================================================
# MCTP MESSAGE ASSEMBLER
#==========================================================================

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
        key = (mctp_data['src_eid'], mctp_data['dest_eid'], mctp_data['msg_tag'])

        # SOM - start new session
        if mctp_data['som']:
            self.sessions[key] = {
                'fragments': [mctp_data],
                'expected_seq': (mctp_data['pkt_seq'] + 1) % 4,
                'src_eid': mctp_data['src_eid'],
                'dest_eid': mctp_data['dest_eid'],
                'msg_tag': mctp_data['msg_tag'],
                'msg_type': mctp_data.get('msg_type'),
                'msg_type_name': mctp_data.get('msg_type_name'),
            }

            # Single packet message (SOM+EOM)
            if mctp_data['eom']:
                session = self.sessions.pop(key)
                return self._assemble_session(session)

            return None

        # Middle or end fragment
        if key not in self.sessions:
            # Fragment without SOM - ignore or could be error
            return None

        session = self.sessions[key]

        # Check sequence number
        if mctp_data['pkt_seq'] != session['expected_seq']:
            # Sequence error - drop session
            del self.sessions[key]
            return None

        session['fragments'].append(mctp_data)
        session['expected_seq'] = (mctp_data['pkt_seq'] + 1) % 4

        # EOM - assemble complete message
        if mctp_data['eom']:
            complete_session = self.sessions.pop(key)
            return self._assemble_session(complete_session)

        return None

    def _assemble_session(self, session: Dict) -> Dict:
        """Assemble fragments into complete message"""
        # Combine all payloads
        complete_payload = []
        for frag in session['fragments']:
            if 'payload' in frag and frag['payload']:
                complete_payload.extend(frag['payload'])

        result = {
            'src_eid': session['src_eid'],
            'dest_eid': session['dest_eid'],
            'msg_tag': session['msg_tag'],
            'msg_type': session['msg_type'],
            'msg_type_name': session['msg_type_name'],
            'payload': complete_payload,
            'payload_len': len(complete_payload),
            'fragment_count': len(session['fragments']),
        }

        # Parse SPDM if applicable
        if session['msg_type'] == 0x05 and complete_payload:
            result['spdm'] = SPDMParser.parse(complete_payload)

        return result


#==========================================================================
# BEAGLE DEVICE FUNCTIONS
#==========================================================================

def find_and_connect():
    """Find and connect to the first available Beagle device."""
    print("Searching for Beagle devices...")

    # Find all attached devices
    (num, ports, unique_ids) = bg_find_devices_ext(16, 16)

    if num == 0:
        print("Error: No Beagle devices found!")
        sys.exit(1)

    print(f"Found {num} device(s)")

    # Find the first available (not in-use) device
    device_port = None
    for i in range(num):
        port = ports[i]
        unique_id = unique_ids[i]

        if not (port & BG_PORT_NOT_FREE):
            device_port = port
            print(f"Connecting to device on port {port} (S/N: {unique_id:04d}-{unique_id % 1000000:06d})")
            break
        else:
            print(f"Port {port & ~BG_PORT_NOT_FREE} is in use")

    if device_port is None:
        print("Error: All devices are in use!")
        sys.exit(1)

    # Open the device
    beagle = bg_open(device_port)
    if beagle <= 0:
        print(f"Error: Unable to open Beagle device on port {device_port}")
        print(f"Error code = {beagle}")
        sys.exit(1)

    print(f"Successfully opened Beagle device on port {device_port}")
    return beagle


def configure_device(beagle, samplerate_khz=10000, timeout_ms=500, latency_ms=200):
    """Configure the Beagle device for I2C monitoring."""
    # Set sampling rate
    samplerate = bg_samplerate(beagle, samplerate_khz)
    if samplerate < 0:
        print(f"Error setting sample rate: {bg_status_string(samplerate)}")
        sys.exit(1)
    print(f"Sample rate set to {samplerate} kHz")

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


#==========================================================================
# MONITORING FUNCTIONS
#==========================================================================

def monitor_i2c(beagle, args, max_packet_len=256):
    """Monitor and print I2C transactions with optional MCTP/SPDM parsing."""
    # Calculate timing size
    timing_size = bg_bit_timing_size(BG_PROTOCOL_I2C, max_packet_len)

    # Get sample rate for timestamp conversion
    samplerate_khz = bg_samplerate(beagle, 0)

    # Enable I2C capture
    if bg_enable(beagle, BG_PROTOCOL_I2C) != BG_OK:
        print("Error: Could not enable I2C capture!")
        sys.exit(1)

    print("\n" + "="*80)
    print("I2C Monitoring Started - Press Ctrl+C to stop")
    print("="*80)

    if args.spdm:
        print("Mode: SPDM (with MCTP and SMBus)")
        if args.mctp_hide:
            print("      MCTP layer hidden")
        if args.smbus_hide:
            print("      Non-MCTP SMBus traffic hidden")
    elif args.mctp:
        print("Mode: MCTP (with SMBus)")
        if args.mctp_no_partial:
            print("      Hiding partial MCTP fragments")
        if args.smbus_hide:
            print("      Non-MCTP SMBus traffic hidden")
    elif args.smbus:
        print("Mode: SMBus")
        if args.with_pec:
            print("      PEC validation enabled")
        if args.smbus_hide:
            print("      Non-MCTP SMBus traffic hidden")
    else:
        print("Mode: Raw I2C")

    print("="*80 + "\n")

    # Allocate buffers
    data_in = array_u16(max_packet_len)
    timing = array_u32(timing_size)

    packet_count = 0
    mctp_assembler = MCTPAssembler() if (args.mctp or args.spdm) else None

    try:
        while True:
            # Read I2C transaction
            (count, status, time_sop, time_duration,
             time_dataoffset, data_in, timing) = \
                bg_i2c_read_bit_timing(beagle, data_in, timing)

            # Convert timestamp to nanoseconds
            time_sop_ns = (time_sop * 1000) // (samplerate_khz // 1000)

            # Skip if no data
            if count <= 0:
                if count < 0:
                    print(f"Error reading I2C data: {count}")
                    break
                continue

            packet_count += 1

            # Extract raw data bytes (strip NACK bits)
            i2c_addr = None
            i2c_rw = None
            offset = 0

            # Get address if present
            if not (status & BG_READ_ERR_MIDDLE_OF_PACKET) and count >= 1:
                nack = data_in[0] & BG_I2C_MONITOR_NACK
                if count == 1 or (data_in[0] & 0xf9) != 0xf0 or nack:
                    # 7-bit address
                    i2c_addr = (int(data_in[0] & 0xff) >> 1)
                    i2c_rw = "R" if (data_in[0] & 0x01) else "W"
                    offset = 1
                else:
                    # 10-bit address
                    i2c_addr = ((data_in[0] << 7) & 0x300) | (data_in[1] & 0xff)
                    i2c_rw = "R" if (data_in[0] & 0x01) else "W"
                    offset = 2

            # Extract payload bytes
            raw_data = [int(data_in[i] & 0xff) for i in range(offset, count)]

            # Process based on mode
            if args.smbus and raw_data:
                smbus_result = parse_smbus_header(raw_data, args.with_pec)

                if smbus_result and (args.mctp or args.spdm):
                    # Parse MCTP
                    try:
                        # Create synthetic MCTP packet
                        synthetic_mctp = [
                            smbus_result['dest_eid'],
                            (smbus_result['mctp_hdr_version'] << 4) | smbus_result['mctp_reserved'],
                            smbus_result['src_eid']
                        ] + smbus_result['mctp_data']

                        mctp_parsed = MCTPParser.parse(synthetic_mctp)

                        # Handle MCTP assembly
                        if args.mctp_no_partial or args.spdm:
                            complete_msg = mctp_assembler.add_fragment(mctp_parsed)

                            if complete_msg is None:
                                # Incomplete fragment - skip or show minimal info
                                if not args.mctp_no_partial:
                                    print(f"[{time_sop_ns:12d} ns] MCTP Fragment: SOM={mctp_parsed['som']} EOM={mctp_parsed['eom']} Seq={mctp_parsed['pkt_seq']}")
                                continue

                            # Complete message assembled
                            mctp_parsed = complete_msg

                        # Display based on mode
                        if args.spdm and 'spdm' in mctp_parsed:
                            print_spdm_message(time_sop_ns, i2c_addr, i2c_rw, mctp_parsed, args.mctp_hide)
                        elif not args.spdm:
                            print_mctp_message(time_sop_ns, i2c_addr, i2c_rw, smbus_result, mctp_parsed)

                    except Exception as e:
                        print(f"[{time_sop_ns:12d} ns] MCTP Parse Error: {e}")

                elif smbus_result:
                    # SMBus mode only - skip if smbus_hide is enabled
                    if not args.smbus_hide:
                        print_smbus_message(time_sop_ns, i2c_addr, i2c_rw, smbus_result, raw_data)

                else:
                    # Not valid SMBus MCTP - skip if smbus_hide is enabled, otherwise print raw
                    if not args.smbus_hide:
                        print_raw_i2c(time_sop_ns, i2c_addr, i2c_rw, raw_data, status)

            else:
                # Raw I2C mode
                print_raw_i2c(time_sop_ns, i2c_addr, i2c_rw, raw_data, status)

            sys.stdout.flush()

    except KeyboardInterrupt:
        print("\n\nCapture stopped by user")
        print(f"Total packets captured: {packet_count}")

    finally:
        # Disable capture
        bg_disable(beagle)


def print_raw_i2c(timestamp, addr, rw, data, status):
    """Print raw I2C transaction"""
    print(f"[{timestamp:12d} ns] ", end='')

    if addr is not None:
        print(f"[S] <0x{addr:02X}:{rw}> ", end='')

    if data:
        hex_str = ' '.join(f'0x{b:02X}' for b in data)
        print(hex_str, end=' ')

    if not (status & BG_READ_I2C_NO_STOP):
        print("[P]", end=' ')

    print()


def print_smbus_message(timestamp, addr, rw, smbus_info, raw_data):
    """Print SMBus message with PEC info"""
    print(f"[{timestamp:12d} ns] SMBus: ", end='')
    print(f"Cmd=0x{smbus_info['cmd_code']:02X} Len={smbus_info['byte_count']} ", end='')

    if smbus_info['pec_valid'] is not None:
        if smbus_info['pec_valid']:
            print(f"PEC=✓", end=' ')
        else:
            print(f"PEC=✗", end=' ')

    # Show data
    hex_str = ' '.join(f'{b:02X}' for b in raw_data[:16])
    if len(raw_data) > 16:
        hex_str += "..."
    print(f"[{hex_str}]")


def print_mctp_message(timestamp, addr, rw, smbus_info, mctp_parsed):
    """Print MCTP message with highlighting"""
    som_marker = "🟢 SOM" if mctp_parsed.get('som') else ""
    eom_marker = "🔴 EOM" if mctp_parsed.get('eom') else ""
    markers = f"{som_marker} {eom_marker}".strip()

    print(f"[{timestamp:12d} ns] MCTP: ", end='')
    if markers:
        print(f"{markers} ", end='')

    print(f"EID {mctp_parsed['src_eid']:02X}→{mctp_parsed['dest_eid']:02X} ", end='')
    print(f"Seq={mctp_parsed.get('pkt_seq', '?')} Tag={mctp_parsed.get('msg_tag', '?')} ", end='')
    print(f"Type=0x{mctp_parsed.get('msg_type', 0):02X} ({mctp_parsed.get('msg_type_name', 'Unknown')})", end='')

    # Show fragment count if assembled
    if 'fragment_count' in mctp_parsed and mctp_parsed['fragment_count'] > 1:
        print(f" [{mctp_parsed['fragment_count']} fragments]", end='')

    print()


def print_spdm_message(timestamp, addr, rw, mctp_parsed, hide_mctp):
    """Print SPDM message with request/response highlighting"""
    spdm = mctp_parsed.get('spdm')
    if not spdm:
        return

    # Highlight request vs response
    if spdm['msg_direction'] == 'Request':
        direction_marker = "📤 REQ"
    elif spdm['msg_direction'] == 'Response':
        direction_marker = "📥 RSP"
    else:
        direction_marker = "❓"

    print(f"[{timestamp:12d} ns] SPDM: {direction_marker} ", end='')

    if not hide_mctp:
        print(f"EID {mctp_parsed['src_eid']:02X}→{mctp_parsed['dest_eid']:02X} ", end='')
        if 'fragment_count' in mctp_parsed and mctp_parsed['fragment_count'] > 1:
            print(f"[{mctp_parsed['fragment_count']} frags] ", end='')

    print(f"v{spdm['version']} {spdm['msg_name']} ", end='')

    if 'param1' in spdm:
        print(f"P1=0x{spdm['param1']:02X} ", end='')
    if 'param2' in spdm:
        print(f"P2=0x{spdm['param2']:02X} ", end='')

    if spdm.get('data_len', 0) > 0:
        print(f"+{spdm['data_len']}B", end='')

    print()


#==========================================================================
# MAIN
#==========================================================================

def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description='I2C/SMBus Monitor with MCTP/SPDM Support',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  %(prog)s                                    # Raw I2C monitoring
  %(prog)s --smbus --with-pec                 # SMBus with PEC validation
  %(prog)s --mctp                             # MCTP packet monitoring
  %(prog)s --mctp --smbus-hide                # MCTP only (hide non-MCTP SMBus)
  %(prog)s --spdm                             # SPDM message monitoring
  %(prog)s --spdm --mctp-hide                 # SPDM only (hide MCTP layer)
  %(prog)s --spdm --smbus-hide --mctp-hide    # SPDM only (minimal output)
  %(prog)s --samplerate 5000 --timeout 1000   # Custom sample rate and timeout
        '''
    )

    # Protocol mode arguments
    parser.add_argument('--smbus', action='store_true',
                        help='Treat all packets as having SMBus header')
    parser.add_argument('--smbus-hide', action='store_true',
                        help='Hide non-MCTP SMBus traffic (requires --smbus or higher)')
    parser.add_argument('--with-pec', action='store_true',
                        help='Assume PEC is present (requires --smbus)')
    parser.add_argument('--mctp', action='store_true',
                        help='MCTP mode (implies --smbus --with-pec)')
    parser.add_argument('--mctp-no-partial', action='store_true',
                        help='Hide MCTP fragments until EOM (requires --mctp or --spdm)')
    parser.add_argument('--spdm', action='store_true',
                        help='SPDM mode (implies --mctp)')
    parser.add_argument('--mctp-hide', action='store_true',
                        help='Hide MCTP layer details (requires --spdm)')

    # Device configuration arguments
    parser.add_argument('--samplerate', type=int, default=10000, metavar='KHZ',
                        help='Sample rate in kHz (default: 10000)')
    parser.add_argument('--timeout', type=int, default=500, metavar='MS',
                        help='Idle timeout in milliseconds (default: 500)')
    parser.add_argument('--latency', type=int, default=200, metavar='MS',
                        help='Latency in milliseconds (default: 200)')

    args = parser.parse_args()

    # Handle argument implications
    if args.spdm:
        args.mctp = True
        args.mctp_no_partial = True  # SPDM mode always waits for complete messages

    if args.mctp:
        args.smbus = True
        args.with_pec = True

    # Validate argument combinations
    if args.with_pec and not args.smbus:
        parser.error("--with-pec requires --smbus")

    if args.mctp_no_partial and not (args.mctp or args.spdm):
        parser.error("--mctp-no-partial requires --mctp or --spdm")

    if args.mctp_hide and not args.spdm:
        parser.error("--mctp-hide requires --spdm")

    if args.smbus_hide and not args.smbus:
        parser.error("--smbus-hide requires --smbus (or --mctp/--spdm which imply --smbus)")

    # Validate configuration arguments
    if args.samplerate <= 0:
        parser.error("Sample rate must be positive")
    if args.timeout < 0:
        parser.error("Timeout must be non-negative")
    if args.latency < 0:
        parser.error("Latency must be non-negative")

    # Find and connect to device
    beagle = find_and_connect()

    try:
        # Configure device
        print()
        configure_device(beagle, args.samplerate, args.timeout, args.latency)

        # Start monitoring
        monitor_i2c(beagle, args)

    finally:
        # Clean up
        bg_close(beagle)
        print("Beagle device closed")


if __name__ == "__main__":
    main()
