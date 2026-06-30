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

/// Maximum HMAC key length accepted by [`HmacKey`]. Sized to cover the longest
/// RFC-4231 vector (131 B, case 6/7); keys longer than the algorithm's block
/// size are reduced via `H(K)` anyway, so larger values buy nothing except
/// stack pressure.
pub const HMAC_KEY_CAP: usize = 131;

/// Largest HMAC message accepted (one-shot, like the authoritative model).
pub const HMAC_MSG_CAP: usize = 1024;


// ----- DMA-safe HMAC working buffers ----------------------------------------
//
// All large HMAC buffers live in a single `.ram_nc` static so they are never
// on the stack. This is critical: `run_hace_sha2_kats` runs many back-to-back
// HMAC operations in a single stack frame; each `HaceHmacCtx` previously
// carried 256+ bytes of `ipad`/`opad` and 1024 bytes of `msg`, causing stack
// overflows (HardFault lockup) by rfc4231-3. With all big buffers here,
// `HaceHmacCtx` is just three scalars (~20 bytes) on the stack.
//
// Safety contract: only one HMAC operation is live at a time (the
// single-instance/non-reentrant `HaceDevice` contract). `init()` zeroes all
// fields before starting a new operation.
struct HmacNcBufs {
    key:  core::cell::UnsafeCell<[u8; HMAC_KEY_CAP]>,
    ipad: core::cell::UnsafeCell<[u8; 128]>,
    opad: core::cell::UnsafeCell<[u8; 128]>,
    msg:  core::cell::UnsafeCell<[u8; HMAC_MSG_CAP]>,
}
// SAFETY: single-threaded HACE driver; non-reentrant `HaceDevice` contract
// serialises all HMAC operations.
unsafe impl Sync for HmacNcBufs {}

#[unsafe(link_section = ".ram_nc")]
static HMAC_NC: HmacNcBufs = HmacNcBufs {
    key:  core::cell::UnsafeCell::new([0u8; HMAC_KEY_CAP]),
    ipad: core::cell::UnsafeCell::new([0u8; 128]),
    opad: core::cell::UnsafeCell::new([0u8; 128]),
    msg:  core::cell::UnsafeCell::new([0u8; HMAC_MSG_CAP]),
};

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
        if let Some(dst) = bytes.get_mut(..key.len()) {
            for (d, s) in dst.iter_mut().zip(key.iter()) { *d = *s; }
        }
        Ok(Self {
            bytes,
            len: key.len(),
        })
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        self.bytes.get(..self.len).unwrap_or(&[])
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

/// In-flight HMAC operation. All large buffers (ipad, opad, key, msg) live in
/// the `.ram_nc` static `HMAC_NC`; this struct holds only the scalars needed
/// to drive `finalize()` (~20 bytes on the stack).
pub struct HaceHmacCtx<Inner> {
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
                    let key_nc: &[u8] = unsafe {
                        let buf = &mut *HMAC_NC.key.get();
                        if let Some(dst) = buf.get_mut(..k.len()) {
                            for (d, s) in dst.iter_mut().zip(k.iter()) { *d = *s; }
                        }
                        buf.get(..k.len()).unwrap_or(&[])
                    };
                    let kh = one_shot!($inner, $algo, pb, key_nc);
                    let hb = kh.as_bytes();
                    if let Some(dst) = k0.get_mut(..hb.len()) {
                        for (d, s) in dst.iter_mut().zip(hb.iter()) { *d = *s; }
                    }
                } else {
                    if let Some(dst) = k0.get_mut(..k.len()) {
                        for (d, s) in dst.iter_mut().zip(k.iter()) { *d = *s; }
                    }
                }

                // Build ipad/opad in .ram_nc directly.
                // SAFETY: non-reentrant contract — exclusive access guaranteed.
                unsafe {
                    let ipad = &mut *HMAC_NC.ipad.get();
                    let opad = &mut *HMAC_NC.opad.get();
                    ipad.iter_mut().zip(k0.iter()).take($b).for_each(|(d, s)| *d = *s ^ 0x36);
                    opad.iter_mut().zip(k0.iter()).take($b).for_each(|(d, s)| *d = *s ^ 0x5c);
                    if let Some(rest) = ipad.get_mut($b..) { rest.fill(0); }
                    if let Some(rest) = opad.get_mut($b..) { rest.fill(0); }
                    (*HMAC_NC.msg.get()).fill(0);
                }

                Ok(HaceHmacCtx {
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
                // SAFETY: non-reentrant HMAC contract guarantees exclusive
                // access to HMAC_NC.msg for the lifetime of this operation.
                let buf = unsafe { &mut *HMAC_NC.msg.get() };
                // zip-copy: dst.len() == input.len() holds because the range is
                // [msg_len..end] where end = msg_len + input.len(), but the
                // compiler cannot prove that through get_mut, so copy_from_slice
                // would retain a len_mismatch_fail branch. zip stops at the
                // shorter side, making this provably panic-free.
                if let Some(dst) = buf.get_mut(self.msg_len..end) {
                    for (d, s) in dst.iter_mut().zip(input.iter()) {
                        *d = *s;
                    }
                }
                self.msg_len = end;
                Ok(())
            }

            fn finalize(self) -> Result<Self::Output, HaceError> {
                let pb = self.poll_budget;

                // inner = H(K0^ipad ‖ msg)
                // Feed ipad and msg via two update() calls — both sourced from
                // HMAC_NC (.ram_nc), so DMA-safe without any extra copy.
                let inner = {
                    let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
                    let mut dev = dev.with_timeout_polls(pb);
                    let mut dd = unsafe { HaceDigest::<$inner>::from_device(&mut dev) };
                    let mut op = dd.init($algo)?;
                    // SAFETY: HMAC_NC is exclusively owned by this operation.
                    let ipad = unsafe { &*HMAC_NC.ipad.get() };
                    op.update(ipad.get(..$b).unwrap_or(&[]))?;
                    let msg = unsafe { &*HMAC_NC.msg.get() };
                    op.update(msg.get(..self.msg_len).ok_or(HaceError::InvalidInput)?)?;
                    op.finalize()?
                };
                let inner_bytes = inner.as_bytes();

                // outer = H(K0^opad ‖ inner)
                let dev = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
                let mut dev = dev.with_timeout_polls(pb);
                let mut dd = unsafe { HaceDigest::<$inner>::from_device(&mut dev) };
                let mut op = dd.init($algo)?;
                // SAFETY: HMAC_NC is exclusively owned by this operation.
                let opad = unsafe { &*HMAC_NC.opad.get() };
                op.update(opad.get(..$b).unwrap_or(&[]))?;
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
