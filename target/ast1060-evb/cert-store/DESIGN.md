# AST1060-EVB Certificate Store Design

## Overview

This directory contains the **software-based reference implementation** of the SPDM certificate store for the AST1060-EVB platform. It is a platform-specific implementation, not a generic service.

## Architecture Decision

### Why Target-Specific (not a Service)?

The certificate store is **platform-dependent** because:

1. **Hardware Integration** - Future implementations will use:
   - OTP (One-Time Programmable) memory for key storage
   - Hardware Security Modules (HSM)
   - Platform-specific crypto accelerators
   - Secure enclaves or trust zones

2. **Security Requirements** - Key material storage is deeply tied to:
   - Platform security architecture
   - Boot chain trust model
   - Hardware root of trust

3. **Not Uniform Across Platforms** - Different targets will have:
   - Different security capabilities
   - Different key provisioning methods
   - Different hardware crypto support

### This Implementation: Software-Based Reference

This implementation is **intentionally simple** to serve as a reference:

- ✅ Uses static data (placeholder certificates)
- ✅ Delegates signing to crypto service via IPC
- ✅ No hardware dependencies
- ✅ Demonstrates correct trait implementation
- ✅ Complete error handling patterns
- ✅ Comprehensive unit tests

## Future Hardware-Backed Implementations

When implementing hardware-backed versions for other platforms:

### 1. Key Storage

Replace static arrays with hardware storage:

```rust
// Instead of:
static SLOT_0_PRIVATE_KEY: [u8; 48] = [0xCC; 48];

// Use platform-specific key storage:
fn get_private_key(slot_id: u8) -> Result<&[u8; 48], Error> {
    // Read from OTP, HSM, or secure storage
    platform::read_secure_key(slot_id)
}
```

### 2. Signing Operations

Use hardware accelerators when available:

```rust
fn sign_hash(&self, slot_id: u8, hash: &[u8], signature: &mut [u8]) -> Result<()> {
    // Option 1: Hardware crypto engine
    if let Some(hw_crypto) = platform::get_hw_crypto() {
        hw_crypto.ecdsa_sign(slot_id, hash, signature)?;
    }
    // Option 2: Fall back to IPC
    else {
        self.crypto.ecdsa_p384_sign(&key, hash)?;
    }
    Ok(())
}
```

### 3. Certificate Provisioning

Implement platform-specific provisioning:

```rust
// Runtime provisioning via secure boot or manufacturing mode
impl Ast1060CertStore {
    pub fn provision_slot(&mut self, slot_id: u8, cert_chain: &[u8],
                          private_key: &[u8]) -> Result<()> {
        platform::write_secure_storage(slot_id, cert_chain, private_key)?;
        platform::lock_storage(slot_id)?;
        Ok(())
    }
}
```

## Build System Integration

This library is built as part of userspace applications:

```python
# Example: SPDM server build
rust_binary(
    name = "spdm_server",
    deps = [
        "//target/ast1060-evb/cert-store:cert_store",
        # ... other deps
    ],
)
```

**Note:** Cannot be built standalone due to `crypto_client` dependency on `pw_kernel/userspace`. This is correct - it's meant to be used by applications, not tested in isolation.

## Testing Strategy

### Unit Tests (In-Library)

- ✅ Validation logic (slot IDs, offsets, etc.)
- ✅ Error handling
- ✅ Data copying and zero-fill behavior

### Integration Tests (System Image)

Create test applications that:
1. Initialize cert store with crypto service
2. Call `sign_hash()` and verify signature
3. Test SPDM protocol operations (GET_DIGESTS, GET_CERTIFICATE, CHALLENGE)

### Hardware Tests (Platform-Specific)

For hardware-backed implementations:
1. Test secure key storage and retrieval
2. Verify hardware crypto operations
3. Test key provisioning flows
4. Validate secure boot integration

## Security Considerations

### Current (Software) Implementation

⚠️ **NOT FOR PRODUCTION USE** - This implementation:
- Stores private keys in plaintext firmware
- Uses placeholder data (0xCC pattern)
- Has no runtime key protection

### Production Requirements

For production deployment:
1. **Key Protection**
   - Store private keys in OTP or HSM
   - Implement key access control
   - Use hardware key derivation if available

2. **Certificate Validation**
   - Verify certificate chain integrity
   - Check certificate expiration
   - Validate certificate signatures

3. **Provisioning Security**
   - Secure manufacturing provisioning flow
   - One-time write protection for keys
   - Audit logging for key operations

## File Organization

```
target/ast1060-evb/cert-store/
├── src/
│   └── lib.rs              # Implementation
├── BUILD.bazel             # Bazel build config
├── README.md               # User documentation
├── IMPLEMENTATION.md       # Implementation details
└── DESIGN.md              # This file (architecture)
```

## Related Components

- **Crypto Service**: `/services/crypto/` - Provides ECDSA signing via IPC
- **SPDM Hash**: `/services/spdm/hash/` - Hash operations (service, not target-specific)
- **SPDM RNG**: `/services/spdm/rng/` - Random number generation (service, not target-specific)
- **SPDM Transport**: `/services/spdm/transport-mctp/` - Transport layer (service)

## Migration Path

When migrating to hardware-backed storage:

1. **Phase 1: Keep Software Implementation**
   - Use this as fallback/reference
   - Implement hardware version alongside

2. **Phase 2: Feature Parity**
   - Ensure hardware version passes all tests
   - Maintain same trait implementation

3. **Phase 3: Production Switch**
   - Configure build to use hardware version
   - Disable software fallback in production
   - Keep software version for testing/development

## License

Licensed under the Apache-2.0 license.
