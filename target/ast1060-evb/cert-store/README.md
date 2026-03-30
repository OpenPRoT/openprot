# SPDM Certificate Store - AST1060-EVB Software Implementation

This crate provides a **software-based reference implementation** of the SPDM certificate store for the AST1060-EVB target. It manages X.509 certificate chains used for device attestation and secure communication.

> **Note:** This is a platform-specific implementation using static data and IPC-based signing. Future hardware-backed implementations (using OTP memory, HSM, or crypto accelerators) should use this as a reference pattern while storing keys securely in hardware.

## Overview

The certificate store implements the `SpdmCertStore` trait from spdm-lib, providing:

- Certificate chain storage and retrieval
- Root certificate hash calculation
- Digital signature generation via crypto service IPC
- Support for multiple certificate slots

## Architecture

```text
┌─────────────────────────┐
│  SPDM Responder         │
└───────────┬─────────────┘
            │ get_cert_chain()
            │ root_cert_hash()
            │ sign_hash()
            ▼
┌─────────────────────────┐
│  Ast1060CertStore       │◄── This crate
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Crypto Service (IPC)   │
│  - ECDSA P-384 signing  │
└─────────────────────────┘
```

## Features

- **Build-time provisioning**: Certificates embedded as static data
- **Two certificate slots**: Slot 0 provisioned, slot 1 reserved
- **ECC P-384 support**: Per spdm-lib v0.1.0 requirements
- **IPC-based signing**: Delegates cryptographic operations to crypto service
- **No dynamic allocation**: Fully `no_std` compatible

## Usage

```rust
use openprot_spdm_cert_store::Ast1060CertStore;
use spdm_lib::cert_store::SpdmCertStore;

// Create certificate store with crypto service handle
let mut store = Ast1060CertStore::new(crypto_handle);

// Query capabilities
assert_eq!(store.slot_count(), 2);
assert!(store.is_provisioned(0));

// Get certificate chain length
let len = store.cert_chain_len(AsymAlgo::EccP384, 0)?;

// Retrieve certificate chain
let mut buffer = [0u8; 256];
let bytes_read = store.get_cert_chain(0, AsymAlgo::EccP384, 0, &mut buffer)?;

// Get root certificate hash
let mut hash = [0u8; 48];
store.root_cert_hash(0, AsymAlgo::EccP384, &mut hash)?;

// Sign a hash (delegates to crypto service)
let mut signature = [0u8; 96];
store.sign_hash(0, &hash, &mut signature)?;
```

## Current Implementation

### Placeholder Data

The current implementation uses placeholder data for development:

- **Certificate chain**: 32 bytes of `0xAA`
- **Root hash**: 48 bytes of `0xBB` (SHA-384)
- **Private key**: 48 bytes of `0xCC` (P-384 scalar)

### Slot Configuration

- **Slot 0**: Provisioned with placeholder data
- **Slot 1**: Reserved for future use (not provisioned)

## Production Deployment

For production use, replace placeholder data with real certificates:

```rust
// Option 1: Dynamic size cert chain
static SLOT_0_CERT_CHAIN: &[u8] = include_bytes!("certs/device_cert_chain.der");
const SLOT_0_CERT_CHAIN_LEN: usize = SLOT_0_CERT_CHAIN.len();

// Option 2: Fixed size cert chain
const CERT_CHAIN_SIZE: usize = 2048;
static SLOT_0_CERT_CHAIN: [u8; CERT_CHAIN_SIZE] =
    *include_bytes!("certs/device_cert_chain.der");

// Update root hash and private key
static SLOT_0_ROOT_HASH: [u8; SHA384_HASH_SIZE] =
    *include_bytes!("certs/root_hash.bin");
static SLOT_0_PRIVATE_KEY: [u8; ECDSA_P384_PRIVATE_KEY_SIZE] =
    *include_bytes!("certs/private_key.bin");
```

**Security Note**: In production, private keys should be stored in OTP memory or encrypted storage, not in plaintext firmware.

## Error Handling

| Error | Cause |
|-------|-------|
| `InvalidSlotId` | Slot ID >= 2 |
| `CertReadError` | Slot not provisioned (slot 1) |
| `UnsupportedHashAlgo` | Algorithm != ECC P-384 |
| `InvalidOffset` | Offset >= certificate chain length |
| `PlatformError` | Crypto service IPC failure |

## Testing

The implementation includes comprehensive unit tests:

```bash
# Run unit tests
bazel test //services/spdm/cert-store:spdm_cert_store_test

# Build as part of a system image
bazel build --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm:system_image
```

## Dependencies

- `spdm-lib`: SPDM protocol library (trait definitions)
- `crypto-client`: OpenPRoT crypto service client
- `crypto-api`: Protocol definitions for crypto operations

## Limitations

1. **Static certificate size**: 32 bytes is placeholder; real X.509 chains are 1-4KB
2. **Single provisioned slot**: Only slot 0 is provisioned
3. **Software signing**: Uses IPC instead of hardware acceleration
4. **No runtime provisioning**: Certificates must be embedded at build time
5. **ECC P-384 only**: No RSA support (per spdm-lib v0.1.0)

## Future Enhancements

- Replace placeholder data with real X.509 certificate chains
- Move private key to OTP memory or HSM
- Add API for runtime provisioning of slot 1
- Add X.509 parsing and validation
- Use AST1060 crypto accelerator for signing operations

## License

Licensed under the Apache-2.0 license.
