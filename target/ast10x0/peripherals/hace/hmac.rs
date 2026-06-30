// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Software RFC-2104 HMAC over the HACE SHA-2 hasher.
//!
//! Per `plans/goal.md` §2.1 / §3 item 2, HMAC is **not** the engine-native
//! `aspeed_hash_*_hmac` path: it is implemented in software as
//! `H((K0 ^ opad) ‖ H((K0 ^ ipad) ‖ msg))` on top of the already
//! parity-verified [`HaceDigest`] digest path. The key-reduction threshold is
//! the RFC-2104-correct, algorithm-dependent `key_len > block_size` (64 for
//! SHA-256, 128 for SHA-384/512) — *not* `aspeed-rust`'s flat `>128`.
//!
//! One-shot, like the authoritative model (goal.md §1.7: "HMAC — NOT
//! streaming"). Each sub-hash runs through the verified public [`HaceDigest`]
//! path (`HaceDevice` → `from_device` → `DigestInit`).
//!
//! Key reduction always copies the raw key bytes through a `.ram_nc` staging
//! buffer before DMA, ensuring the HACE engine reads from non-cacheable SRAM
//! regardless of where the caller allocated the `HmacKey` (mirrors
//! aspeed-rust's `hash_key` strategy).
//!
//! Correctness authority: published RFC-4231 known-answer vectors (§2.1).

use super::device::HaceDevice;
use super::digest::HaceDigest;
use super::error::HaceError;
use core::marker::PhantomData;
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Digest, Sha2_256, Sha2_384, Sha2_512};
use openprot_hal_blocking::mac::scoped::{MacInit, MacOp};
use openprot_hal_blocking::mac::{
    ErrorType as MacErrorType, HmacSha2_256, HmacSha2_384, HmacSha2_512, KeyHandle,
};

/// Maximum HMAC key length accepted by [`HmacKey`]. Comfortably covers the
/// RFC-4231 vectors (longest key 131 B); longer keys are reduced via `H(K)`
/// anyway when `len > block_size`.
pub const HMAC_KEY_CAP: usize = 256;

/// Largest HMAC message accepted (one-shot, like the authoritative model).
pub const HMAC_MSG_CAP: usize = 1024;


// ----- DMA-safe key staging buffer ------------------------------------
//
// The HACE engine performs DMA from the SG-list source addresses.  When a
// key arrives from an arbitrary call site (e.g. from a `HmacKey` on the
// caller's stack, which is in cacheable SRAM), the DMA engine may observe
// stale zeros from physical RAM instead of the CPU-cached key bytes.
// Aspeed-rust fixes this in `hash_key()` by always copying the key bytes
// into `ctx.buffer` (`.ram_nc`) before hashing.  We do the same here: any
// key that requires reduction (len > block_size) is first CPU-copied into
// this `.ram_nc` staging buffer, then `one_shot!` reads it from there.
struct HmacKeyNc(core::cell::UnsafeCell<[u8; HMAC_KEY_CAP]>);
// SAFETY: single-threaded HACE driver; key-reduction calls are serialised by
// the single-instance/non-reentrant `HaceDevice` contract.
unsafe impl Sync for HmacKeyNc {}

#[unsafe(link_section = ".ram_nc")]
static HMAC_KEY_NC: HmacKeyNc = HmacKeyNc(core::cell::UnsafeCell::new([0u8; HMAC_KEY_CAP]));

/// Run one full digest of `$input` via the verified public path, yielding the
/// algorithm's `Digest`. Used for all three HMAC sub-hashes so HMAC rides
/// exactly the digest path the KAT suite covers.
macro_rules! one_shot {
    ($inner:ty, $algo:expr, $pb:expr, $input:expr) => {{
        // SAFETY: the whole HACE driver is single-threaded / non-reentrant;
        // the HMAC controller upholds that contract for its sub-hashes.
        let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut dev = dev.with_timeout_polls($pb);
        // SAFETY: same single-threaded exclusivity contract.
        let mut dd = unsafe { HaceDigest::<$inner>::from_device(&mut dev) };
        let mut op = dd.init($algo)?;
        op.update($input)?;
        op.finalize()?
    }};
}

/// Owned, bounded HMAC key handle. Variable length up to [`HMAC_KEY_CAP`].
#[derive(Clone)]
pub struct HmacKey {
    bytes: [u8; HMAC_KEY_CAP],
    len: usize,
}

impl HmacKey {
    /// Build a key from a byte slice.
    ///
    /// Returns [`HaceError::InvalidInput`] if `key.len() > HMAC_KEY_CAP`.
    pub fn from_slice(key: &[u8]) -> Result<Self, HaceError> {
        if key.len() > HMAC_KEY_CAP {
            return Err(HaceError::InvalidInput);
        }
        let mut bytes = [0u8; HMAC_KEY_CAP];
        bytes[..key.len()].copy_from_slice(key);
        Ok(Self {
            bytes,
            len: key.len(),
        })
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl KeyHandle for HmacKey {}

/// HMAC controller. Holds only the poll budget; each sub-hash binds the
/// singleton HACE through the standard [`HaceDevice`] path.
pub struct HaceHmac {
    poll_budget: u32,
}

impl HaceHmac {
    pub(crate) fn new(poll_budget: u32) -> Self {
        Self { poll_budget }
    }

    /// Construct an HMAC controller from a [`super::device::HaceDevice`].
    ///
    /// # Safety
    /// Caller must ensure no concurrent or reentrant HACE access for the
    /// lifetime of HMAC operations created from this controller.
    pub unsafe fn from_device<Y: FnMut(u32)>(device: &mut super::device::HaceDevice<Y>) -> Self {
        Self::new(device.poll_budget)
    }
}

impl MacErrorType for HaceHmac {
    type Error = HaceError;
}

/// In-flight HMAC operation: retained `K0^ipad` / `K0^opad` blocks plus the
/// buffered message. The HMAC is computed entirely at `finalize`.
pub struct HaceHmacCtx<Inner> {
    ipad: [u8; 128],
    opad: [u8; 128],
    block: usize,
    msg: [u8; HMAC_MSG_CAP],
    msg_len: usize,
    poll_budget: u32,
    _inner: PhantomData<Inner>,
}

impl<Inner> MacErrorType for HaceHmacCtx<Inner> {
    type Error = HaceError;
}

macro_rules! hmac_variant {
    ($mac:ty, $inner:ty, $algo:expr, $b:expr, $nw:expr) => {
        impl MacInit<$mac> for HaceHmac {
            type Key = HmacKey;
            type OpContext<'a>
                = HaceHmacCtx<$inner>
            where
                Self: 'a;

            fn init<'a>(
                &'a mut self,
                _algo: $mac,
                key: HmacKey,
            ) -> Result<Self::OpContext<'a>, HaceError> {
                let pb = self.poll_budget;
                let k = key.as_slice();

                // K0: key reduced to <= block_size, then zero-padded to block.
                // RFC-2104-correct threshold: reduce only when len > block_size.
                let mut k0 = [0u8; 128];
                if k.len() > $b {
                    // Copy the raw key into the .ram_nc staging buffer so that
                    // the HACE DMA engine reads from non-cacheable SRAM.
                    // Passing a stack pointer directly as the DMA source can
                    // produce wrong results when the CPU cache has not been
                    // written back (observed: SHA-512 key reduction reads zeros
                    // from physical RAM while correct bytes are cache-resident).
                    // This mirrors aspeed-rust's `hash_key()` which always does
                    // `ctx.buffer[..key_len].copy_from_slice(key_bytes)` first.
                    let key_nc: &[u8] = unsafe {
                        let buf = &mut *HMAC_KEY_NC.0.get();
                        buf[..k.len()].copy_from_slice(k);
                        &buf[..k.len()]
                    };
                    let kh = one_shot!($inner, $algo, pb, key_nc);
                    let hb = kh.as_bytes();
                    k0[..hb.len()].copy_from_slice(hb);
                } else {
                    k0[..k.len()].copy_from_slice(k);
                }

                let mut ipad = [0u8; 128];
                let mut opad = [0u8; 128];
                ipad[..$b].copy_from_slice(&k0[..$b]);
                opad[..$b].copy_from_slice(&k0[..$b]);
                for i in 0..$b {
                    ipad[i] ^= 0x36;
                    opad[i] ^= 0x5c;
                }

                Ok(HaceHmacCtx {
                    ipad,
                    opad,
                    block: $b,
                    msg: [0u8; HMAC_MSG_CAP],
                    msg_len: 0,
                    poll_budget: pb,
                    _inner: PhantomData,
                })
            }
        }

        impl MacOp for HaceHmacCtx<$inner> {
            type Output = Digest<$nw>;

            fn update(&mut self, input: &[u8]) -> Result<(), HaceError> {
                let end = self
                    .msg_len
                    .checked_add(input.len())
                    .ok_or(HaceError::InvalidInput)?;
                if end > HMAC_MSG_CAP {
                    return Err(HaceError::InvalidInput);
                }
                self.msg[self.msg_len..end].copy_from_slice(input);
                self.msg_len = end;
                Ok(())
            }

            fn finalize(self) -> Result<Self::Output, HaceError> {
                let b = self.block;
                let pb = self.poll_budget;

                // inner = H(K0^ipad ‖ msg)
                //
                // Feed ipad and msg via two update() calls — no contiguous scratch
                // buffer needed on the stack. update() copies each slice through
                // ctx.buffer (.ram_nc) before DMA, so stack-resident sources
                // (ipad, msg) are DMA-safe (D1/D2 guarantee).
                let inner = {
                    // SAFETY: single-threaded HMAC controller upholds non-reentrant contract.
                    let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
                    let mut dev = dev.with_timeout_polls(pb);
                    // SAFETY: same single-threaded exclusivity contract.
                    let mut dd = unsafe { HaceDigest::<$inner>::from_device(&mut dev) };
                    let mut op = dd.init($algo)?;
                    op.update(&self.ipad[..b])?;
                    op.update(&self.msg[..self.msg_len])?;
                    op.finalize()?
                };
                let inner_bytes = inner.as_bytes();

                // outer = H(K0^opad ‖ inner)
                // SAFETY: same single-threaded exclusivity contract.
                let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
                let mut dev = dev.with_timeout_polls(pb);
                // SAFETY: same contract.
                let mut dd = unsafe { HaceDigest::<$inner>::from_device(&mut dev) };
                let mut op = dd.init($algo)?;
                op.update(&self.opad[..b])?;
                op.update(inner_bytes)?;
                op.finalize()
            }
        }
    };
}

// Block sizes / output words: SHA-256 -> 64 B block, 8 words (32 B);
// SHA-384 -> 128 B, 12 words (48 B); SHA-512 -> 128 B, 16 words (64 B).
hmac_variant!(HmacSha2_256, Sha2_256, Sha2_256, 64, 8);
hmac_variant!(HmacSha2_384, Sha2_384, Sha2_384, 128, 12);
hmac_variant!(HmacSha2_512, Sha2_512, Sha2_512, 128, 16);
