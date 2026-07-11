# Boot Info Test

The `bootinfo` test suite is a custom userspace application that validates and prints critical chip identity, cryptographic certification, and secure boot state data extracted from retention SRAM and hardware flash INFO partitions.

## Overview of Operation

The test process executes sequentially across three main data sources and safely outputs its findings to the console via `pw_log`:

### 1. Retention SRAM Dump (Boot Log & Reset Reasons)
*   Maps and directly reads the 4KiB Retention RAM structure (`0x4060_0000`).
*   Verifies the SHA256 checksum guarding the `BootLog` structure using the embedded RustCrypto SHA256 engine.
*   Displays the primary and chosen `ROM_EXT` and `BL0` boot slots, firmware domains, software/hardware strap state, minimum security versions, and the chip's current `OwnershipState` (e.g. `LockedOwner`).
*   Dumps the 32-bit hardware reset reasons and the last recorded kernel shutdown reason.

### 2. DICE Certificate Extraction
*   Connects to the `FlashIpcServer` via userspace IPC channels to safely request Read transactions against Bank 0 Page 9 (the UDS/FactoryCerts page) and Bank 1 Page 9 (containing the `CDI0` and `CDI1` DICE chain certificates).
*   Sequentially parses all embedded DER-encoded X.509/CWT personalization certificates using `PersoCertificate::from_bytes`.
*   Encodes and PEM-wraps the DER data utilizing the secure, constant-time `base64ct` crate, neatly chunking output into 64-character line-wrapped standard headers.

### 3. Ownership Info Extraction (`OWNER_PAGE_0`)
*   Issues IPC reads targeting Flash INFO Bank 1, Page 2 to extract the 2048-byte hardware `owner_block_t`.
*   Parses the `TlvHeader` tag, `config_version`, and `update_mode` strings.
*   Supports decoding, extracting, and hex-encoding coordinates for all supported OpenTitan public key algorithms:
    *   **ECDSA P-256**: Hexadecimal `X` and `Y` public coordinates.
    *   **SPX+ (SLH-DSA)**: Pure or Prehashed SPX data.
    *   **Hybrid P256 + SPX+**: Combined hexadecimal coordinates for both public key mechanisms in sequence.

## Resilient Hardware Fallback

Physical silicon and FPGA hardware boards (such as the CW340 or CW310) often start with unprovisioned or blank FactoryCerts partitions, which naturally trigger hardware Flash Read Errors (`0x464f0004`) due to ECC integrity mismatches.

To ensure safe and thorough execution on developer workstations, the `bootinfo` test implements resilient error-handlers for all INFO partition reads. Rather than aborting, the test gracefully skips faulted or unprovisioned regions, allowing valid structures (like `CDI_0` or the Ownership page) to be perfectly extracted, dumped, and validated.

## Executing the Test

### To compile the bootinfo image:
```bash
bazel build //target/earlgrey/tests/bootinfo:bootinfo_image
```

### To run on the CW340 (Hyper340 Luna Board) FPGA:
```bash
bazel test //target/earlgrey/tests/bootinfo:bootinfo_hyper340_test
```

### To run on the CW310 (Hyper310 Bergen Board) FPGA:
```bash
bazel test //target/earlgrey/tests/bootinfo:bootinfo_hyper310_test
```
