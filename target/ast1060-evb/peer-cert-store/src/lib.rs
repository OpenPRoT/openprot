// Licensed under the Apache-2.0 license

//! SPDM Peer Certificate Store - AST1060-EVB Software Implementation
//!
//! **Software-based reference implementation** for storing peer certificates
//! received during SPDM protocol operations. Used by SPDM requester (client)
//! to store and validate certificates from the SPDM responder (server).
//!
//! ## Architecture
//!
//! - **Runtime storage:** Certificates received during protocol execution
//! - **Fixed-size buffers:** Static allocation, no heap required
//! - **Two slots:** Support for up to 2 peer certificate chains
//! - **No_std compatible:** Embedded-friendly design
//!
//! ## Hardware-Backed Implementations
//!
//! This software implementation serves as a reference. Future hardware-backed
//! versions could:
//! - Store peer certificates in secure flash
//! - Use hardware-accelerated certificate validation
//! - Implement certificate revocation checking
//! - Cache validated certificates across reboots
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ast1060_peer_cert_store::Ast1060PeerCertStore;
//! use spdm_lib::cert_store::PeerCertStore;
//!
//! let mut peer_store = Ast1060PeerCertStore::new();
//!
//! // During SPDM GET_DIGESTS response
//! peer_store.set_supported_slots(0b11)?; // Slots 0 and 1
//! peer_store.set_provisioned_slots(0b01)?; // Only slot 0 provisioned
//!
//! // During SPDM GET_CERTIFICATE response (may be fragmented)
//! peer_store.assemble(0, &cert_portion1)?;
//! peer_store.assemble(0, &cert_portion2)?;
//! // ... until complete
//!
//! // Retrieve for verification
//! let cert_chain = peer_store.get_cert_chain(0, hash_algo)?;
//! ```

#![no_std]

use spdm_lib::cert_store::{CertStoreError, CertStoreResult, PeerCertStore, ReassemblyStatus};
use spdm_lib::commands::challenge::MeasurementSummaryHashType;
use spdm_lib::protocol::algorithms::SHA384_HASH_SIZE;
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use spdm_lib::protocol::{BaseHashAlgoType, SpdmCertChainHeader};

/// Maximum certificate chain size (including header)
/// Typical X.509 chains are 1-4KB; using 4KB for safety
const MAX_CERT_CHAIN_SIZE: usize = 4096;

/// Maximum number of peer certificate slots
const MAX_PEER_SLOTS: usize = 2;

/// Per-slot peer certificate storage
#[derive(Clone)]
struct PeerSlot {
    /// Complete certificate chain (SpdmCertChainHeader + root_hash + DER certs)
    cert_chain: [u8; MAX_CERT_CHAIN_SIZE],
    /// Actual length of data in cert_chain buffer
    cert_chain_len: usize,

    /// Digest from GET_DIGESTS response
    digest: [u8; SHA384_HASH_SIZE],
    /// Whether digest has been set
    digest_valid: bool,

    /// Optional KeyPairID
    keypair_id: Option<u8>,

    /// Optional CertificateInfo
    cert_info: Option<CertificateInfo>,

    /// Optional KeyUsageMask
    key_usage_mask: Option<KeyUsageMask>,

    /// MeasurementSummaryHashType requested in CHALLENGE
    requested_msh_type: Option<MeasurementSummaryHashType>,
}

impl PeerSlot {
    const fn new() -> Self {
        Self {
            cert_chain: [0u8; MAX_CERT_CHAIN_SIZE],
            cert_chain_len: 0,
            digest: [0u8; SHA384_HASH_SIZE],
            digest_valid: false,
            keypair_id: None,
            cert_info: None,
            key_usage_mask: None,
            requested_msh_type: None,
        }
    }

    fn reset(&mut self) {
        self.cert_chain_len = 0;
        self.digest_valid = false;
        self.keypair_id = None;
        self.cert_info = None;
        self.key_usage_mask = None;
        self.requested_msh_type = None;
    }

    /// Get the root hash from the certificate chain
    fn get_root_hash(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        if self.cert_chain_len == 0 {
            return None;
        }

        // Certificate chain format:
        // [SpdmCertChainHeader (4 bytes) | RootHash (hash_size bytes) | DER Certs]
        let hash_size = hash_algo.hash_byte_size();
        let header_size = core::mem::size_of::<SpdmCertChainHeader>();
        let total_header = header_size + hash_size;

        if self.cert_chain_len < total_header {
            return None;
        }

        Some(&self.cert_chain[header_size..total_header])
    }

    /// Get the DER certificate chain (without header and root hash)
    fn get_cert_chain(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        if self.cert_chain_len == 0 {
            return None;
        }

        let hash_size = hash_algo.hash_byte_size();
        let header_size = core::mem::size_of::<SpdmCertChainHeader>();
        let total_header = header_size + hash_size;

        if self.cert_chain_len <= total_header {
            return None;
        }

        Some(&self.cert_chain[total_header..self.cert_chain_len])
    }

    /// Get the raw certificate chain (with header)
    fn get_raw_chain(&self) -> Option<&[u8]> {
        if self.cert_chain_len == 0 {
            return None;
        }
        Some(&self.cert_chain[..self.cert_chain_len])
    }
}

/// SPDM peer certificate store implementation for AST1060-EVB.
///
/// Stores certificates received from SPDM responders during protocol operations.
/// Uses static allocation with fixed-size buffers.
pub struct Ast1060PeerCertStore {
    slots: [PeerSlot; MAX_PEER_SLOTS],
    supported_slot_mask: u8,
    provisioned_slot_mask: u8,
}

impl Ast1060PeerCertStore {
    /// Create a new peer certificate store instance.
    pub const fn new() -> Self {
        Self {
            slots: [PeerSlot::new(), PeerSlot::new()],
            supported_slot_mask: 0,
            provisioned_slot_mask: 0,
        }
    }

    fn validate_slot_id(&self, slot_id: u8) -> CertStoreResult<usize> {
        if (slot_id as usize) >= MAX_PEER_SLOTS {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        Ok(slot_id as usize)
    }
}

impl PeerCertStore for Ast1060PeerCertStore {
    fn slot_count(&self) -> u8 {
        MAX_PEER_SLOTS as u8
    }

    fn set_supported_slots(&mut self, supported_slot_mask: u8) -> CertStoreResult<()> {
        // Only support bits for slots 0 and 1
        if supported_slot_mask > 0b11 {
            return Err(CertStoreError::PlatformError);
        }
        self.supported_slot_mask = supported_slot_mask;
        Ok(())
    }

    fn get_supported_slots(&self) -> CertStoreResult<u8> {
        Ok(self.supported_slot_mask)
    }

    fn set_provisioned_slots(&mut self, provisioned_slot_mask: u8) -> CertStoreResult<()> {
        // Only support bits for slots 0 and 1
        if provisioned_slot_mask > 0b11 {
            return Err(CertStoreError::PlatformError);
        }
        self.provisioned_slot_mask = provisioned_slot_mask;
        Ok(())
    }

    fn get_provisioned_slots(&self) -> CertStoreResult<u8> {
        Ok(self.provisioned_slot_mask)
    }

    fn get_cert_chain(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .get_cert_chain(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn set_cert_chain(&mut self, slot_id: u8, cert_chain: &[u8]) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;

        if cert_chain.len() > MAX_CERT_CHAIN_SIZE {
            return Err(CertStoreError::BufferTooSmall);
        }

        let slot = &mut self.slots[idx];
        slot.cert_chain[..cert_chain.len()].copy_from_slice(cert_chain);
        slot.cert_chain_len = cert_chain.len();

        Ok(())
    }

    fn get_digest(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        let idx = self.validate_slot_id(slot_id)?;
        if !self.slots[idx].digest_valid {
            return Err(CertStoreError::CertReadError);
        }
        Ok(&self.slots[idx].digest)
    }

    fn set_digest(&mut self, slot_id: u8, digest: &[u8]) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;

        if digest.len() > SHA384_HASH_SIZE {
            return Err(CertStoreError::BufferTooSmall);
        }

        let slot = &mut self.slots[idx];
        slot.digest[..digest.len()].copy_from_slice(digest);
        slot.digest_valid = true;

        Ok(())
    }

    fn get_keypair(&self, slot_id: u8) -> CertStoreResult<u8> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .keypair_id
            .ok_or(CertStoreError::Undefined)
    }

    fn set_keypair(&mut self, slot_id: u8, keypair: u8) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx].keypair_id = Some(keypair);
        Ok(())
    }

    fn get_cert_info(&self, slot_id: u8) -> CertStoreResult<CertificateInfo> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .cert_info
            .ok_or(CertStoreError::Undefined)
    }

    fn set_cert_info(&mut self, slot_id: u8, cert_info: CertificateInfo) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx].cert_info = Some(cert_info);
        Ok(())
    }

    fn get_key_usage_mask(&self, slot_id: u8) -> CertStoreResult<KeyUsageMask> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .key_usage_mask
            .ok_or(CertStoreError::Undefined)
    }

    fn set_key_usage_mask(
        &mut self,
        slot_id: u8,
        key_usage_mask: KeyUsageMask,
    ) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx].key_usage_mask = Some(key_usage_mask);
        Ok(())
    }

    fn assemble(
        &mut self,
        slot_id: u8,
        portion: &[u8],
    ) -> Result<ReassemblyStatus, CertStoreError> {
        let idx = self.validate_slot_id(slot_id)?;
        let slot = &mut self.slots[idx];

        // Check if adding this portion would exceed buffer
        if slot.cert_chain_len + portion.len() > MAX_CERT_CHAIN_SIZE {
            return Err(CertStoreError::BufferTooSmall);
        }

        // Append portion to existing data
        let start = slot.cert_chain_len;
        let end = start + portion.len();
        slot.cert_chain[start..end].copy_from_slice(portion);
        slot.cert_chain_len = end;

        // Return InProgress - caller determines when complete
        Ok(ReassemblyStatus::InProgress)
    }

    fn reset(&mut self, slot_id: u8) {
        if let Ok(idx) = self.validate_slot_id(slot_id) {
            self.slots[idx].reset();
        }
    }

    fn get_root_hash(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .get_root_hash(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn get_raw_chain(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .get_raw_chain()
            .ok_or(CertStoreError::CertReadError)
    }

    fn get_requested_msh_type(&self, slot_id: u8) -> CertStoreResult<MeasurementSummaryHashType> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx]
            .requested_msh_type
            .clone()
            .ok_or(CertStoreError::Undefined)
    }

    fn set_requested_msh_type(
        &mut self,
        slot_id: u8,
        msh_type: MeasurementSummaryHashType,
    ) -> CertStoreResult<()> {
        let idx = self.validate_slot_id(slot_id)?;
        self.slots[idx].requested_msh_type = Some(msh_type);
        Ok(())
    }
}

impl Default for Ast1060PeerCertStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_count() {
        let store = Ast1060PeerCertStore::new();
        assert_eq!(store.slot_count(), 2);
    }

    #[test]
    fn test_supported_slots() {
        let mut store = Ast1060PeerCertStore::new();
        assert_eq!(store.get_supported_slots().unwrap(), 0);

        store.set_supported_slots(0b11).unwrap();
        assert_eq!(store.get_supported_slots().unwrap(), 0b11);

        // Should reject invalid masks
        assert!(store.set_supported_slots(0b100).is_err());
    }

    #[test]
    fn test_provisioned_slots() {
        let mut store = Ast1060PeerCertStore::new();
        assert_eq!(store.get_provisioned_slots().unwrap(), 0);

        store.set_provisioned_slots(0b01).unwrap();
        assert_eq!(store.get_provisioned_slots().unwrap(), 0b01);
    }

    #[test]
    fn test_cert_chain_storage() {
        let mut store = Ast1060PeerCertStore::new();
        let test_chain = [0xAA; 100];

        store.set_cert_chain(0, &test_chain).unwrap();

        // Raw chain should include what we stored
        let raw = store.get_raw_chain(0).unwrap();
        assert_eq!(raw.len(), 100);
        assert_eq!(&raw[..], &test_chain);
    }

    #[test]
    fn test_cert_chain_too_large() {
        let mut store = Ast1060PeerCertStore::new();
        let huge_chain = [0xBB; MAX_CERT_CHAIN_SIZE + 1];

        assert!(matches!(
            store.set_cert_chain(0, &huge_chain),
            Err(CertStoreError::BufferTooSmall)
        ));
    }

    #[test]
    fn test_assemble() {
        let mut store = Ast1060PeerCertStore::new();

        let portion1 = [0xAA; 50];
        let portion2 = [0xBB; 50];

        // Assemble two portions
        let status1 = store.assemble(0, &portion1).unwrap();
        assert!(matches!(status1, ReassemblyStatus::InProgress));

        let status2 = store.assemble(0, &portion2).unwrap();
        assert!(matches!(status2, ReassemblyStatus::InProgress));

        // Verify concatenated data
        let raw = store.get_raw_chain(0).unwrap();
        assert_eq!(raw.len(), 100);
        assert_eq!(&raw[..50], &portion1);
        assert_eq!(&raw[50..], &portion2);
    }

    #[test]
    fn test_digest_storage() {
        let mut store = Ast1060PeerCertStore::new();
        let test_digest = [0xCC; SHA384_HASH_SIZE];

        // Should fail before digest is set
        assert!(store.get_digest(0).is_err());

        store.set_digest(0, &test_digest).unwrap();
        let digest = store.get_digest(0).unwrap();
        assert_eq!(digest, &test_digest);
    }

    #[test]
    fn test_keypair() {
        let mut store = Ast1060PeerCertStore::new();

        assert!(store.get_keypair(0).is_err());

        store.set_keypair(0, 42).unwrap();
        assert_eq!(store.get_keypair(0).unwrap(), 42);
    }

    #[test]
    fn test_cert_info() {
        let mut store = Ast1060PeerCertStore::new();

        assert!(store.get_cert_info(0).is_err());

        let info = CertificateInfo(0x12);
        store.set_cert_info(0, info).unwrap();
        assert_eq!(store.get_cert_info(0).unwrap(), info);
    }

    #[test]
    fn test_key_usage_mask() {
        let mut store = Ast1060PeerCertStore::new();

        assert!(store.get_key_usage_mask(0).is_err());

        let mask = KeyUsageMask::default();
        store.set_key_usage_mask(0, mask).unwrap();
        assert_eq!(store.get_key_usage_mask(0).unwrap(), mask);
    }

    #[test]
    fn test_reset() {
        let mut store = Ast1060PeerCertStore::new();

        // Populate slot 0
        store.set_cert_chain(0, &[0xAA; 100]).unwrap();
        store.set_digest(0, &[0xBB; SHA384_HASH_SIZE]).unwrap();
        store.set_keypair(0, 42).unwrap();

        // Verify data is there
        assert!(store.get_raw_chain(0).is_ok());
        assert!(store.get_digest(0).is_ok());
        assert!(store.get_keypair(0).is_ok());

        // Reset
        store.reset(0);

        // Verify data is cleared
        assert!(store.get_raw_chain(0).is_err());
        assert!(store.get_digest(0).is_err());
        assert!(store.get_keypair(0).is_err());
    }

    #[test]
    fn test_invalid_slot_id() {
        let mut store = Ast1060PeerCertStore::new();

        assert!(matches!(
            store.set_cert_chain(5, &[0xAA; 10]),
            Err(CertStoreError::InvalidSlotId(5))
        ));
    }

    #[test]
    fn test_requested_msh_type() {
        let mut store = Ast1060PeerCertStore::new();

        assert!(store.get_requested_msh_type(0).is_err());

        let msh_type = MeasurementSummaryHashType::All;
        store.set_requested_msh_type(0, msh_type).unwrap();
        assert_eq!(store.get_requested_msh_type(0).unwrap(), msh_type);
    }
}
