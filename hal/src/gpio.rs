// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIO pin capability: the [`Gpio`] role tag, its per-pin [`GpioData`] datum, the target's
//! [`GpioMap`] register-table, and the confined [`GpioPin`] accessor bound from a pin token.

use crate::field_mux::{coalesce_routes, Mmio, MuxRoutes, RegBatch, MAX_REGS};
use crate::resource::{field, Capability, Pin, Routes};
use core::marker::PhantomData;

/// GPIO-function capability: a pin muxable to GPIO, carrying [`GpioData`].
pub struct Gpio;

impl Capability for Gpio {
    type Data = GpioData;
}

/// One GPIO pin's datum: its bit slot in the GPIO register group.
#[derive(Clone, Copy)]
pub struct GpioData {
    /// This pin's bit position within its GPIO register group.
    pub bit: u8,
    /// This pin's register group as data: base + per-role field ops + read handles at absolute offsets.
    pub map: &'static GpioMap,
}

/// One inert GPIO field op: write `value` into this pin's `width`-wide slot at absolute `offset`.
#[derive(Clone, Copy)]
pub struct RegOp {
    /// Absolute byte offset of the register this op touches.
    pub offset: u32,
    /// Field width in bits per pin (1 for a plain data/dir bit, 2 for an STM32 `MODER`-style field).
    pub width: u8,
    /// Value written into this pin's field (its low `width` bits).
    pub value: u32,
}

impl RegOp {
    /// Set the pin's single bit at `offset` (width-1 field, value 1) — the common one-bit-per-pin fact.
    #[must_use]
    pub const fn set(offset: u32) -> Self {
        Self::field(offset, 1, 1)
    }

    /// Clear the pin's single bit at `offset`.
    #[must_use]
    pub const fn clear(offset: u32) -> Self {
        Self::field(offset, 1, 0)
    }

    /// Write `value` into this pin's `width`-wide field at `offset` — the multi-bit escape (STM32 `MODER`).
    #[must_use]
    pub const fn field(offset: u32, width: u8, value: u32) -> Self {
        Self {
            offset,
            width,
            value,
        }
    }
}

/// A checked read handle over an absolute register offset; `W1C` marks a write-1-to-clear (status) register.
#[derive(Clone, Copy)]
pub struct Reg<const W1C: bool>(u32);

impl<const W1C: bool> Reg<W1C> {
    /// Wrap a register offset; `W1C` marks a write-1-to-clear (interrupt-status) register.
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }
}

/// A generic GPIO role; a target's [`GpioMap`] supplies the field ops that realize each one.
#[derive(Clone, Copy)]
pub enum GpioRole {
    /// Drive as output.
    Output,
    /// Switch to input.
    Input,
    /// Drive high.
    SetHigh,
    /// Drive low.
    SetLow,
    /// Enable rising-edge interrupt.
    EnableRising,
    /// Enable falling-edge interrupt.
    EnableFalling,
    /// Enable high-level interrupt.
    EnableLevelHigh,
    /// Enable low-level interrupt.
    EnableLevelLow,
    /// Enable both-edge interrupt.
    EnableBoth,
    /// Disable interrupt.
    DisableInt,
}

/// One GPIO register group as data: base + per-role field ops (absolute offsets) + read handles; a pin carries `&'static`.
#[derive(Clone, Copy)]
pub struct GpioMap {
    /// Base of the GPIO register block these offsets are relative to — one per chip, folded at use.
    pub base: *const (),
    /// Field ops to drive as output.
    pub output: &'static [RegOp],
    /// Field ops to switch to input.
    pub input: &'static [RegOp],
    /// Field ops to drive high.
    pub set_high: &'static [RegOp],
    /// Field ops to drive low.
    pub set_low: &'static [RegOp],
    /// Field ops to enable rising-edge interrupt.
    pub enable_rising: &'static [RegOp],
    /// Field ops to enable falling-edge interrupt.
    pub enable_falling: &'static [RegOp],
    /// Field ops to enable high-level interrupt.
    pub enable_level_high: &'static [RegOp],
    /// Field ops to enable low-level interrupt.
    pub enable_level_low: &'static [RegOp],
    /// Field ops to enable both-edge interrupt.
    pub enable_both: &'static [RegOp],
    /// Field ops to disable interrupt.
    pub disable_int: &'static [RegOp],
    /// Read handle for the sampled input-level register — bit=1 reads high.
    pub in_level: Reg<false>,
    /// Read handle for the output-latch register — bit=1 latched high.
    pub out_level: Reg<false>,
    /// Read handle for the interrupt-enable register.
    pub int_enable: Reg<false>,
    /// Read/write-1-to-clear handle for the interrupt-status register.
    pub int_status: Reg<true>,
    /// Read handle for the both-edge sensitivity register — the IRQ verify read.
    pub sense_both: Reg<false>,
}

impl GpioMap {
    /// The field ops realizing `role` in this register group.
    #[must_use]
    pub const fn role(&self, role: GpioRole) -> &'static [RegOp] {
        match role {
            GpioRole::Output => self.output,
            GpioRole::Input => self.input,
            GpioRole::SetHigh => self.set_high,
            GpioRole::SetLow => self.set_low,
            GpioRole::EnableRising => self.enable_rising,
            GpioRole::EnableFalling => self.enable_falling,
            GpioRole::EnableLevelHigh => self.enable_level_high,
            GpioRole::EnableLevelLow => self.enable_level_low,
            GpioRole::EnableBoth => self.enable_both,
            GpioRole::DisableInt => self.disable_int,
        }
    }
}

/// Fold a role_cfg's field writes at ordinal 0 — the merge asserts on any set/clear contradiction; true if it merges clean.
const fn role_cfg_ok(role_cfg: &[RegOp]) -> bool {
    let mut folded = RegBatch::new();
    let mut i = 0;
    while i < role_cfg.len() {
        let op = role_cfg[i];
        let mask = (1u32 << op.width) - 1;
        folded.append(&[field(op.offset, 0, mask, op.value)]);
        i += 1;
    }
    true
}

/// Prove every register in the map declares one field width, so per-pin bit slots never overlap.
const fn pin_slots_disjoint(role_cfgs: &[&[RegOp]]) -> bool {
    let mut regs = [0u32; MAX_REGS];
    let mut width = [0u8; MAX_REGS];
    let mut n = 0;

    let mut v = 0;
    while v < role_cfgs.len() {
        let role_cfg = role_cfgs[v];
        let mut i = 0;
        while i < role_cfg.len() {
            let op = role_cfg[i];
            let mut k = 0;
            while k < n && regs[k] != op.offset {
                k += 1;
            }
            if k == n {
                assert!(n < MAX_REGS, "GpioMap touches more registers than MAX_REGS");
                regs[n] = op.offset;
                width[n] = op.width;
                n += 1;
            } else {
                assert!(
                    width[k] == op.width,
                    "GpioMap: two ops on one register declare different field widths — \
                     pin slots would overlap, so a pin could clobber a neighbor"
                );
            }
            i += 1;
        }
        v += 1;
    }
    true
}

/// Const-check every role_cfg in a map: the target asserts this once so a contradictory role_cfg is a compile
/// error, not a runtime panic. Enumerates each role_cfg; no loop, since the trait's role_cfg set is fixed.
/// Also proves the whole map is consistently slotted (`pin_slots_disjoint`) so per-pin confinement
/// holds against any role_cfg the map could name.
#[must_use]
pub const fn role_cfgs_ok(map: &GpioMap) -> bool {
    role_cfg_ok(map.output)
        && role_cfg_ok(map.input)
        && role_cfg_ok(map.set_high)
        && role_cfg_ok(map.set_low)
        && role_cfg_ok(map.enable_rising)
        && role_cfg_ok(map.enable_falling)
        && role_cfg_ok(map.enable_level_high)
        && role_cfg_ok(map.enable_level_low)
        && role_cfg_ok(map.enable_both)
        && role_cfg_ok(map.disable_int)
        && pin_slots_disjoint(&[
            map.output,
            map.input,
            map.set_high,
            map.set_low,
            map.enable_rising,
            map.enable_falling,
            map.enable_level_high,
            map.enable_level_low,
            map.enable_both,
            map.disable_int,
        ])
}

/// One GPIO pin's confined access: just the consumed pin token (a ZST). This pin's base, bit, and
/// register map all ride in on const `<P as Routes<Gpio>>::DATA`, so every op folds to this pin's own
/// base and slot — handles for different pins never alias, with no shared block value to carry.
pub struct GpioPin<P> {
    /// The pin's type is the authority (consumed at bind); the handle is a move-only ZST that reads no field.
    _pin: PhantomData<P>,
}

impl<P: Routes<Gpio>> GpioPin<P> {
    /// Bind a (consumed) pin token — the token is the authority; base/bit/map ride in on const DATA.
    pub fn new(_pin: P) -> Self {
        Self { _pin: PhantomData }
    }

    /// This pin's register group map — read handles (`int_enable`, `int_status`, `in_level`) hang off it.
    #[must_use]
    pub const fn map(&self) -> &'static GpioMap {
        <P as Routes<Gpio>>::DATA.map
    }

    /// A transient MMIO accessor over this pin's GPIO base — a compile-time static carried in its map.
    fn mmio(&self) -> Mmio<()> {
        Mmio::from_raw(self.map().base as *const u8)
    }

    /// Apply a role at this pin's slot: one panic-free RMW per op; conflicts rejected at compile time by [`role_cfgs_ok`].
    pub fn apply(&self, role: GpioRole) {
        let bit = <P as Routes<Gpio>>::DATA.bit;
        let block = self.mmio();
        for op in self.map().role(role) {
            let mask = (1u32 << op.width) - 1;
            let fw = field(op.offset, bit * op.width, mask, op.value);
            block.write_reg(fw.offset, (block.read_reg(fw.offset) & !fw.clr) | fw.set);
        }
    }

    /// True if this pin's bit is set in register `reg` (a read handle from this pin's [`GpioMap`]).
    #[must_use]
    pub fn read<const W1C: bool>(&self, reg: Reg<W1C>) -> bool {
        self.mmio().read_reg(reg.0) & (1 << <P as Routes<Gpio>>::DATA.bit) != 0
    }

    /// Write-1-to-clear this pin's bit in the w1c register `reg` (typed `W1C`, so only status registers).
    pub fn ack(&self, reg: Reg<true>) {
        self.mmio()
            .write_reg(reg.0, 1 << <P as Routes<Gpio>>::DATA.bit);
    }
}

/// Bind a pin already routed to GPIO by privileged init; touches no SCU register — userspace-safe.
pub fn bind_gpio<P: Routes<Gpio> + Pin>(pin: P) -> GpioPin<P> {
    GpioPin::new(pin)
}

/// A GPIO handle carries its pin's SCU route at the type level, folded to one RMW per register at compile time.
impl<P: Routes<Gpio>> MuxRoutes for GpioPin<P> {
    const COALESCED: RegBatch = coalesce_routes(&[<P as Routes<Gpio>>::ROUTE]);
}

/// `pin.into_gpio()`: consume a GPIO-capable token into a bind-only handle; the SCU applier routes it later.
pub trait IntoGpio: Routes<Gpio> + Pin + Sized {
    /// Bind this pin token to a GPIO handle (no SCU write); its route rides in via [`MuxRoutes`].
    fn into_gpio(self) -> GpioPin<Self> {
        GpioPin::new(self)
    }
}

impl<P: Routes<Gpio> + Pin> IntoGpio for P {}
