// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The AST1060 pin universe: one owned ZST token per pad, with its capability role impls.
//! A SoC fact (same across all AST1060 boards), so it lives here, not in a board.
//!
//! The intended successor to `scu::pinctrl` for pins that move over: see that module's doc for
//! why it isn't marked deprecated yet.

use crate::gpio::{ABCD, EFGH, IJKL};
use openprot_hal::field_mux::{Offset, RegisterAccess};
use openprot_hal::gpio::{Gpio, GpioData};

/// Block-identity marker for the AST10x0 SCU singleton register block (see [`ScuMmio`], its accessor).
pub struct ScuBlock;

/// Recover a peripheral's base address as an integer. svd2rust already carries it as the `usize`
/// const-generic parameter of [`Periph`], but only ever surfaces it pre-cast to a pointer, and a
/// const pointer-to-integer cast is rejected. This re-exposes the parameter itself.
pub trait PeriphAddr {
    /// The peripheral's base address, straight from the SVD.
    const ADDR: usize;
}

impl<RB, const A: usize> PeriphAddr for ast1060_pac::generic::Periph<RB, A> {
    const ADDR: usize = A;
}

/// The GPIO register block base, as an integer a grant check can compare against.
pub(crate) const GPIO_BASE: usize = <ast1060_pac::Gpio as PeriphAddr>::ADDR;

/// The AST10x0 SCU's [`RegisterAccess`] — a ZST, since the SCU is a silicon-fixed singleton with
/// no per-instance state. The `unsafe` this chip's MMIO needs lives here, never in `hal`.
#[derive(Clone, Copy, Default)]
pub struct ScuMmio;

impl RegisterAccess for ScuMmio {
    type Block = ScuBlock;
    const BASE: usize = <ast1060_pac::Scu as PeriphAddr>::ADDR;
    const SIZE: usize = core::mem::size_of::<ast1060_pac::scu::RegisterBlock>();

    fn read(&self, off: Offset<Self>) -> u32 {
        let addr = Self::BASE + off.get() as usize;
        // SAFETY: `Offset<Self>` can only exist if `off + 4 <= Self::SIZE`, and `SIZE` is this
        // block's real extent straight from the PAC's own generated layout; `BASE` is the SCU's
        // real base, recovered from svd2rust's own const-generic parameter via `PeriphAddr`.
        unsafe { core::ptr::with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&self, off: Offset<Self>, val: u32) {
        let addr = Self::BASE + off.get() as usize;
        // SAFETY: same as `read` above.
        unsafe { core::ptr::with_exposed_provenance_mut::<u32>(addr).write_volatile(val) }
    }
}

openprot_hal::pins! {
    scu414_28 { Gpio: &[clear(0x414, 28), clear(0x4b4, 28), clear(0x694, 28)] => GpioData { bit: 28, map: &EFGH } },
    scu414_29 { Gpio: &[clear(0x414, 29), clear(0x4b4, 29), clear(0x694, 29)] => GpioData { bit: 29, map: &EFGH } },
    scu414_30 { Gpio: &[clear(0x414, 30)] => GpioData { bit: 30, map: &EFGH } },
    scu414_31 { Gpio: &[clear(0x414, 31)] => GpioData { bit: 31, map: &EFGH } },
    scu418_0 { Gpio: &[clear(0x418, 0)] => GpioData { bit: 0, map: &IJKL } },
    scu418_1 { Gpio: &[clear(0x418, 1)] => GpioData { bit: 1, map: &IJKL } },
    scu418_8 { Gpio: &[clear(0x418, 8), clear(0x4b8, 8)] => GpioData { bit: 8, map: &IJKL } },
    scu418_13 { Gpio: &[clear(0x418, 13), clear(0x4b8, 13)] => GpioData { bit: 13, map: &IJKL } },
    scu418_16 { Gpio: &[clear(0x418, 16), clear(0x4b8, 16)] => GpioData { bit: 16, map: &IJKL } },
    scu418_21 { Gpio: &[clear(0x418, 21), clear(0x4b8, 21)] => GpioData { bit: 21, map: &IJKL } },

    scu410_0 { Gpio: &[clear(0x410, 0), clear(0x4b0, 0), clear(0x690, 0)] => GpioData { bit: 0, map: &ABCD } },
    scu410_1 { Gpio: &[clear(0x410, 1), clear(0x4b0, 1), clear(0x690, 1)] => GpioData { bit: 1, map: &ABCD } },
    scu410_2 { Gpio: &[clear(0x410, 2), clear(0x4b0, 2), clear(0x690, 2)] => GpioData { bit: 2, map: &ABCD } },
    scu410_3 { Gpio: &[clear(0x410, 3), clear(0x4b0, 3), clear(0x690, 3)] => GpioData { bit: 3, map: &ABCD } },
    scu410_4 { Gpio: &[clear(0x410, 4), clear(0x4b0, 4), clear(0x690, 4)] => GpioData { bit: 4, map: &ABCD } },
    scu410_5 { Gpio: &[clear(0x410, 5), clear(0x4b0, 5), clear(0x690, 5)] => GpioData { bit: 5, map: &ABCD } },
    scu410_6 { Gpio: &[clear(0x410, 6), clear(0x4b0, 6), clear(0x690, 6)] => GpioData { bit: 6, map: &ABCD } },
    scu410_7 { Gpio: &[clear(0x410, 7), clear(0x4b0, 7), clear(0x690, 7)] => GpioData { bit: 7, map: &ABCD } },
}

// Every GPIO-capable pin this chip defines above, proved pairwise distinct at compile time: the
// group is identified by `in_level`'s offset — one physical GPIO controller, with the groups as
// offset ranges within it, not separate blocks, so a bare bit number alone would fold pins in
// *different* groups — e.g. `scu410_0` (ABCD bit 0) and `scu418_0` (IJKL bit 0) — onto the same
// slot and flag them as a collision. `in_level`'s offset is distinct per group (`0x000`/`0x020`/
// `0x070`), so it's what actually tells two same-numbered bits in different groups apart.
openprot_hal::gpio_slots!(
    scu414_28, scu414_29, scu414_30, scu414_31, scu418_0, scu418_1, scu418_8, scu418_13, scu418_16,
    scu418_21, scu410_0, scu410_1, scu410_2, scu410_3, scu410_4, scu410_5, scu410_6, scu410_7,
);
