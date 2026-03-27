// Licensed under the Apache-2.0 license

//! SPDM Hash Implementation
//!
//! Provides cryptographic hashing for SPDM protocol operations by
//! delegating to the OpenPRoT crypto service via IPC.
//!
//! ## Architecture
//!
//! This crate implements the `SpdmHash` trait from spdm-lib by wrapping
//! the `CryptoClient` and calling its hash methods. It supports both
//! stateless one-shot hashing and stateful streaming operations.
//!
//! ## Supported Algorithms
//!
//! - **SHA-384** (48-byte output) — Default per SPDM spec
//! - **SHA-512** (64-byte output)
//!
//! ## Usage
//!
//! ### Stateless (One-Shot)
//!
//! ```rust,no_run
//! use openprot_spdm_hash::SpdmCryptoHash;
//! use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType};
//!
//! let mut hasher = SpdmCryptoHash::new(crypto_handle);
//! let mut output = [0u8; 48];
//! hasher.hash(SpdmHashAlgoType::SHA384, b"data", &mut output).unwrap();
//! ```
//!
//! ### Stateful (Streaming)
//!
//! ```rust,no_run
//! use openprot_spdm_hash::SpdmCryptoHash;
//! use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType};
//!
//! let mut hasher = SpdmCryptoHash::new(crypto_handle);
//!
//! // Initialize
//! hasher.init(SpdmHashAlgoType::SHA384, None).unwrap();
//!
//! // Accumulate data
//! hasher.update(b"chunk1").unwrap();
//! hasher.update(b"chunk2").unwrap();
//!
//! // Finalize
//! let mut output = [0u8; 48];
//! hasher.finalize(&mut output).unwrap();
//!
//! // Clean up
//! hasher.reset();
//! ```

#![no_std]
#![warn(missing_docs)]

use crypto_client::{CryptoClient, Sha384Session, Sha512Session};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};

/// SPDM hash implementation using OpenPRoT crypto service.
///
/// This struct wraps a `CryptoClient` handle and maintains internal state
/// to support both stateless one-shot hashing and stateful streaming operations.
pub struct SpdmCryptoHash {
    crypto: CryptoClient,
    state: HashState,
}

/// Internal state tracking for streaming hash operations.
enum HashState {
    /// No active hash session
    Idle,
    /// Active SHA-384 streaming session
    Sha384(Sha384Session),
    /// Active SHA-512 streaming session
    Sha512(Sha512Session),
}

impl SpdmCryptoHash {
    /// Create a new SPDM hash implementation using the crypto service.
    ///
    /// # Arguments
    ///
    /// * `crypto_handle` — IPC channel handle for the crypto service
    ///   (typically from your app's generated handle module, e.g., `handle::CRYPTO`)
    pub const fn new(crypto_handle: u32) -> Self {
        Self {
            crypto: CryptoClient::new(crypto_handle),
            state: HashState::Idle,
        }
    }
}

impl SpdmHash for SpdmCryptoHash {
    fn hash(
        &mut self,
        hash_algo: SpdmHashAlgoType,
        data: &[u8],
        hash: &mut [u8],
    ) -> SpdmHashResult<()> {
        match hash_algo {
            SpdmHashAlgoType::SHA384 => {
                if hash.len() < 48 {
                    return Err(SpdmHashError::BufferTooSmall);
                }
                let result = self
                    .crypto
                    .sha384(data)
                    .map_err(|_| SpdmHashError::PlatformError)?;
                hash[..48].copy_from_slice(&result);
                Ok(())
            }
            SpdmHashAlgoType::SHA512 => {
                if hash.len() < 64 {
                    return Err(SpdmHashError::BufferTooSmall);
                }
                let result = self
                    .crypto
                    .sha512(data)
                    .map_err(|_| SpdmHashError::PlatformError)?;
                hash[..64].copy_from_slice(&result);
                Ok(())
            }
        }
    }

    fn init(&mut self, hash_algo: SpdmHashAlgoType, data: Option<&[u8]>) -> SpdmHashResult<()> {
        // Ensure we're in Idle state before starting a new session
        if !matches!(self.state, HashState::Idle) {
            return Err(SpdmHashError::PlatformError);
        }

        // Start the appropriate session
        match hash_algo {
            SpdmHashAlgoType::SHA384 => {
                let session = self
                    .crypto
                    .sha384_begin()
                    .map_err(|_| SpdmHashError::PlatformError)?;
                self.state = HashState::Sha384(session);
            }
            SpdmHashAlgoType::SHA512 => {
                let session = self
                    .crypto
                    .sha512_begin()
                    .map_err(|_| SpdmHashError::PlatformError)?;
                self.state = HashState::Sha512(session);
            }
        }

        // If initial data was provided, update with it
        if let Some(initial_data) = data {
            self.update(initial_data)?;
        }

        Ok(())
    }

    fn update(&mut self, data: &[u8]) -> SpdmHashResult<()> {
        match &mut self.state {
            HashState::Idle => Err(SpdmHashError::PlatformError),
            HashState::Sha384(session) => session
                .update(data)
                .map_err(|_| SpdmHashError::PlatformError),
            HashState::Sha512(session) => session
                .update(data)
                .map_err(|_| SpdmHashError::PlatformError),
        }
    }

    fn finalize(&mut self, hash: &mut [u8]) -> SpdmHashResult<()> {
        // Take the session out of state, replacing with Idle
        let state = core::mem::replace(&mut self.state, HashState::Idle);

        match state {
            HashState::Idle => Err(SpdmHashError::PlatformError),
            HashState::Sha384(session) => {
                if hash.len() < 48 {
                    return Err(SpdmHashError::BufferTooSmall);
                }
                let result = session
                    .finalize()
                    .map_err(|_| SpdmHashError::PlatformError)?;
                hash[..48].copy_from_slice(&result);
                Ok(())
            }
            HashState::Sha512(session) => {
                if hash.len() < 64 {
                    return Err(SpdmHashError::BufferTooSmall);
                }
                let result = session
                    .finalize()
                    .map_err(|_| SpdmHashError::PlatformError)?;
                hash[..64].copy_from_slice(&result);
                Ok(())
            }
        }
    }

    fn reset(&mut self) {
        // Simply drop the current session and return to Idle
        self.state = HashState::Idle;
    }

    fn algo(&self) -> SpdmHashAlgoType {
        match &self.state {
            HashState::Idle => SpdmHashAlgoType::SHA384, // Default
            HashState::Sha384(_) => SpdmHashAlgoType::SHA384,
            HashState::Sha512(_) => SpdmHashAlgoType::SHA512,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_creation() {
        let _hasher = SpdmCryptoHash::new(42);
    }

    #[test]
    fn test_default_algo() {
        let hasher = SpdmCryptoHash::new(42);
        assert_eq!(hasher.algo(), SpdmHashAlgoType::SHA384);
    }

    #[test]
    fn test_reset_idempotent() {
        let mut hasher = SpdmCryptoHash::new(42);
        hasher.reset();
        hasher.reset();
        assert_eq!(hasher.algo(), SpdmHashAlgoType::SHA384);
    }
}
