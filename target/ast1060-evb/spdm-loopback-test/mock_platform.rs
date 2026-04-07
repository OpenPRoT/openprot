// Licensed under the Apache-2.0 license

//! Mock platform implementations for SPDM loopback testing
//!
//! Provides minimal implementations of SPDM platform traits without
//! requiring hardware or IPC dependencies.

use spdm_lib::cert_store::{CertStoreError, CertStoreResult, SpdmCertStore};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};
use spdm_lib::platform::rng::{SpdmRng, SpdmRngResult};
use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};

/// Mock certificate store with fixed placeholder data
pub struct MockCertStore;

impl MockCertStore {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmCertStore for MockCertStore {
    fn slot_count(&self) -> u8 {
        1
    }

    fn is_provisioned(&self, slot_id: u8) -> bool {
        slot_id == 0
    }

    fn cert_chain_len(&mut self, asym_algo: AsymAlgo, slot_id: u8) -> CertStoreResult<usize> {
        if slot_id != 0 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }
        Ok(32) // Mock cert chain is 32 bytes
    }

    fn get_cert_chain(
        &mut self,
        slot_id: u8,
        asym_algo: AsymAlgo,
        offset: usize,
        cert_portion: &mut [u8],
    ) -> CertStoreResult<usize> {
        if slot_id != 0 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }

        // Mock cert chain: 32 bytes of 0xAA
        const CERT_CHAIN: [u8; 32] = [0xAA; 32];

        if offset >= CERT_CHAIN.len() {
            return Err(CertStoreError::InvalidOffset);
        }

        let remaining = CERT_CHAIN.len() - offset;
        let to_copy = remaining.min(cert_portion.len());
        cert_portion[..to_copy].copy_from_slice(&CERT_CHAIN[offset..offset + to_copy]);

        // Fill remaining with zeros if any
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
        if slot_id != 0 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        if asym_algo != AsymAlgo::EccP384 {
            return Err(CertStoreError::UnsupportedHashAlgo);
        }

        // Mock root hash: 48 bytes of 0xBB (SHA-384)
        const ROOT_HASH: [u8; SHA384_HASH_SIZE] = [0xBB; SHA384_HASH_SIZE];
        cert_hash.copy_from_slice(&ROOT_HASH);
        Ok(())
    }

    fn sign_hash(
        &self,
        slot_id: u8,
        _hash: &[u8; SHA384_HASH_SIZE],
        signature: &mut [u8; ECC_P384_SIGNATURE_SIZE],
    ) -> CertStoreResult<()> {
        if slot_id != 0 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }

        // Mock signature: 96 bytes of 0xCC (ECDSA P-384)
        const SIGNATURE: [u8; ECC_P384_SIGNATURE_SIZE] = [0xCC; ECC_P384_SIGNATURE_SIZE];
        signature.copy_from_slice(&SIGNATURE);
        Ok(())
    }

    fn key_pair_id(&self, _slot_id: u8) -> Option<u8> {
        None
    }

    fn cert_info(&self, _slot_id: u8) -> Option<CertificateInfo> {
        None
    }

    fn key_usage_mask(&self, _slot_id: u8) -> Option<KeyUsageMask> {
        None
    }
}

/// Mock hash implementation
pub struct MockHash {
    buffer: [u8; 256],
    len: usize,
    algo: Option<SpdmHashAlgoType>,
}

impl MockHash {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; 256],
            len: 0,
            algo: None,
        }
    }
}

impl SpdmHash for MockHash {
    fn init(&mut self, algo: SpdmHashAlgoType, _secret: Option<&[u8]>) -> SpdmHashResult<()> {
        self.len = 0;
        self.algo = Some(algo);
        Ok(())
    }

    fn update(&mut self, data: &[u8]) -> SpdmHashResult<()> {
        if self.len + data.len() > self.buffer.len() {
            return Err(SpdmHashError::BufferTooSmall);
        }
        self.buffer[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        Ok(())
    }

    fn finalize(&mut self, dest: &mut [u8]) -> SpdmHashResult<()> {
        let hash_size = match self.algo {
            Some(SpdmHashAlgoType::SHA384) => 48,
            Some(SpdmHashAlgoType::SHA512) => 64,
            _ => return Err(SpdmHashError::InvalidAlgorithm),
        };

        if dest.len() < hash_size {
            return Err(SpdmHashError::BufferTooSmall);
        }

        // Simple mock hash: XOR all input bytes and repeat
        let mut hash_byte = 0u8;
        for i in 0..self.len {
            hash_byte ^= self.buffer[i];
        }
        dest[..hash_size].fill(hash_byte);

        Ok(())
    }

    fn hash(&mut self, algo: SpdmHashAlgoType, data: &[u8], dest: &mut [u8]) -> SpdmHashResult<()> {
        self.init(algo, None)?;
        self.update(data)?;
        self.finalize(dest)
    }

    fn reset(&mut self) {
        self.len = 0;
        self.algo = None;
    }

    fn algo(&self) -> SpdmHashAlgoType {
        self.algo.unwrap_or(SpdmHashAlgoType::SHA384)
    }
}

/// Mock RNG implementation
pub struct MockRng;

impl MockRng {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmRng for MockRng {
    fn get_random_bytes(&mut self, buf: &mut [u8]) -> SpdmRngResult<()> {
        // Mock: fill with incrementing pattern
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i & 0xFF) as u8;
        }
        Ok(())
    }

    fn generate_random_number(&mut self, random_number: &mut [u8]) -> SpdmRngResult<()> {
        // Mock: fill with incrementing pattern starting from 0x42
        for (i, byte) in random_number.iter_mut().enumerate() {
            *byte = ((i + 0x42) & 0xFF) as u8;
        }
        Ok(())
    }
}

/// Mock evidence implementation
pub struct MockEvidence;

impl MockEvidence {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmEvidence for MockEvidence {
    fn pcr_quote_size(&self, _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        // Mock: 2 measurements
        // Format: count(1) + [index(1) + size(2) + data]*2
        Ok(1 + (1 + 2 + 23) + (1 + 2 + 20))
    }

    fn pcr_quote(&self, dest: &mut [u8], _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        let required_size = self.pcr_quote_size(false)?;
        if dest.len() < required_size {
            return Err(SpdmEvidenceError::InvalidEvidenceFormat);
        }

        let mut offset = 0;

        // Measurement count
        dest[offset] = 2;
        offset += 1;

        // Measurement 0
        dest[offset] = 0; // index
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&23u16.to_le_bytes()); // size
        offset += 2;
        dest[offset..offset + 23].copy_from_slice(b"OpenPRoT SPDM Loopback");
        offset += 23;

        // Measurement 1
        dest[offset] = 1; // index
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&20u16.to_le_bytes()); // size
        offset += 2;
        dest[offset..offset + 20].copy_from_slice(b"MCTP Loopback Test  ");
        offset += 20;

        Ok(offset)
    }
}
