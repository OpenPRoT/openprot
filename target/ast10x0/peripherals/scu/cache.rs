// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 data-cache maintenance.

use super::registers::ScuRegisters;

impl ScuRegisters {
    /// Invalidate the entire AST10x0 data cache.
    ///
    /// Toggles `DCACHE_CLEAN` (SCUA58 bit 1) low→high, mirroring the Zephyr
    /// authority `cache_data_invd_all()` (`drivers/cache/cache_aspeed.c`, also
    /// called after every crypto operation in `hace_aspeed.c:140`).
    ///
    /// Required after any HACE DMA write into cacheable SRAM (below
    /// `0x000A0000`): the engine's writes bypass the CPU cache, so the CPU
    /// would read stale pre-operation cache lines without this invalidation.
    #[inline]
    pub fn dcache_invd_all(&self) {
        const DCACHE_CLEAN: u32 = 1 << 1;
        self.unlock_and_lock_after(|scu| {
            // SAFETY: SCU MMIO read-modify-write; DSB only orders memory accesses.
            unsafe {
                let ctrl = scu.regs().scua58().read().bits();
                scu.regs().scua58().write(|w| w.bits(ctrl & !DCACHE_CLEAN));
                core::arch::asm!("dsb sy", options(nostack, preserves_flags));
                scu.regs().scua58().write(|w| w.bits(ctrl | DCACHE_CLEAN));
                core::arch::asm!("dsb sy", options(nostack, preserves_flags));
            }
        });
    }
}

/// Thin free-function wrapper around [`ScuRegisters::dcache_invd_all`].
///
/// Exists solely so the function can be stored as a bare `fn()` pointer in
/// [`HaceDevice::cache_flush`], which is wired at construction by the board
/// crate. All SCU register access goes through [`ScuRegisters`].
pub fn dcache_invd_all() {
    // SAFETY: singleton SCU; single-threaded HACE use is upheld by HaceDevice's
    // single-instance contract.
    unsafe { ScuRegisters::new_global() }.dcache_invd_all();
}
