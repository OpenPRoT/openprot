// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Generic HACE Digest HAL adapter for OpenPRoT

use super::constants::{
    HACE_SG_LAST, POLL_YIELD_NS, SHA256_HASH_CMD, SHA384_HASH_CMD, SHA512_HASH_CMD,
};
use super::context::{
    HashContext, HACE_BLOCK_SIZE, HACE_BLOCK_SIZE_128, SHA256_DIGEST_SIZE, SHA256_IV,
    SHA384_DIGEST_SIZE, SHA384_IV, SHA512_DIGEST_SIZE, SHA512_IV,
};
use super::device::Hace;
use super::error::HaceError;
use super::helpers::{fill_padding, load_iv, ptr_to_u32};
use core::marker::PhantomData;
use openprot_hal_blocking::digest::scoped::{DigestCtrlReset, DigestInit, DigestOp};
use openprot_hal_blocking::digest::{
    Digest, DigestAlgorithm, ErrorType, Sha2_256, Sha2_384, Sha2_512,
};
use zerocopy::IntoBytes;

/// Per-algorithm constants required by the HACE driver.
///
/// Mirrors the role of `HashAlgo` methods in aspeed-rust: provides the hardware
/// command word, block size, digest size, and IV for each supported algorithm.
pub(crate) trait HaceDigestSpec: DigestAlgorithm
where
    Self::Digest: IntoBytes,
{
    const HASH_CMD: u32;
    const BLOCK_SIZE: usize;

    fn iv() -> &'static [u32];
    fn digest_from_context(ctx: &HashContext) -> Self::Digest;
}

impl HaceDigestSpec for Sha2_256 {
    const HASH_CMD: u32 = SHA256_HASH_CMD;
    const BLOCK_SIZE: usize = HACE_BLOCK_SIZE;

    fn iv() -> &'static [u32] {
        &SHA256_IV
    }

    fn digest_from_context(ctx: &HashContext) -> Self::Digest {
        let mut out = [0u32; SHA256_DIGEST_SIZE / 4];
        for (i, dst) in out.iter_mut().enumerate() {
            let off = i * 4;
            if let Some(chunk) = ctx.digest.get(off..off + 4) {
                *dst = u32::from_ne_bytes(<[u8; 4]>::try_from(chunk).unwrap_or([0u8; 4]));
            }
        }
        Digest::new(out)
    }
}

impl HaceDigestSpec for Sha2_384 {
    const HASH_CMD: u32 = SHA384_HASH_CMD;
    const BLOCK_SIZE: usize = HACE_BLOCK_SIZE_128;

    fn iv() -> &'static [u32] {
        &SHA384_IV
    }

    fn digest_from_context(ctx: &HashContext) -> Self::Digest {
        let mut out = [0u32; SHA384_DIGEST_SIZE / 4];
        for (i, dst) in out.iter_mut().enumerate() {
            let off = i * 4;
            if let Some(chunk) = ctx.digest.get(off..off + 4) {
                *dst = u32::from_ne_bytes(<[u8; 4]>::try_from(chunk).unwrap_or([0u8; 4]));
            }
        }
        Digest::new(out)
    }
}

impl HaceDigestSpec for Sha2_512 {
    const HASH_CMD: u32 = SHA512_HASH_CMD;
    const BLOCK_SIZE: usize = HACE_BLOCK_SIZE_128;

    fn iv() -> &'static [u32] {
        &SHA512_IV
    }

    fn digest_from_context(ctx: &HashContext) -> Self::Digest {
        let mut out = [0u32; SHA512_DIGEST_SIZE / 4];
        for (i, dst) in out.iter_mut().enumerate() {
            let off = i * 4;
            if let Some(chunk) = ctx.digest.get(off..off + 4) {
                *dst = u32::from_ne_bytes(<[u8; 4]>::try_from(chunk).unwrap_or([0u8; 4]));
            }
        }
        Digest::new(out)
    }
}

pub struct HaceDigest<'a, T: DigestAlgorithm, Y: FnMut(u32)> {
    /// The engine owner. The exclusive borrow is the arbiter: a second `HaceDigest`/`HaceHmac`/
    /// `AesCipher` needs `&mut Hace` again, which is a borrow-check error while this one lives.
    pub(crate) dev: &'a mut Hace<Y>,
    pub(crate) ctx: &'a mut HashContext,
    _algo: PhantomData<T>,
}

impl<'a, T: DigestAlgorithm, Y: FnMut(u32)> HaceDigest<'a, T, Y> {
    /// Construct a digest adapter from a [`Hace`].
    ///
    /// Safe: a `Hace` can only be built from the HACE ownership token, so the exclusive borrow
    /// is exclusive access to the engine for the lifetime of the returned [`HaceDigest`].
    pub fn from_device(dev: &'a mut Hace<Y>) -> Self {
        // SAFETY: `dev` holds the sole pointer to this `.ram_nc` context (acquired once at
        // construction from the token), and the live `&'a mut Hace` gates it, so no other `&mut`
        // to it is live. The context is not part of `Hace`'s own memory, so holding both `&mut`
        // does not alias.
        let ctx: &'a mut HashContext = unsafe { &mut *dev.ctx };
        Self {
            dev,
            ctx,
            _algo: PhantomData,
        }
    }

    /// DMA one full block held in `ctx.buffer` (always `.ram_nc`) to the engine.
    ///
    /// Called only when `ctx.bufcnt == ctx.block_size`. Resets `bufcnt` to 0
    /// on success so the buffer can be reused for the next block.
    fn flush_block(&mut self) -> Result<(), HaceError> {
        let buf_ptr = ptr_to_u32(self.ctx.buffer.as_ptr())?;
        let bufcnt = self.ctx.bufcnt;
        self.ctx.sg[0].addr = buf_ptr;
        self.ctx.sg[0].len = bufcnt | HACE_SG_LAST;

        let sg_addr = ptr_to_u32(self.ctx.sg.as_ptr())?;
        let digest_addr = ptr_to_u32(self.ctx.digest.as_ptr())?;
        let method = self.ctx.method;

        self.dev.clear_hash_intflag();
        self.dev
            .program_hash_operation(sg_addr, digest_addr, bufcnt, method);

        for _ in 0..self.dev.poll_budget {
            if self.dev.hash_intflag_is_set() {
                self.ctx.bufcnt = 0;
                return Ok(());
            }
            (self.dev.yield_fn)(POLL_YIELD_NS);
        }

        self.dev.stop_hash_operation();
        Err(HaceError::Timeout)
    }
}

impl<'a, T: DigestAlgorithm, Y: FnMut(u32)> ErrorType for HaceDigest<'a, T, Y> {
    type Error = HaceError;
}

impl<'a, T: HaceDigestSpec, Y: FnMut(u32)> DigestInit<T> for HaceDigest<'a, T, Y>
where
    T::Digest: IntoBytes,
{
    type OpContext<'b>
        = HaceDigest<'b, T, Y>
    where
        Self: 'b;

    fn init(&mut self, _algo: T) -> Result<Self::OpContext<'_>, Self::Error> {
        // Mirror aspeed-rust init sequence exactly:
        // 1. Set method (hardware command word, includes HACE_SG_EN)
        // 2. Load IV into digest buffer
        // 3. Set block_size
        // 4. Zero bufcnt and digcnt
        let iv = T::iv();
        self.ctx.method = T::HASH_CMD;
        let iv_bytes = iv.len().saturating_mul(4);
        match self.ctx.digest.get_mut(..iv_bytes) {
            Some(dst) => load_iv(dst, iv)?,
            None => return Err(HaceError::InvalidInput),
        }
        self.ctx.block_size = T::BLOCK_SIZE as u32;
        self.ctx.bufcnt = 0;
        self.ctx.digcnt = [0; 2];
        Ok(HaceDigest {
            dev: &mut *self.dev,
            ctx: &mut *self.ctx,
            _algo: PhantomData,
        })
    }
}

impl<'a, T: HaceDigestSpec, Y: FnMut(u32)> DigestOp for HaceDigest<'a, T, Y>
where
    T::Digest: IntoBytes,
{
    type Output = T::Digest;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        let input_len = u32::try_from(input.len()).map_err(|_| HaceError::InvalidInput)?;

        // Accumulate total byte count (with carry into digcnt[1]).
        let (new_digcnt, carry) = self.ctx.digcnt[0].overflowing_add(u64::from(input_len));
        self.ctx.digcnt[0] = new_digcnt;
        if carry {
            self.ctx.digcnt[1] += 1;
        }

        // Copy all input through ctx.buffer (.ram_nc) one block at a time.
        //
        // DMA safety (D1/D2): the HACE engine reads data by physical address
        // via SG descriptors. If the caller's `input` slice is in flash, rodata,
        // or a cacheable stack buffer, the engine may read stale zeros from
        // physical RAM. By staging every byte through `ctx.buffer` (which is
        // placed in `.ram_nc` by the linker) we guarantee the DMA source is
        // always in non-cacheable SRAM. This also covers HMAC's `scratch`
        // buffer (D2) since it routes through `update()` via `one_shot!`.
        let block_size = self.ctx.block_size as usize;
        let mut offset = 0usize;
        while offset < input.len() {
            let bufcnt = self.ctx.bufcnt as usize;
            let space = block_size.saturating_sub(bufcnt);
            let chunk_len = core::cmp::min(space, input.len() - offset);
            // Invariant: block_size is 64 or 128 (set by init()); if somehow
            // zero, bail rather than infinite-loop.
            if chunk_len == 0 {
                return Err(HaceError::InvalidInput);
            }

            let dst = self
                .ctx
                .buffer
                .get_mut(bufcnt..bufcnt.saturating_add(chunk_len))
                .ok_or(HaceError::InvalidInput)?;
            let src = input
                .get(offset..offset.saturating_add(chunk_len))
                .ok_or(HaceError::InvalidInput)?;
            // Element-wise copy instead of `copy_from_slice`: `dst` and `src` are
            // both `chunk_len` long by construction, but the optimizer cannot
            // prove `dst.len() == src.len()` through the two `get`/`get_mut`
            // range slices, so `copy_from_slice` would keep its length-mismatch
            // panic branch. The zip-copy is provably panic-free.
            for (d, s) in dst.iter_mut().zip(src.iter()) {
                *d = *s;
            }

            self.ctx.bufcnt += chunk_len as u32;
            offset += chunk_len;

            // When the buffer holds exactly one full block, flush it via DMA.
            if self.ctx.bufcnt == self.ctx.block_size {
                self.flush_block()?;
            }
        }

        Ok(())
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        let this = self;

        // Append SHA padding at ctx.buffer[bufcnt..]; updates ctx.bufcnt.
        fill_padding(this.ctx, 0);

        // Set up SG[0] descriptor pointing at the padded buffer (mirror aspeed-rust finalize).
        let buf_ptr = ptr_to_u32(this.ctx.buffer.as_ptr())?;
        let bufcnt = this.ctx.bufcnt;
        this.ctx.sg[0].addr = buf_ptr;
        this.ctx.sg[0].len = bufcnt | HACE_SG_LAST;

        let sg_addr = ptr_to_u32(this.ctx.sg.as_ptr())?;
        let digest_addr = ptr_to_u32(this.ctx.digest.as_ptr())?;
        let method = this.ctx.method;

        this.dev.clear_hash_intflag();
        this.dev
            .program_hash_operation(sg_addr, digest_addr, bufcnt, method);

        for _ in 0..this.dev.poll_budget {
            if this.dev.hash_intflag_is_set() {
                // Invalidate stale CPU cache lines over the digest buffer.
                (this.dev.cache_flush)();
                let result = T::digest_from_context(this.ctx);
                // Cleanup context (mirrors cleanup_context in aspeed-rust).
                this.ctx.bufcnt = 0;
                this.ctx.digcnt = [0; 2];
                this.ctx.buffer.fill(0);
                this.ctx.digest.fill(0);
                this.dev.stop_hash_operation();
                return Ok(result);
            }
            (this.dev.yield_fn)(POLL_YIELD_NS);
        }

        this.dev.stop_hash_operation();
        Err(HaceError::Timeout)
    }
}

impl<'a, T: DigestAlgorithm, Y: FnMut(u32)> DigestCtrlReset for HaceDigest<'a, T, Y> {
    fn reset(&mut self) -> Result<(), Self::Error> {
        self.ctx.bufcnt = 0;
        self.ctx.digcnt = [0; 2];
        self.ctx.buffer.fill(0);
        self.ctx.digest.fill(0);
        self.dev.stop_hash_operation();
        Ok(())
    }
}
