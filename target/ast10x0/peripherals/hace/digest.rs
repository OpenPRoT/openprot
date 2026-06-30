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
use super::error::HaceError;
use super::helpers::{fill_padding, load_iv, ptr_to_u32};
use super::registers::HaceRegisters;
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

pub struct HaceDigest<'a, T: DigestAlgorithm> {
    pub(crate) regs: HaceRegisters,
    pub(crate) ctx: &'a mut HashContext,
    pub(crate) poll_budget: u32,
    /// Cooperative yield hook, borrowed from the originating [`HaceDevice`] and
    /// invoked once between every completion poll. Type-erased so the adapter
    /// (and the `Digest*` trait impls) need not be generic over the strategy.
    pub(crate) yield_fn: &'a mut dyn FnMut(u32),
    _algo: PhantomData<T>,
}

impl<'a, T: DigestAlgorithm> HaceDigest<'a, T> {
    /// Construct a digest adapter from an existing register handle, context,
    /// poll budget, and cooperative yield hook.
    pub(crate) fn new(
        regs: HaceRegisters,
        ctx: &'a mut HashContext,
        poll_budget: u32,
        yield_fn: &'a mut dyn FnMut(u32),
    ) -> Self {
        Self {
            regs,
            ctx,
            poll_budget,
            yield_fn,
            _algo: PhantomData,
        }
    }

    /// Construct a digest adapter from a [`HaceDevice`].
    ///
    /// # Safety
    /// Caller must ensure no concurrent or reentrant HACE access for the
    /// lifetime of the returned [`HaceDigest`].
    pub unsafe fn from_device<Y: FnMut(u32)>(device: &'a mut super::device::HaceDevice<Y>) -> Self {
        // Borrow split. `regs`/`poll_budget`/`ctx` are `Copy`d out; the
        // retained `&'a mut device.yield_fn` reborrow pins `&'a mut HaceDevice`
        // for the whole life of the returned op — that is the arbiter: a
        // second `HaceDigest`/`HaceHmac` needs `&mut device` again and is a
        // borrow-check error while this one lives. The context is no longer
        // minted by a `shared_ctx_ptr()` free accessor at each call; it is the
        // device's sole pointer, so its transient `&mut` is reached only
        // *through* that exclusive device borrow
        // (`borrow-arbitrated-engine-exclusivity`, Checklist box 2/4).
        let regs = device.regs;
        let poll_budget = device.poll_budget;
        // SAFETY: the device holds the sole pointer to this `.ram_nc` context
        // (acquired once at its `unsafe fn new*` single-instance gate); the
        // caller upholds non-reentrancy and the live `&'a mut device` (pinned
        // by `yield_fn` below) gates it, so no other `&mut` to it is live.
        let ctx: &'a mut HashContext = unsafe { &mut *device.ctx };
        let yield_fn: &'a mut dyn FnMut(u32) = &mut device.yield_fn;
        Self::new(regs, ctx, poll_budget, yield_fn)
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

        self.regs.clear_hash_intflag();
        self.regs
            .program_hash_operation(sg_addr, digest_addr, bufcnt, method);

        for _ in 0..self.poll_budget {
            if self.regs.hash_intflag_is_set() {
                self.ctx.bufcnt = 0;
                return Ok(());
            }
            (self.yield_fn)(POLL_YIELD_NS);
        }

        self.regs.stop_hash_operation();
        Err(HaceError::Timeout)
    }
}

impl<'a, T: DigestAlgorithm> ErrorType for HaceDigest<'a, T> {
    type Error = HaceError;
}

impl<'a, T: HaceDigestSpec> DigestInit<T> for HaceDigest<'a, T>
where
    T::Digest: IntoBytes,
{
    type OpContext<'b>
        = HaceDigest<'b, T>
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
            regs: self.regs,
            ctx: &mut *self.ctx,
            poll_budget: self.poll_budget,
            yield_fn: &mut *self.yield_fn,
            _algo: PhantomData,
        })
    }
}

impl<'a, T: HaceDigestSpec> DigestOp for HaceDigest<'a, T>
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

        this.regs.clear_hash_intflag();
        this.regs
            .program_hash_operation(sg_addr, digest_addr, bufcnt, method);

        for _ in 0..this.poll_budget {
            if this.regs.hash_intflag_is_set() {
                let result = T::digest_from_context(this.ctx);
                // Cleanup context (mirrors cleanup_context in aspeed-rust).
                this.ctx.bufcnt = 0;
                this.ctx.digcnt = [0; 2];
                this.ctx.buffer.fill(0);
                this.ctx.digest.fill(0);
                this.regs.stop_hash_operation();
                return Ok(result);
            }
            (this.yield_fn)(POLL_YIELD_NS);
        }

        this.regs.stop_hash_operation();
        Err(HaceError::Timeout)
    }
}

impl<'a, T: DigestAlgorithm> DigestCtrlReset for HaceDigest<'a, T> {
    fn reset(&mut self) -> Result<(), Self::Error> {
        self.ctx.bufcnt = 0;
        self.ctx.digcnt = [0; 2];
        self.ctx.buffer.fill(0);
        self.ctx.digest.fill(0);
        self.regs.stop_hash_operation();
        Ok(())
    }
}
