// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The platform of this test's SPDM responder.
//!
//! The hash and the random number generator use the Caliptra core through
//! its mailbox.
//!
//! The certificate store has one slot, which holds one ECDSA P-384 chain.
//! The chain is the certificate chain of Caliptra's DPE (the LDevID, FMC
//! alias and runtime alias certificates), followed by a leaf certificate
//! that the DPE issues with `CertifyKey`. The chain has no root CA
//! certificate and no IDevID certificate, so the root hash of the SPDM
//! certificate chain is the hash of the LDevID certificate. caliptra-mcu-sw
//! builds the same chain for a slot that has no endorsement certificates
//! (`set_dpe_only_slot` in
//! `runtime/userspace/api/spdm-lib/pal/src/cert/store.rs`).
//!
//! The store signs with the key of the leaf certificate, through the `Sign`
//! command of the DPE. The evidence source has no evidence, so the responder
//! does not advertise measurements.
//!
//! The service takes the nonce of `CHALLENGE_AUTH` from `CaliptraRandom`.

use core::cell::Cell;

use openprot_hal_blocking::certificate::{self, CertificateChain, CertificateStore, KeyAlgorithm};
use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Digest, Sha2_384};
use openprot_hal_blocking::dpe::{CertificateFormat, CertifyKey, ContextHandle, Sign};
use openprot_hal_blocking::ecdsa::{P384PublicKey, P384Signature};
use openprot_hal_blocking::rng::CryptoRng;
use spdm_lib::cert_store::{CertStoreError, CertStoreResult, SpdmCertStore};
use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};
use spdm_lib::platform::hash::{SpdmHash, SpdmHashAlgoType, SpdmHashError, SpdmHashResult};
use spdm_lib::platform::rng::{SpdmRng, SpdmRngError, SpdmRngResult};
use spdm_lib::protocol::algorithms::{AsymAlgo, ECC_P384_SIGNATURE_SIZE, SHA384_HASH_SIZE};
use spdm_lib::protocol::certs::{CertificateInfo, KeyUsageMask};
use veer_dpe::CaliptraDpe;
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

/// The slot of the certificate store.
const SLOT: u8 = 0;

/// Selects the key of the leaf certificate. caliptra-mcu-sw uses the same
/// label for the key of its SPDM responder (`DPE_LEAF_LABEL`).
const LEAF_LABEL: [u8; 48] = {
    let mut label = [0; 48];
    let mut i = 0;
    while i < label.len() {
        label[i] = i as u8;
        i += 1;
    }
    label
};

/// The leaf key: the key that the default context of Caliptra's DPE derives
/// for [`LEAF_LABEL`].
pub struct LeafKey<'m> {
    mailbox: &'m Mailbox,
    /// The handle of the context for the next command.
    handle: Cell<ContextHandle>,
}

impl<'m> LeafKey<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self {
            mailbox,
            handle: Cell::new(ContextHandle::DEFAULT),
        }
    }

    /// Asks the DPE for an X.509 certificate of the key and stores the
    /// certificate in `buf`.
    ///
    /// Returns the certificate, or `None` when the command fails or `buf` is
    /// too small.
    pub fn certify<'a>(&self, buf: &'a mut [u8]) -> Option<&'a [u8]> {
        let mut public_key = P384PublicKey::new([0; 48], [0; 48]);
        let output = CaliptraDpe::new(self.mailbox)
            .certify_key(
                &self.handle.get(),
                &LEAF_LABEL,
                CertificateFormat::X509,
                &mut public_key,
                buf,
            )
            .ok()?;
        self.handle.set(output.handle);
        buf.get(..output.certificate_len)
    }

    /// Signs `hash` with the key. Returns the signature as `r` then `s`, or
    /// `None` when the command fails.
    fn sign(&self, hash: &[u8; SHA384_HASH_SIZE]) -> Option<[u8; ECC_P384_SIGNATURE_SIZE]> {
        let digest = Digest::new(zerocopy::transmute!(*hash));
        let mut signature = P384Signature::new([0; 48], [0; 48]);
        let handle = CaliptraDpe::new(self.mailbox)
            .sign(&self.handle.get(), &LEAF_LABEL, &digest, &mut signature)
            .ok()?;
        self.handle.set(handle);
        Some(zerocopy::transmute!(signature))
    }
}

/// Errors from [`OneSlot`].
#[derive(Debug)]
pub enum OneSlotError<E> {
    /// The slot is not slot 0.
    InvalidSlot,
    /// The slot has no chain for the key algorithm.
    NotProvisioned,
    /// The chain could not be read.
    Chain(E),
}

impl<E: certificate::Error> certificate::Error for OneSlotError<E> {
    fn kind(&self) -> certificate::ErrorKind {
        match self {
            Self::InvalidSlot => certificate::ErrorKind::InvalidSlot,
            Self::NotProvisioned => certificate::ErrorKind::NotProvisioned,
            Self::Chain(error) => error.kind(),
        }
    }
}

/// Certificate store with one slot, which holds one ECDSA P-384 chain.
pub struct OneSlot<C> {
    chain: C,
}

impl<C: CertificateChain> OneSlot<C> {
    pub fn new(chain: C) -> Self {
        Self { chain }
    }

    fn chain(
        &mut self,
        slot: u8,
        algorithm: KeyAlgorithm,
    ) -> Result<&mut C, OneSlotError<C::Error>> {
        match (slot, algorithm) {
            (SLOT, KeyAlgorithm::EcdsaP384) => Ok(&mut self.chain),
            (SLOT, _) => Err(OneSlotError::NotProvisioned),
            _ => Err(OneSlotError::InvalidSlot),
        }
    }
}

impl<C: CertificateChain> certificate::ErrorType for OneSlot<C> {
    type Error = OneSlotError<C::Error>;
}

impl<C: CertificateChain> CertificateStore for OneSlot<C> {
    fn slot_count(&self) -> u8 {
        1
    }

    fn chain_len(&mut self, slot: u8, algorithm: KeyAlgorithm) -> Result<usize, Self::Error> {
        self.chain(slot, algorithm)?
            .len()
            .map_err(OneSlotError::Chain)
    }

    fn read_chain(
        &mut self,
        slot: u8,
        algorithm: KeyAlgorithm,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.chain(slot, algorithm)?
            .read(offset, buf)
            .map_err(OneSlotError::Chain)
    }
}

/// Returns the length in bytes, header included, of the DER SEQUENCE whose
/// first bytes are `start`.
fn der_sequence_len(start: &[u8]) -> Option<usize> {
    const SEQUENCE: u8 = 0x30;
    let (&tag, rest) = start.split_first()?;
    let (&first, rest) = rest.split_first()?;
    if tag != SEQUENCE {
        return None;
    }
    // Short form: the byte is the length of the content.
    if first < 0x80 {
        return Some(usize::from(first) + 2);
    }
    // Long form: the byte gives the number of length bytes that follow.
    let count = usize::from(first & 0x7f);
    if count == 0 || count > size_of::<u32>() {
        return None;
    }
    let content = rest
        .get(..count)?
        .iter()
        .fold(0u32, |len, byte| (len << 8) | u32::from(*byte));
    usize::try_from(content).ok()?.checked_add(2 + count)
}

/// The largest number of bytes of a DER header: the tag, the first length
/// byte and four more length bytes.
const DER_HEADER_MAX: usize = 6;

/// The number of bytes of a certificate that one hash command takes.
const HASH_PART: usize = 256;

/// Returns the SHA-384 hash of the first certificate of the chain that
/// [`SLOT`] of `store` holds.
fn first_certificate_hash<S: CertificateStore>(
    store: &mut S,
    mailbox: &Mailbox,
) -> Option<[u8; SHA384_HASH_SIZE]> {
    let mut read = |offset: usize, buf: &mut [u8]| {
        store
            .read_chain(SLOT, KeyAlgorithm::EcdsaP384, offset, buf)
            .ok()
    };
    let mut header = [0u8; DER_HEADER_MAX];
    let stored = read(0, &mut header)?;
    let len = der_sequence_len(header.get(..stored)?)?;

    let mut op = CaliptraSha::new(mailbox).init(Sha2_384).ok()?;
    let mut buf = [0u8; HASH_PART];
    let mut offset = 0usize;
    while offset < len {
        let part = buf.get_mut(..len.saturating_sub(offset).min(HASH_PART))?;
        if read(offset, part)? != part.len() {
            return None;
        }
        op = op.update(part).ok()?;
        offset = offset.checked_add(part.len())?;
    }
    let (digest, _) = op.finalize().ok()?;
    Some(zerocopy::transmute!(digest.value))
}

/// The certificate store of the SPDM responder: slot 0 of a HAL certificate
/// store, and the key of the last certificate of that slot's chain.
pub struct SpdmCertificates<'m, S> {
    store: S,
    key: LeafKey<'m>,
    /// SHA-384 hash of the first certificate of the chain.
    root_hash: [u8; SHA384_HASH_SIZE],
}

impl<'m, S: CertificateStore> SpdmCertificates<'m, S> {
    /// Hashes the first certificate of the chain that slot 0 of `store`
    /// holds, with the Caliptra core. `key` must be the key of the last
    /// certificate of that chain.
    ///
    /// Returns `None` when the chain cannot be read or hashed.
    pub fn new(mut store: S, key: LeafKey<'m>, mailbox: &Mailbox) -> Option<Self> {
        let root_hash = first_certificate_hash(&mut store, mailbox)?;
        Some(Self {
            store,
            key,
            root_hash,
        })
    }

    /// Returns the length in bytes of the chain.
    pub fn chain_len(&mut self) -> Option<usize> {
        self.store.chain_len(SLOT, KeyAlgorithm::EcdsaP384).ok()
    }
}

/// Returns the key algorithm of the chains that SPDM selects with
/// `asym_algo`.
fn key_algorithm(asym_algo: AsymAlgo) -> KeyAlgorithm {
    match asym_algo {
        AsymAlgo::EccP384 => KeyAlgorithm::EcdsaP384,
    }
}

/// Maps a HAL certificate store error to an SPDM error.
fn store_error<E: certificate::Error>(slot_id: u8, error: E) -> CertStoreError {
    match error.kind() {
        certificate::ErrorKind::InvalidSlot | certificate::ErrorKind::NotProvisioned => {
            CertStoreError::InvalidSlotId(slot_id)
        }
        certificate::ErrorKind::Busy
        | certificate::ErrorKind::ReadFailed
        | certificate::ErrorKind::InvalidData => CertStoreError::CertReadError,
        _ => CertStoreError::PlatformError,
    }
}

impl<S: CertificateStore> SpdmCertStore for SpdmCertificates<'_, S> {
    fn slot_count(&self) -> u8 {
        1
    }

    fn is_provisioned(&self, slot_id: u8) -> bool {
        slot_id == SLOT
    }

    fn cert_chain_len(&mut self, asym_algo: AsymAlgo, slot_id: u8) -> CertStoreResult<usize> {
        self.store
            .chain_len(slot_id, key_algorithm(asym_algo))
            .map_err(|error| store_error(slot_id, error))
    }

    fn get_cert_chain(
        &mut self,
        slot_id: u8,
        asym_algo: AsymAlgo,
        offset: usize,
        cert_portion: &mut [u8],
    ) -> CertStoreResult<usize> {
        let stored = self
            .store
            .read_chain(slot_id, key_algorithm(asym_algo), offset, cert_portion)
            .map_err(|error| store_error(slot_id, error))?;
        // spdm-lib expects zeros after the end of the chain.
        cert_portion
            .iter_mut()
            .skip(stored)
            .for_each(|byte| *byte = 0);
        Ok(stored)
    }

    fn root_cert_hash(
        &mut self,
        slot_id: u8,
        _asym_algo: AsymAlgo,
        cert_hash: &mut [u8; SHA384_HASH_SIZE],
    ) -> CertStoreResult<()> {
        if slot_id != SLOT {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        *cert_hash = self.root_hash;
        Ok(())
    }

    fn sign_hash(
        &self,
        slot_id: u8,
        hash: &[u8; SHA384_HASH_SIZE],
        signature: &mut [u8; ECC_P384_SIGNATURE_SIZE],
    ) -> CertStoreResult<()> {
        if slot_id != SLOT {
            return Err(CertStoreError::InvalidSlotId(slot_id));
        }
        *signature = self.key.sign(hash).ok_or(CertStoreError::PlatformError)?;
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
