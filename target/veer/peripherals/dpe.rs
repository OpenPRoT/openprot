// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Client for the DICE Protection Environment (DPE) of the Caliptra core.
//!
//! This driver sends DPE commands with the `INVOKE_DPE_ECC384` mailbox
//! command, as caliptra-mcu-sw
//! `runtime/userspace/api/caliptra-api/src/dpe.rs` does. The mailbox request
//! and response are defined in the caliptra-sw `caliptra-api` crate
//! (`api/src/mailbox.rs`). The DPE commands and responses are defined in the
//! `caliptra-dpe` crate (`dpe/src/commands` and `dpe/src/response.rs`). This
//! driver imports the layouts from those crates.
//!
//! The driver implements the `Sign`, `CertifyKey` and `GetCertificateChain`
//! traits of the DPE HAL for the P-384 profile. `CaliptraDpeChain` implements
//! the `CertificateChain` trait of the certificate HAL for the certificate
//! chain of the DPE.
//!
//! Caliptra takes the locality of a command from the AXI user of the
//! mailbox request. When a command succeeds, its DPE returns the default
//! handle unchanged and replaces any other handle.
//!
//! # Errors and the handle
//!
//! `Sign` in the `caliptra-dpe` crate stores the new handle as its last step,
//! after every step that can fail. `CertifyKey` stores the new handle after
//! it has written the certificate. It then writes the response header in
//! front of the certificate. The header fits, because the certificate after
//! it fits. A response with an error status means that the DPE has not
//! replaced the handle.
//!
//! The mailbox driver does not send the request when it reports
//! `MailboxError::Busy`, `RequestTooLarge` or `Unreadable`, so the DPE has
//! not run.
//!
//! After every other mailbox error, and after
//! `CaliptraDpeError::InvalidResponse`, the DPE may have replaced a handle
//! that is not the default handle. This includes `CommandFailed`. Caliptra
//! fails the mailbox command in two cases: it rejects the request before the
//! DPE runs, or it has a fatal error, possibly while the DPE runs. The error
//! does not say which.
//!
//! `Sign` and `CertifyKey` do not replace the default handle, so a caller
//! that uses the default handle can send either command again after any
//! error.
//!
//! # Certificates
//!
//! The certificate chain of `GetCertificateChain` holds the LDevID, FMC alias
//! and runtime alias certificates, in that order. The runtime alias key signs
//! the certificates that `CertifyKey` returns.
//!
//! Caliptra returns X.509 certificates only to a caller of its privilege
//! level 0 (PL0). For another caller it fails the mailbox command, and the
//! driver returns `MailboxError::CommandFailed`.

#![no_std]

use caliptra_api::mailbox::{CommandId, InvokeDpeReq, MailboxRespHeaderVarSize};
use caliptra_dpe::commands::{
    CertifyKeyCommand, CertifyKeyFlags, CertifyKeyP384Cmd, Command, CommandHdr,
    GetCertificateChainCmd, SignFlags, SignP384Cmd,
};
use caliptra_dpe::response::{
    CertifyKeyP384RespHdr, GetCertificateChainResp, ResponseHdr, SignP384Resp,
};
use caliptra_dpe::DpeProfile;
use openprot_hal_blocking::certificate::{self, CertificateChain};
use openprot_hal_blocking::digest::Digest;
use openprot_hal_blocking::dpe::{
    CertificateFormat, CertifyKey, CertifyKeyOutput, ContextHandle, Error, ErrorKind, ErrorType,
    GetCertificateChain, P384Sha384, Sign,
};
use openprot_hal_blocking::ecdsa::{P384PublicKey, P384Signature};
use veer_mailbox::{Mailbox, MailboxError};
use zerocopy::{Immutable, IntoBytes, TryFromBytes};

const INVOKE_DPE_ECC384: u32 = CommandId::INVOKE_DPE_ECC384.0;

/// DPE status codes (`DpeErrorCode` in the `caliptra-dpe` crate).
mod status {
    pub const NO_ERROR: u32 = 0;
    pub const INVALID_COMMAND: u32 = 2;
    pub const INVALID_ARGUMENT: u32 = 3;
    pub const INVALID_HANDLE: u32 = 0x80;
    pub const INVALID_LOCALITY: u32 = 0x81;
    pub const ARGUMENT_NOT_SUPPORTED: u32 = 0x83;
    /// `DpeErrorCode::Platform(PlatformError::CertificateChainError)`.
    pub const CERTIFICATE_CHAIN_ERROR: u32 = 0x0100_0001;
}

/// The body of an `INVOKE_DPE_ECC384` request that holds a `Sign` command:
/// `InvokeDpeReq` without its checksum, with the command as its data.
#[repr(C)]
#[derive(IntoBytes, Immutable)]
struct SignRequest {
    data_size: u32,
    header: CommandHdr,
    command: SignP384Cmd,
}

/// Size in bytes of the DPE command in a [`SignRequest`].
const SIGN_COMMAND_BYTES: usize = size_of::<CommandHdr>() + size_of::<SignP384Cmd>();

/// Size in words of a [`SignRequest`].
const SIGN_REQUEST_WORDS: usize = size_of::<SignRequest>() / 4;

const _: () = assert!(SIGN_COMMAND_BYTES <= InvokeDpeReq::DATA_MAX_SIZE);

/// The response to a [`SignRequest`]: `InvokeDpeResp` with the response to
/// the `Sign` command as its data.
///
/// When the DPE command fails, the data is only a `ResponseHdr`. The
/// mailbox driver leaves the rest of the buffer zero.
#[repr(C)]
#[derive(TryFromBytes)]
struct SignResponse {
    header: MailboxRespHeaderVarSize,
    dpe: SignP384Resp,
}

/// Size in words of a [`SignResponse`].
const SIGN_RESPONSE_WORDS: usize = size_of::<SignResponse>() / 4;

/// Size in bytes of the shortest valid response: the mailbox header and a
/// `ResponseHdr`.
const MIN_RESPONSE_BYTES: usize = size_of::<MailboxRespHeaderVarSize>() + size_of::<ResponseHdr>();

/// The body of an `INVOKE_DPE_ECC384` request that holds a `CertifyKey`
/// command.
#[repr(C)]
#[derive(IntoBytes, Immutable)]
struct CertifyKeyRequest {
    data_size: u32,
    header: CommandHdr,
    command: CertifyKeyP384Cmd,
}

/// Size in bytes of the DPE command in a [`CertifyKeyRequest`].
const CERTIFY_KEY_COMMAND_BYTES: usize = size_of::<CommandHdr>() + size_of::<CertifyKeyP384Cmd>();

/// Size in words of a [`CertifyKeyRequest`].
const CERTIFY_KEY_REQUEST_WORDS: usize = size_of::<CertifyKeyRequest>() / 4;

const _: () = assert!(CERTIFY_KEY_COMMAND_BYTES <= InvokeDpeReq::DATA_MAX_SIZE);

/// The start of the response to a [`CertifyKeyRequest`]. The certificate
/// follows it.
#[repr(C)]
#[derive(TryFromBytes)]
struct CertifyKeyResponse {
    header: MailboxRespHeaderVarSize,
    dpe: CertifyKeyP384RespHdr,
}

/// Size in words of a [`CertifyKeyResponse`].
const CERTIFY_KEY_RESPONSE_WORDS: usize = size_of::<CertifyKeyResponse>() / 4;

/// The body of an `INVOKE_DPE_ECC384` request that holds a
/// `GetCertificateChain` command.
#[repr(C)]
#[derive(IntoBytes, Immutable)]
struct ChainRequest {
    data_size: u32,
    header: CommandHdr,
    command: GetCertificateChainCmd,
}

/// Size in bytes of the DPE command in a [`ChainRequest`].
const CHAIN_COMMAND_BYTES: usize = size_of::<CommandHdr>() + size_of::<GetCertificateChainCmd>();

/// Size in words of a [`ChainRequest`].
const CHAIN_REQUEST_WORDS: usize = size_of::<ChainRequest>() / 4;

/// The start of the response to a [`ChainRequest`]: the header of
/// `InvokeDpeResp`, then the `resp_hdr` and `certificate_size` fields of
/// `GetCertificateChainResp`. The chain data follows it.
#[repr(C)]
#[derive(TryFromBytes)]
struct ChainResponse {
    header: MailboxRespHeaderVarSize,
    resp_hdr: ResponseHdr,
    certificate_size: u32,
}

/// Size in words of a [`ChainResponse`].
const CHAIN_RESPONSE_WORDS: usize = size_of::<ChainResponse>() / 4;

/// Size in bytes of the chain data field of a `GetCertificateChainResp`. One
/// command returns at most this many bytes of the chain. The field always has
/// this size. The bytes after the returned part of the chain are zero.
const CHAIN_DATA_BYTES: usize =
    size_of::<GetCertificateChainResp>() - size_of::<ResponseHdr>() - size_of::<u32>();

/// The largest number of `GetCertificateChain` commands that
/// [`CaliptraDpeChain::new`] sends to find the length of the chain. A chain
/// of 64 KB or longer needs more commands.
const MAX_CHAIN_PARTS: usize = 32;

/// Errors from the driver.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CaliptraDpeError {
    /// The mailbox command failed.
    Mailbox(MailboxError),
    /// The response does not have the layout of a DPE response.
    InvalidResponse,
    /// The DPE command failed. The value is the DPE status code.
    Dpe(u32),
}

impl From<MailboxError> for CaliptraDpeError {
    fn from(error: MailboxError) -> Self {
        Self::Mailbox(error)
    }
}

impl Error for CaliptraDpeError {
    fn kind(&self) -> ErrorKind {
        match *self {
            Self::Mailbox(MailboxError::Busy) => ErrorKind::Busy,
            Self::Mailbox(MailboxError::Timeout) => ErrorKind::Timeout,
            Self::Mailbox(MailboxError::RequestTooLarge | MailboxError::Unreadable) => {
                ErrorKind::NotDelivered
            }
            Self::Mailbox(
                MailboxError::CommandFailed(_)
                | MailboxError::NoResponse
                | MailboxError::UnknownStatus(_)
                | MailboxError::InvalidChecksum,
            ) => ErrorKind::ResponseLost,
            Self::InvalidResponse => ErrorKind::InvalidResponse,
            Self::Dpe(status::INVALID_HANDLE) => ErrorKind::InvalidHandle,
            Self::Dpe(status::INVALID_LOCALITY) => ErrorKind::InvalidLocality,
            Self::Dpe(status::INVALID_ARGUMENT) => ErrorKind::InvalidArgument,
            Self::Dpe(status::INVALID_COMMAND | status::ARGUMENT_NOT_SUPPORTED) => {
                ErrorKind::NotSupported
            }
            Self::Dpe(_) => ErrorKind::Internal,
        }
    }
}

impl certificate::Error for CaliptraDpeError {
    fn kind(&self) -> certificate::ErrorKind {
        match *self {
            Self::Mailbox(MailboxError::Busy) => certificate::ErrorKind::Busy,
            _ => certificate::ErrorKind::ReadFailed,
        }
    }
}

/// Client that reaches the DPE of the Caliptra core through the mailbox.
pub struct CaliptraDpe<'m> {
    mailbox: &'m Mailbox,
}

impl<'m> CaliptraDpe<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self { mailbox }
    }

    /// Sends a DPE command and stores its response: the first `head.len()`
    /// words of the mailbox response in `head` and the bytes after them in
    /// `tail`. Bytes that do not fit in `tail` are discarded.
    ///
    /// Returns the length of the response in bytes.
    fn invoke(
        &self,
        request: impl Iterator<Item = u32> + Clone,
        head: &mut [u32],
        tail: &mut [u8],
    ) -> Result<usize, CaliptraDpeError> {
        let head_words = head.len();
        let len = self
            .mailbox
            .execute_with(INVOKE_DPE_ECC384, request, |index, word| {
                if let Some(slot) = head.get_mut(index) {
                    *slot = word;
                } else if let Some(rest) = index
                    .checked_sub(head_words)
                    .and_then(|index| index.checked_mul(4))
                    .and_then(|offset| tail.get_mut(offset..))
                {
                    rest.iter_mut()
                        .zip(word.to_le_bytes())
                        .for_each(|(to, from)| *to = from);
                }
            })?;
        Ok(len)
    }

    /// Asks the DPE for `size` bytes of its certificate chain, from byte
    /// `offset` of the chain, and stores them in `chain`. Bytes that do not
    /// fit in `chain` are discarded. `size` is at most [`CHAIN_DATA_BYTES`].
    ///
    /// Returns the number of bytes that the DPE returned, which is 0 when
    /// `offset` is at or after the end of the chain.
    fn chain_part(
        &self,
        offset: u32,
        size: usize,
        chain: &mut [u8],
    ) -> Result<usize, CaliptraDpeError> {
        let request = ChainRequest {
            data_size: CHAIN_COMMAND_BYTES as u32,
            header: CommandHdr::new(DpeProfile::P384Sha384, Command::GET_CERTIFICATE_CHAIN),
            command: GetCertificateChainCmd {
                offset,
                size: size as u32,
            },
        };
        let request: [u32; CHAIN_REQUEST_WORDS] = zerocopy::transmute!(request);
        let mut head = [0u32; CHAIN_RESPONSE_WORDS];
        let len = self.invoke(request.into_iter(), &mut head, chain)?;
        let response: ChainResponse =
            zerocopy::try_transmute!(head).map_err(|_| CaliptraDpeError::InvalidResponse)?;
        match check_header(len, &response.resp_hdr) {
            // Caliptra's DPE reports this status when `offset` is at or after
            // the end of the chain.
            Err(CaliptraDpeError::Dpe(status::CERTIFICATE_CHAIN_ERROR)) => return Ok(0),
            result => result?,
        }
        let stored = response.certificate_size as usize;
        if stored > size
            || response.header.data_len as usize != size_of::<GetCertificateChainResp>()
            || len.checked_sub(size_of::<MailboxRespHeaderVarSize>())
                < Some(size_of::<GetCertificateChainResp>())
        {
            return Err(CaliptraDpeError::InvalidResponse);
        }
        Ok(stored)
    }
}

/// Checks the header of the DPE response in a mailbox response of `len`
/// bytes.
fn check_header(len: usize, header: &ResponseHdr) -> Result<(), CaliptraDpeError> {
    if len < MIN_RESPONSE_BYTES || header.magic != ResponseHdr::DPE_RESPONSE_MAGIC {
        return Err(CaliptraDpeError::InvalidResponse);
    }
    if header.status != status::NO_ERROR {
        return Err(CaliptraDpeError::Dpe(header.status));
    }
    Ok(())
}

impl ErrorType for CaliptraDpe<'_> {
    type Error = CaliptraDpeError;
}

impl Sign<P384Sha384> for CaliptraDpe<'_> {
    fn sign(
        &mut self,
        handle: &ContextHandle,
        label: &[u8; 48],
        digest: &Digest<12>,
        signature: &mut P384Signature,
    ) -> Result<ContextHandle, Self::Error> {
        let request = SignRequest {
            data_size: SIGN_COMMAND_BYTES as u32,
            header: CommandHdr::new(DpeProfile::P384Sha384, Command::SIGN),
            command: SignP384Cmd {
                handle: caliptra_dpe::context::ContextHandle(handle.0),
                label: *label,
                flags: SignFlags(0),
                digest: zerocopy::transmute!(digest.value),
            },
        };
        let request: [u32; SIGN_REQUEST_WORDS] = zerocopy::transmute!(request);
        let mut response = [0u32; SIGN_RESPONSE_WORDS];
        let len = self
            .mailbox
            .execute(INVOKE_DPE_ECC384, request.into_iter(), &mut response)?;
        if len < MIN_RESPONSE_BYTES {
            return Err(CaliptraDpeError::InvalidResponse);
        }
        let response: SignResponse =
            zerocopy::try_transmute!(response).map_err(|_| CaliptraDpeError::InvalidResponse)?;
        let SignP384Resp {
            resp_hdr,
            new_context_handle,
            sig_r,
            sig_s,
        } = response.dpe;
        if resp_hdr.magic != ResponseHdr::DPE_RESPONSE_MAGIC {
            return Err(CaliptraDpeError::InvalidResponse);
        }
        if resp_hdr.status != status::NO_ERROR {
            return Err(CaliptraDpeError::Dpe(resp_hdr.status));
        }
        if response.header.data_len as usize != size_of::<SignP384Resp>()
            || len < size_of::<SignResponse>()
        {
            return Err(CaliptraDpeError::InvalidResponse);
        }
        *signature = P384Signature::new(sig_r, sig_s);
        Ok(ContextHandle(new_context_handle.0))
    }
}

impl CertifyKey<P384Sha384> for CaliptraDpe<'_> {
    fn certify_key(
        &mut self,
        handle: &ContextHandle,
        label: &[u8; 48],
        format: CertificateFormat,
        public_key: &mut P384PublicKey,
        certificate: &mut [u8],
    ) -> Result<CertifyKeyOutput, Self::Error> {
        let request = CertifyKeyRequest {
            data_size: CERTIFY_KEY_COMMAND_BYTES as u32,
            header: CommandHdr::new(DpeProfile::P384Sha384, Command::CERTIFY_KEY),
            command: CertifyKeyP384Cmd {
                handle: caliptra_dpe::context::ContextHandle(handle.0),
                flags: CertifyKeyFlags(0),
                format: match format {
                    CertificateFormat::X509 => CertifyKeyCommand::FORMAT_X509,
                    CertificateFormat::Csr => CertifyKeyCommand::FORMAT_CSR,
                },
                label: *label,
            },
        };
        let request: [u32; CERTIFY_KEY_REQUEST_WORDS] = zerocopy::transmute!(request);
        let mut head = [0u32; CERTIFY_KEY_RESPONSE_WORDS];
        let len = self.invoke(request.into_iter(), &mut head, certificate)?;
        let response: CertifyKeyResponse =
            zerocopy::try_transmute!(head).map_err(|_| CaliptraDpeError::InvalidResponse)?;
        let CertifyKeyP384RespHdr {
            resp_hdr,
            new_context_handle,
            derived_pubkey_x,
            derived_pubkey_y,
            cert_size,
        } = response.dpe;
        check_header(len, &resp_hdr)?;
        let certificate_len = cert_size as usize;
        let data_len = size_of::<CertifyKeyP384RespHdr>()
            .checked_add(certificate_len)
            .ok_or(CaliptraDpeError::InvalidResponse)?;
        if response.header.data_len as usize != data_len
            || len.checked_sub(size_of::<MailboxRespHeaderVarSize>()) < Some(data_len)
        {
            return Err(CaliptraDpeError::InvalidResponse);
        }
        *public_key = P384PublicKey::new(derived_pubkey_x, derived_pubkey_y);
        Ok(CertifyKeyOutput {
            handle: ContextHandle(new_context_handle.0),
            certificate_len,
        })
    }
}

impl GetCertificateChain<P384Sha384> for CaliptraDpe<'_> {
    fn get_certificate_chain(
        &mut self,
        offset: usize,
        chain: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // The command takes a 32-bit offset. A larger one is after the end
        // of the chain.
        let Ok(offset) = u32::try_from(offset) else {
            return Ok(0);
        };
        if chain.is_empty() {
            return Ok(0);
        }
        let size = chain.len().min(CHAIN_DATA_BYTES);
        self.chain_part(offset, size, chain)
    }
}

/// The certificate chain of Caliptra's DPE: the LDevID, FMC alias and
/// runtime alias certificates.
///
/// A read sends `GetCertificateChain` commands until `buf` is full or the
/// chain ends. The chain is not kept in the memory of the MCU.
pub struct CaliptraDpeChain<'m> {
    dpe: CaliptraDpe<'m>,
    /// Length of the chain in bytes.
    len: usize,
}

impl<'m> CaliptraDpeChain<'m> {
    /// Asks the DPE for the length of its chain, which takes one command for
    /// each [`CHAIN_DATA_BYTES`] bytes of the chain.
    ///
    /// # Errors
    ///
    /// `CaliptraDpeError::InvalidResponse` when the chain is 64 KB or
    /// longer.
    pub fn new(mailbox: &'m Mailbox) -> Result<Self, CaliptraDpeError> {
        let dpe = CaliptraDpe::new(mailbox);
        let mut len = 0usize;
        for _ in 0..MAX_CHAIN_PARTS {
            let offset = u32::try_from(len).map_err(|_| CaliptraDpeError::InvalidResponse)?;
            let part = dpe.chain_part(offset, CHAIN_DATA_BYTES, &mut [])?;
            len = len.saturating_add(part);
            if part < CHAIN_DATA_BYTES {
                return Ok(Self { dpe, len });
            }
        }
        Err(CaliptraDpeError::InvalidResponse)
    }
}

impl certificate::ErrorType for CaliptraDpeChain<'_> {
    type Error = CaliptraDpeError;
}

impl CertificateChain for CaliptraDpeChain<'_> {
    fn len(&mut self) -> Result<usize, Self::Error> {
        Ok(self.len)
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut stored = 0usize;
        while let Some(rest) = buf.get_mut(stored..).filter(|rest| !rest.is_empty()) {
            let Some(part_offset) = offset.checked_add(stored) else {
                break;
            };
            let part = self.dpe.get_certificate_chain(part_offset, rest)?;
            if part == 0 {
                break;
            }
            stored = stored.saturating_add(part);
        }
        Ok(stored)
    }
}
