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
//! The driver implements the `Sign` trait of the DPE HAL for the P-384
//! profile.
//!
//! Caliptra takes the locality of a command from the AXI user of the
//! mailbox request. When a command succeeds, its DPE returns the default
//! handle unchanged and replaces any other handle.
//!
//! # Errors and the handle
//!
//! `Sign` in the `caliptra-dpe` crate stores the new handle as its last step,
//! after every step that can fail. A response with an error status means that
//! the DPE has not replaced the handle.
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
//! `Sign` does not replace the default handle, so a caller that signs with
//! the default handle can sign again after any error.

#![no_std]

use caliptra_api::mailbox::{CommandId, InvokeDpeReq, MailboxRespHeaderVarSize};
use caliptra_dpe::commands::{Command, CommandHdr, SignFlags, SignP384Cmd};
use caliptra_dpe::response::{ResponseHdr, SignP384Resp};
use caliptra_dpe::DpeProfile;
use openprot_hal_blocking::digest::Digest;
use openprot_hal_blocking::dpe::{ContextHandle, Error, ErrorKind, ErrorType, P384Sha384, Sign};
use openprot_hal_blocking::ecdsa::P384Signature;
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

/// Client that reaches the DPE of the Caliptra core through the mailbox.
pub struct CaliptraDpe<'m> {
    mailbox: &'m Mailbox,
}

impl<'m> CaliptraDpe<'m> {
    pub fn new(mailbox: &'m Mailbox) -> Self {
        Self { mailbox }
    }
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
