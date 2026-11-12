// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SHA-2 driver that uses the Caliptra core.
//!
//! The VeeR core of the MCU has no hash hardware of its own. This driver
//! sends the `CM_SHA_INIT`, `CM_SHA_UPDATE` and `CM_SHA_FINAL` commands of the
//! Caliptra cryptographic mailbox, as caliptra-mcu-sw
//! `runtime/userspace/api/caliptra-api/src/sha.rs` does. The requests and
//! responses are defined in the caliptra-sw `caliptra-api` crate
//! (`api/src/mailbox.rs`). This driver imports its command codes and sizes
//! from that crate.
//!
//! Implements the scoped and owned digest HAL traits for SHA-384 and SHA-512.
//! The `CM_SHA` commands do not support SHA-256.
//!
//! Caliptra keeps no state for a SHA computation between commands.
//! `CM_SHA_INIT` and `CM_SHA_UPDATE` return a 200-byte context that holds the
//! state of the computation, and the next command passes it back. The driver
//! stores the context in the computation, so a computation that is dropped
//! needs no cleanup. After a method of a computation returns an error, do not
//! continue the computation: part of the input may already have been hashed.
//!
//! Each `update` with data sends at least one command, so many small updates
//! are slow.

#![no_std]

use caliptra_api::mailbox::{
    CmHashAlgorithm, CmShaInitResp, CommandId, MailboxRespHeader, CMB_SHA_CONTEXT_SIZE,
    MAX_CMB_DATA_SIZE,
};
use openprot_hal_blocking::digest::{
    owned, scoped, Digest, Error, ErrorKind, ErrorType, Sha2_384, Sha2_512,
};
use veer_mailbox::{Mailbox, MailboxError};

const CM_SHA_INIT: u32 = CommandId::CM_SHA_INIT.0;
const CM_SHA_UPDATE: u32 = CommandId::CM_SHA_UPDATE.0;
const CM_SHA_FINAL: u32 = CommandId::CM_SHA_FINAL.0;

/// Size in words of the context.
const CONTEXT_WORDS: usize = CMB_SHA_CONTEXT_SIZE / 4;

/// Size in words of the header of a response: the checksum and FIPS status.
const HEADER_WORDS: usize = size_of::<MailboxRespHeader>() / 4;

/// Size in words of the largest digest, SHA-512.
const MAX_DIGEST_WORDS: usize = 16;

/// The response to a `CM_SHA_INIT` or `CM_SHA_UPDATE` command: the header and
/// the context. A computation keeps the whole response. The driver sends
/// the context from this buffer and receives the next response into it, so
/// it needs no second buffer.
type State = [u32; HEADER_WORDS + CONTEXT_WORDS];

const _: () = assert!(size_of::<State>() == size_of::<CmShaInitResp>());

/// Returns the context words that are in `state`.
fn context(state: &State) -> impl Iterator<Item = u32> + Clone + '_ {
    state.iter().skip(HEADER_WORDS).copied()
}

/// Errors from the driver.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaliptraShaError {
    /// The mailbox command failed.
    Mailbox(MailboxError),
    /// The response did not have the expected length.
    InvalidResponse,
}

impl From<MailboxError> for CaliptraShaError {
    fn from(error: MailboxError) -> Self {
        Self::Mailbox(error)
    }
}

impl Error for CaliptraShaError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Mailbox(MailboxError::Busy) => ErrorKind::Busy,
            _ => ErrorKind::HardwareFailure,
        }
    }
}

/// SHA-2 driver that uses the Caliptra core through its mailbox.
pub struct CaliptraSha {
    mailbox: Mailbox,
}

impl CaliptraSha {
    pub fn new(mailbox: Mailbox) -> Self {
        Self { mailbox }
    }

    /// Receives the response to a `CM_SHA_INIT` or `CM_SHA_UPDATE` command.
    fn receive_state(&mut self, state: &mut State) -> Result<(), CaliptraShaError> {
        let result = match self.mailbox.receive(state) {
            Ok(len) if len == size_of::<State>() => Ok(()),
            Ok(_) => Err(CaliptraShaError::InvalidResponse),
            Err(error) => Err(error.into()),
        };
        if result.is_err() {
            // A failed receive may have overwritten part of the context,
            // so this clears it. Caliptra rejects a context of zeros, because
            // its hash algorithm field is then the reserved value 0, so later
            // commands of this computation fail.
            state.fill(0);
        }
        result
    }

    /// Starts a computation with no message bytes.
    fn start(&mut self, algorithm: u32) -> Result<State, CaliptraShaError> {
        // The body of CmShaInitReq is the algorithm, the input size and the
        // input.
        self.mailbox.send(CM_SHA_INIT, [algorithm, 0].into_iter())?;
        let mut state = [0; HEADER_WORDS + CONTEXT_WORDS];
        self.receive_state(&mut state)?;
        Ok(state)
    }

    /// Appends `data` to the message.
    fn update(&mut self, state: &mut State, data: &[u8]) -> Result<(), CaliptraShaError> {
        // One command takes at most `MAX_CMB_DATA_SIZE` message bytes.
        for input in data.chunks(MAX_CMB_DATA_SIZE) {
            // The mailbox takes words. The last word is padded with zeros,
            // which the input size does not count.
            let (words, tail) = input.as_chunks::<4>();
            let tail = (!tail.is_empty()).then(|| {
                let mut word = [0; 4];
                word.iter_mut().zip(tail).for_each(|(dst, &src)| *dst = src);
                u32::from_le_bytes(word)
            });
            // The body of CmShaUpdateReq is the context, the input size and
            // the input.
            let body = context(state)
                .chain([input.len() as u32])
                .chain(words.iter().map(|word| u32::from_le_bytes(*word)))
                .chain(tail);
            self.mailbox.send(CM_SHA_UPDATE, body)?;
            self.receive_state(state)?;
        }
        Ok(())
    }

    /// Returns the digest of the message.
    fn finish<const N: usize>(&mut self, state: &State) -> Result<Digest<N>, CaliptraShaError> {
        // The request is a CmShaFinalReq with no input. The response is the
        // header, the digest size in bytes and the digest.
        let body = context(state).chain([0]);
        let mut response = [0u32; HEADER_WORDS + 1 + MAX_DIGEST_WORDS];
        let len = self.mailbox.execute(CM_SHA_FINAL, body, &mut response)?;
        let [_checksum, _fips_status, digest_len, digest @ ..] = response;
        if len != (HEADER_WORDS + 1 + N) * 4 || digest_len as usize != N * 4 {
            return Err(CaliptraShaError::InvalidResponse);
        }
        // Caliptra returns the digest as bytes. The mailbox puts the first
        // byte of each four in the low bits of a word, and the CPU is
        // little-endian, so the words in memory hold the digest bytes in
        // order.
        let mut value = [0u32; N];
        value
            .iter_mut()
            .zip(digest)
            .for_each(|(dst, src)| *dst = src);
        Ok(Digest::new(value))
    }
}

impl ErrorType for CaliptraSha {
    type Error = CaliptraShaError;
}

impl scoped::DigestCtrlReset for CaliptraSha {
    fn reset(&mut self) -> Result<(), Self::Error> {
        // Caliptra keeps no state for a SHA computation between commands,
        // so there is nothing to reset.
        Ok(())
    }
}

/// A SHA-2 computation with an `N`-word digest that borrows the driver.
pub struct Sha2Op<'a, const N: usize> {
    sha: &'a mut CaliptraSha,
    state: State,
}

impl<const N: usize> ErrorType for Sha2Op<'_, N> {
    type Error = CaliptraShaError;
}

impl<const N: usize> scoped::DigestOp for Sha2Op<'_, N> {
    type Output = Digest<N>;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.sha.update(&mut self.state, input)
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        self.sha.finish(&self.state)
    }
}

/// A SHA-2 computation with an `N`-word digest that owns the driver.
pub struct OwnedSha2Op<const N: usize> {
    sha: CaliptraSha,
    state: State,
}

impl<const N: usize> ErrorType for OwnedSha2Op<N> {
    type Error = CaliptraShaError;
}

impl<const N: usize> owned::DigestOp for OwnedSha2Op<N> {
    type Output = Digest<N>;
    type Controller = CaliptraSha;

    fn update(mut self, data: &[u8]) -> Result<Self, Self::Error> {
        self.sha.update(&mut self.state, data)?;
        Ok(self)
    }

    fn finalize(mut self) -> Result<(Self::Output, Self::Controller), Self::Error> {
        let digest = self.sha.finish(&self.state)?;
        Ok((digest, self.sha))
    }

    fn cancel(self) -> Self::Controller {
        self.sha
    }
}

macro_rules! impl_sha2 {
    ($algorithm:ty, $code:expr, $words:literal) => {
        impl scoped::DigestInit<$algorithm> for CaliptraSha {
            type OpContext<'a>
                = Sha2Op<'a, $words>
            where
                Self: 'a;

            fn init(&mut self, _: $algorithm) -> Result<Self::OpContext<'_>, Self::Error> {
                let state = self.start($code)?;
                Ok(Sha2Op { sha: self, state })
            }
        }

        impl owned::DigestInit<$algorithm> for CaliptraSha {
            type Context = OwnedSha2Op<$words>;

            fn init(mut self, _: $algorithm) -> Result<Self::Context, Self::Error> {
                let state = self.start($code)?;
                Ok(OwnedSha2Op { sha: self, state })
            }
        }
    };
}

impl_sha2!(Sha2_384, CmHashAlgorithm::Sha384 as u32, 12);
impl_sha2!(Sha2_512, CmHashAlgorithm::Sha512 as u32, 16);
