// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Checks the Caliptra RNG driver. The driver asks the Caliptra core for
//! random bytes over its mailbox.
//!
//! The test checks that the driver writes every byte of buffers of several
//! lengths, that two results differ, that the driver works through the
//! `rand_core` adapter, and that the SHA driver and the RNG driver can share
//! the mailbox. It does not measure the quality of the
//! random bytes.
//!
//! The test is a userspace app because the stack of the kernel's bootstrap
//! thread is too small for it.

#![no_std]
#![no_main]

use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::Sha2_384;
use openprot_hal_blocking::rng::{CryptoRng, RandCore};
use pw_status::Error;
use rand_core::TryRngCore;
use userspace::{entry, syscall};
use veer_mailbox::Mailbox;
use veer_rng::CaliptraRng;
use veer_sha::CaliptraSha;

/// Buffer lengths to fill. The driver requests at most 240 bytes with one
/// command, and the mailbox transfers 4 bytes at a time.
const LENGTHS: [usize; 12] = [1, 2, 3, 4, 5, 47, 48, 49, 239, 240, 241, 500];

/// The largest of `LENGTHS`.
const MAX_LEN: usize = 500;

/// Number of times that each buffer is filled.
const ROUNDS: usize = 16;

/// The SHA-384 digest of "abc".
const SHA384_ABC: [u8; 48] = [
    0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50, 0x07,
    0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff, 0x5b, 0xed,
    0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34, 0xc8, 0x25, 0xa7,
];

/// Checks that the driver writes every byte of a buffer of `len` bytes.
///
/// Fills a buffer of zeros `ROUNDS` times and ORs the results together. A
/// byte that the driver does not write stays zero. A byte that the driver
/// writes is zero in every round with probability 2^-128.
fn test_len(rng: &mut CaliptraRng<'_>, len: usize) -> Result<(), &'static str> {
    let mut seen = [0u8; MAX_LEN];
    let mut buffer = [0u8; MAX_LEN];
    let seen = seen.get_mut(..len).ok_or("length is too large")?;
    let buffer = buffer.get_mut(..len).ok_or("length is too large")?;
    for _ in 0..ROUNDS {
        buffer.fill(0);
        rng.fill(buffer).map_err(|_| "fill failed")?;
        seen.iter_mut()
            .zip(buffer.iter())
            .for_each(|(seen, &byte)| *seen |= byte);
    }
    if seen.contains(&0) {
        pw_log::error!("{} bytes: a byte was not written", len as u32);
        return Err("byte not written");
    }
    pw_log::info!("{} bytes: ok", len as u32);
    Ok(())
}

/// Checks that two results differ. They are equal when Caliptra returns its
/// fixed pattern, which it does when it is not debug-locked and its
/// manufacturing flag `RNG_SUPPORT_UNAVAILABLE` is set.
fn test_distinct(rng: &mut impl CryptoRng) -> Result<(), &'static str> {
    let mut first = [0u8; 32];
    let mut second = [0u8; 32];
    rng.fill(&mut first).map_err(|_| "fill failed")?;
    rng.fill(&mut second).map_err(|_| "fill failed")?;
    if first == second {
        return Err("two results are equal");
    }

    // An empty buffer needs no command.
    rng.fill(&mut []).map_err(|_| "empty fill failed")?;
    pw_log::info!("distinct: ok");
    Ok(())
}

/// Checks the driver through the `rand_core` adapter.
fn test_rand_core(rng: &mut CaliptraRng<'_>) -> Result<(), &'static str> {
    let mut rng = RandCore::new(rng);
    let first = rng.try_next_u64().map_err(|_| "try_next_u64 failed")?;
    let second = rng.try_next_u64().map_err(|_| "try_next_u64 failed")?;
    if first == second {
        return Err("two u64 results are equal");
    }
    rng.try_next_u32().map_err(|_| "try_next_u32 failed")?;
    let mut bytes = [0u8; 5];
    rng.try_fill_bytes(&mut bytes)
        .map_err(|_| "try_fill_bytes failed")?;
    pw_log::info!("rand_core: ok");
    Ok(())
}

/// Checks that random bytes can be drawn in the middle of a SHA computation
/// on the same mailbox.
fn test_shared(mailbox: &Mailbox) -> Result<(), &'static str> {
    let mut sha = CaliptraSha::new(mailbox);
    let mut rng = CaliptraRng::new(mailbox);
    let mut nonce = [0u8; 32];

    let mut op = sha.init(Sha2_384).map_err(|_| "init failed")?;
    op.update(b"a").map_err(|_| "update failed")?;
    rng.fill(&mut nonce).map_err(|_| "fill failed")?;
    op.update(b"bc").map_err(|_| "update failed")?;
    rng.fill(&mut nonce).map_err(|_| "fill failed")?;
    let digest = op.finalize().map_err(|_| "finalize failed")?;
    if AsRef::<[u8]>::as_ref(&digest) != SHA384_ABC {
        return Err("wrong digest");
    }
    pw_log::info!("shared mailbox: ok");
    Ok(())
}

fn run(mailbox: &Mailbox) -> Result<(), &'static str> {
    let mut rng = CaliptraRng::new(mailbox);
    for len in LENGTHS {
        test_len(&mut rng, len)?;
    }
    test_distinct(&mut rng)?;
    test_rand_core(&mut rng)?;
    test_shared(mailbox)
}

#[entry]
fn entry() -> pw_status::Result<()> {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };
    let result = run(&mailbox).map_err(|reason| {
        pw_log::error!("RNG test failed: {}", reason as &str);
        Error::Internal
    });
    // The kernel logs PASS or FAIL, which the test runner looks for.
    let _ = syscall::debug_shutdown(result);
    result
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in RNG test");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
