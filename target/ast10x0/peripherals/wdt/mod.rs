// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 watchdog peripheral driver.
//!
//! The AST10x0 provides four independent watchdog timers clocked at 1 MHz.
//! Obtain an instance via one of the `WdtRegisters` constructors (all unsafe),
//! wrap it in a [`Watchdog`], then [`start`](Watchdog::start) the timer and
//! [`feed`](Watchdog::feed) it before the window elapses.
//!
//! ```no_run
//! use peripherals::wdt::{ResetMode, Watchdog, WdtConfig, WdtRegisters};
//!
//! // SAFETY: single owner of WDT0 for the lifetime of `wdt`.
//! let mut wdt = Watchdog::new(unsafe { WdtRegisters::new_wdt0() });
//! wdt.start(WdtConfig {
//!     timeout_ms: 1000,
//!     reset_on_timeout: true,
//!     reset_mode: ResetMode::SocSystem,
//! })
//! .unwrap();
//! wdt.feed();
//! ```

mod registers;
mod types;

pub use registers::WdtRegisters;
pub use types::{ResetMode, WdtConfig, WdtError, WDT_CLOCK_HZ};

use types::RESTART_MAGIC;

/// Blocking AST10x0 watchdog driver bound to one watchdog instance.
pub struct Watchdog {
    regs: WdtRegisters,
}

impl Watchdog {
    /// Wrap a watchdog register accessor in a driver.
    #[must_use]
    pub const fn new(regs: WdtRegisters) -> Self {
        Self { regs }
    }

    /// Program the reload window and start counting.
    ///
    /// The counter is loaded from the reload value and the timer is enabled with
    /// the requested reset behavior. Returns [`WdtError::InvalidTimeout`] if the
    /// window does not fit the 32-bit counter.
    pub fn start(&mut self, config: WdtConfig) -> Result<(), WdtError> {
        let ticks = config.reload_ticks()?;
        let regs = self.regs.regs();

        // Load the reload value, then trigger a reload so the counter starts from it.
        regs.wdt004().write(|w| unsafe { w.bits(ticks) });
        regs.wdt008().write(|w| unsafe { w.bits(RESTART_MAGIC) });

        regs.wdt00c().write(|w| {
            match config.reset_mode {
                ResetMode::SocSystem => w
                    .rst_sys_mode()
                    .soc_system_ewvergated_by_reset_mask_registers(),
                ResetMode::FullChip => w.rst_sys_mode().full_chip(),
                ResetMode::CpuFmcOnly => w
                    .rst_sys_mode()
                    .cpufmc_only_just_reboot_firmware_no_any_other_ips_will_be_reset(),
            };
            if config.reset_on_timeout {
                w.rst_sys_after_timeout().enable();
            } else {
                w.rst_sys_after_timeout().disable();
            }
            w.wdtenbl_sig().enable()
        });

        Ok(())
    }

    /// Reload the counter, restarting the timeout window.
    pub fn feed(&mut self) {
        self.regs
            .regs()
            .wdt008()
            .write(|w| unsafe { w.bits(RESTART_MAGIC) });
    }

    /// Stop the watchdog by clearing its enable bit.
    pub fn disable(&mut self) {
        self.regs
            .regs()
            .wdt00c()
            .modify(|_, w| w.wdtenbl_sig().disable());
    }

    /// Report whether the watchdog counter has reached zero at least once.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        self.regs
            .regs()
            .wdt010()
            .read()
            .indicate_timeout()
            .is_timeout_occur()
    }

    /// Clear the latched timeout / interrupt status.
    pub fn clear_timeout(&mut self) {
        self.regs.regs().wdt014().write(|w| unsafe { w.bits(0x01) });
    }
}
