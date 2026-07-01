// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidPin,
    InvalidNgpios,
    UnsupportedFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialLevel {
    Low,
    High,
}

/// Low-level per-pin configuration used by [`crate::sgpiom::registers::Sgpiom::configure_pin`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SgpiomPinConfig {
    pub direction: Direction,
    pub initial: Option<InitialLevel>,
    pub pull_up: bool,
    pub pull_down: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptMode {
    Disabled,
    Level,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptTrigger {
    Low,
    High,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Bank {
    Ad = 0,
    Eh = 1,
    Il = 2,
    Mp = 3,
}

impl Bank {
    #[inline]
    pub const fn from_pin_offset(pin_offset: u8) -> Option<Self> {
        match pin_offset >> 5 {
            0 => Some(Self::Ad),
            1 => Some(Self::Eh),
            2 => Some(Self::Il),
            3 => Some(Self::Mp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankDevice {
    pub bank: Bank,
    pub pin_offset: u8,
    pub ngpios: u8,
    /// Bitmask of pins reserved/unusable within this bank (Zephyr `gpio-reserved`).
    /// A set bit is excluded from [`Self::valid_mask`] and rejected by
    /// [`Self::validate_pin`] / [`Self::validate_mask`].
    pub reserved: u32,
}

impl BankDevice {
    /// The only constructor: derives the bank from `pin_offset` (`pin_offset >> 5`,
    /// the same mapping Zephyr uses) so the bank and offset can never disagree.
    ///
    /// Returns `None` unless `pin_offset` is one of `{0, 32, 64, 96}` (a 32-pin
    /// bank boundary) and `ngpios` is in `1..=32`. This makes a mis-targeted
    /// descriptor unrepresentable rather than relying on a debug assertion.
    #[inline]
    pub const fn from_pin_offset(pin_offset: u8, ngpios: u8) -> Option<Self> {
        // Must land exactly on a bank boundary; `pin_offset >> 5` alone would
        // accept e.g. 33 (-> Eh) which is not a valid bank offset.
        if pin_offset % 32 != 0 || ngpios == 0 || ngpios > 32 {
            return None;
        }
        match Bank::from_pin_offset(pin_offset) {
            Some(bank) => Some(Self {
                bank,
                pin_offset,
                ngpios,
                reserved: 0,
            }),
            None => None,
        }
    }

    /// Set the reserved-pin bitmask (builder), mirroring Zephyr `gpio-reserved`.
    #[inline]
    #[must_use]
    pub const fn with_reserved(mut self, reserved: u32) -> Self {
        self.reserved = reserved;
        self
    }

    #[inline]
    pub fn validate_pin(&self, pin: u8) -> Result<(), Error> {
        if pin < self.ngpios && pin < 32 && (self.reserved & (1u32 << pin)) == 0 {
            Ok(())
        } else {
            Err(Error::InvalidPin)
        }
    }

    /// Bitmask of the pins usable on this bank (`pin < ngpios`, capped at 32,
    /// minus reserved pins).
    ///
    /// Mirrors Zephyr's per-child `port_pin_mask` (which folds in `gpio-reserved`).
    #[inline]
    #[must_use]
    pub const fn valid_mask(&self) -> u32 {
        let window = if self.ngpios >= 32 {
            u32::MAX
        } else {
            (1u32 << self.ngpios) - 1
        };
        window & !self.reserved
    }

    /// Reject any mask bit outside this bank's usable window (out-of-range or
    /// reserved).
    #[inline]
    pub fn validate_mask(&self, mask: u32) -> Result<(), Error> {
        if (mask & !self.valid_mask()) != 0 {
            Err(Error::InvalidPin)
        } else {
            Ok(())
        }
    }
}
