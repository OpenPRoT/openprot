// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! ECDSA P-384 signature verification that uses the Caliptra core.
//!
//! This driver sends the `ECDSA384_SIGNATURE_VERIFY` mailbox command. The
//! request is defined in the caliptra-sw `caliptra-api` crate
//! (`api/src/mailbox.rs`). This driver imports the command code and the size
//! of the request from that crate, and the error codes from the
//! `caliptra-error` crate.
//!
//! The driver implements the `EcdsaVerify` HAL trait for the P-384 curve.
//! The caller passes the SHA-384 digest of the message. Caliptra does not
//! hash the message.
//!
//! # Errors
//!
//! The driver reports `CaliptraEcdsaError::InvalidSignature` for two error
//! codes of the mailbox command. Caliptra returns
//! `RUNTIME_ECDSA_VERIFY_FAILED` when the signature does not verify, and
//! `DRIVER_ECC384_SCALAR_RANGE_CHECK_FAILED` when `r` or `s` is zero or not
//! less than the order of the curve.
//!
//! The driver reports `CaliptraEcdsaError::InvalidPoint` for
//! `DRIVER_ECC384_HW_ERROR`, which Caliptra returns when its ECC hardware
//! raises an error. In `ecc_dsa_ctrl.sv` of caliptra-rtl, a verify operation
//! raises an error in four cases. Two are about the public key: a coordinate
//! is not less than the prime of the curve, or the point is not on the
//! curve. The VCK190 FPGA returns `DRIVER_ECC384_HW_ERROR` for both. The
//! other two are an `r` or `s` that is out of range and a verify operation
//! with PCR signing selected. The hardware does not raise them for this
//! command: Caliptra's firmware checks the range of `r` and `s` before it
//! writes them to the ECC hardware and it does not select PCR signing. The
//! Caliptra emulator panics when it gets a public key that is not a point
//! on the curve.
//!
//! Every other failure of the command is a `CaliptraEcdsaError::Mailbox`
//! error.

#![no_std]

use caliptra_api::mailbox::{CommandId, EcdsaVerifyReq, MailboxReqHeader, MailboxRespHeader};
use caliptra_error::CaliptraError;
use openprot_hal_blocking::digest::Digest;
use openprot_hal_blocking::ecdsa::{
    EcdsaVerify, Error, ErrorKind, ErrorType, P384PublicKey, P384Signature, PublicKey, Signature,
    P384,
};
use veer_mailbox::{Mailbox, MailboxError};

const ECDSA384_SIGNATURE_VERIFY: u32 = CommandId::ECDSA384_SIGNATURE_VERIFY.0;

/// The signature is not a signature of the digest by the key.
const VERIFY_FAILED: u32 = CaliptraError::RUNTIME_ECDSA_VERIFY_FAILED.0.get();

/// `r` or `s` is zero or not less than the order of the curve.
const SCALAR_RANGE_CHECK_FAILED: u32 = CaliptraError::DRIVER_ECC384_SCALAR_RANGE_CHECK_FAILED
    .0
    .get();

/// The ECC hardware of Caliptra raised an error.
const HW_ERROR: u32 = CaliptraError::DRIVER_ECC384_HW_ERROR.0.get();

/// Size in words of the public key and the signature in a request: `x`, `y`,
/// `r` and `s`, 48 bytes each.
const KEY_AND_SIGNATURE_WORDS: usize = size_of::<[[u8; 48]; 4]>() / 4;

/// Size in words of the digest in a request.
const DIGEST_WORDS: usize = 12;

/// Size in words of a response: the checksum and the FIPS status.
const RESPONSE_WORDS: usize = size_of::<MailboxRespHeader>() / 4;

// The body of the request is the public key, the signature and the digest.
const _: () = assert!(
    size_of::<EcdsaVerifyReq>()
        == size_of::<MailboxReqHeader>() + (KEY_AND_SIGNATURE_WORDS + DIGEST_WORDS) * 4
);

/// Errors from the driver.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaliptraEcdsaError {
    /// The signature is not a signature of the digest by the key.
    InvalidSignature,
    /// The public key is not a point on the curve.
    InvalidPoint,
    /// The mailbox command failed for another reason.
    Mailbox(MailboxError),
    /// The response is shorter than a response header.
    InvalidResponse,
}

impl Error for CaliptraEcdsaError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::InvalidSignature => ErrorKind::InvalidSignature,
            Self::InvalidPoint => ErrorKind::InvalidPoint,
            Self::Mailbox(MailboxError::Busy) => ErrorKind::Busy,
            _ => ErrorKind::Other,
        }
    }
}

/// ECDSA verifier that uses the Caliptra core through its mailbox.
pub struct CaliptraEcdsa<'m> {
    mailbox: &'m Mailbox,
}

impl<'m> CaliptraEcdsa<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self { mailbox }
    }
}

impl ErrorType for CaliptraEcdsa<'_> {
    type Error = CaliptraEcdsaError;
}

impl EcdsaVerify<P384> for CaliptraEcdsa<'_> {
    type PublicKey = P384PublicKey;
    type Signature = P384Signature;

    /// Returns `Ok` only when Caliptra reports that `signature` is a
    /// signature of `digest` by `public_key`.
    fn verify(
        &mut self,
        public_key: &P384PublicKey,
        digest: Digest<DIGEST_WORDS>,
        signature: &P384Signature,
    ) -> Result<(), Self::Error> {
        let mut x = [0; 48];
        let mut y = [0; 48];
        public_key.coordinates(&mut x, &mut y);
        let mut r = [0; 48];
        let mut s = [0; 48];
        signature.coordinates(&mut r, &mut s);
        let key_and_signature: [u32; KEY_AND_SIGNATURE_WORDS] = zerocopy::transmute!([x, y, r, s]);
        let request = key_and_signature.into_iter().chain(digest.value);

        let mut response = [0u32; RESPONSE_WORDS];
        match self
            .mailbox
            .execute(ECDSA384_SIGNATURE_VERIFY, request, &mut response)
        {
            Ok(len) if len >= size_of::<MailboxRespHeader>() => Ok(()),
            Ok(_) => Err(CaliptraEcdsaError::InvalidResponse),
            Err(MailboxError::CommandFailed(VERIFY_FAILED | SCALAR_RANGE_CHECK_FAILED)) => {
                Err(CaliptraEcdsaError::InvalidSignature)
            }
            Err(MailboxError::CommandFailed(HW_ERROR)) => Err(CaliptraEcdsaError::InvalidPoint),
            Err(error) => Err(CaliptraEcdsaError::Mailbox(error)),
        }
    }
}
