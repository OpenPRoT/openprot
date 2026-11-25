// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Checks the Caliptra DPE driver. The driver uses the mailbox to send
//! commands to the DPE of the Caliptra core.
//!
//! The test certifies a key, signs a digest with it and checks the signature
//! with Caliptra's `ECDSA384_SIGNATURE_VERIFY` command, which the test sends
//! itself. It also reads the certificate chain.
//!
//! The test is a userspace app because the stack of the kernel's bootstrap
//! thread is too small for it.

#![no_std]
#![no_main]

use caliptra_api::mailbox::{CommandId, EcdsaVerifyReq, MailboxReqHeader, MailboxRespHeader};
use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Digest, Sha2_384};
use openprot_hal_blocking::dpe::{
    CertificateFormat, CertifyKey, ContextHandle, Error as _, ErrorKind, GetCertificateChain, Sign,
};
use openprot_hal_blocking::ecdsa::{P384PublicKey, P384Signature, PublicKey, Signature};
use pw_status::Error;
use userspace::{entry, syscall};
use veer_dpe::{CaliptraDpe, CaliptraDpeError};
use veer_mailbox::{Mailbox, MailboxError};
use veer_sha::CaliptraSha;

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

/// The number of certificates in the certificate chain of Caliptra's DPE:
/// LDevID, FMC alias and runtime alias.
const CHAIN_CERTIFICATES: usize = 3;

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

/// Logs `error` and returns the failure message of the test.
fn failed(command: &'static str, error: CaliptraDpeError) -> &'static str {
    match error {
        CaliptraDpeError::Dpe(status) => {
            pw_log::error!("{}: DPE status {:#x}", command as &str, status as u32);
        }
        CaliptraDpeError::Mailbox(MailboxError::CommandFailed(code)) => {
            pw_log::error!("{}: Caliptra error {:#x}", command as &str, code as u32);
        }
        _ => pw_log::error!("{}: failed", command as &str),
    }
    "DPE command failed"
}

/// Signs `digest` and logs the error if the command fails.
fn sign(
    dpe: &mut CaliptraDpe<'_>,
    handle: &ContextHandle,
    label: &[u8; 48],
    digest: &Digest<12>,
) -> Result<(P384Signature, ContextHandle), &'static str> {
    let mut signature = P384Signature::new([0; 48], [0; 48]);
    let handle = dpe
        .sign(handle, label, digest, &mut signature)
        .map_err(|error| failed("Sign", error))?;
    Ok((signature, handle))
}

/// Returns the total length, header included, of the DER element at the
/// start of `der`, if it is a SEQUENCE with a two-byte long-form length
/// (`30 82 hh ll`).
fn der_sequence_len(der: &[u8]) -> Option<usize> {
    match der {
        [0x30, 0x82, high, low, ..] => {
            Some(usize::from(u16::from_be_bytes([*high, *low])).saturating_add(4))
        }
        _ => None,
    }
}

/// Returns the x and y coordinates of `key`.
fn coordinates(key: &P384PublicKey) -> ([u8; 48], [u8; 48]) {
    let mut x = [0; 48];
    let mut y = [0; 48];
    key.coordinates(&mut x, &mut y);
    (x, y)
}

/// Certifies the key that the default context derives for `label`, checks
/// the length of the certificate and returns the public key.
fn certify(dpe: &mut CaliptraDpe<'_>, label: &[u8; 48]) -> Result<P384PublicKey, &'static str> {
    let mut key = P384PublicKey::new([0; 48], [0; 48]);
    // The buffer holds only the start of the certificate.
    let mut certificate = [0; 64];
    let output = dpe
        .certify_key(
            &ContextHandle::DEFAULT,
            label,
            CertificateFormat::X509,
            &mut key,
            &mut certificate,
        )
        .map_err(|error| failed("CertifyKey", error))?;
    if output.handle != ContextHandle::DEFAULT {
        return Err("CertifyKey replaced the default handle");
    }
    if der_sequence_len(&certificate) != Some(output.certificate_len) {
        return Err("certificate is not one DER sequence");
    }

    // The command gives the same key and length when the buffer is empty.
    let mut same_key = P384PublicKey::new([0; 48], [0; 48]);
    let same = dpe
        .certify_key(
            &ContextHandle::DEFAULT,
            label,
            CertificateFormat::X509,
            &mut same_key,
            &mut [],
        )
        .map_err(|error| failed("CertifyKey", error))?;
    if same != output || coordinates(&same_key) != coordinates(&key) {
        return Err("CertifyKey gives another result for an empty buffer");
    }
    Ok(key)
}

/// Reads the certificate chain and checks that it holds the expected number
/// of DER certificates.
fn check_chain(dpe: &mut CaliptraDpe<'_>) -> Result<(), &'static str> {
    let mut read = |offset: usize, chain: &mut [u8]| {
        dpe.get_certificate_chain(offset, chain)
            .map_err(|error| failed("GetCertificateChain", error))
    };

    // Each certificate starts where the previous one ends.
    let mut end = 0usize;
    for _ in 0..CHAIN_CERTIFICATES {
        let mut start = [0; 4];
        if read(end, &mut start)? != start.len() {
            return Err("chain ends before its last certificate");
        }
        let len = der_sequence_len(&start).ok_or("chain does not hold a DER sequence")?;
        end = end.saturating_add(len);
    }
    if read(end, &mut [0; 4])? != 0 {
        return Err("chain has data after its last certificate");
    }

    // A read that starts 10 bytes before the end of the chain returns 10
    // bytes.
    let mut tail = [0; 64];
    if read(end.saturating_sub(10), &mut tail)? != 10 {
        return Err("read at the end of the chain has the wrong length");
    }

    // Two reads of the same bytes agree. The lengths are not multiples of 4,
    // so each read ends inside a mailbox word.
    let mut first = [0; 101];
    let mut second = [0; 37];
    if read(0, &mut first)? != first.len() || read(64, &mut second)? != second.len() {
        return Err("read of the chain is short");
    }
    if first.get(64..) != Some(second.as_slice()) {
        return Err("two reads of the chain differ");
    }
    Ok(())
}

/// Uses Caliptra's `ECDSA384_SIGNATURE_VERIFY` command to check whether
/// `signature` is a signature of `digest` by `key`.
fn verify(
    mailbox: &Mailbox,
    key: &P384PublicKey,
    signature: &P384Signature,
    digest: &Digest<12>,
) -> Result<bool, &'static str> {
    let mut r = [0; 48];
    let mut s = [0; 48];
    signature.coordinates(&mut r, &mut s);
    let hash: [u8; 48] = zerocopy::transmute!(digest.value);
    let (x, y) = coordinates(key);
    let request: [u32; size_of::<[[u8; 48]; 5]>() / 4] = zerocopy::transmute!([x, y, r, s, hash]);
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
    let key = certify(&mut dpe, &LABEL)?;
    pw_log::info!("certify key: ok");

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
    let mut unused = P384Signature::new([0; 48], [0; 48]);
    match dpe.sign(&unknown, &LABEL, &digest, &mut unused) {
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

    check_chain(&mut dpe)?;
    pw_log::info!("certificate chain: ok");
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
