// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! no_std mock peer certificate store for the SPDM VCA stress test.
//!
//! Requester-only: the responder's `SpdmResponder::new` takes no peer cert
//! store, so this lives outside the shared `mock_platform` module.

use spdm_lib::cert_store::{CertStoreError, CertStoreResult, PeerCertStore};
use spdm_lib::commands::challenge::MeasurementSummaryHashType;
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use spdm_lib::protocol::{BaseHashAlgoType, SpdmCertChainHeader};
use zerocopy::FromBytes;

const MAX_CERT_CHAIN_SIZE: usize = 512;
const MAX_DIGEST_SIZE: usize = 64;

pub struct PeerSlot {
    cert_chain: [u8; MAX_CERT_CHAIN_SIZE],
    cert_chain_len: usize,
    digest: [u8; MAX_DIGEST_SIZE],
    digest_len: usize,
    keypair_id: Option<u8>,
    certificate_info: Option<CertificateInfo>,
    key_usage_mask: Option<KeyUsageMask>,
    requested_msh_type: Option<MeasurementSummaryHashType>,
}

impl PeerSlot {
    const fn empty() -> Self {
        Self {
            cert_chain: [0u8; MAX_CERT_CHAIN_SIZE],
            cert_chain_len: 0,
            digest: [0u8; MAX_DIGEST_SIZE],
            digest_len: 0,
            keypair_id: None,
            certificate_info: None,
            key_usage_mask: None,
            requested_msh_type: None,
        }
    }

    fn get_root_hash(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        let chain = &self.cert_chain[..self.cert_chain_len];
        let (_, rest) = SpdmCertChainHeader::ref_from_prefix(chain).ok()?;
        Some(&rest[..hash_algo.hash_byte_size()])
    }

    fn get_cert_chain_data(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        let chain = &self.cert_chain[..self.cert_chain_len];
        let (_, rest) = SpdmCertChainHeader::ref_from_prefix(chain).ok()?;
        Some(&rest[hash_algo.hash_byte_size()..])
    }
}

pub struct MockPeerCertStore {
    slot: PeerSlot,
    supported_slots_mask: u8,
    provisioned_slots_mask: u8,
    occupied: bool,
}

impl MockPeerCertStore {
    pub fn new() -> Self {
        Self {
            slot: PeerSlot::empty(),
            supported_slots_mask: 0,
            provisioned_slots_mask: 0,
            occupied: false,
        }
    }
}

impl PeerCertStore for MockPeerCertStore {
    fn slot_count(&self) -> u8 {
        1
    }

    fn assemble(
        &mut self,
        slot_id: u8,
        portion: &[u8],
    ) -> Result<spdm_lib::cert_store::ReassemblyStatus, CertStoreError> {
        if slot_id != 0 {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        if !self.occupied {
            return Err(CertStoreError::PlatformError);
        }
        let remaining = MAX_CERT_CHAIN_SIZE - self.slot.cert_chain_len;
        let to_copy = portion.len().min(remaining);
        self.slot.cert_chain[self.slot.cert_chain_len..self.slot.cert_chain_len + to_copy]
            .copy_from_slice(&portion[..to_copy]);
        self.slot.cert_chain_len += to_copy;
        Ok(spdm_lib::cert_store::ReassemblyStatus::InProgress)
    }

    fn reset(&mut self, slot_id: u8) {
        if slot_id == 0 && self.occupied {
            self.slot = PeerSlot::empty();
        }
    }

    fn get_raw_chain(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        Ok(&self.slot.cert_chain[..self.slot.cert_chain_len])
    }

    fn get_cert_chain(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .get_cert_chain_data(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn set_supported_slots(&mut self, slot_mask: u8) -> CertStoreResult<()> {
        if slot_mask & 1 != 0 {
            self.occupied = true;
        }
        self.supported_slots_mask = slot_mask;
        Ok(())
    }

    fn get_supported_slots(&self) -> CertStoreResult<u8> {
        Ok(self.supported_slots_mask)
    }

    fn set_provisioned_slots(&mut self, provisioned_slot_mask: u8) -> CertStoreResult<()> {
        self.provisioned_slots_mask = provisioned_slot_mask;
        Ok(())
    }

    fn get_provisioned_slots(&self) -> CertStoreResult<u8> {
        Ok(self.provisioned_slots_mask)
    }

    fn set_cert_chain(&mut self, slot_id: u8, cert_chain: &[u8]) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        let to_copy = cert_chain.len().min(MAX_CERT_CHAIN_SIZE);
        self.slot.cert_chain[..to_copy].copy_from_slice(&cert_chain[..to_copy]);
        self.slot.cert_chain_len = to_copy;
        Ok(())
    }

    fn get_digest(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        Ok(&self.slot.digest[..self.slot.digest_len])
    }

    fn set_digest(&mut self, slot_id: u8, digest: &[u8]) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        let to_copy = digest.len().min(MAX_DIGEST_SIZE);
        self.slot.digest[..to_copy].copy_from_slice(&digest[..to_copy]);
        self.slot.digest_len = to_copy;
        Ok(())
    }

    fn get_cert_info(&self, slot_id: u8) -> CertStoreResult<CertificateInfo> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .certificate_info
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_cert_info(&mut self, slot_id: u8, cert_info: CertificateInfo) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot.certificate_info = Some(cert_info);
        Ok(())
    }

    fn get_key_usage_mask(&self, slot_id: u8) -> CertStoreResult<KeyUsageMask> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .key_usage_mask
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_key_usage_mask(
        &mut self,
        slot_id: u8,
        key_usage_mask: KeyUsageMask,
    ) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot.key_usage_mask = Some(key_usage_mask);
        Ok(())
    }

    fn get_keypair(&self, slot_id: u8) -> CertStoreResult<u8> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .keypair_id
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_keypair(&mut self, slot_id: u8, keypair: u8) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot.keypair_id = Some(keypair);
        Ok(())
    }

    fn get_root_hash(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .get_root_hash(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn get_requested_msh_type(&self, slot_id: u8) -> CertStoreResult<MeasurementSummaryHashType> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot
            .requested_msh_type
            .clone()
            .ok_or(CertStoreError::Undefined)
    }

    fn set_requested_msh_type(
        &mut self,
        slot_id: u8,
        msh_type: MeasurementSummaryHashType,
    ) -> CertStoreResult<()> {
        if slot_id != 0 || !self.occupied {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        self.slot.requested_msh_type = Some(msh_type);
        Ok(())
    }
}
