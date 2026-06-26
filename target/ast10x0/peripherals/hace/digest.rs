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
        for (i, chunk) in ctx.digest[..SHA256_DIGEST_SIZE].chunks_exact(4).enumerate() {
            out[i] = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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
        for (i, chunk) in ctx.digest[..SHA384_DIGEST_SIZE].chunks_exact(4).enumerate() {
            out[i] = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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
        for (i, chunk) in ctx.digest[..SHA512_DIGEST_SIZE].chunks_exact(4).enumerate() {
            out[i] = u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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
        load_iv(&mut self.ctx.digest[..iv.len() * 4], iv)?;
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

        // If all input fits without filling a complete block, buffer it.
        if self.ctx.bufcnt + input_len < self.ctx.block_size {
            let start = self.ctx.bufcnt as usize;
            let end = start + input_len as usize;
            self.ctx.buffer[start..end].copy_from_slice(input);
            self.ctx.bufcnt += input_len;
            return Ok(());
        }

        // Process one or more full blocks via SG.
        let remaining = (input_len + self.ctx.bufcnt) % self.ctx.block_size;
        let total_len = (input_len + self.ctx.bufcnt) - remaining;
        let mut sg_idx = 0usize;

        // Capture pointers before mutating the SG table.
        let buf_ptr = ptr_to_u32(self.ctx.buffer.as_ptr())?;
        let input_ptr = ptr_to_u32(input.as_ptr())?;

        if self.ctx.bufcnt != 0 {
            self.ctx.sg[0].addr = buf_ptr;
            self.ctx.sg[0].len = self.ctx.bufcnt;
            if total_len == self.ctx.bufcnt {
                // Existing buffer is the only SG entry; input becomes the tail.
                self.ctx.sg[0].addr = input_ptr;
                self.ctx.sg[0].len |= HACE_SG_LAST;
            }
            sg_idx += 1;
        }

        if total_len != self.ctx.bufcnt {
            self.ctx.sg[sg_idx].addr = input_ptr;
            self.ctx.sg[sg_idx].len = (total_len - self.ctx.bufcnt) | HACE_SG_LAST;
        }

        let sg_addr = ptr_to_u32(self.ctx.sg.as_ptr())?;
        let digest_addr = ptr_to_u32(self.ctx.digest.as_ptr())?;
        let method = self.ctx.method;

        self.regs.clear_hash_intflag();
        self.regs
            .program_hash_operation(sg_addr, digest_addr, total_len, method);

        let mut done = false;
        for _ in 0..self.poll_budget {
            if self.regs.hash_intflag_is_set() {
                done = true;
                break;
            }
            (self.yield_fn)(POLL_YIELD_NS);
        }
        if !done {
            self.regs.stop_hash_operation();
            return Err(HaceError::Timeout);
        }

        // Copy remainder of input into the buffer for the next call.
        if remaining != 0 {
            let src_start = (total_len - self.ctx.bufcnt) as usize;
            self.ctx.buffer[..remaining as usize]
                .copy_from_slice(&input[src_start..src_start + remaining as usize]);
        }
        self.ctx.bufcnt = remaining;

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
