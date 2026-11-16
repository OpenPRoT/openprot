// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Random number generator that uses the Caliptra core.
//!
//! This driver sends the `CM_RANDOM_GENERATE` command of the Caliptra
//! cryptographic mailbox, as caliptra-mcu-sw
//! `runtime/userspace/api/caliptra-api/src/rng.rs` does. The request and
//! response are defined in the caliptra-sw `caliptra-api` crate
//! (`api/src/mailbox.rs`). This driver imports the command code and sizes
//! from that crate.
//!
//! The driver implements the `CryptoRng` HAL trait. Caliptra uses the same
//! generator for its own keys and signatures. When Caliptra is not
//! debug-locked and its manufacturing flag `RNG_SUPPORT_UNAVAILABLE` is set,
//! it returns a fixed pattern, which is not suitable for secrets.
//!
//! Each `fill` with a buffer that is not empty sends at least one command,
//! so many small fills are slow.

#![no_std]

use caliptra_api::mailbox::{
    CmRandomGenerateReq, CommandId, MailboxRespHeaderVarSize, MAX_CMB_DATA_SIZE,
};
use openprot_hal_blocking::rng::{CryptoRng, Error, ErrorKind, ErrorType};
use veer_mailbox::{Mailbox, MailboxError};

const CM_RANDOM_GENERATE: u32 = CommandId::CM_RANDOM_GENERATE.0;

/// Size in words of the header of a response: the checksum, the FIPS status
/// and the number of random bytes.
const HEADER_WORDS: usize = size_of::<MailboxRespHeaderVarSize>() / 4;

/// Largest number of random bytes that the driver requests with one command.
/// Caliptra generates 48 bytes at a time. It discards the part of the last
/// 48 bytes that the request does not need.
const CHUNK_BYTES: usize = 240;

/// Number of words that hold the random bytes of the largest response.
const CHUNK_WORDS: usize = CHUNK_BYTES / 4;

const _: () = assert!(HEADER_WORDS == 3);
const _: () = assert!(CHUNK_BYTES <= MAX_CMB_DATA_SIZE && CHUNK_BYTES.is_multiple_of(4));
// The body of the request is one word, the number of random bytes.
const _: () = assert!(size_of::<CmRandomGenerateReq>() == 8);

/// Errors from the driver.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaliptraRngError {
    /// The mailbox command failed.
    Mailbox(MailboxError),
    /// The response did not have the requested number of random bytes.
    InvalidResponse,
}

impl From<MailboxError> for CaliptraRngError {
    fn from(error: MailboxError) -> Self {
        Self::Mailbox(error)
    }
}

impl Error for CaliptraRngError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Mailbox(MailboxError::Busy) => ErrorKind::Busy,
            Self::Mailbox(MailboxError::Timeout) => ErrorKind::Timeout,
            _ => ErrorKind::HardwareFailure,
        }
    }
}

/// Random number generator that uses the Caliptra core through its mailbox.
pub struct CaliptraRng<'m> {
    mailbox: &'m Mailbox,
}

impl<'m> CaliptraRng<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self { mailbox }
    }
}

impl ErrorType for CaliptraRng<'_> {
    type Error = CaliptraRngError;
}

impl CryptoRng for CaliptraRng<'_> {
    fn fill(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        for chunk in dest.chunks_mut(CHUNK_BYTES) {
            let mut response = [0u32; HEADER_WORDS + CHUNK_WORDS];
            let len = self.mailbox.execute(
                CM_RANDOM_GENERATE,
                [chunk.len() as u32].into_iter(),
                &mut response,
            )?;
            let [_checksum, _fips_status, data_len, data @ ..] = response;
            if data_len as usize != chunk.len() || len < HEADER_WORDS * 4 + chunk.len() {
                return Err(CaliptraRngError::InvalidResponse);
            }
            // The mailbox puts the first of each four bytes in the low bits
            // of a word.
            for (bytes, word) in chunk.chunks_mut(4).zip(data) {
                bytes
                    .iter_mut()
                    .zip(word.to_le_bytes())
                    .for_each(|(dst, src)| *dst = src);
            }
        }
        Ok(())
    }
}
