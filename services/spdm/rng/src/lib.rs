// Licensed under the Apache-2.0 license

//! SPDM RNG Implementation
//!
//! Provides random number generation for SPDM protocol operations by
//! delegating to the OpenPRoT crypto service via IPC.
//!
//! ## Architecture
//!
//! This crate implements the `SpdmRng` trait from spdm-lib by wrapping
//! the `CryptoClient` and calling its `get_random_bytes()` method, which
//! uses ChaCha20 CSPRNG seeded from system entropy.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use openprot_spdm_rng::SpdmCryptoRng;
//! use spdm_lib::platform::rng::SpdmRng;
//!
//! let mut rng = SpdmCryptoRng::new(crypto_handle);
//! let mut buffer = [0u8; 32];
//! rng.get_random_bytes(&mut buffer).unwrap();
//! ```

#![no_std]
#![warn(missing_docs)]

use crypto_client::CryptoClient;
use spdm_lib::platform::rng::{SpdmRng, SpdmRngResult};

/// SPDM RNG implementation using OpenPRoT crypto service.
///
/// This struct wraps a `CryptoClient` handle and delegates all RNG
/// operations to the centralized crypto service via IPC.
pub struct SpdmCryptoRng {
    crypto: CryptoClient,
}

impl SpdmCryptoRng {
    /// Create a new SPDM RNG using the crypto service.
    ///
    /// # Arguments
    ///
    /// * `crypto_handle` — IPC channel handle for the crypto service
    ///   (typically from your app's generated handle module, e.g., `handle::CRYPTO`)
    pub const fn new(crypto_handle: u32) -> Self {
        Self {
            crypto: CryptoClient::new(crypto_handle),
        }
    }
}

impl SpdmRng for SpdmCryptoRng {
    fn get_random_bytes(&mut self, buf: &mut [u8]) -> SpdmRngResult<()> {
        self.crypto
            .get_random_bytes(buf)
            .map_err(|_| spdm_lib::platform::rng::SpdmRngError::InvalidSize)
    }

    fn generate_random_number(&mut self, random_number: &mut [u8]) -> SpdmRngResult<()> {
        // Both methods are identical in spdm-lib: fill a buffer with random bytes
        self.crypto
            .get_random_bytes(random_number)
            .map_err(|_| spdm_lib::platform::rng::SpdmRngError::InvalidSize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running crypto service, so they're
    // integration tests that would run in a QEMU environment

    #[test]
    fn test_struct_creation() {
        let _rng = SpdmCryptoRng::new(42);
    }
}
