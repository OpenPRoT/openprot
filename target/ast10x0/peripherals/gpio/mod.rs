// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO peripheral: pin-token-driven, `unsafe`-free access; kernel-side init routes a pin
//! via `crate::hal::field_mux::Route`, `bind_gpio` binds it for userspace to drive.

mod map;

use crate::hal::field_mux::{Offset, RegisterAccess};
pub use crate::hal::gpio::{
    bind_gpio, ActiveHigh, ActiveLow, Gpio, Input, InputPin, IntTrigger, OpenDrain, OpenDrainMode,
    Output, OutputPin, StatefulOutputPin,
};
use crate::scu::pins::GPIO_BASE;
pub(crate) use map::{ABCD, EFGH, IJKL};

/// Target-side alias so call sites outside `hal` don't spell out the GPIO accessor — `hal`'s
/// `GpioPin` takes the accessor as a type parameter, this chip has exactly one.
pub type GpioPin<P, R, D = crate::hal::gpio::Unset> = crate::hal::gpio::GpioPin<P, R, GpioMmio, D>;
/// Target-side alias for [`crate::hal::gpio::GpioBlock`], pinned to this chip's one GPIO accessor.
pub type GpioBlock<R> = crate::hal::gpio::GpioBlock<R, GpioMmio>;

/// The AST10x0 GPIO controller's [`RegisterAccess`] — a ZST, since the controller is a
/// silicon-fixed singleton with no per-instance state. The `unsafe` this chip's MMIO needs lives
/// here, never in `hal`. No `MuxRoutes`/routing goes through this accessor (that's the SCU's
/// `ScuMmio`), so `Block` names nothing but itself.
#[derive(Clone, Copy, Default)]
pub struct GpioMmio;

impl RegisterAccess for GpioMmio {
    type Block = Self;
    const BASE: usize = GPIO_BASE;
    const SIZE: usize = core::mem::size_of::<ast1060_pac::gpio::RegisterBlock>();

    fn read(&self, off: Offset<Self>) -> u32 {
        let addr = Self::BASE + off.get() as usize;
        // SAFETY: `Offset<Self>` can only exist if `off + 4 <= Self::SIZE`, and `SIZE` is this
        // block's real extent straight from the PAC's own generated layout; `BASE` is the GPIO
        // controller's real base, recovered from svd2rust's own const-generic parameter.
        unsafe { core::ptr::with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&self, off: Offset<Self>, val: u32) {
        let addr = Self::BASE + off.get() as usize;
        // SAFETY: same as `read` above.
        unsafe { core::ptr::with_exposed_provenance_mut::<u32>(addr).write_volatile(val) }
    }
}
