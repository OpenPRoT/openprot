// Licensed under the Apache-2.0 license

//! SPDM Certificate Store - AST1060-EVB Software Implementation
//!
//! **Software-based reference implementation** for SPDM certificate store operations.
//! This manages X.509 certificate chains used for device attestation and secure
//! communication.
//!
//! ## Architecture
//!
//! - **Build-time provisioning:** Certificates are embedded as static data
//! - **Two slots:** Slot 0 is provisioned with placeholder data, slot 1 is reserved
//! - **Runtime signing:** Delegated to crypto service via IPC
//! - **Algorithm support:** ECC P-384 only (per spdm-lib v0.1.0)
//!
//! ## Hardware-Backed Implementations
//!
//! This software implementation serves as a reference. Future hardware-backed
//! versions should:
//! - Store private keys in OTP memory or HSM
//! - Use hardware crypto accelerators for signing
//! - Implement runtime key protection mechanisms
//! - Follow the same trait implementation pattern
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ast1060_cert_store::Ast1060CertStore;
//! use spdm_lib::cert_store::SpdmCertStore;
//!
//! let mut store = Ast1060CertStore::new(crypto_handle);
//! assert_eq!(store.slot_count(), 2);
//! assert!(store.is_provisioned(0));
//! ```

#![no_std]

use crypto_api::protocol::ECDSA_P384_PRIVATE_KEY_SIZE;
use crypto_client::CryptoClient;
use spdm_lib::cert_store::{CertStoreError, CertStoreResult, SpdmCertStore};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};

/// Certificate chain size (placeholder for development, will expand in production)
const CERT_CHAIN_PLACEHOLDER_SIZE: usize = 32;

/// SPDM certificate store implementation for AST1060-EVB.
///
/// This struct manages certificate chains and private keys for SPDM protocol
/// operations, delegating cryptographic signing to the crypto service via IPC.
pub struct Ast1060CertStore {
    crypto: CryptoClient,
}

// Static backing stores using library constants
static SLOT_0_CERT_CHAIN: [u8; CERT_CHAIN_PLACEHOLDER_SIZE] =
    [0xAA; CERT_CHAIN_PLACEHOLDER_SIZE];
static SLOT_0_ROOT_HASH: [u8; SHA384_HASH_SIZE] = [0xBB; SHA384_HASH_SIZE];
static SLOT_0_PRIVATE_KEY: [u8; ECDSA_P384_PRIVATE_KEY_SIZE] =
    [0xCC; ECDSA_P384_PRIVATE_KEY_SIZE];
const SLOT_0_CERT_CHAIN_LEN: usize = CERT_CHAIN_PLACEHOLDER_SIZE;

impl Ast1060CertStore {
    /// Create a new certificate store instance.
    ///
    /// # Arguments
    ///
    /// * `crypto_handle` — IPC channel handle for the crypto service
    pub const fn new(crypto_handle: u32) -> Self {
        Self {
            crypto: CryptoClient::new(crypto_handle),
        }
    }
}

impl SpdmCertStore for Ast1060CertStore {
    fn slot_count(&self) -> u8 {
        2
    }

    fn is_provisioned(&self, slot_id: u8) -> bool {
        slot_id == 0
    }

    fn cert_chain_len(&mut self, asym_algo: AsymAlgo, slot_id: u8) -> CertStoreResult<usize> {
        // Validate slot ID
        if slot_id >= 2 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }

        // Only ECC P-384 is supported
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }

        // Only slot 0 is provisioned
        if slot_id != 0 {
            return Err(CertStoreError::CertReadError);
        }

        Ok(SLOT_0_CERT_CHAIN_LEN)
    }

    fn get_cert_chain(
        &mut self,
        slot_id: u8,
        asym_algo: AsymAlgo,
        offset: usize,
        cert_portion: &mut [u8],
    ) -> CertStoreResult<usize> {
        // Validate slot ID
        if slot_id >= 2 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }

        // Only ECC P-384 is supported
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }

        // Only slot 0 is provisioned
        if slot_id != 0 {
            return Err(CertStoreError::CertReadError);
        }

        // Validate offset
        if offset >= SLOT_0_CERT_CHAIN_LEN {
            return Err(CertStoreError::InvalidOffset);
        }

        // Calculate how many bytes to copy
        let remaining = SLOT_0_CERT_CHAIN_LEN - offset;
        let to_copy = remaining.min(cert_portion.len());

        // Copy certificate data
        cert_portion[..to_copy].copy_from_slice(&SLOT_0_CERT_CHAIN[offset..offset + to_copy]);

        // Fill remaining buffer with zeros (if any)
        if to_copy < cert_portion.len() {
            cert_portion[to_copy..].fill(0);
        }

        Ok(to_copy)
    }

    fn root_cert_hash(
        &mut self,
        slot_id: u8,
        asym_algo: AsymAlgo,
        cert_hash: &mut [u8; SHA384_HASH_SIZE],
    ) -> CertStoreResult<()> {
        // Validate slot ID
        if slot_id >= 2 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }

        // Only ECC P-384 is supported
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }

        // Only slot 0 is provisioned
        if slot_id != 0 {
            return Err(CertStoreError::CertReadError);
        }

        // Copy pre-calculated root hash
        cert_hash.copy_from_slice(&SLOT_0_ROOT_HASH);

        Ok(())
    }

    fn sign_hash(
        &self,
        slot_id: u8,
        hash: &[u8; SHA384_HASH_SIZE],
        signature: &mut [u8; ECC_P384_SIGNATURE_SIZE],
    ) -> CertStoreResult<()> {
        // Validate slot ID
        if slot_id >= 2 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }

        // Only slot 0 is provisioned
        if slot_id != 0 {
            return Err(CertStoreError::CertReadError);
        }

        // Call crypto service via IPC
        // Note: The input `hash` is a pre-computed SHA-384 digest, but
        // ecdsa_p384_sign() will hash it again internally. This double-hashing
        // is intentional per SPDM protocol spec.
        let result = self
            .crypto
            .ecdsa_p384_sign(&SLOT_0_PRIVATE_KEY, hash)
            .map_err(|_| CertStoreError::PlatformError)?;

        // Copy signature to output buffer
        signature.copy_from_slice(&result);

        Ok(())
    }

    fn key_pair_id(&self, _slot_id: u8) -> Option<u8> {
        // Multi-key connection feature not needed
        None
    }

    fn cert_info(&self, _slot_id: u8) -> Option<CertificateInfo> {
        // Optional metadata not needed
        None
    }

    fn key_usage_mask(&self, _slot_id: u8) -> Option<KeyUsageMask> {
        // Optional feature not needed
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_count() {
        let store = Ast1060CertStore::new(42);
        assert_eq!(store.slot_count(), 2);
    }

    #[test]
    fn test_is_provisioned() {
        let store = Ast1060CertStore::new(42);
        assert!(store.is_provisioned(0));
        assert!(!store.is_provisioned(1));
        assert!(!store.is_provisioned(2));
    }

    #[test]
    fn test_invalid_slot() {
        let mut store = Ast1060CertStore::new(42);
        let result = store.cert_chain_len(AsymAlgo::EccP384, 5);
        assert!(matches!(result, Err(CertStoreError::InvalidSlotId(5))));
    }

    #[test]
    fn test_unprovisioned_slot() {
        let mut store = Ast1060CertStore::new(42);
        let result = store.cert_chain_len(AsymAlgo::EccP384, 1);
        assert!(matches!(result, Err(CertStoreError::CertReadError)));
    }

    #[test]
    fn test_cert_chain_len() {
        let mut store = Ast1060CertStore::new(42);
        assert_eq!(
            store.cert_chain_len(AsymAlgo::EccP384, 0),
            Ok(CERT_CHAIN_PLACEHOLDER_SIZE)
        );
    }

    #[test]
    fn test_get_cert_chain_full() {
        let mut store = Ast1060CertStore::new(42);
        let mut buffer = [0u8; CERT_CHAIN_PLACEHOLDER_SIZE];
        let bytes_read = store
            .get_cert_chain(0, AsymAlgo::EccP384, 0, &mut buffer)
            .unwrap();
        assert_eq!(bytes_read, CERT_CHAIN_PLACEHOLDER_SIZE);
        assert_eq!(&buffer[..], &[0xAA; CERT_CHAIN_PLACEHOLDER_SIZE]);
    }

    #[test]
    fn test_get_cert_chain_with_offset() {
        let mut store = Ast1060CertStore::new(42);
        let mut buffer = [0u8; 16];
        let bytes_read = store
            .get_cert_chain(0, AsymAlgo::EccP384, 16, &mut buffer)
            .unwrap();
        assert_eq!(bytes_read, 16);
        assert_eq!(&buffer[..], &[0xAA; 16]);
    }

    #[test]
    fn test_get_cert_chain_zero_fill() {
        let mut store = Ast1060CertStore::new(42);
        let mut buffer = [0xFFu8; 64]; // Larger than cert chain
        let bytes_read = store
            .get_cert_chain(0, AsymAlgo::EccP384, 0, &mut buffer)
            .unwrap();
        assert_eq!(bytes_read, CERT_CHAIN_PLACEHOLDER_SIZE);
        assert_eq!(
            &buffer[..CERT_CHAIN_PLACEHOLDER_SIZE],
            &[0xAA; CERT_CHAIN_PLACEHOLDER_SIZE]
        );
        assert_eq!(
            &buffer[CERT_CHAIN_PLACEHOLDER_SIZE..],
            &[0u8; 64 - CERT_CHAIN_PLACEHOLDER_SIZE]
        );
    }

    #[test]
    fn test_get_cert_chain_invalid_offset() {
        let mut store = Ast1060CertStore::new(42);
        let mut buffer = [0u8; 16];
        let result = store.get_cert_chain(0, AsymAlgo::EccP384, 100, &mut buffer);
        assert!(matches!(result, Err(CertStoreError::InvalidOffset)));
    }

    #[test]
    fn test_root_hash_copy() {
        let mut store = Ast1060CertStore::new(42);
        let mut hash = [0u8; SHA384_HASH_SIZE];
        store
            .root_cert_hash(0, AsymAlgo::EccP384, &mut hash)
            .unwrap();
        assert_eq!(hash, [0xBB; SHA384_HASH_SIZE]);
    }

    #[test]
    fn test_optional_methods_return_none() {
        let store = Ast1060CertStore::new(42);
        assert_eq!(store.key_pair_id(0), None);
        assert_eq!(store.cert_info(0), None);
        assert_eq!(store.key_usage_mask(0), None);
    }
}
