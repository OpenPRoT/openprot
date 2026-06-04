// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Public SCU types.

/// Result type for SCU operations.
pub type Result<T> = core::result::Result<T, ScuError>;

/// SCU error conditions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScuError {
    InvalidMonitorInstance,
    InvalidMuxSelection,
}

/// Selects the lower or upper 32-bit control register half for reset domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScuRegisterHalf {
    Lower,
    Upper,
}

/// Selects the lower or upper 32-bit control register half for clock domains.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockRegisterHalf {
    Lower,
    Upper,
}

/// AST10x0 SPI monitor instance identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiMonitorInstance {
    Spim0 = 0,
    Spim1 = 1,
    Spim2 = 2,
    Spim3 = 3,
}

/// Select which internal SPI master is routed through a monitor path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiMonitorSource {
    Spi1,
    Spi2,
}

/// External mux selection for a SPI monitor instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScuExtMuxSelect {
    Mux0,
    Mux1,
}

impl ScuExtMuxSelect {
    #[must_use]
    pub const fn as_bool(self) -> bool {
        match self {
            Self::Mux0 => false,
            Self::Mux1 => true,
        }
    }
}

/// SCU passthrough enable state for a SPI monitor instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpiMonitorPassthrough {
    Disabled,
    Enabled,
}

impl SpiMonitorPassthrough {
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}
