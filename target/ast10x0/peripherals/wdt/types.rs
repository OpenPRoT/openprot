// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 watchdog types and configuration.

/// Frequency of the AST10x0 watchdog counter clock (1 MHz), so one reload tick
/// equals one microsecond.
pub const WDT_CLOCK_HZ: u32 = 1_000_000;

/// Magic value written to the restart register to reload the counter.
pub(crate) const RESTART_MAGIC: u32 = 0x0000_4755;

/// Watchdog driver errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WdtError {
    /// The requested timeout is zero or maps to a reload value that does not fit
    /// in the 32-bit counter.
    InvalidTimeout,
}

/// Selects what the watchdog resets when the counter expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResetMode {
    /// Reset the SoC system, gated by the reset-mask registers.
    #[default]
    SocSystem,
    /// Full-chip reset.
    FullChip,
    /// Reboot only the CPU/FMC firmware; other IPs keep running.
    CpuFmcOnly,
}

/// Watchdog start-up configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WdtConfig {
    /// Timeout window in milliseconds.
    pub timeout_ms: u32,
    /// Drive a hardware reset when the counter expires. When `false`, expiry
    /// only latches the timeout status (and any enabled interrupt).
    pub reset_on_timeout: bool,
    /// What to reset when `reset_on_timeout` is set.
    pub reset_mode: ResetMode,
}

impl WdtConfig {
    /// Convert the configured millisecond window into counter reload ticks.
    pub(crate) fn reload_ticks(&self) -> Result<u32, WdtError> {
        let ticks = u64::from(self.timeout_ms) * u64::from(WDT_CLOCK_HZ / 1000);
        if ticks == 0 || ticks > u64::from(u32::MAX) {
            return Err(WdtError::InvalidTimeout);
        }
        Ok(ticks as u32)
    }
}
