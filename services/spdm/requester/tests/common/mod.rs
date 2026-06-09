// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared test fixtures for SPDM host integration tests.
//!
//! Provides:
//! - [`BufferSender`] — captures outbound packets into a `Vec`
//! - [`DirectClient`] — implements `MctpClient` directly against a `Server`
//! - [`MockCertStore`] — mock certificate store for SPDM
//! - [`MockHash`] — mock hash implementation
//! - [`MockRng`] — mock RNG implementation
//! - [`MockEvidence`] — mock evidence/measurements provider
//! - [`DemoPeerCertStore`] — peer certificate store for requester

#![allow(dead_code)]

use std::cell::RefCell;

use mctp::{Eid, Tag};
use mctp_lib::fragment::{Fragmenter, SendOutput};
use mctp_lib::Sender;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::Server;

use spdm_lib::cert_store::{CertStoreError, CertStoreResult, PeerCertStore, SpdmCertStore};
use spdm_lib::commands::challenge::MeasurementSummaryHashType;
use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};
use spdm_lib::platform::rng::{SpdmRng, SpdmRngResult};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use spdm_lib::protocol::{BaseHashAlgoType, SpdmCertChainHeader};
use zerocopy::FromBytes;

// ---------------------------------------------------------------------------
// BufferSender — captures outbound MCTP packets
// ---------------------------------------------------------------------------

/// A mock [`Sender`] that captures every outbound MCTP packet into a shared buffer.
pub struct BufferSender<'a> {
    pub packets: &'a RefCell<Vec<Vec<u8>>>,
}

/// MTU for MCTP payload (without header)
const MCTP_MTU: usize = 255;
/// MCTP header size (4 bytes)
const MCTP_HEADER_SIZE: usize = 4;

impl Sender for BufferSender<'_> {
    fn send_vectored(
        &mut self,
        mut fragmenter: Fragmenter,
        payload: &[&[u8]],
    ) -> mctp::Result<Tag> {
        loop {
            // Buffer must be MTU + header size
            let mut buf = [0u8; MCTP_MTU + MCTP_HEADER_SIZE];
            match fragmenter.fragment_vectored(payload, &mut buf) {
                SendOutput::Packet(p) => {
                    self.packets.borrow_mut().push(p.to_vec());
                }
                SendOutput::Complete { tag, .. } => return Ok(tag),
                SendOutput::Error { err, .. } => return Err(err),
            }
        }
    }

    fn get_mtu(&self) -> usize {
        MCTP_MTU
    }
}

// ---------------------------------------------------------------------------
// transfer — moves packets between servers
// ---------------------------------------------------------------------------

/// Drain `packets` into `dest` as inbound MCTP packets.
pub fn transfer<S: Sender, const N: usize>(
    packets: &RefCell<Vec<Vec<u8>>>,
    dest: &mut Server<S, N>,
) {
    let pkts = packets.borrow();
    for pkt in pkts.iter() {
        dest.inbound(pkt).unwrap();
    }
}

// ---------------------------------------------------------------------------
// DirectClient — implements MctpClient against a Server
// ---------------------------------------------------------------------------

/// Implements [`MctpClient`] by calling [`Server`] methods directly.
pub struct DirectClient<'a, S: Sender, const N: usize> {
    pub server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: Sender, const N: usize> DirectClient<'a, S, N> {
    pub fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: Sender, const N: usize> MctpClient for DirectClient<'_, S, N> {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().req(eid)
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        self.server.borrow_mut().listener(msg_type)
    }

    fn get_eid(&self) -> u8 {
        self.server.borrow().get_eid()
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        self.server.borrow_mut().set_eid(eid)
    }

    fn recv(
        &self,
        handle: Handle,
        _timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        self.server
            .borrow_mut()
            .try_recv(handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        self.server
            .borrow_mut()
            .send(handle, msg_type, eid, tag, integrity_check, buf)
    }

    fn drop_handle(&self, handle: Handle) {
        let _ = self.server.borrow_mut().unbind(handle);
    }
}

// ---------------------------------------------------------------------------
// make_server helper
// ---------------------------------------------------------------------------

/// Construct a `Server` + its outbound packet buffer.
pub fn make_server(eid: u8, packets: &RefCell<Vec<Vec<u8>>>) -> Server<BufferSender<'_>, 16> {
    Server::new(Eid(eid), 0, BufferSender { packets })
}

// ---------------------------------------------------------------------------
// MockCertStore — mock certificate store
// ---------------------------------------------------------------------------

/// Mock certificate store with fixed placeholder data.
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
// MockHash — mock hash implementation
// ---------------------------------------------------------------------------

/// Mock hash implementation that XORs all input bytes.
pub struct MockHash {
    buffer: Vec<u8>,
    algo: Option<SpdmHashAlgoType>,
}

impl MockHash {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            algo: None,
        }
    }
}

impl SpdmHash for MockHash {
    fn init(&mut self, algo: SpdmHashAlgoType, _secret: Option<&[u8]>) -> SpdmHashResult<()> {
        self.buffer.clear();
        self.algo = Some(algo);
        Ok(())
    }

    fn update(&mut self, data: &[u8]) -> SpdmHashResult<()> {
        self.buffer.extend_from_slice(data);
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
        for byte in &self.buffer {
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
        self.buffer.clear();
        self.algo = None;
    }

    fn algo(&self) -> SpdmHashAlgoType {
        self.algo.unwrap_or(SpdmHashAlgoType::SHA384)
    }
}

// ---------------------------------------------------------------------------
// MockRng — mock RNG implementation
// ---------------------------------------------------------------------------

/// Mock RNG that produces deterministic incrementing patterns.
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
// MockEvidence — mock evidence/measurements provider
// ---------------------------------------------------------------------------

/// Mock evidence implementation with fixed measurements.
pub struct MockEvidence;

impl MockEvidence {
    pub fn new() -> Self {
        Self
    }
}

impl SpdmEvidence for MockEvidence {
    fn pcr_quote_size(&self, _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
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
        dest[offset] = 0;
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&23u16.to_le_bytes());
        offset += 2;
        dest[offset..offset + 23].copy_from_slice(b"OpenPRoT SPDM Loopback");
        offset += 23;

        // Measurement 1
        dest[offset] = 1;
        offset += 1;
        dest[offset..offset + 2].copy_from_slice(&20u16.to_le_bytes());
        offset += 2;
        dest[offset..offset + 20].copy_from_slice(b"MCTP Loopback Test  ");
        offset += 20;

        Ok(offset)
    }
}

// ---------------------------------------------------------------------------
// PeerSlot — peer certificate slot data
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct PeerSlot {
    pub cert_chain: Vec<u8>,
    pub digest: Vec<u8>,
    pub keypair_id: Option<u8>,
    pub certificate_info: Option<CertificateInfo>,
    pub key_usage_mask: Option<KeyUsageMask>,
    pub requested_msh_type: Option<MeasurementSummaryHashType>,
}

impl PeerSlot {
    fn get_root_hash(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        let (length, rest) = SpdmCertChainHeader::ref_from_prefix(&self.cert_chain).ok()?;
        if length.get_length() != self.cert_chain.len() as u32 {
            return None;
        }
        Some(&rest[..hash_algo.hash_byte_size()])
    }

    fn get_cert_chain(&self, hash_algo: BaseHashAlgoType) -> Option<&[u8]> {
        let (length, rest) = SpdmCertChainHeader::ref_from_prefix(&self.cert_chain).ok()?;
        if length.get_length() != self.cert_chain.len() as u32 {
            return None;
        }
        Some(&rest[hash_algo.hash_byte_size()..])
    }
}

// ---------------------------------------------------------------------------
// DemoPeerCertStore — peer certificate store for requester
// ---------------------------------------------------------------------------

/// Peer certificate store for SPDM requester (stores responder's certificates).
#[derive(Debug, Default)]
pub struct DemoPeerCertStore {
    supported_slots_mask: u8,
    provisioned_slots_mask: u8,
    pub peer_slots: Vec<Option<PeerSlot>>,
}

impl DemoPeerCertStore {
    pub fn new() -> Self {
        let slots = vec![None];
        Self {
            supported_slots_mask: 0,
            provisioned_slots_mask: 0,
            peer_slots: slots,
        }
    }
}

impl PeerCertStore for DemoPeerCertStore {
    fn slot_count(&self) -> u8 {
        self.peer_slots.len() as u8
    }

    fn assemble(
        &mut self,
        slot_id: u8,
        portion: &[u8],
    ) -> Result<spdm_lib::cert_store::ReassemblyStatus, CertStoreError> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;

        slot.cert_chain.extend_from_slice(portion);
        Ok(spdm_lib::cert_store::ReassemblyStatus::InProgress)
    }

    fn reset(&mut self, slot_id: u8) {
        if let Some(Some(slot)) = self.peer_slots.get_mut(slot_id as usize) {
            *slot = PeerSlot::default();
        }
    }

    fn get_raw_chain(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        Ok(&slot.cert_chain)
    }

    fn get_cert_chain(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        slot.get_cert_chain(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn set_supported_slots(&mut self, slot_mask: u8) -> CertStoreResult<()> {
        for b in 0..8 {
            if slot_mask & (1 << b) != 0
                && let Some(slot) = self.peer_slots.get_mut(b as usize)
                && slot.is_none()
            {
                *slot = Some(PeerSlot::default());
            }
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
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;

        slot.cert_chain = cert_chain.to_vec();
        Ok(())
    }

    fn get_digest(&self, slot_id: u8) -> CertStoreResult<&[u8]> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        Ok(&slot.digest)
    }

    fn set_digest(&mut self, slot_id: u8, digest: &[u8]) -> CertStoreResult<()> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;
        slot.digest = digest.to_vec();
        Ok(())
    }

    fn get_cert_info(&self, slot_id: u8) -> CertStoreResult<CertificateInfo> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        slot.certificate_info
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_cert_info(&mut self, slot_id: u8, cert_info: CertificateInfo) -> CertStoreResult<()> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;
        slot.certificate_info = Some(cert_info);
        Ok(())
    }

    fn get_key_usage_mask(&self, slot_id: u8) -> CertStoreResult<KeyUsageMask> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        slot.key_usage_mask
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_key_usage_mask(
        &mut self,
        slot_id: u8,
        key_usage_mask: KeyUsageMask,
    ) -> CertStoreResult<()> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;
        slot.key_usage_mask = Some(key_usage_mask);
        Ok(())
    }

    fn get_keypair(&self, slot_id: u8) -> CertStoreResult<u8> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        slot.keypair_id
            .ok_or(CertStoreError::InvalidSlotId(slot_id))
    }

    fn set_keypair(&mut self, slot_id: u8, keypair: u8) -> CertStoreResult<()> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;
        slot.keypair_id = Some(keypair);
        Ok(())
    }

    fn get_root_hash(&self, slot_id: u8, hash_algo: BaseHashAlgoType) -> CertStoreResult<&[u8]> {
        let slot = self
            .peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?;
        slot.get_root_hash(hash_algo)
            .ok_or(CertStoreError::CertReadError)
    }

    fn get_requested_msh_type(&self, slot_id: u8) -> CertStoreResult<MeasurementSummaryHashType> {
        self.peer_slots
            .get(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_ref()
            .ok_or(CertStoreError::PlatformError)?
            .requested_msh_type
            .clone()
            .ok_or(CertStoreError::Undefined)
    }

    fn set_requested_msh_type(
        &mut self,
        slot_id: u8,
        msh_type: MeasurementSummaryHashType,
    ) -> CertStoreResult<()> {
        let slot = self
            .peer_slots
            .get_mut(slot_id as usize)
            .ok_or(CertStoreError::InvalidSlotId(slot_id))?
            .as_mut()
            .ok_or(CertStoreError::PlatformError)?;
        slot.requested_msh_type = Some(msh_type);
        Ok(())
    }
}
