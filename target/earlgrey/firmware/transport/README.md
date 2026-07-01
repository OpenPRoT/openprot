# OpenPRoT Transport Firmware

## Overview
The Transport Firmware is a minimal payload loaded into the Earlgrey chip during device personalization at manufacturing. Its primary responsibility is to facilitate subsequent firmware updates and ownership transfers, making as few assumptions about the final deployment environment as possible.

## Requirements
The transport firmware must support:
1.  **Firmware Update**: Retrieving and programming a new firmware payload.
2.  **Ownership Transfer**: Applying ownership updates.

These actions are supported via two interfaces:
*   **SPI EEPROM 0**: Auto-update on boot by scanning SPI EEPROM 0.
*   **USB-DFU**: Manual or programmatic update/transfer over USB.

## Architecture
The firmware consists of the following components (processes):
*   **System Manager (`sysmgr`)**: Manages bootup environment and reset sequencing.
*   **Log Manager (`logmgr`)**: System-wide logging sink, flushing logs to UART0 and/or USB CDC-ACM.
*   **USB Manager (`usbmgr`)**: Exposes a composite USB device with:
    *   CDC-ACM interface for diagnostic logging.
    *   USB-DFU interface for firmware/ownership updates and certificate retrieval.
*   **Flash Service (`flash_server`)**: Provides IPC interface to access internal flash and SPI EEPROMs.
*   **Platform Task (`platform`)**: Currently a placeholder. It should host the **Update Task**.
*   **Update Task**: Scans SPI EEPROM 0 for updates, programs internal flash, handles ownership transfer blobs, and triggers reboots.

## USB-DFU Interface Specification
The DFU interface exposes the following alternate settings:
*   `0`: DFU download. Earlgrey firmware updates.
*   `1`: DFU upload/download. Access to `OWNER_PAGE_1`.
*   `2`: DFU upload/download. Access to SPI EEPROM 0.
*   `3`: DFU upload/download. Access to SPI EEPROM 1 (exclusive access not guaranteed).
*   `4`: DFU upload. Earlgrey UDS certificate.
*   `5`: DFU upload. Earlgrey `CDI_0` certificate.
*   `6`: DFU upload. Earlgrey `CDI_1` certificate.

---

## TODO List
The current code is a copy of the Hardware Verification Environment (`hwe`) firmware and needs to be adapted to meet the transport firmware requirements.

### 1. General Cleanup & Renaming
- [x] Rename application from `hwe` to `transport` in `target/earlgrey/firmware/transport/system.json5` (line 19: `name: "hwe"` -> `name: "transport"`).
- [x] Update `BUILD.bazel` targets if necessary to align with the new name.


### 2. Drivers
- [ ] Implement **SPI Host driver** for Earlgrey (`SPI_HOST0`).
- [ ] Implement **SPI EEPROM driver** utilizing the SPI Host driver.

### 3. Flash Service (`flash_server.rs`)
- [ ] Integrate SPI and EEPROM drivers.
- [ ] Implement support for accessing SPI EEPROM 0 and SPI EEPROM 1.
- [ ] Define address routing/mapping to distinguish between internal flash, SPI EEPROM 0, and SPI EEPROM 1.
- [ ] Expose SPI EEPROM access via IPC (either via unified `Flash` trait implementation with address routing or separate IPC services).

### 4. USB DFU Adaptations (`usbmgr.rs` & DFU Handler)
- [ ] Specialize the DFU handler for transport firmware:
    -   Copy/diverge from `target/earlgrey/util/dfu.rs` to avoid affecting `hwe` firmware.
    -   Update `BUILD.bazel` to use the local DFU handler.
- [ ] Update `usbmgr.rs` to configure 7 DFU alt-settings (currently 4).
- [ ] Define new string descriptors for the new alt-settings (`OWNER_PAGE_1`, `SPI_EEPROM_0`, `SPI_EEPROM_1`).
- [ ] Update DFU handler to support:
    -   Alt 1: Read/Write to `OWNER_PAGE_1` (internal flash info page).
    -   Alt 2: Read/Write to SPI EEPROM 0.
    -   Alt 3: Read/Write to SPI EEPROM 1.
    -   Alt 4-6: Read certificates (UDS, CDI_0, CDI_1) - remapped from Alt 1-3.

### 5. Update Task
- [ ] Implement the Update Task (can be placed in `platform.rs`).
- [ ] Implement SPI EEPROM 0 scanning logic:
    -   Scan at 64K boundaries.
    -   Look for valid manifest header (`ROM_EXT` / `OTRE` or `Application` / `OTB0`).
    -   If found, read payload, program to internal flash, and request reset via `sysmgr`.
    -   If SPI EEPROM 0 is unavailable, sleep for 1 second and retry.
- [ ] Implement **Ownership Transfer** support in the Update Task:
    -   Detect ownership transfer blob extension in the manifest.
    -   Copy the 2K owner block into `OWNER_PAGE_1` before reset.
    -   Ensure 2K alignment for detached signatures.

### 6. Testing
- [ ] Add unit tests for SPI Host and EEPROM drivers.
- [ ] Add integration tests for the update loop (simulating EEPROM content and verifying flash programming/reset).
