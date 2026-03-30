# SPDM Peer Certificate Store - AST1060-EVB Software Implementation

This crate provides a **software-based reference implementation** of the SPDM peer certificate store for the AST1060-EVB target. It stores and manages certificates received from peer SPDM responders during protocol operations.

> **Note:** This is a platform-specific implementation using static buffers. Future hardware-backed implementations could use secure flash, implement certificate caching across reboots, or provide hardware-accelerated certificate validation.

## Overview

The peer certificate store is used by SPDM **requester** (client) applications to store certificates received from **responder** (server) devices during attestation and authentication flows.

### Key Differences from SpdmCertStore

| Aspect | SpdmCertStore | PeerCertStore |
|--------|---------------|---------------|
| **Used by** | SPDM Responder | SPDM Requester |
| **Stores** | Local device certificates | Remote peer certificates |
| **Purpose** | Sign challenge responses | Verify peer signatures |
| **Provisioning** | Build-time (embedded) | Runtime (received via protocol) |
| **Signing** | Yes (with private key) | No (verification only) |

## Architecture

```text
┌─────────────────────────┐
│  SPDM Requester         │
│  (Client)               │
└───────────┬─────────────┘
            │ GET_DIGESTS
            │ GET_CERTIFICATE
            │ CHALLENGE
            ▼
┌─────────────────────────┐
│  Ast1060PeerCertStore   │◄── This crate
│  - Store peer certs     │
│  - Assemble fragments   │
│  - Provide for verify   │
└─────────────────────────┘
```

## Features

- **Runtime storage**: Stores certificates received during protocol execution
- **Fragment assembly**: Handles multi-message certificate transfer
- **Two certificate slots**: Support for up to 2 peer certificates
- **Fixed-size buffers**: 4KB per slot, no dynamic allocation
- **Metadata storage**: Digest, KeyPairID, CertificateInfo, KeyUsageMask
- **No_std compatible**: Fully embedded-friendly

## Usage

### Basic Storage

```rust
use ast1060_peer_cert_store::Ast1060PeerCertStore;
use spdm_lib::cert_store::PeerCertStore;

// Create peer certificate store
let mut peer_store = Ast1060PeerCertStore::new();

// Configure slots (from GET_DIGESTS response)
peer_store.set_supported_slots(0b11)?;      // Slots 0 and 1 supported
peer_store.set_provisioned_slots(0b01)?;    // Only slot 0 provisioned

// Store digest
peer_store.set_digest(0, &digest_from_response)?;

// Store metadata
peer_store.set_cert_info(0, cert_info)?;
peer_store.set_key_usage_mask(0, key_usage)?;
```

### Certificate Fragment Assembly

```rust
// GET_CERTIFICATE responses may be fragmented across multiple messages
loop {
    let portion = receive_certificate_portion()?;

    let status = peer_store.assemble(0, &portion)?;

    match status {
        ReassemblyStatus::InProgress => continue,
        ReassemblyStatus::Done => break,
        _ => {}
    }
}

// Retrieve complete certificate chain
let cert_chain = peer_store.get_cert_chain(0, hash_algo)?;
let root_hash = peer_store.get_root_hash(0, hash_algo)?;
```

### Verification

```rust
// Get certificate for signature verification
let cert_chain = peer_store.get_cert_chain(0, BaseHashAlgoType::Sha384)?;

// Verify signature using crypto service
crypto_client.verify_signature(cert_chain, message, signature)?;
```

## Implementation Details

### Storage Layout

Each slot stores:
- **Certificate chain**: Up to 4KB (includes SPDM header, root hash, DER certificates)
- **Digest**: 48 bytes (SHA-384 from GET_DIGESTS)
- **KeyPairID**: Optional u8
- **CertificateInfo**: Optional metadata
- **KeyUsageMask**: Optional usage flags
- **MeasurementSummaryHashType**: From CHALLENGE request

### Memory Usage

- 2 slots × 4KB = **8KB** for certificate storage
- 2 slots × ~100 bytes = **~200 bytes** for metadata
- **Total: ~8.2KB** static RAM

### Certificate Chain Format

```
┌─────────────────────────────────────────────┐
│ SpdmCertChainHeader (4 bytes)               │
├─────────────────────────────────────────────┤
│ Root Certificate Hash (48 bytes, SHA-384)   │
├─────────────────────────────────────────────┤
│ Root Certificate (DER)                      │
├─────────────────────────────────────────────┤
│ Intermediate Certificate(s) (DER)           │
├─────────────────────────────────────────────┤
│ End Entity Certificate (DER)                │
└─────────────────────────────────────────────┘
```

The `get_cert_chain()` method returns only the DER certificates (without header and root hash).

The `get_raw_chain()` method returns the complete chain (with header).

## Error Handling

| Error | Cause |
|-------|-------|
| `InvalidSlotId` | Slot ID >= 2 |
| `BufferTooSmall` | Certificate chain > 4KB |
| `CertReadError` | Slot empty or data not available |
| `Undefined` | Optional field not set |
| `PlatformError` | Invalid bitmask or internal error |

## Testing

The implementation includes comprehensive unit tests:

```bash
# Tests are included in the library
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/peer-cert-store:peer_cert_store
```

### Test Coverage

- ✅ Slot count verification
- ✅ Supported/provisioned slot masks
- ✅ Certificate chain storage and retrieval
- ✅ Buffer overflow protection
- ✅ Fragment assembly
- ✅ Digest storage
- ✅ Metadata storage (KeyPairID, CertInfo, KeyUsage)
- ✅ Slot reset functionality
- ✅ Invalid slot ID handling
- ✅ MeasurementSummaryHashType storage

## Limitations

1. **Fixed slot count**: 2 slots maximum (can be increased by changing `MAX_PEER_SLOTS`)
2. **Fixed certificate size**: 4KB per slot (typical X.509 chains are 1-4KB)
3. **No persistence**: Stored certificates lost on reset/power cycle
4. **No validation**: Does not validate certificate structure or signatures
5. **Simple assembly**: Caller must determine when reassembly is complete

## Future Hardware-Backed Implementations

When implementing hardware-backed versions:

### Secure Storage

```rust
// Store peer certificates in secure flash
impl Ast1060PeerCertStore {
    fn persist_to_flash(&self, slot_id: u8) -> Result<()> {
        platform::secure_flash::write(slot_id, &self.slots[slot_id])?;
        Ok(())
    }

    fn load_from_flash(&mut self, slot_id: u8) -> Result<()> {
        platform::secure_flash::read(slot_id, &mut self.slots[slot_id])?;
        Ok(())
    }
}
```

### Certificate Validation

```rust
// Add X.509 parsing and validation
fn validate_cert_chain(&self, slot_id: u8) -> Result<()> {
    let chain = self.get_raw_chain(slot_id)?;

    // Parse X.509 certificates
    let certs = x509_parser::parse_chain(chain)?;

    // Verify chain validity
    certs.verify_chain()?;

    // Check expiration
    certs.check_validity(current_time())?;

    Ok(())
}
```

### Certificate Revocation

```rust
// Check certificate revocation status
fn check_revocation(&self, slot_id: u8) -> Result<()> {
    let cert = self.get_cert_chain(slot_id, hash_algo)?;

    // Query OCSP or CRL
    platform::check_cert_revocation(cert)?;

    Ok(())
}
```

## Integration with SPDM Requester

```rust
use ast1060_peer_cert_store::Ast1060PeerCertStore;
use spdm_lib::requester::SpdmRequester;

// Create peer store
let peer_store = Ast1060PeerCertStore::new();

// Create SPDM requester with peer store
let requester = SpdmRequester::new(
    transport,
    hash_impl,
    rng_impl,
    peer_store,  // ← Our implementation
);

// Run SPDM protocol
requester.get_version()?;
requester.get_capabilities()?;
requester.negotiate_algorithms()?;
requester.get_digests()?;       // Populates peer_store
requester.get_certificate(0)?;  // Fragments assembled in peer_store
requester.challenge(0)?;        // Uses peer_store for verification
```

## License

Licensed under the Apache-2.0 license.
