// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HACE AES (128/256, ECB/CBC, raw-key path).
//!
//! Reference behavior follows pinned Zephyr `aspeed_hace` (`cfe94dc`).
//! Correctness is checked with NIST AESAVS/CAVP KATs.
//!
//! Deltas:
//! - A1: borrow-arbitrated exclusivity via `&mut Hace`
//! - A2: ciphertext is plain `CT` (IV is separate)
//! - A3: key/IV context bytes are zeroized after each op and on drop
//! - A4: invalid block sizing returns `InvalidInput` before programming HW
//! - A5: AES-192 and OTP/secret-vault keys are out of scope

use super::constants::{
    AES_CMD_BASE, HACE_CMD_AES128, HACE_CMD_AES256, HACE_CMD_CBC, HACE_CMD_ECB, HACE_CMD_ENCRYPT,
    HACE_SG_LAST, POLL_YIELD_NS,
};
use super::context::CryptoContext;
use super::device::Hace;
use super::error::HaceError;
use super::helpers::ptr_to_u32;
use core::marker::PhantomData;
use openprot_hal_blocking::cipher::{
    BlockCipherMode, CipherInit, CipherMode, CipherOp, ErrorType, SymmetricCipher,
};

/// AES block size in bytes. ECB/CBC input must be block-aligned.
pub const AES_BLOCK: usize = 16;

/// Borrow-arbitrated AES operation over HACE.
///
/// Created via [`AesCipher::from_device`]. The retained `&mut` borrow prevents
/// overlapping AES/digest/HMAC operations.
pub struct AesCipher<'a, Y: FnMut(u32)> {
    /// The engine owner; the exclusive borrow is the arbiter.
    dev: &'a mut Hace<Y>,
    ctx: &'a mut CryptoContext,
}

impl<'a, Y: FnMut(u32)> AesCipher<'a, Y> {
    /// Build an AES adapter from a [`Hace`].
    ///
    /// Safe: a `Hace` can only be built from the HACE ownership token, so the exclusive borrow
    /// is exclusive access to the engine for the returned lifetime.
    pub fn from_device(dev: &'a mut Hace<Y>) -> Self {
        // SAFETY: single-owner `Hace` + exclusive live borrow gate access; the context is not
        // part of `Hace`'s own memory, so holding both `&mut` does not alias.
        let ctx: &'a mut CryptoContext = unsafe { &mut *dev.crypto_ctx };
        Self { dev, ctx }
    }

    /// Map AES key length to command bits. Reject AES-192.
    fn keylen_bits(key: &[u8]) -> Result<u32, HaceError> {
        match key.len() {
            16 => Ok(HACE_CMD_AES128),
            32 => Ok(HACE_CMD_AES256),
            _ => Err(HaceError::InvalidInput),
        }
    }

    /// One-shot AES transform.
    ///
    /// `mode_bits` is ECB or CBC. `iv` is set only for CBC. Buffers are plain
    /// data (no in-band IV). Context key/IV bytes are always zeroized.
    fn crypt(
        &mut self,
        mode_bits: u32,
        encrypt: bool,
        key: &[u8],
        iv: Option<&[u8; AES_BLOCK]>,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(), HaceError> {
        // Enforce block-aligned sizing before programming.
        if input.is_empty() || input.len() % AES_BLOCK != 0 || output.len() < input.len() {
            return Err(HaceError::InvalidInput);
        }
        let kbits = Self::keylen_bits(key)?;
        let len = u32::try_from(input.len()).map_err(|_| HaceError::InvalidInput)?;

        // Engine context: IV at [0..16) for IV modes, key at [16..16+keylen).
        self.ctx.ctx = [0u8; 64];
        if let Some(iv) = iv {
            if let Some(dst) = self.ctx.ctx.get_mut(..AES_BLOCK) {
                if let Ok(dst) = <&mut [u8; AES_BLOCK]>::try_from(dst) {
                    *dst = *iv;
                }
            }
        }
        if let Some(dst) = self.ctx.ctx.get_mut(AES_BLOCK..AES_BLOCK + key.len()) {
            dst.copy_from_slice(key);
        }

        // Point SG descriptors directly at the caller's SRAM buffers.
        //
        // The AST10x0 crypto MBUS reads and writes payload data from ordinary
        // cacheable SRAM (below 0x000A0000). Staging through the `.ram_nc`
        // window (0x000A0000+) works for the engine context and SG descriptors
        // but not for source/destination payload data: putting payloads in
        // `.ram_nc` causes the engine to fire the completion intflag without
        // writing any output (observed on hardware). Callers must therefore
        // supply buffers in ordinary SRAM (e.g. `static mut` or heap); the
        // KAT uses `static mut AES_IN / AES_OUT` which are in `.bss` / regular
        // SRAM and work correctly.
        //
        // After the engine completes, `dcache_invd_all()` is called to
        // invalidate any stale cache lines over the output buffer so the CPU
        // reads the engine-written data rather than pre-op cached zeros
        // (authority: `hace_aspeed.c` calls `cache_data_invd_all()` after
        // every crypto op; `aspeed-rust` does the same).
        let in_ptr = ptr_to_u32(input.as_ptr())?;
        let out_ptr = ptr_to_u32(output.as_mut_ptr())?;
        self.ctx.src.addr = in_ptr;
        self.ctx.src.len = len | HACE_SG_LAST;
        self.ctx.dst.addr = out_ptr;
        self.ctx.dst.len = len | HACE_SG_LAST;

        let cmd = AES_CMD_BASE | kbits | mode_bits | if encrypt { HACE_CMD_ENCRYPT } else { 0 };

        let src_desc = ptr_to_u32(core::ptr::addr_of!(self.ctx.src))?;
        let dst_desc = ptr_to_u32(core::ptr::addr_of!(self.ctx.dst))?;
        let ctx_base = ptr_to_u32(self.ctx.ctx.as_ptr())?;
        let data_len = len;

        self.dev.clear_crypto_intflag();
        self.dev
            .program_crypto_operation(src_desc, dst_desc, ctx_base, data_len, cmd);

        let mut done = false;
        for _ in 0..self.dev.poll_budget {
            if self.dev.crypto_intflag_is_set() {
                done = true;
                break;
            }
            (self.dev.yield_fn)(POLL_YIELD_NS);
        }

        // Invalidate the data cache so the CPU reads what the engine wrote.
        (self.dev.cache_flush)();

        // Always clear key/IV material from the DMA context buffer.
        self.ctx.ctx = [0u8; 64];

        if done {
            Ok(())
        } else {
            pw_log::error!(
                "hace: AES timeout: HACE1C={:#010x}, cmd={:#010x}, len={}",
                self.dev.read_hace1c() as u32,
                cmd as u32,
                len as u32,
            );
            Err(HaceError::Timeout)
        }
    }

    /// AES-ECB encrypt.
    ///
    /// `key` is 16 or 32 bytes. `pt` must be non-empty and block-aligned.
    /// `ct` must be at least `pt.len()`. Output is ciphertext only.
    pub fn ecb_encrypt(&mut self, key: &[u8], pt: &[u8], ct: &mut [u8]) -> Result<(), HaceError> {
        self.crypt(HACE_CMD_ECB, true, key, None, pt, ct)
    }

    /// AES-ECB decrypt (inverse of [`ecb_encrypt`](Self::ecb_encrypt)).
    pub fn ecb_decrypt(&mut self, key: &[u8], ct: &[u8], pt: &mut [u8]) -> Result<(), HaceError> {
        self.crypt(HACE_CMD_ECB, false, key, None, ct, pt)
    }

    /// AES-CBC encrypt with a 16-byte IV. Output excludes IV prefix.
    pub fn cbc_encrypt(
        &mut self,
        key: &[u8],
        iv: &[u8; AES_BLOCK],
        pt: &[u8],
        ct: &mut [u8],
    ) -> Result<(), HaceError> {
        self.crypt(HACE_CMD_CBC, true, key, Some(iv), pt, ct)
    }

    /// AES-CBC decrypt with a 16-byte IV; input excludes IV prefix.
    pub fn cbc_decrypt(
        &mut self,
        key: &[u8],
        iv: &[u8; AES_BLOCK],
        ct: &[u8],
        pt: &mut [u8],
    ) -> Result<(), HaceError> {
        self.crypt(HACE_CMD_CBC, false, key, Some(iv), ct, pt)
    }
}

impl<Y: FnMut(u32)> Drop for AesCipher<'_, Y> {
    fn drop(&mut self) {
        // Defensive scrub on drop.
        self.ctx.ctx = [0u8; 64];
    }
}

// ===== Optional openprot cipher-trait skin (ADR-A1) =====================
//
// Thin fixed-`N` wrapper over `AesCipher`. Kept separate because
// `SymmetricCipher` uses fixed associated buffer types and cannot express
// large streaming DMA paths.

/// AES-ECB mode marker (port-defined; the hal declares no concrete modes).
#[derive(Debug, Clone, Copy)]
pub struct Ecb;
/// AES-CBC mode marker.
#[derive(Debug, Clone, Copy)]
pub struct Cbc;

impl CipherMode for Ecb {}
impl BlockCipherMode for Ecb {}
impl CipherMode for Cbc {}
impl BlockCipherMode for Cbc {}

/// Owned AES key for the trait skin (raw-key path only).
///
/// Size selects variant: 16 => AES-128, 32 => AES-256.
///
/// The key bytes are zeroized when this value is dropped.
#[derive(Clone)]
pub enum AesKey {
    Aes128([u8; 16]),
    Aes256([u8; 32]),
}

impl AesKey {
    #[inline]
    fn as_slice(&self) -> &[u8] {
        match self {
            AesKey::Aes128(k) => k,
            AesKey::Aes256(k) => k,
        }
    }

    fn zeroize(&mut self) {
        match self {
            AesKey::Aes128(k) => k.fill(0),
            AesKey::Aes256(k) => k.fill(0),
        }
    }
}

impl Drop for AesKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Fixed-`N` openprot cipher-trait skin bound to one [`Hace`].
///
/// Each context wraps borrow-arbitrated [`AesCipher`].
pub struct AesSkin<'d, Y: FnMut(u32), const N: usize> {
    dev: &'d mut Hace<Y>,
}

impl<'d, Y: FnMut(u32), const N: usize> AesSkin<'d, Y, N> {
    /// Bind the cipher skin to the device. The exclusive borrow covers this skin and every
    /// context derived from it.
    pub fn new(dev: &'d mut Hace<Y>) -> Self {
        Self { dev }
    }
}

/// In-flight trait-skin op: core plus session key/IV.
pub struct AesOp<'a, Y: FnMut(u32), const N: usize, M> {
    core: AesCipher<'a, Y>,
    key: AesKey,
    iv: [u8; AES_BLOCK],
    _m: PhantomData<M>,
}

impl<'d, Y: FnMut(u32), const N: usize> ErrorType for AesSkin<'d, Y, N> {
    type Error = HaceError;
}

impl<'d, Y: FnMut(u32), const N: usize> SymmetricCipher for AesSkin<'d, Y, N> {
    type Key = AesKey;
    type Nonce = [u8; AES_BLOCK];
    type PlainText = [u8; N];
    type CipherText = [u8; N];
}

impl<'a, Y: FnMut(u32), const N: usize, M> ErrorType for AesOp<'a, Y, N, M> {
    type Error = HaceError;
}

impl<'a, Y: FnMut(u32), const N: usize, M> SymmetricCipher for AesOp<'a, Y, N, M> {
    type Key = AesKey;
    type Nonce = [u8; AES_BLOCK];
    type PlainText = [u8; N];
    type CipherText = [u8; N];
}

macro_rules! cipher_init {
    ($mode:ty) => {
        impl<'d, Y: FnMut(u32), const N: usize> CipherInit<$mode> for AesSkin<'d, Y, N> {
            type CipherContext<'a>
                = AesOp<'a, Y, N, $mode>
            where
                Self: 'a;

            fn init<'a>(
                &'a mut self,
                key: &Self::Key,
                nonce: &Self::Nonce,
            ) -> Result<Self::CipherContext<'a>, Self::Error> {
                // The skin's exclusive borrow is reborrowed for this op.
                let core = AesCipher::from_device(&mut *self.dev);
                Ok(AesOp {
                    core,
                    key: key.clone(),
                    iv: *nonce,
                    _m: PhantomData,
                })
            }
        }
    };
}
cipher_init!(Ecb);
cipher_init!(Cbc);

impl<'a, Y: FnMut(u32), const N: usize> CipherOp<Ecb> for AesOp<'a, Y, N, Ecb> {
    fn encrypt(&mut self, plaintext: [u8; N]) -> Result<[u8; N], HaceError> {
        const { assert!(N % AES_BLOCK == 0, "AesSkin<N>: N must be a multiple of 16") };
        let mut ct = [0u8; N];
        self.core
            .ecb_encrypt(self.key.as_slice(), &plaintext, &mut ct)?;
        Ok(ct)
    }

    fn decrypt(&mut self, ciphertext: [u8; N]) -> Result<[u8; N], HaceError> {
        const { assert!(N % AES_BLOCK == 0, "AesSkin<N>: N must be a multiple of 16") };
        let mut pt = [0u8; N];
        self.core
            .ecb_decrypt(self.key.as_slice(), &ciphertext, &mut pt)?;
        Ok(pt)
    }
}

impl<'a, Y: FnMut(u32), const N: usize> CipherOp<Cbc> for AesOp<'a, Y, N, Cbc> {
    fn encrypt(&mut self, plaintext: [u8; N]) -> Result<[u8; N], HaceError> {
        const { assert!(N % AES_BLOCK == 0, "AesSkin<N>: N must be a multiple of 16") };
        let mut ct = [0u8; N];
        self.core
            .cbc_encrypt(self.key.as_slice(), &self.iv, &plaintext, &mut ct)?;
        // Advance IV to last ciphertext block so sequential encrypt() calls
        // form a correct CBC chain instead of reusing the original IV.
        if let Some(last) = ct.get(N - AES_BLOCK..) {
            if let Ok(last) = <[u8; AES_BLOCK]>::try_from(last) {
                self.iv = last;
            }
        }
        Ok(ct)
    }

    fn decrypt(&mut self, ciphertext: [u8; N]) -> Result<[u8; N], HaceError> {
        const { assert!(N % AES_BLOCK == 0, "AesSkin<N>: N must be a multiple of 16") };
        let mut pt = [0u8; N];
        self.core
            .cbc_decrypt(self.key.as_slice(), &self.iv, &ciphertext, &mut pt)?;
        // Advance IV to last ciphertext block (CBC decrypt chaining).
        if let Some(last) = ciphertext.get(N - AES_BLOCK..) {
            if let Ok(last) = <[u8; AES_BLOCK]>::try_from(last) {
                self.iv = last;
            }
        }
        Ok(pt)
    }
}
