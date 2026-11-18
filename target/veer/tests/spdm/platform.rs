// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The platform of this test's SPDM responder.
//!
//! The hash and the random number generator use the Caliptra core through
//! its mailbox. The certificate store has no slots and the evidence source
//! has no evidence, so the responder does not advertise certificates,
//! challenges or measurements.
//!
//! `GET_VERSION`, `GET_CAPABILITIES` and `NEGOTIATE_ALGORITHMS` need no hash
//! and no random number, so this test does not call `CaliptraHash` or
//! `CaliptraRandom`.

use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::Sha2_384;
use openprot_hal_blocking::rng::CryptoRng;
use spdm_lib::cert_store::{CertStoreError, CertStoreResult, SpdmCertStore};
use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};
use spdm_lib::platform::rng::{SpdmRng, SpdmRngError, SpdmRngResult};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use veer_mailbox::Mailbox;
use veer_rng::CaliptraRng;
use veer_sha::{CaliptraSha, OwnedSha2Op};

/// SHA-384 that uses the Caliptra core.
pub struct CaliptraHash<'m> {
    mailbox: &'m Mailbox,
    /// The computation in progress.
    op: Option<OwnedSha2Op<'m, 12>>,
}

impl<'m> CaliptraHash<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self { mailbox, op: None }
    }
}

impl SpdmHash for CaliptraHash<'_> {
    fn hash(
        &mut self,
        hash_algo: SpdmHashAlgoType,
        data: &[u8],
        hash: &mut [u8],
    ) -> SpdmHashResult<()> {
        self.init(hash_algo, Some(data))?;
        self.finalize(hash)
    }

    fn init(&mut self, hash_algo: SpdmHashAlgoType, data: Option<&[u8]>) -> SpdmHashResult<()> {
        self.op = None;
        if hash_algo != SpdmHashAlgoType::SHA384 {
            return Err(SpdmHashError::InvalidAlgorithm);
        }
        let op = CaliptraSha::new(self.mailbox)
            .init(Sha2_384)
            .map_err(|_| SpdmHashError::PlatformError)?;
        let op = match data {
            Some(data) => op.update(data).map_err(|_| SpdmHashError::PlatformError)?,
            None => op,
        };
        self.op = Some(op);
        Ok(())
    }

    fn update(&mut self, data: &[u8]) -> SpdmHashResult<()> {
        let op = self.op.take().ok_or(SpdmHashError::PlatformError)?;
        self.op = Some(op.update(data).map_err(|_| SpdmHashError::PlatformError)?);
        Ok(())
    }

    fn finalize(&mut self, hash: &mut [u8]) -> SpdmHashResult<()> {
        let op = self.op.take().ok_or(SpdmHashError::PlatformError)?;
        let (digest, _) = op.finalize().map_err(|_| SpdmHashError::PlatformError)?;
        let digest: [u8; SHA384_HASH_SIZE] = zerocopy::transmute!(digest.value);
        let hash = hash
            .get_mut(..SHA384_HASH_SIZE)
            .ok_or(SpdmHashError::BufferTooSmall)?;
        hash.iter_mut()
            .zip(digest)
            .for_each(|(to, from)| *to = from);
        Ok(())
    }

    fn reset(&mut self) {
        self.op = None;
    }

    fn algo(&self) -> SpdmHashAlgoType {
        SpdmHashAlgoType::SHA384
    }
}

/// Random number generator that uses the Caliptra core.
pub struct CaliptraRandom<'m> {
    rng: CaliptraRng<'m>,
}

impl<'m> CaliptraRandom<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self {
            rng: CaliptraRng::new(mailbox),
        }
    }
}

impl SpdmRng for CaliptraRandom<'_> {
    fn get_random_bytes(&mut self, buf: &mut [u8]) -> SpdmRngResult<()> {
        // `SpdmRngError` has no variant for a failure of the generator, so
        // the failure is reported as `InvalidSize`.
        self.rng.fill(buf).map_err(|_| SpdmRngError::InvalidSize)
    }

    fn generate_random_number(&mut self, random_number: &mut [u8]) -> SpdmRngResult<()> {
        self.get_random_bytes(random_number)
    }
}

/// Certificate store with no slots.
pub struct NoCertStore;

impl SpdmCertStore for NoCertStore {
    fn slot_count(&self) -> u8 {
        0
    }

    fn is_provisioned(&self, _slot_id: u8) -> bool {
        false
    }

    fn cert_chain_len(&mut self, _asym_algo: AsymAlgo, slot_id: u8) -> CertStoreResult<usize> {
        Err(CertStoreError::InvalidSlotId(slot_id))
    }

    fn get_cert_chain(
        &mut self,
        slot_id: u8,
        _asym_algo: AsymAlgo,
        _offset: usize,
        _cert_portion: &mut [u8],
    ) -> CertStoreResult<usize> {
        Err(CertStoreError::InvalidSlotId(slot_id))
    }

    fn root_cert_hash(
        &mut self,
        slot_id: u8,
        _asym_algo: AsymAlgo,
        _cert_hash: &mut [u8; SHA384_HASH_SIZE],
    ) -> CertStoreResult<()> {
        Err(CertStoreError::InvalidSlotId(slot_id))
    }

    fn sign_hash(
        &self,
        slot_id: u8,
        _hash: &[u8; SHA384_HASH_SIZE],
        _signature: &mut [u8; ECC_P384_SIGNATURE_SIZE],
    ) -> CertStoreResult<()> {
        Err(CertStoreError::InvalidSlotId(slot_id))
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

/// Evidence source with no evidence.
pub struct NoEvidence;

impl SpdmEvidence for NoEvidence {
    fn pcr_quote(&self, _buffer: &mut [u8], _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        Err(SpdmEvidenceError::UnsupportedEvidenceType)
    }

    fn pcr_quote_size(&self, _with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        Err(SpdmEvidenceError::UnsupportedEvidenceType)
    }
}
