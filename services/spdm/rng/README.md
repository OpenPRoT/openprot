# SPDM RNG

Random number generator for SPDM protocol operations, implemented via OpenPRoT crypto service.

## Overview

This crate implements the `SpdmRng` trait from spdm-lib by delegating to the
centralized crypto service. All randomness is generated using ChaCha20 CSPRNG
seeded from system entropy.

## Architecture

```
SpdmCryptoRng → CryptoClient → IPC → CryptoServer → RustCryptoBackend → ChaCha20Rng
```

## Security Model

- **Entropy Source:** getrandom crate (platform-dependent: hardware RNG, /dev/urandom, etc.)
- **PRNG:** ChaCha20 stream cipher (NIST approved, used in TLS 1.3)
- **Seeding:** Fresh seed from system entropy on each crypto service operation

## Usage

```rust
use openprot_spdm_rng::SpdmCryptoRng;
use spdm_lib::platform::rng::SpdmRng;

let mut rng = SpdmCryptoRng::new(handle::CRYPTO);
let mut challenge = [0u8; 32];
rng.get_random_bytes(&mut challenge)?;
```

## Future Enhancements

When AST1060 hardware RNG driver is available, only the crypto backend needs updating:
- Modify `RustCryptoBackend::OneShot<GetRandomBytes>` to use hardware RNG
- No changes needed to this crate or any SPDM code

## Dependencies

- `spdm-lib` — SPDM protocol library
- `crypto-client` — OpenPRoT crypto service client

## License

Apache-2.0
