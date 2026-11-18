// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! A [`Pec`] engine for RISC-V cores with the `Zbc` carryless multiply.
//!
//! `util_pec` computes the PEC with a lookup table by default. On a core with
//! `Zbc` a CRC-8 step is two `clmul`s, a shift and an xor, with no table and no
//! memory access. Pass [`ZbcPec`] to a transport's `with_pec`.
//!
//! This is its own crate so the transports stay free of `unsafe`: the one
//! `clmul` wrapper below is the only place that needs it. [`ZbcPec`] exists only
//! when the crate is built with `-C target-feature=+zbc` for RISC-V; elsewhere
//! the crate is empty and `util_pec::SoftwarePec` is used.

#![cfg_attr(not(test), no_std)]

#[cfg(all(target_arch = "riscv32", target_feature = "zbc"))]
use util_pec::Pec;

/// Polynomial `x^8 + x^2 + x + 1` without its top bit.
#[cfg(any(test, all(target_arch = "riscv32", target_feature = "zbc")))]
const POLY: u32 = 0x07;

/// One SMBus CRC-8 step (poly 0x07, no reflection) from a carryless multiply.
///
/// With `x = crc ^ data`, a step is `x * x^8 mod P`. Since `x^8 = 0x07 (mod P)`
/// that is `x * 0x07`, a value of at most 10 bits. The bits above the low byte
/// are folded back by one more multiply, and the low byte of the xor is the CRC.
///
/// This is the sequence of `romtime::crc8` in caliptra-mcu-sw. The multiply is a
/// parameter so the tests can run the same arithmetic on the host.
#[cfg(any(test, all(target_arch = "riscv32", target_feature = "zbc")))]
#[inline(always)]
fn crc8_step(clmul: impl Fn(u32, u32) -> u32, crc: u8, data: u8) -> u8 {
    let a = clmul(u32::from(crc ^ data), POLY);
    let b = clmul(a >> 8, POLY);
    (a ^ b) as u8
}

/// The `clmul` instruction.
///
/// `target_feature` makes the assembler accept the instruction in whichever
/// crate this ends up in. It is `#[inline]` into [`update_zbc`], which has the
/// same feature; without the attribute, inlining `ZbcPec::update` into a crate
/// built without `+zbc` fails to assemble.
#[cfg(all(target_arch = "riscv32", target_feature = "zbc"))]
#[target_feature(enable = "zbc")]
#[inline]
fn clmul(a: u32, b: u32) -> u32 {
    let r: u32;
    // SAFETY: `clmul` reads two registers and writes one. It touches no memory,
    // has no side effects and cannot trap, and this function is compiled with
    // `zbc` enabled.
    unsafe {
        core::arch::asm!(
            "clmul {r}, {a}, {b}",
            r = out(reg) r,
            a = in(reg) a,
            b = in(reg) b,
            options(pure, nomem, nostack),
        );
    }
    r
}

/// The whole CRC loop with `zbc` enabled, so the `clmul`s inline into it.
#[cfg(all(target_arch = "riscv32", target_feature = "zbc"))]
#[target_feature(enable = "zbc")]
fn update_zbc(crc: u8, data: &[u8]) -> u8 {
    data.iter()
        .fold(crc, |crc, &b| crc8_step(|a, m| clmul(a, m), crc, b))
}

/// Computes the PEC with the `Zbc` `clmul` instruction.
#[cfg(all(target_arch = "riscv32", target_feature = "zbc"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct ZbcPec;

#[cfg(all(target_arch = "riscv32", target_feature = "zbc"))]
impl Pec for ZbcPec {
    fn update(&self, crc: u8, data: &[u8]) -> u8 {
        // SAFETY: this impl only exists when the build enables `zbc`, so the core
        // is known to implement the extension that `update_zbc` requires.
        unsafe { update_zbc(crc, data) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Software carryless multiply, standing in for the instruction.
    fn clmul_model(a: u32, mut b: u32) -> u32 {
        let (mut a, mut r) = (a, 0);
        while b != 0 {
            if b & 1 != 0 {
                r ^= a;
            }
            a <<= 1;
            b >>= 1;
        }
        r
    }

    /// The definition: eight shift-and-xor rounds.
    fn crc8_bitwise(mut crc: u8, data: u8) -> u8 {
        crc ^= data;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
        crc
    }

    #[test]
    fn clmul_step_matches_the_definition_for_all_inputs() {
        for crc in 0..=255u8 {
            for data in 0..=255u8 {
                assert_eq!(
                    crc8_step(clmul_model, crc, data),
                    crc8_bitwise(crc, data),
                    "crc {crc:#04x} data {data:#04x}"
                );
            }
        }
    }

    #[test]
    fn clmul_step_check_value() {
        // CRC-8/SMBUS check value for "123456789".
        let crc = b"123456789"
            .iter()
            .fold(0, |crc, &b| crc8_step(clmul_model, crc, b));
        assert_eq!(crc, 0xF4);
    }
}
