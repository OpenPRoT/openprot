// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SMBus PEC engines: how a transport computes the CRC-8 that follows a packet.
//!
//! MCTP bindings such as DSP0233 (I3C) and DSP0237 (SMBus/I2C) put a PEC after
//! every packet. A transport is SoC-agnostic, so it does not decide how the CRC
//! is computed. It calls a [`Pec`] engine, and [`SoftwarePec`] (a table) is the
//! default everywhere. A target that can do better supplies its own engine: a
//! RISC-V core with the `Zbc` carryless multiply (`util/pec_zbc`), or a hardware
//! CRC block.
//!
//! An engine only *computes* the CRC. A controller that generates and checks the
//! PEC itself needs the transport to leave the byte out, which an engine cannot
//! express, and is not covered here.

#![cfg_attr(not(test), no_std)]

/// Computes the SMBus PEC: CRC-8 with polynomial 0x07, zero seed, no reflection
/// (CRC-8/SMBUS).
///
/// Engines are stateless and cheap to copy: the transport types hold one by
/// value and copy it where they need another.
pub trait Pec {
    /// Fold `data` into a running CRC and return the new value.
    ///
    /// `crc` is the CRC of everything before `data`, or 0 to start a new one.
    /// `update(update(0, a), b)` must equal `update(0, a ++ b)`.
    fn update(&self, crc: u8, data: &[u8]) -> u8;
}

/// Bitwise SMBus CRC-8 step: polynomial 0x07, no reflection. Only used to build
/// [`CRC8_TABLE`] at compile time.
const fn crc8_step_bitwise(mut crc: u8, value: u8) -> u8 {
    crc ^= value;
    let mut i = 0;
    while i < 8 {
        crc = if crc & 0x80 != 0 {
            (crc << 1) ^ 0x07
        } else {
            crc << 1
        };
        i += 1;
    }
    crc
}

/// CRC-8 (poly 0x07) of every single byte value, so a step is one lookup. The
/// bitwise loop costs ~60-90 instructions per byte, which dominated frame
/// decode on the slow core.
static CRC8_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = crc8_step_bitwise(0, i as u8);
        i += 1;
    }
    table
};

/// One SMBus CRC-8 step.
#[inline(always)]
fn crc8_step(crc: u8, value: u8) -> u8 {
    CRC8_TABLE[(crc ^ value) as usize]
}

/// The default engine: a 256-byte lookup table, portable to every target.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwarePec;

impl Pec for SoftwarePec {
    #[inline]
    fn update(&self, crc: u8, data: &[u8]) -> u8 {
        data.iter().fold(crc, |crc, &value| crc8_step(crc, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_bitwise_for_all_inputs() {
        for crc in 0..=255u8 {
            for value in 0..=255u8 {
                assert_eq!(crc8_step(crc, value), crc8_step_bitwise(crc, value));
            }
        }
    }

    #[test]
    fn software_pec_check_value() {
        // CRC-8/SMBUS check value for "123456789".
        assert_eq!(SoftwarePec.update(0, b"123456789"), 0xF4);
    }

    #[test]
    fn update_is_incremental() {
        let whole = SoftwarePec.update(0, b"123456789");
        let split = SoftwarePec.update(SoftwarePec.update(0, b"1234"), b"56789");
        assert_eq!(whole, split);
    }
}
