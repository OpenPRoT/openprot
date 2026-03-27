# SPDM Hash

Hash functions for SPDM protocol operations, implemented via OpenPRoT crypto service.

## Overview

Implements the `SpdmHash` trait from spdm-lib supporting SHA-384 and SHA-512 algorithms. All cryptographic operations are delegated to the centralized crypto service via IPC.

## Architecture

```
SpdmCryptoHash → CryptoClient → IPC → CryptoServer → RustCryptoBackend → SHA2
```

## Supported Algorithms

- **SHA-384** (48 bytes) — Default per SPDM spec
- **SHA-512** (64 bytes)

## Usage Patterns

### Stateless (One-Shot)

For small messages that fit in a single IPC call:

```rust
use openprot_spdm_hash::SpdmCryptoHash;
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType};

let mut hasher = SpdmCryptoHash::new(handle::CRYPTO);
let mut output = [0u8; 48];
hasher.hash(SpdmHashAlgoType::SHA384, b"data to hash", &mut output)?;
```

### Stateful (Streaming)

For large messages or data that arrives in chunks:

```rust
let mut hasher = SpdmCryptoHash::new(handle::CRYPTO);

// Initialize
hasher.init(SpdmHashAlgoType::SHA384, None)?;

// Accumulate data
hasher.update(chunk1)?;
hasher.update(chunk2)?;
hasher.update(chunk3)?;

// Finalize
let mut output = [0u8; 48];
hasher.finalize(&mut output)?;

// Clean up for next use
hasher.reset();
```

### With Initial Data

The `init()` method supports providing initial data:

```rust
// Initialize with VCA (Version/Capabilities/Algorithms) data
hasher.init(SpdmHashAlgoType::SHA384, Some(vca_buffer))?;

// Then add additional messages
hasher.update(request_data)?;
hasher.update(response_data)?;

// Finalize
hasher.finalize(&mut output)?;
```

## SPDM Use Cases

This implementation is used by spdm-lib for:

- **Transcript Hashing**: M1 and L1 transcript hashes for CHALLENGE and MEASUREMENTS
- **Signature Context**: Hash of signing context for CHALLENGE responses
- **Measurement Summaries**: Hashing measurement blocks for attestation
- **Certificate Verification**: Hashing certificate chains

## Dependencies

- `spdm-lib` — SPDM protocol library (https://github.com/9elements/spdm-lib.git, branch: buildup)
- `crypto-client` — OpenPRoT crypto service client

## State Management

The implementation maintains internal state to support streaming operations:

- **Idle**: No active session
- **SHA-384 Session**: Active SHA-384 streaming hash
- **SHA-512 Session**: Active SHA-512 streaming hash

State transitions:
- `init()` → Creates session (Idle → Sha384/Sha512)
- `update()` → Feeds data (stays in current session)
- `finalize()` → Completes hash (Sha384/Sha512 → Idle)
- `reset()` → Aborts session (Any → Idle)
- `hash()` → Operates independently of state

## Performance

- **One-shot operations**: Single IPC round-trip (~10-50μs)
- **Streaming operations**: One IPC call per begin/update/finish
- **Maximum one-shot size**: ~900 bytes (IPC buffer limit)
- **Streaming advantage**: Can handle arbitrarily large data

## Security

- **Algorithms**: FIPS 180-4 compliant SHA-384/512 via RustCrypto
- **Constant-time**: RustCrypto implementations aim for constant-time where feasible
- **Trust boundary**: Assumes crypto service is trusted
- **IPC protection**: Relies on kernel IPC channel security

## Future Enhancements

- Hardware acceleration via AST1060 HACE (transparent when backend is upgraded)
- Additional hash algorithms if required by future SPDM versions

## License

Apache-2.0
