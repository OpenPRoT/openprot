// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The test image that the host serves and the firmware checks.

#![no_std]

/// Recovery image index of the test image. Boot uses indices 0 to 2 (Caliptra
/// firmware, SoC manifest, MCU firmware), so the test asks for the next one.
pub const INDEX: u8 = 3;

/// Length of the test image in 4-byte words. It is more than one FIFO's worth
/// (64 words) and ends partway through a FIFO.
pub const WORDS: u32 = 1000;

/// Returns word `i` of the test image.
pub const fn word(i: u32) -> u32 {
    i.wrapping_mul(0x9e37_79b9) ^ 0xa5a5_0000
}
