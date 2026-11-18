// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Checks the Caliptra ECDSA driver. The driver asks the Caliptra core over
//! its mailbox to verify a P-384 signature.
//!
//! The test verifies the signature of a known-answer vector. It then checks
//! that verification fails with another digest, with another key, with a
//! changed signature, and with an `r` and an `s` that are out of range. On
//! the FPGA it also checks that the driver rejects a public key that is not
//! a point on the curve.
//!
//! The test is a userspace app because the stack of the kernel's bootstrap
//! thread is too small for it.

#![no_std]
#![no_main]

use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Digest, Sha2_384};
use openprot_hal_blocking::ecdsa::{
    EcdsaVerify, Error as _, ErrorKind, P384PublicKey, P384Signature,
};
use pw_status::Error;
use userspace::{entry, syscall};
use veer_ecdsa::{CaliptraEcdsa, CaliptraEcdsaError};
use veer_mailbox::{Mailbox, MailboxError};
use veer_sha::CaliptraSha;

// The vector is from RFC 6979, appendix A.2.6: the signature of the message
// "sample" with SHA-384.

/// The message that is signed.
const MESSAGE: &[u8] = b"sample";

/// The x coordinate of the public key.
const KEY_X: [u8; 48] = [
    0xec, 0x3a, 0x4e, 0x41, 0x5b, 0x4e, 0x19, 0xa4, 0x56, 0x86, 0x18, 0x02, 0x9f, 0x42, 0x7f, 0xa5,
    0xda, 0x9a, 0x8b, 0xc4, 0xae, 0x92, 0xe0, 0x2e, 0x06, 0xaa, 0xe5, 0x28, 0x6b, 0x30, 0x0c, 0x64,
    0xde, 0xf8, 0xf0, 0xea, 0x90, 0x55, 0x86, 0x60, 0x64, 0xa2, 0x54, 0x51, 0x54, 0x80, 0xbc, 0x13,
];

/// The y coordinate of the public key.
const KEY_Y: [u8; 48] = [
    0x80, 0x15, 0xd9, 0xb7, 0x2d, 0x7d, 0x57, 0x24, 0x4e, 0xa8, 0xef, 0x9a, 0xc0, 0xc6, 0x21, 0x89,
    0x67, 0x08, 0xa5, 0x93, 0x67, 0xf9, 0xdf, 0xb9, 0xf5, 0x4c, 0xa8, 0x4b, 0x3f, 0x1c, 0x9d, 0xb1,
    0x28, 0x8b, 0x23, 0x1c, 0x3a, 0xe0, 0xd4, 0xfe, 0x73, 0x44, 0xfd, 0x25, 0x33, 0x26, 0x47, 0x20,
];

/// The y coordinate of the negative of the public key. The point with x
/// coordinate `KEY_X` and this y coordinate is on the curve.
const NEGATED_KEY_Y: [u8; 48] = [
    0x7f, 0xea, 0x26, 0x48, 0xd2, 0x82, 0xa8, 0xdb, 0xb1, 0x57, 0x10, 0x65, 0x3f, 0x39, 0xde, 0x76,
    0x98, 0xf7, 0x5a, 0x6c, 0x98, 0x06, 0x20, 0x46, 0x0a, 0xb3, 0x57, 0xb4, 0xc0, 0xe3, 0x62, 0x4d,
    0xd7, 0x74, 0xdc, 0xe2, 0xc5, 0x1f, 0x2b, 0x01, 0x8c, 0xbb, 0x02, 0xdb, 0xcc, 0xd9, 0xb8, 0xdf,
];

/// The `r` component of the signature.
const SIGNATURE_R: [u8; 48] = [
    0x94, 0xed, 0xbb, 0x92, 0xa5, 0xec, 0xb8, 0xaa, 0xd4, 0x73, 0x6e, 0x56, 0xc6, 0x91, 0x91, 0x6b,
    0x3f, 0x88, 0x14, 0x06, 0x66, 0xce, 0x9f, 0xa7, 0x3d, 0x64, 0xc4, 0xea, 0x95, 0xad, 0x13, 0x3c,
    0x81, 0xa6, 0x48, 0x15, 0x2e, 0x44, 0xac, 0xf9, 0x6e, 0x36, 0xdd, 0x1e, 0x80, 0xfa, 0xbe, 0x46,
];

/// The `s` component of the signature.
const SIGNATURE_S: [u8; 48] = [
    0x99, 0xef, 0x4a, 0xeb, 0x15, 0xf1, 0x78, 0xce, 0xa1, 0xfe, 0x40, 0xdb, 0x26, 0x03, 0x13, 0x8f,
    0x13, 0x0e, 0x74, 0x0a, 0x19, 0x62, 0x45, 0x26, 0x20, 0x3b, 0x63, 0x51, 0xd0, 0xa3, 0xa9, 0x4f,
    0xa3, 0x29, 0xc1, 0x45, 0x78, 0x6e, 0x67, 0x9e, 0x7b, 0x82, 0xc7, 0x1a, 0x38, 0x62, 0x8a, 0xc8,
];

/// The order of the P-384 curve. A valid `r` or `s` is less than the order.
const ORDER: [u8; 48] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xc7, 0x63, 0x4d, 0x81, 0xf4, 0x37, 0x2d, 0xdf,
    0x58, 0x1a, 0x0d, 0xb2, 0x48, 0xb0, 0xa7, 0x7a, 0xec, 0xec, 0x19, 0x6a, 0xcc, 0xc5, 0x29, 0x73,
];

/// Returns the SHA-384 digest of `message`.
fn sha384(mailbox: &Mailbox, message: &[u8]) -> Result<Digest<12>, &'static str> {
    let mut sha = CaliptraSha::new(mailbox);
    let mut op = sha.init(Sha2_384).map_err(|_| "SHA init failed")?;
    op.update(message).map_err(|_| "SHA update failed")?;
    op.finalize().map_err(|_| "SHA finalize failed")
}

/// Returns `bytes` with its last byte changed.
fn changed(mut bytes: [u8; 48]) -> [u8; 48] {
    if let Some(last) = bytes.last_mut() {
        *last ^= 1;
    }
    bytes
}

/// Checks that the driver verifies the signature.
fn accepts(
    ecdsa: &mut CaliptraEcdsa<'_>,
    key: &P384PublicKey,
    digest: Digest<12>,
    signature: &P384Signature,
    failure: &'static str,
) -> Result<(), &'static str> {
    ecdsa.verify(key, digest, signature).map_err(|error| {
        if let CaliptraEcdsaError::Mailbox(MailboxError::CommandFailed(code)) = error {
            pw_log::error!("verify: Caliptra error {:#x}", code as u32);
        }
        failure
    })
}

/// Checks that the driver reports that the signature is invalid.
fn rejects(
    ecdsa: &mut CaliptraEcdsa<'_>,
    key: &P384PublicKey,
    digest: Digest<12>,
    signature: &P384Signature,
    failure: &'static str,
) -> Result<(), &'static str> {
    match ecdsa.verify(key, digest, signature) {
        Err(error) if error.kind() == ErrorKind::InvalidSignature => Ok(()),
        Err(CaliptraEcdsaError::Mailbox(MailboxError::CommandFailed(code))) => {
            pw_log::error!("verify: Caliptra error {:#x}", code as u32);
            Err(failure)
        }
        _ => Err(failure),
    }
}

fn run(mailbox: &Mailbox) -> Result<(), &'static str> {
    let mut ecdsa = CaliptraEcdsa::new(mailbox);
    let digest = sha384(mailbox, MESSAGE)?;
    let key = P384PublicKey::new(KEY_X, KEY_Y);
    let signature = P384Signature::new(SIGNATURE_R, SIGNATURE_S);

    accepts(
        &mut ecdsa,
        &key,
        digest,
        &signature,
        "signature does not verify",
    )?;
    pw_log::info!("valid signature: ok");

    let other_digest = sha384(mailbox, b"simple")?;
    rejects(
        &mut ecdsa,
        &key,
        other_digest,
        &signature,
        "signature verifies with another digest",
    )?;
    pw_log::info!("other digest: ok");

    let other_key = P384PublicKey::new(KEY_X, NEGATED_KEY_Y);
    rejects(
        &mut ecdsa,
        &other_key,
        digest,
        &signature,
        "signature verifies with another key",
    )?;
    pw_log::info!("other key: ok");

    let changed_r = P384Signature::new(changed(SIGNATURE_R), SIGNATURE_S);
    rejects(
        &mut ecdsa,
        &key,
        digest,
        &changed_r,
        "signature with a changed r verifies",
    )?;
    let changed_s = P384Signature::new(SIGNATURE_R, changed(SIGNATURE_S));
    rejects(
        &mut ecdsa,
        &key,
        digest,
        &changed_s,
        "signature with a changed s verifies",
    )?;
    pw_log::info!("changed signature: ok");

    let zero_r = P384Signature::new([0; 48], SIGNATURE_S);
    rejects(
        &mut ecdsa,
        &key,
        digest,
        &zero_r,
        "signature with a zero r verifies",
    )?;
    let large_s = P384Signature::new(SIGNATURE_R, ORDER);
    rejects(
        &mut ecdsa,
        &key,
        digest,
        &large_s,
        "signature with an s that equals the order verifies",
    )?;
    pw_log::info!("out of range: ok");

    // The Caliptra emulator panics when it gets a public key that is not a
    // point on the curve, so these cases run only on the FPGA.
    #[cfg(feature = "fpga")]
    {
        let off_curve = P384PublicKey::new(KEY_X, changed(KEY_Y));
        let large_x = P384PublicKey::new([0xff; 48], KEY_Y);
        for invalid_key in [off_curve, large_x] {
            match ecdsa.verify(&invalid_key, digest, &signature) {
                Err(error) if error.kind() == ErrorKind::InvalidPoint => {}
                Err(_) => return Err("invalid key gives the wrong error"),
                Ok(()) => return Err("signature verifies with an invalid key"),
            }
        }
        pw_log::info!("invalid key: ok");
    }

    // The driver still verifies the signature after commands that failed.
    accepts(
        &mut ecdsa,
        &key,
        digest,
        &signature,
        "signature does not verify after errors",
    )?;
    pw_log::info!("verify after errors: ok");
    Ok(())
}

#[entry]
fn entry() -> pw_status::Result<()> {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };
    let result = run(&mailbox).map_err(|reason| {
        pw_log::error!("ECDSA test failed: {}", reason as &str);
        Error::Internal
    });
    // The kernel logs PASS or FAIL, which the test runner looks for.
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in ECDSA test");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
