// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! no_std mock platform implementations for the SPDM auth stress test.

use spdm_lib::cert_store::{CertStoreError, CertStoreResult, SpdmCertStore};
use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};
use spdm_lib::platform::rng::{SpdmRng, SpdmRngResult};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};

// ---------------------------------------------------------------------------
// MockCertStore
// ---------------------------------------------------------------------------

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
        Ok(32)
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

        const CERT_CHAIN: [u8; 32] = [0xAA; 32];

        if offset >= CERT_CHAIN.len() {
            return Err(CertStoreError::InvalidOffset);
        }

        let remaining = CERT_CHAIN.len() - offset;
        let to_copy = remaining.min(cert_portion.len());
        cert_portion[..to_copy].copy_from_slice(&CERT_CHAIN[offset..offset + to_copy]);

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

// ---------------------------------------------------------------------------
// MockHash
// ---------------------------------------------------------------------------

const HASH_BUF_SIZE: usize = 4096;

pub struct MockHash {
    buffer: [u8; HASH_BUF_SIZE],
    len: usize,
    algo: Option<SpdmHashAlgoType>,
}

impl MockHash {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; HASH_BUF_SIZE],
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
        let remaining = HASH_BUF_SIZE - self.len;
        let to_copy = data.len().min(remaining);
        self.buffer[self.len..self.len + to_copy].copy_from_slice(&data[..to_copy]);
        self.len += to_copy;
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

        let mut hash_byte = 0u8;
        for byte in &self.buffer[..self.len] {
            hash_byte ^= byte;
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

// ---------------------------------------------------------------------------
// MockRng
// ---------------------------------------------------------------------------

pub struct MockRng;

impl MockRng {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmRng for MockRng {
    fn get_random_bytes(&mut self, buf: &mut [u8]) -> SpdmRngResult<()> {
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i & 0xFF) as u8;
        }
        Ok(())
    }

    fn generate_random_number(&mut self, random_number: &mut [u8]) -> SpdmRngResult<()> {
        for (i, byte) in random_number.iter_mut().enumerate() {
            *byte = ((i + 0x42) & 0xFF) as u8;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MockEvidence
// ---------------------------------------------------------------------------

pub struct MockEvidence;

impl MockEvidence {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmEvidence for MockEvidence {
    fn pcr_quote_size(&self, _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        Ok(1 + (1 + 2 + 23) + (1 + 2 + 20))
    }

    fn pcr_quote(&self, dest: &mut [u8], _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        let required_size = self.pcr_quote_size(false)?;
        if dest.len() < required_size {
            return Err(SpdmEvidenceError::InvalidEvidenceFormat);
        }

        let mut offset = 0;

        dest[offset] = 2;
        offset += 1;

        dest[offset] = 0;
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&23u16.to_le_bytes());
        offset += 2;
        dest[offset..offset + 23].copy_from_slice(b"OpenPRoT SPDM Loopback");
        offset += 23;

        dest[offset] = 1;
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&20u16.to_le_bytes());
        offset += 2;
        dest[offset..offset + 20].copy_from_slice(b"MCTP Loopback Test  ");
        offset += 20;

        Ok(offset)
    }
}
