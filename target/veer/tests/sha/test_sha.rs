// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Checks that the Caliptra SHA driver computes the expected SHA-384 and
//! SHA-512 digests of known messages, through the scoped and owned digest HAL
//! traits. The driver sends each part of a message to the Caliptra core over
//! its mailbox.
//!
//! The test is a userspace app because the stack of the kernel's bootstrap
//! thread is too small for it. That stack is 2 KB and interrupts use it too.
//! The test needs about 2.1 KB.

#![no_std]
#![no_main]

use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::digest::{DigestAlgorithm, Sha2_384, Sha2_512};
use pw_status::Error;
use userspace::{entry, syscall};
use vectors::Digests;
use veer_mailbox::Mailbox;
use veer_sha::CaliptraSha;

mod vectors;

fn check(algorithm: &str, case: &str, digest: &[u8], expected: &[u8]) -> Result<(), &'static str> {
    if digest != expected {
        pw_log::error!("{} {}: wrong digest", algorithm as &str, case as &str);
        return Err("wrong digest");
    }
    pw_log::info!("{} {}: ok", algorithm as &str, case as &str);
    Ok(())
}

/// Hashes `message` with the scoped digest API. If `pieces` is set, the
/// message is written in pieces of 1 to 13 bytes, so that most commands end
/// with a partial word.
fn hash<A>(
    sha: &mut CaliptraSha,
    algorithm: A,
    message: &[u8],
    pieces: bool,
) -> Result<A::Digest, &'static str>
where
    A: DigestAlgorithm,
    CaliptraSha: openprot_hal_blocking::digest::scoped::DigestInit<A>,
{
    use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};

    let mut op = sha.init(algorithm).map_err(|_| "init failed")?;
    if pieces {
        let mut rest = message;
        let mut len = 1;
        while let Some((piece, after)) = rest.split_at_checked(len.min(rest.len())) {
            if piece.is_empty() {
                break;
            }
            op.update(piece).map_err(|_| "update failed")?;
            rest = after;
            len = len % 13 + 1;
        }
    } else {
        op.update(message).map_err(|_| "update failed")?;
    }
    op.finalize().map_err(|_| "finalize failed")
}

/// Starts a computation with the scoped digest API and drops it.
fn start_and_drop<A>(sha: &mut CaliptraSha, algorithm: A) -> Result<(), &'static str>
where
    A: DigestAlgorithm,
    CaliptraSha: openprot_hal_blocking::digest::scoped::DigestInit<A>,
{
    use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};

    let mut op = sha.init(algorithm).map_err(|_| "init failed")?;
    op.update(b"dropped").map_err(|_| "update failed")
}

/// Tests the scoped digest API for one algorithm.
fn test_scoped<A, const L: usize>(
    sha: &mut CaliptraSha,
    algorithm: A,
    name: &str,
    digests: &Digests<L>,
) -> Result<(), &'static str>
where
    A: DigestAlgorithm,
    A::Digest: AsRef<[u8]>,
    CaliptraSha: openprot_hal_blocking::digest::scoped::DigestInit<A>,
{
    use openprot_hal_blocking::digest::scoped::DigestCtrlReset;

    for ((case, message), expected) in vectors::MESSAGES.iter().zip(&digests.messages) {
        let digest = hash(sha, algorithm, message, false)?;
        check(name, case, digest.as_ref(), expected)?;
    }

    // The pattern message does not fit in one mailbox command.
    let digest = hash(sha, algorithm, &vectors::PATTERN, false)?;
    check(name, "pattern", digest.as_ref(), &digests.pattern)?;

    let start = vectors::PATTERN
        .get(..vectors::PIECES_LEN)
        .ok_or("pattern is too short")?;
    let digest = hash(sha, algorithm, start, true)?;
    check(name, "pieces", digest.as_ref(), &digests.pieces)?;

    // A computation that is dropped must not affect the next one.
    start_and_drop(sha, algorithm)?;
    sha.reset().map_err(|_| "reset failed")?;
    let digest = hash(sha, algorithm, b"", false)?;
    let [empty, ..] = &digests.messages;
    check(name, "after drop", digest.as_ref(), empty)
}

/// Hashes "abc" in two parts with the owned digest API.
fn hash_owned<A>(sha: CaliptraSha, algorithm: A) -> Result<(A::Digest, CaliptraSha), &'static str>
where
    A: DigestAlgorithm,
    CaliptraSha: openprot_hal_blocking::digest::owned::DigestInit<A>,
{
    use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

    let op = sha.init(algorithm).map_err(|_| "init failed")?;
    let op = op.update(b"a").map_err(|_| "update failed")?;
    let op = op.update(b"bc").map_err(|_| "update failed")?;
    op.finalize().map_err(|_| "finalize failed")
}

/// Starts a computation with the owned digest API and cancels it.
fn start_and_cancel(sha: CaliptraSha) -> Result<CaliptraSha, &'static str> {
    use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

    let op = sha.init(Sha2_384).map_err(|_| "init failed")?;
    let op = op.update(b"cancelled").map_err(|_| "update failed")?;
    Ok(op.cancel())
}

/// Tests the owned digest API.
fn test_owned(sha: CaliptraSha) -> Result<(), &'static str> {
    let (digest, sha) = hash_owned(sha, Sha2_384)?;
    let [_, abc, ..] = &vectors::SHA384.messages;
    check("owned sha384", "abc", digest.as_ref(), abc)?;

    let sha = start_and_cancel(sha)?;
    let (digest, _) = hash_owned(sha, Sha2_512)?;
    let [_, abc, ..] = &vectors::SHA512.messages;
    check("owned sha512", "after cancel", digest.as_ref(), abc)
}

fn run(mut sha: CaliptraSha) -> Result<(), &'static str> {
    test_scoped(&mut sha, Sha2_384, "sha384", &vectors::SHA384)?;
    test_scoped(&mut sha, Sha2_512, "sha512", &vectors::SHA512)?;
    test_owned(sha)
}

#[entry]
fn entry() -> pw_status::Result<()> {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };
    let result = run(CaliptraSha::new(mailbox)).map_err(|reason| {
        pw_log::error!("SHA test failed: {}", reason as &str);
        Error::Internal
    });
    // The kernel logs PASS or FAIL, which the test runner looks for.
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in SHA test");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
