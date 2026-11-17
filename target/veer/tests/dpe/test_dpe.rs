// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Checks the Caliptra DPE driver. The driver uses the mailbox to ask the DPE
//! of the Caliptra core for a signature.
//!
//! The test signs a digest with the default context and checks the signature
//! with Caliptra's `ECDSA384_SIGNATURE_VERIFY` command. It gets the public
//! key from the DPE `CertifyKey` command. The driver has neither command, so
//! the test sends them itself.
//!
//! The test is a userspace app because the stack of the kernel's bootstrap
//! thread is too small for it.

#![no_std]
#![no_main]

use caliptra_api::mailbox::MailboxRespHeaderVarSize;
use caliptra_api::mailbox::{CommandId, EcdsaVerifyReq, MailboxReqHeader, MailboxRespHeader};
use caliptra_dpe::commands::{CertifyKeyCommand, CertifyKeyFlags, CertifyKeyP384Cmd};
use caliptra_dpe::commands::{Command, CommandHdr};
use caliptra_dpe::response::{CertifyKeyP384RespHdr, ResponseHdr};
use caliptra_dpe::DpeProfile;
use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Digest, Sha2_384};
use openprot_hal_blocking::dpe::{ContextHandle, Error as _, ErrorKind, Sign};
use openprot_hal_blocking::ecdsa::{P384Signature, Signature};
use pw_status::Error;
use userspace::{entry, syscall};
use veer_dpe::{CaliptraDpe, CaliptraDpeError};
use veer_mailbox::{Mailbox, MailboxError};
use veer_sha::CaliptraSha;
use zerocopy::{Immutable, IntoBytes, TryFromBytes};

/// Selects the key that signs. caliptra-mcu-sw uses the same label for the
/// key of its SPDM responder (`DPE_LEAF_LABEL`).
const LABEL: [u8; 48] = {
    let mut label = [0; 48];
    let mut i = 0;
    while i < label.len() {
        label[i] = i as u8;
        i += 1;
    }
    label
};

/// Selects another key of the same context.
const OTHER_LABEL: [u8; 48] = [0x5a; 48];

/// A point on P-384: its x and y coordinates.
type PublicKey = ([u8; 48], [u8; 48]);

/// The body of an `INVOKE_DPE_ECC384` request that holds a `CertifyKey`
/// command.
#[repr(C)]
#[derive(IntoBytes, Immutable)]
struct CertifyKeyRequest {
    data_size: u32,
    header: CommandHdr,
    command: CertifyKeyP384Cmd,
}

/// The start of the response to a [`CertifyKeyRequest`]. The certificate
/// follows it.
#[repr(C)]
#[derive(TryFromBytes)]
struct CertifyKeyResponse {
    header: MailboxRespHeaderVarSize,
    dpe: CertifyKeyP384RespHdr,
}

const _: () = assert!(
    size_of::<EcdsaVerifyReq>() == size_of::<MailboxReqHeader>() + size_of::<[[u8; 48]; 5]>()
);

/// Returns the SHA-384 digest of `message`.
fn sha384(mailbox: &Mailbox, message: &[u8]) -> Result<Digest<12>, &'static str> {
    let mut sha = CaliptraSha::new(mailbox);
    let mut op = sha.init(Sha2_384).map_err(|_| "SHA init failed")?;
    op.update(message).map_err(|_| "SHA update failed")?;
    op.finalize().map_err(|_| "SHA finalize failed")
}

/// Signs `digest` and logs the error if the command fails.
fn sign(
    dpe: &mut CaliptraDpe<'_>,
    handle: &ContextHandle,
    label: &[u8; 48],
    digest: &Digest<12>,
) -> Result<(P384Signature, ContextHandle), &'static str> {
    match dpe.sign(handle, label, digest) {
        Ok(output) => Ok((output.signature, output.handle)),
        Err(CaliptraDpeError::Dpe(status)) => {
            pw_log::error!("Sign: DPE status {:#x}", status as u32);
            Err("Sign failed in the DPE")
        }
        Err(CaliptraDpeError::Mailbox(MailboxError::CommandFailed(code))) => {
            pw_log::error!("Sign: Caliptra error {:#x}", code as u32);
            Err("Sign failed in Caliptra")
        }
        Err(_) => Err("Sign failed"),
    }
}

/// Uses the DPE `CertifyKey` command to return the public key that the
/// default context derives for `label`.
fn public_key(mailbox: &Mailbox, label: &[u8; 48]) -> Result<PublicKey, &'static str> {
    let request = CertifyKeyRequest {
        data_size: (size_of::<CommandHdr>() + size_of::<CertifyKeyP384Cmd>()) as u32,
        header: CommandHdr::new(DpeProfile::P384Sha384, Command::CERTIFY_KEY),
        command: CertifyKeyP384Cmd {
            handle: caliptra_dpe::context::ContextHandle::default(),
            flags: CertifyKeyFlags(0),
            format: CertifyKeyCommand::FORMAT_X509,
            label: *label,
        },
    };
    let request: [u32; size_of::<CertifyKeyRequest>() / 4] = zerocopy::transmute!(request);
    // The buffer has no room for the certificate, so the mailbox driver
    // discards it.
    let mut response = [0u32; size_of::<CertifyKeyResponse>() / 4];
    let len = mailbox
        .execute(
            CommandId::INVOKE_DPE_ECC384.0,
            request.into_iter(),
            &mut response,
        )
        .map_err(|_| "CertifyKey failed in Caliptra")?;
    let response: CertifyKeyResponse =
        zerocopy::try_transmute!(response).map_err(|_| "CertifyKey response is malformed")?;
    let header = response.dpe.resp_hdr;
    if header.magic != ResponseHdr::DPE_RESPONSE_MAGIC {
        return Err("CertifyKey response has the wrong magic");
    }
    if header.status != 0 {
        pw_log::error!("CertifyKey: DPE status {:#x}", header.status as u32);
        return Err("CertifyKey failed in the DPE");
    }
    if len < size_of::<CertifyKeyResponse>() {
        return Err("CertifyKey response is too short");
    }
    Ok((response.dpe.derived_pubkey_x, response.dpe.derived_pubkey_y))
}

/// Uses Caliptra's `ECDSA384_SIGNATURE_VERIFY` command to check whether
/// `signature` is a signature of `digest` by `key`.
fn verify(
    mailbox: &Mailbox,
    key: &PublicKey,
    signature: &P384Signature,
    digest: &Digest<12>,
) -> Result<bool, &'static str> {
    let mut r = [0; 48];
    let mut s = [0; 48];
    signature.coordinates(&mut r, &mut s);
    let hash: [u8; 48] = zerocopy::transmute!(digest.value);
    let request: [u32; size_of::<[[u8; 48]; 5]>() / 4] =
        zerocopy::transmute!([key.0, key.1, r, s, hash]);
    let mut response = [0u32; size_of::<MailboxRespHeader>() / 4];
    match mailbox.execute(
        CommandId::ECDSA384_SIGNATURE_VERIFY.0,
        request.into_iter(),
        &mut response,
    ) {
        Ok(_) => Ok(true),
        // Caliptra fails the command when the signature is wrong.
        Err(MailboxError::CommandFailed(_)) => Ok(false),
        Err(_) => Err("verify command failed"),
    }
}

fn run(mailbox: &Mailbox) -> Result<(), &'static str> {
    let mut dpe = CaliptraDpe::new(mailbox);
    let digest = sha384(mailbox, b"abc")?;
    let key = public_key(mailbox, &LABEL)?;

    let (signature, handle) = sign(&mut dpe, &ContextHandle::DEFAULT, &LABEL, &digest)?;
    if !verify(mailbox, &key, &signature, &digest)? {
        return Err("signature does not verify");
    }
    pw_log::info!("sign: ok");

    // The signature does not verify for another digest.
    let other_digest = sha384(mailbox, b"abd")?;
    if verify(mailbox, &key, &signature, &other_digest)? {
        return Err("signature verifies with another digest");
    }

    // The next command takes the handle that the previous one returned.
    let (signature, handle) = sign(&mut dpe, &handle, &OTHER_LABEL, &digest)?;
    if verify(mailbox, &key, &signature, &digest)? {
        return Err("another label signs with the same key");
    }
    pw_log::info!("other label: ok");

    let unknown = ContextHandle([0xa5; 16]);
    match dpe.sign(&unknown, &LABEL, &digest) {
        Err(error) if error.kind() == ErrorKind::InvalidHandle => {}
        Err(_) => return Err("unknown handle gives the wrong error"),
        Ok(_) => return Err("unknown handle signs"),
    }
    pw_log::info!("unknown handle: ok");

    // The DPE still signs after a command that failed.
    let (signature, _handle) = sign(&mut dpe, &handle, &LABEL, &digest)?;
    if !verify(mailbox, &key, &signature, &digest)? {
        return Err("signature after an error does not verify");
    }
    pw_log::info!("sign after error: ok");
    Ok(())
}

#[entry]
fn entry() -> pw_status::Result<()> {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };
    let result = run(&mailbox).map_err(|reason| {
        pw_log::error!("DPE test failed: {}", reason as &str);
        Error::Internal
    });
    // The kernel logs PASS or FAIL, which the test runner looks for.
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in DPE test");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
