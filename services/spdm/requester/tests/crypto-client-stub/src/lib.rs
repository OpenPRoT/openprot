// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
//
// STUB LIBRARY — Placeholder crypto client for SPDM hash/rng compilation.
// All operations return CryptoError::NotImplemented.
//
// TODO: Remove this stub when services/crypto/client is imported.

#![no_std]

/// Crypto operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// Operation not implemented (stub).
    NotImplemented,
}

/// Result type for crypto operations.
pub type CryptoResult<T> = Result<T, CryptoError>;

/// Stub crypto client.
///
/// This placeholder provides the interface expected by `openprot_spdm_hash`
/// and `openprot_spdm_rng`, but all operations fail with `NotImplemented`.
pub struct CryptoClient {
    _handle: u32,
}

impl CryptoClient {
    /// Create a new crypto client (stub).
    pub const fn new(handle: u32) -> Self {
        Self { _handle: handle }
    }

    /// One-shot SHA-384 hash (stub — always fails).
    pub fn sha384(&self, _data: &[u8]) -> CryptoResult<[u8; 48]> {
        Err(CryptoError::NotImplemented)
    }

    /// One-shot SHA-512 hash (stub — always fails).
    pub fn sha512(&self, _data: &[u8]) -> CryptoResult<[u8; 64]> {
        Err(CryptoError::NotImplemented)
    }

    /// Begin streaming SHA-384 session (stub — always fails).
    pub fn sha384_begin(&self) -> CryptoResult<Sha384Session> {
        Err(CryptoError::NotImplemented)
    }

    /// Begin streaming SHA-512 session (stub — always fails).
    pub fn sha512_begin(&self) -> CryptoResult<Sha512Session> {
        Err(CryptoError::NotImplemented)
    }

    /// Fill buffer with random bytes (stub — always fails).
    pub fn get_random_bytes(&self, _buf: &mut [u8]) -> CryptoResult<()> {
        Err(CryptoError::NotImplemented)
    }
}

/// Stub SHA-384 streaming session.
pub struct Sha384Session {
    _private: (),
}

impl Sha384Session {
    /// Update hash with data (stub — always fails).
    pub fn update(&mut self, _data: &[u8]) -> CryptoResult<()> {
        Err(CryptoError::NotImplemented)
    }

    /// Finalize and return digest (stub — always fails).
    pub fn finalize(self) -> CryptoResult<[u8; 48]> {
        Err(CryptoError::NotImplemented)
    }
}

/// Stub SHA-512 streaming session.
pub struct Sha512Session {
    _private: (),
}

impl Sha512Session {
    /// Update hash with data (stub — always fails).
    pub fn update(&mut self, _data: &[u8]) -> CryptoResult<()> {
        Err(CryptoError::NotImplemented)
    }

    /// Finalize and return digest (stub — always fails).
    pub fn finalize(self) -> CryptoResult<[u8; 64]> {
        Err(CryptoError::NotImplemented)
    }
}
