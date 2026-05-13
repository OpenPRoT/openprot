// Licensed under the Apache-2.0 license

//! Timing configuration for I2C
//!
//! The AST1060 I2C controller uses a hierarchical clock system:
//! 1. HPLL (1 `GHz`) → APB clock (typically 50 `MHz`)
//! 2. APB clock → 4 base clocks (configured in I2CG10)
//! 3. Base clock → SCL frequency via per-controller divider
//!
//! This module calculates the optimal divider chain to achieve
//! the requested I2C bus speed while meeting timing specifications.
//!
//! # Design: Dependency Injection
//!
//! Clock frequencies are provided via `ClockConfig` in `I2cConfig`, rather than
//! being read directly from SCU/I2C global registers. This provides:
//! - **Decoupling**: I2C module has no hidden dependencies on SCU
//! - **Testability**: Unit tests can inject known clock values
//! - **Portability**: Different board configurations are easily supported
//! - **Clear ownership**: System init owns clock detection, I2C just uses values
//!
//! # I2C Timing Requirements
//!
//! | Mode       | Speed    | tLOW (min) | tHIGH (min) | tBuf (min) |
//! |------------|----------|------------|-------------|------------|
//! | Standard   | 100 kHz  | 4.7 µs     | 4.0 µs      | 4.7 µs     |
//! | Fast       | 400 kHz  | 1.3 µs     | 0.6 µs      | 1.3 µs     |
//! | Fast-plus  | 1 `MHz`    | 0.5 µs     | 0.26 µs     | 0.5 µs     |
//!
//! # Clock Derivation
//!
//! The I2C global register I2CG10 configures four base clock dividers:
//! ```text
//! I2CG10[7:0]   = base_clk1 divisor → Fast-plus (1 MHz) source
//! I2CG10[15:8]  = base_clk2 divisor → Fast (400 kHz) source
//! I2CG10[23:16] = base_clk3 divisor → Standard (100 kHz) source
//! I2CG10[31:24] = base_clk4 divisor → Recovery timeout source
//! ```
//!
//! Default I2CG10 value: `0x6222_0803`
//! With 50 `MHz` APB clock:
//! - `base_clk1` (0x03): 50M / ((3+2)/2) = 20 `MHz` → 1 `MHz` with div=20
//! - `base_clk2` (0x08): 50M / ((8+2)/2) = 10 `MHz` → 400 kHz with div=25
//! - `base_clk3` (0x22): 50M / ((34+2)/2) = 2.77 `MHz` → 100 kHz with div=28

use super::error::I2cError;
use super::types::{ClockConfig, I2cConfig, I2cSpeed};
use ast1060_pac::i2c::RegisterBlock;
use core::cmp::min;

/// Maximum divider ratio per base clock
const MAX_DIVIDER_RATIO: u32 = 32;

/// I2C speed frequencies in Hz
const SPEED_STANDARD: u32 = 100_000;
const SPEED_FAST: u32 = 400_000;
const SPEED_FAST_PLUS: u32 = 1_000_000;

/// Configure I2C timing based on speed and injected clock configuration
///
/// Uses the `ClockConfig` from `I2cConfig` to calculate proper SCL timing
/// that meets I2C specifications. No direct hardware register access for
/// clock detection - all clock values come from the injected config.
///
/// # Arguments
///
/// * `regs` - Reference to the I2C controller register block
/// * `config` - I2C configuration containing speed, timeout, and clock settings
///
/// # Returns
///
/// * `Ok(())` on successful configuration
/// * `Err(I2cError::Invalid)` if timing cannot be achieved
pub fn configure_timing(regs: &RegisterBlock, config: &I2cConfig) -> Result<(), I2cError> {
    let clocks = &config.clock_config;

    // Step 1: Get target speed
    let target_speed = match config.speed {
        I2cSpeed::Standard => SPEED_STANDARD,
        I2cSpeed::Fast => SPEED_FAST,
        I2cSpeed::FastPlus => SPEED_FAST_PLUS,
    };

    // Step 2: Find best base clock and divider using injected clock config
    let (div, divider_ratio) = calculate_divider(clocks, target_speed)?;

    // Step 3: Calculate SCL high/low times
    // I2C spec: tLOW should be slightly longer than tHIGH
    // Default ratio: 9/16 low, 7/16 high (asymmetric for spec compliance)
    let scl_low = calculate_scl_low(divider_ratio);
    let scl_high = calculate_scl_high(divider_ratio, scl_low);
    let scl_high_min = scl_high.saturating_sub(1);

    // Step 4: Build timeout configuration if enabled
    let (timeout_divisor, timeout_timer) = if config.smbus_timeout {
        // SMBus timeout: 25-35ms per specification
        // With base_clk_divisor=2, timer=8 gives approximately 35ms timeout
        (2u8, 8u8)
    } else {
        (0u8, 0u8)
    };

    // Step 5: Write AC timing register with all fields in a single write
    // This avoids corrupting fields with multiple separate writes
    regs.i2cc04().write(|w| unsafe {
        w.base_clk_divisor_tbase_clk()
            .bits((div & 0xf) as u8)
            .cycles_of_master_sclclklow_pulse_width_tcklow()
            .bits(scl_low)
            .cycles_of_master_sclclkhigh_pulse_width_tckhigh()
            .bits(scl_high)
            .cycles_of_master_sclclkhigh_minimum_pulse_width_tckhigh_min()
            .bits(scl_high_min)
            .timeout_base_clk_divisor_tout_base_clk()
            .bits(timeout_divisor)
            .timeout_timer()
            .bits(timeout_timer)
    });

    Ok(())
}

/// Calculate optimal base clock selection and divider ratio
///
/// Tries each base clock in order (APB, clk1, clk2, clk3, clk4) and finds
/// the first one that can achieve the target speed with a divider ≤ 32.
///
/// # Arguments
///
/// * `clocks` - Injected clock configuration
/// * `target_speed` - Desired I2C bus speed in Hz
///
/// # Returns
///
/// * `(div, ratio)` - Base clock selector (0-4+) and divider ratio
fn calculate_divider(clocks: &ClockConfig, target_speed: u32) -> Result<(u32, u32), I2cError> {
    // Avoid division by zero
    if target_speed == 0 {
        return Err(I2cError::Invalid);
    }

    // Try each base clock in order, finding first that works with divider <= 32
    let candidates = [
        (0u32, clocks.apb_clock_hz),
        (1u32, clocks.base_clk1_hz),
        (2u32, clocks.base_clk2_hz),
        (3u32, clocks.base_clk3_hz),
    ];

    for (div, base_clk) in candidates {
        if base_clk == 0 {
            continue; // Skip unconfigured clocks
        }
        if base_clk / target_speed <= MAX_DIVIDER_RATIO {
            let mut ratio = base_clk / target_speed;
            // Round up if we'd exceed target speed
            if ratio > 0 && base_clk / ratio > target_speed {
                ratio = ratio.saturating_add(1);
            }
            // Ensure ratio is at least 1
            if ratio == 0 {
                ratio = 1;
            }
            return Ok((div, ratio));
        }
    }

    // Fall back to base_clk4 with extended divider for very slow speeds
    if clocks.base_clk4_hz == 0 {
        return Err(I2cError::Invalid);
    }

    let mut div = 4u32;
    let mut ratio = clocks.base_clk4_hz / target_speed;
    let mut inc = 0u32;

    // Shift ratio down until it fits in MAX_DIVIDER_RATIO
    while ratio.saturating_add(inc) > MAX_DIVIDER_RATIO {
        inc |= ratio & 1; // Track if we need to round up
        ratio >>= 1;
        div = div.saturating_add(1);
    }
    ratio = ratio.saturating_add(inc);

    // Round up if needed
    if ratio > 0 && clocks.base_clk4_hz / ratio > target_speed {
        ratio = ratio.saturating_add(1);
    }

    // Clamp to valid range
    ratio = min(ratio, MAX_DIVIDER_RATIO);
    if ratio == 0 {
        ratio = 1;
    }
    div &= 0xf;

    Ok((div, ratio))
}

/// Calculate SCL low time cycles
///
/// I2C specification requires tLOW to be longer than tHIGH.
/// We use 9/16 of the period for low time (56.25%).
fn calculate_scl_low(divider_ratio: u32) -> u8 {
    // scl_low = (ratio * 9/16) - 1, clamped to 4 bits
    let scl_low = (divider_ratio * 9 / 16).saturating_sub(1);
    #[allow(clippy::cast_possible_truncation)]
    let result = min(scl_low, 0xf) as u8;
    result
}

/// Calculate SCL high time cycles
///
/// High time is the remainder after low time and 2 cycles for edges.
fn calculate_scl_high(divider_ratio: u32, scl_low: u8) -> u8 {
    // scl_high = ratio - scl_low - 2 (2 cycles for rise/fall edges)
    let scl_high = divider_ratio
        .saturating_sub(u32::from(scl_low))
        .saturating_sub(2);
    #[allow(clippy::cast_possible_truncation)]
    let result = min(scl_high, 0xf) as u8;
    result
}
