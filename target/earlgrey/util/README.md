# Earlgrey Utilities (`earlgrey_util`)

This package provides hardware-specific utility drivers, memory-mapped data structures, and protocol wrappers for the OpenTitan Earlgrey target. These utilities are used across the kernel and userspace processes to interact with target hardware and boot stage configurations.

## Key Components

### 1. Hardware Timer & Clock (`clock.rs`, `timer.rs`)
*   **`EarlGreyTimer`**: A wrapper for the OpenTitan system timer (`RvTimer`). It implements an overflow-safe double-read logic for retrieving the 64-bit tick counter on the 32-bit RISC-V platform.
*   **`now_ticks()`**: A global convenience function to get the current tick count using a read-only static timer driver.

### 2. Retention RAM Layout (`ret_ram.rs`)
*   **`RetRam`**: Maps directly to the 4KiB physical Retention SRAM (`0x4060_0000`). This memory persists across warm resets and contains:
    *   Layout version information.
    *   Reset reasons.
    *   Boot Services payload.
    *   The Boot Log.
    *   The last shutdown reason (`RomError`).
    *   Owner-specific persistent storage space (2KiB).
*   **`mut_ref()`**: Unsafely retrieves a mutable reference to the physical Retention RAM address (requires identity mapping in system configuration).

### 3. Boot Stage Structures (`boot_log.rs`, `boot_svc.rs`)
*   **`BootLog`**: Populated by ROM/ROM_EXT, providing details about the boot process (selected boot slot, versions, sizes, ownership states, and minimum security versions).
*   **`BootSvc`**: Structure for the Boot Services protocol. It allows the running application to request actions (like changing the next boot slot, upgrading minimum security versions, or unlocking/transferring ownership) that are executed upon the next reboot.
*   **`CheckDigest`**: Trait implemented by `BootLog` and `BootSvc` to validate SHA256 integrity digests over their contents. OpenTitan uses a reversed-byte order for digests stored in these structures.
*   **`GetData<T>`**: Trait and helper macros to safely extract typed command and response payloads from the generic `BootSvc` structure's data region.

### 4. Provisioning & Personalization (`perso_tlv.rs`)
*   **`PersoCertificate`**: A Type-Length-Value (TLV) parser for retrieving manufacturing provisioning certificates (X.509, CWT, device seeds) stored in flash info pages. Handles the custom packed header format and 8-byte padding constraint.

### 5. Constants & Hardening (`tags.rs`, `mubi.rs`)
*   **`ManifestIdentifier`, `OwnershipState`, `BootSlot`, `UnlockMode`, `BootSvcKind`, `OwnershipKeyAlg`**: Type-safe wrappers around `u32` constant magic tags used in the boot protocol.
*   **`HardenedBool` & `AsMubi`**: Support for Multi-bit Booleans (MuBi). Earlgrey hardware uses specific 4-bit sequences (`0x6` for True, `0x9` for False) to protect critical boolean choices against single-bit fault injection attacks.

### 6. Diagnostics & Errors (`rom_error.rs`, `error.rs`)
*   **`RomError`**: Strongly-typed mapping of raw `u32` bootloader error codes (e.g. signature verification failures, key manager faults, flash controller errors).
*   **`EG_ERROR`**: Target-specific userspace error module (ASCII `'FL'`) for logging utilities.
