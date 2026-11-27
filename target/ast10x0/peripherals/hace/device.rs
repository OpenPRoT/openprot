// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HACE engine owner: register access, bring-up and cooperative yield.
//!
//! [`Hace`] holds the PAC peripheral it got from its ownership token and reaches the register
//! block through the PAC's safe `Deref`. No raw pointer to the register block is created here.

use super::constants::DEFAULT_POLL_BUDGET;
use super::context::{acquire_crypto_ctx, acquire_shared_ctx, CryptoContext, HashContext};
use crate::scu::cache::dcache_invd_all as scu_dcache_invd_all;
use crate::scu::{ClockRegisterHalf, ScuRegisterHalf, ScuRegisters};
use crate::tokens::Hace as HaceToken;
use core::marker::PhantomData;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
}

/// Cooperative yield used until [`Hace::with_yield`] replaces it: spin.
fn spin_yield(_ns: u32) {
    core::hint::spin_loop();
}

/// The one owned binding over the HACE engine, and its register access.
///
/// Exclusivity member of the *owned-peripheral* family (`design-patterns` ::
/// `borrow-arbitrated-engine-exclusivity`): this value is **not**
/// `Copy`/`Clone` (the `*mut HashContext` field also makes it structurally
/// move-only), and every operation handle — `HaceDigest`, `AesCipher`,
/// `HaceHmac` — is produced *only* by an exclusive `&mut Hace` borrow. Two
/// concurrent ops of the one engine, of any kind, are therefore a borrow-check
/// error: the Rust `&mut`-exclusivity rule is the arbiter, replacing the Zephyr
/// reference's `in_use`/`-EBUSY` caller-serialization discipline. No runtime
/// busy flag and no hardware busy-bit read exists anywhere in this driver.
///
/// Build it with [`Hace::new`], which consumes the HACE token from
/// [`Peripherals`](crate::Peripherals), and set a cooperative yield with
/// [`Hace::with_yield`].
///
/// It keeps `ast1060_pac::Hace`, the PAC peripheral the token owned, and reaches
/// the register block through its `Deref`.
///
/// `ctx` is the engine's operation state. It must be a `.ram_nc`,
/// `#[repr(C, align(64))]` static (DMA targets — SG list / `buffer` /
/// `digest`; `goal.md` §1.3/§5.1) so it cannot be an inline by-value field of
/// this (stack-placeable) value; instead it holds the *sole* pointer
/// to it, acquired once at the construction gate via
/// [`acquire_shared_ctx`](super::context::acquire_shared_ctx) — there is no
/// free accessor handing it out elsewhere (Checklist box 2). The live
/// `&mut HashContext` exists *only* transiently inside an operation, reborrowed
/// through the `&mut Hace`.
pub struct Hace<Y: FnMut(u32) = fn(u32)> {
    /// The HACE register block, through the PAC peripheral the token owned.
    regs: ast1060_pac::Hace,
    // MMIO handles must not cross threads or be shared by reference.
    _not_send_sync: PhantomData<*const ()>,
    /// Sole pointer to the section-placed [`HashContext`]. Reborrowed as a
    /// transient `&mut` through `&mut self` by each operation's `from_device`;
    /// never aliased outside the engine owner.
    pub(crate) ctx: *mut HashContext,
    /// Sole pointer to the section-placed [`CryptoContext`] (AES path). Same
    /// discipline as `ctx`. AES is the engine's third borrow-arbitrated
    /// operation (goal.md §2.3 delta A1 / §5.1).
    pub(crate) crypto_ctx: *mut CryptoContext,
    /// Cooperative yield hook invoked between completion polls.
    /// Argument is a suggested wait window in nanoseconds.
    pub(crate) yield_fn: Y,
    pub(crate) poll_budget: u32,
    /// Cache flush hook called after HACE DMA writes to invalidate stale CPU
    /// cache lines. Injected at construction so operation modules (digest,
    /// aes) have no direct SCU dependency.
    pub(crate) cache_flush: fn(),
}

impl Hace {
    /// Take ownership of the HACE engine, proven by the `hace` token.
    ///
    /// Brings the engine out of reset. The SCU is shared, so it is borrowed, not owned: the
    /// caller keeps its [`ScuRegisters`] for the other drivers. The yield hook starts as a spin;
    /// replace it with [`Hace::with_yield`].
    pub fn new(hace: HaceToken, scu: &ScuRegisters) -> Self {
        // SAFETY: the consumed `Hace` token makes this the sole owner of the engine, hence of the
        // section-placed contexts that belong to it.
        unsafe { Self::init(hace.into_pac(), scu) }
    }

    /// Bring the engine out of reset and build the value.
    ///
    /// # Safety
    /// The caller must own the HACE engine, so this value is the sole holder of the engine's
    /// contexts (the single-instance contract).
    unsafe fn init(regs: ast1060_pac::Hace, scu: &ScuRegisters) -> Self {
        // SCU080 bit 13 = StopYCLKForHACE. Write bit 13 to SCU084 to clear the
        // clock-stop bit and enable the HACE YCLK before touching any HACE registers.
        scu.unlock_and_lock_after(|scu| {
            scu.ungate_clock_mask(ClockRegisterHalf::Lower, 1 << 13);
            // SCU040 bit 4 = HACE reset. Write bit 4 to SCU044 to deassert reset and
            // bring the HACE out of reset before accessing any HACE registers. Without
            // this, the AHB bus hangs on the first HACE register read when the ROM
            // bootloader (UART/fwspick mode) leaves HACE in reset.
            scu.deassert_reset_mask(ScuRegisterHalf::Lower, 1 << 4);
        });

        let hace = Self {
            regs,
            _not_send_sync: PhantomData,
            // SAFETY: the caller guarantees this is the sole live owner, hence the sole
            // holder of these pointers.
            ctx: unsafe { acquire_shared_ctx() },
            crypto_ctx: unsafe { acquire_crypto_ctx() },
            yield_fn: spin_yield as fn(u32),
            poll_budget: DEFAULT_POLL_BUDGET,
            cache_flush: scu_dcache_invd_all,
        };

        // Drain any in-progress hash operation left by the bootloader.
        // On real hardware the engine may be busy when firmware starts; the
        // HACE W1C clear of `hash_intflag` is ignored while `HashEngStsFlag`
        // (bit 0) is set, causing the first poll to return a stale flag and the
        // digest buffer to read back as the IV (never written by the engine).
        // We stop the engine, then spin until it goes idle, then clear the
        // stale flag before handing the engine to callers.
        hace.stop_hash_operation();
        while hace.hash_engine_is_busy() {
            core::hint::spin_loop();
        }
        hace.clear_hash_intflag();
        hace
    }

    /// Returns `true` while the hash engine is actively processing (HACE1C bit 0).
    /// Used to drain any in-progress bootloader operation before starting our own.
    #[inline]
    fn hash_engine_is_busy(&self) -> bool {
        self.regs.hace1c().read().hash_eng_sts_flag().bit_is_set()
    }
}

impl<Y: FnMut(u32)> Hace<Y> {
    /// Replace the cooperative yield hook, called between completion polls with a suggested wait
    /// window in nanoseconds. It may capture state.
    #[must_use]
    pub fn with_yield<F: FnMut(u32)>(self, yield_fn: F) -> Hace<F> {
        Hace {
            regs: self.regs,
            _not_send_sync: PhantomData,
            ctx: self.ctx,
            crypto_ctx: self.crypto_ctx,
            yield_fn,
            poll_budget: self.poll_budget,
            cache_flush: self.cache_flush,
        }
    }

    /// Override polling timeout budget for operation completion.
    #[must_use]
    pub const fn with_timeout_polls(mut self, timeout_polls: u32) -> Self {
        self.poll_budget = timeout_polls;
        self
    }

    #[inline]
    pub(crate) fn clear_hash_intflag(&self) {
        self.regs.hace1c().write(|w| w.hash_intflag().set_bit());
    }

    #[inline]
    pub(crate) fn hash_intflag_is_set(&self) -> bool {
        self.regs.hace1c().read().hash_intflag().bit_is_set()
    }

    #[inline]
    pub(crate) fn program_hash_operation(
        &self,
        src_addr: u32,
        digest_addr: u32,
        data_len: u32,
        cmd: u32,
    ) {
        // SAFETY: Callers provide HACE-usable physical addresses and a valid command.
        self.regs.hace20().write(|w| unsafe { w.bits(src_addr) });
        self.regs.hace24().write(|w| unsafe { w.bits(digest_addr) });
        self.regs.hace28().write(|w| unsafe { w.bits(digest_addr) });
        self.regs.hace2c().write(|w| unsafe { w.bits(data_len) });
        self.regs.hace30().write(|w| unsafe { w.bits(cmd) });
    }

    #[inline]
    pub(crate) fn stop_hash_operation(&self) {
        // SAFETY: Writing 0 to command register is the defined idle/stop state.
        self.regs.hace30().write(|w| unsafe { w.bits(0) });
    }

    // ----- Crypto (AES) sub-engine -----------------------------------------
    //
    // The HACE engine's symmetric-crypto path uses a distinct register file
    // from the hash path (HACE00/04/08/0C/10) and a distinct completion bit
    // in the shared status register HACE1C (`crypto_intflag`). Driven in the
    // exact order of the pinned authority `crypto_trigger`
    // (`zephyr-reference/hace_aspeed.c:88-94`; goal.md §1.9.1).
    //
    // Note: the authority also gates on a HW busy bit
    // (`hace_sts.crypto_engine_sts`, `hace_aspeed.c:83`). The port deliberately
    // does **not** read it — engine exclusivity is structural (one owned
    // `Hace`, borrow-arbitrated), goal.md §2.3 delta A1 / the
    // `borrow-arbitrated-engine-exclusivity` pattern. No busy-bit accessor is
    // exposed here, by design.

    #[inline]
    pub(crate) fn clear_crypto_intflag(&self) {
        self.regs.hace1c().write(|w| w.crypto_intflag().set_bit());
    }

    #[inline]
    pub(crate) fn crypto_intflag_is_set(&self) -> bool {
        self.regs.hace1c().read().crypto_intflag().bit_is_set()
    }

    /// Read the raw HACE1C status register value (for diagnostics).
    #[inline]
    pub(crate) fn read_hace1c(&self) -> u32 {
        self.regs.hace1c().read().bits()
    }

    /// Program one crypto (AES) pass and start the engine.
    ///
    /// Writes, in authority order: source data address (HACE00), destination
    /// data address (HACE04), crypto context base (HACE08), data length
    /// (HACE0C), then the command word (HACE10) — the final write starts the
    /// engine. The SG-terminator (`| 1<<31`) lives in the `src`/`dst` length
    /// words inside the crypto context (goal.md §1.9.3), not here; `data_len`
    /// is the plain byte length the engine consumes.
    #[inline]
    pub(crate) fn program_crypto_operation(
        &self,
        src_addr: u32,
        dst_addr: u32,
        ctx_base: u32,
        data_len: u32,
        cmd: u32,
    ) {
        // SAFETY: Callers provide HACE-usable physical addresses and a valid
        // command word composed per goal.md §1.9.2.
        self.regs.hace00().write(|w| unsafe { w.bits(src_addr) });
        self.regs.hace04().write(|w| unsafe { w.bits(dst_addr) });
        self.regs.hace08().write(|w| unsafe { w.bits(ctx_base) });
        self.regs.hace0c().write(|w| unsafe { w.bits(data_len) });
        self.regs.hace10().write(|w| unsafe { w.bits(cmd) });
    }
}
