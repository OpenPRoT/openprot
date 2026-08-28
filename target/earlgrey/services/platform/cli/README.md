# Platform CLI Interface & Connection Guide

This directory contains the Platform Command Line Interface (CLI) dispatcher framework and command hierarchies for Earlgrey Hardware Enablement (HWE) firmware.

The CLI is accessible concurrently over both:
1. **Physical UART0 Console**: Hardware UART exposed via HyperDebug.
2. **USB CDC-ACM Virtual Serial**: Earlgrey native Full-Speed USB composite endpoint.

---

## Serial Port Identification

The most reliable way to identify serial devices on Linux is via `/dev/serial/by-id/`, where `udev` creates persistent symlinks based on USB vendor/product IDs, serial numbers, and interface numbers:

```bash
ls -l /dev/serial/by-id/
```

### 1. Physical UART0 (Hardware Console)
- **Symlink**: `/dev/serial/by-id/usb-Google_LLC_HyperDebug_CMSIS-DAP_*-if03-port0`
- **Device Node**: Typically `/dev/ttyUSB5`
- **Description**: HyperDebug exposes several USB interfaces. Interface 3 (`if03`) corresponds to `UART2` in HyperDebug firmware, which OpenTitan board configurations (`hyperdebug_chipwhisperer.json`) route to the Earlgrey physical `console` (UART0).

### 2. USB CDC-ACM (Virtual Serial CLI)
- **Symlink**: `/dev/serial/by-id/usb-Google_Inc._OpenPRoT_Earlgrey_*-if00`
- **Device Node**: Typically `/dev/ttyACM2` (VID `0x18d1`, PID `0x503a`)
- **Description**: Earlgrey's native USB Full-Speed device CDC-ACM class. Appears once firmware has booted and VBUS strapping is enabled.

### 3. HyperDebug Control Shell (Reference)
- **Symlink**: `/dev/serial/by-id/usb-Google_LLC_HyperDebug_CMSIS-DAP_*-if00-port0`
- **Device Node**: Typically `/dev/ttyUSB4`
- **Description**: The internal control shell for the HyperDebug MCU itself (used for manual hardware strap manipulation, power, and resets).

---

## Connecting with `minicom`

### Port Settings
- **Baud Rate**: `115200`
- **Data Bits**: `8`
- **Parity**: `None`
- **Stop Bits**: `1`
- **Hardware Flow Control (RTS/CTS)**: `No`
- **Software Flow Control (XON/XOFF)**: `No`

### Connecting to Physical UART0
Using the persistent `/dev/serial/by-id/` symlink prevents connecting to the wrong port if enumeration indices change:

```bash
minicom -w -o -D /dev/serial/by-id/usb-Google_LLC_HyperDebug_CMSIS-DAP_*-if03-port0 -b 115200
```
Or directly with the device node:
```bash
minicom -w -o -D /dev/ttyUSB5 -b 115200
```

### Connecting to USB CDC-ACM
```bash
minicom -w -o -D /dev/serial/by-id/usb-Google_Inc._OpenPRoT_Earlgrey_*-if00 -b 115200
```
Or directly with the device node:
```bash
minicom -w -o -D /dev/ttyACM2 -b 115200
```

### Command Flags:
- `-w`: Enable line-wrapping.
- `-o`: Skip modem initialization strings.
- `-D <path>`: Specify the device path.
- `-b 115200`: Set the baud rate.

### Minicom Navigation Tips:
- **Interactive Prompt (`hwe> `)**: The console displays the `hwe> ` prompt once boot finishes, after each command finishes executing, and whenever you press **Enter** on an empty line as a liveness probe.
- **Toggle Hardware Flow Control**: If typed characters are not echoing or appearing, press `Ctrl-A`, then `O`, navigate to `Serial port setup`, press `F` to toggle `Hardware Flow Control` to `No`, and press `Enter`.
- **Exit Minicom**: Press `Ctrl-A`, then `Q` (exit without reset) or `Ctrl-A`, then `X`.

---

## Alternative: Connecting with `screen`

```bash
# Physical UART0
screen /dev/serial/by-id/usb-Google_LLC_HyperDebug_CMSIS-DAP_*-if03-port0 115200

# USB CDC-ACM
screen /dev/serial/by-id/usb-Google_Inc._OpenPRoT_Earlgrey_*-if00 115200
```

To disconnect and kill `screen`: press `Ctrl-A`, followed by `\` (or `Ctrl-A` then `k`).
