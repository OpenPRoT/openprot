// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_std]

#[cfg(feature = "silicon")]
pub const SYSTEM_CLOCK_HZ: u64 = 100_000_000;
#[cfg(feature = "silicon")]
pub const PERIPHERAL_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "silicon")]
pub const HI_SPEED_PERIPHERAL_CLOCK_HZ: u64 = 96_000_000;
#[cfg(feature = "silicon")]
pub const AON_CLOCK_HZ: u64 = 200_000;

#[cfg(feature = "fpga")]
pub const SYSTEM_CLOCK_HZ: u64 = 6_000_000;
#[cfg(feature = "fpga")]
pub const PERIPHERAL_CLOCK_HZ: u64 = 6_000_000;
#[cfg(feature = "fpga")]
pub const HI_SPEED_PERIPHERAL_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "fpga")]
pub const AON_CLOCK_HZ: u64 = 250_000;

#[cfg(feature = "verilator")]
pub const SYSTEM_CLOCK_HZ: u64 = 125_000;
#[cfg(feature = "verilator")]
pub const PERIPHERAL_CLOCK_HZ: u64 = 125_000;
#[cfg(feature = "verilator")]
pub const HI_SPEED_PERIPHERAL_CLOCK_HZ: u64 = 500_000;
#[cfg(feature = "verilator")]
pub const AON_CLOCK_HZ: u64 = 125_000;

#[cfg(feature = "qemu")]
pub const SYSTEM_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "qemu")]
pub const PERIPHERAL_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "qemu")]
pub const HI_SPEED_PERIPHERAL_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "qemu")]
pub const AON_CLOCK_HZ: u64 = 250_000;
