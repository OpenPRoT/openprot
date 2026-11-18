// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIO pin capability: the [`Gpio`] role tag, its per-pin [`GpioData`] datum, the target's
//! [`GpioMap`] register-table, and the confined [`GpioPin`] accessor bound from a pin token.

use crate::field_mux::{coalesce_routes, Block, Mmio, MuxRoutes, RegBatch, MAX_REGS};
use crate::resource::{field, Capability, FieldWrite, Pin, Routes};
use core::marker::PhantomData;
use util_region::{covers, Mmap, Region};

// A directed pin speaks the embedded-hal vocabulary; re-exported so callers need not name the crate.
pub use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};

/// GPIO-function capability: a pin muxable to GPIO, carrying [`GpioData`].
pub struct Gpio;

impl Capability for Gpio {
    type Data = GpioData;
}

/// Block-identity marker for [`Mmio`]'s GPIO accessor — distinguishes it from any other block's
/// `Mmio<B>` at the type level, same as [`crate::field_mux::Block`] impls do elsewhere; GPIO's
/// base rides on a grant-checked map rather than a fixed `Block::BASE`, so it names no `Block`.
pub struct GpioBlockMarker;

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

/// Marks a [`Reg`] as a plain register — writing doesn't clear any bit the way [`W1C`] does.
pub struct NoW1C;

/// Marks a [`Reg`] as write-1-to-clear (an interrupt-status register: writing 1 to a bit clears it).
pub struct W1C;

/// A checked read handle over an absolute register offset; [`W1C`]/[`NoW1C`] marks whether writing
/// 1 to a bit clears it (a status register) or not.
pub struct Reg<Kind>(u32, PhantomData<Kind>);

impl<Kind> Clone for Reg<Kind> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Kind> Copy for Reg<Kind> {}

impl<Kind> Reg<Kind> {
    /// Wrap a register offset; `Kind` is [`W1C`] or [`NoW1C`].
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset, PhantomData)
    }

    /// The wrapped register offset — needed outside this crate by anything that has to name the
    /// register itself (e.g. a target's pin table, to prove two pins don't share a slot), as
    /// opposed to the common case of reading a pin's own registers through its typed accessors
    /// (e.g. [`GpioPin::int_pending`]).
    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.0
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
    pub base: usize,
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
    pub in_level: Reg<NoW1C>,
    /// Read handle for the output-latch register — bit=1 latched high.
    pub out_level: Reg<NoW1C>,
    /// Read handle for the interrupt-enable register.
    pub int_enable: Reg<NoW1C>,
    /// Read/write-1-to-clear handle for the interrupt-status register.
    pub int_status: Reg<W1C>,
    /// Read handle for the both-edge sensitivity register — the IRQ verify read.
    pub sense_both: Reg<NoW1C>,
    /// Read handle for the direction register — bit=1 configured as output. Open-drain's
    /// `is_set_high`/`is_set_low` read this instead of the data latch, since open-drain drives by
    /// toggling direction, never the data register, once configured.
    pub dir_level: Reg<NoW1C>,
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

/// Fold a role_cfg's field writes at ordinal 0 — const-panics on any set/clear contradiction.
/// Block-agnostic by design: this only checks a role_cfg's writes for internal consistency, never
/// touches real hardware, so it doesn't go through `RegBatch`'s block-extent-bound-checked `push`.
const fn role_cfg_ok(role_cfg: &[RegOp]) {
    let mut folded = [FieldWrite {
        offset: 0,
        set: 0,
        clr: 0,
    }; MAX_REGS];
    let mut len = 0;
    let mut i = 0;
    while i < role_cfg.len() {
        let op = role_cfg[i];
        let mask = (1u32 << op.width) - 1;
        let w = field(op.offset, 0, mask, op.value);
        let mut j = 0;
        while j < len && folded[j].offset != w.offset {
            j += 1;
        }
        if j == len {
            assert!(len < MAX_REGS, "MAX_REGS too small");
            folded[len] = w;
            len += 1;
        } else {
            folded[j].coalesce(w);
        }
        i += 1;
    }
}

/// Prove every register in the map declares one field width, so per-pin bit slots never overlap.
const fn pin_slots_disjoint(role_cfgs: &[&[RegOp]]) {
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
}

/// Const-check every role_cfg in a map: the target calls this once so a contradictory role_cfg is a compile
/// error, not a runtime panic. Enumerates each role_cfg; no loop, since the trait's role_cfg set is fixed.
/// Also proves the whole map is consistently slotted (`pin_slots_disjoint`) so per-pin confinement
/// holds against any role_cfg the map could name.
pub const fn role_cfgs_ok(map: &GpioMap) {
    role_cfg_ok(map.output);
    role_cfg_ok(map.input);
    role_cfg_ok(map.set_high);
    role_cfg_ok(map.set_low);
    role_cfg_ok(map.enable_rising);
    role_cfg_ok(map.enable_falling);
    role_cfg_ok(map.enable_level_high);
    role_cfg_ok(map.enable_level_low);
    role_cfg_ok(map.enable_both);
    role_cfg_ok(map.disable_int);
    pin_slots_disjoint(&[
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
    ]);
}

/// One byte past the last register this map names — the extent a grant must cover.
const fn map_extent(map: &GpioMap) -> usize {
    const fn top(ops: &[RegOp], max: u32) -> u32 {
        let mut max = max;
        let mut i = 0;
        while i < ops.len() {
            if ops[i].offset > max {
                max = ops[i].offset;
            }
            i += 1;
        }
        max
    }
    const fn top_reg(offset: u32, max: u32) -> u32 {
        if offset > max {
            offset
        } else {
            max
        }
    }

    let mut max = 0;
    max = top(map.output, max);
    max = top(map.input, max);
    max = top(map.set_high, max);
    max = top(map.set_low, max);
    max = top(map.enable_rising, max);
    max = top(map.enable_falling, max);
    max = top(map.enable_level_high, max);
    max = top(map.enable_level_low, max);
    max = top(map.enable_both, max);
    max = top(map.disable_int, max);
    max = top_reg(map.in_level.0, max);
    max = top_reg(map.out_level.0, max);
    max = top_reg(map.int_enable.0, max);
    max = top_reg(map.int_status.0, max);
    max = top_reg(map.sense_both.0, max);
    max = top_reg(map.dir_level.0, max);
    max as usize + 4
}

/// True if `bit << op.width` stays inside a register's 32 bits, for every op in every role this
/// map can realize — proven once for the pin's whole map, not just the role actually applied.
const fn bit_shift_fits(map: &GpioMap, bit: u8) -> bool {
    const fn check(ops: &[RegOp], bit: u8) -> bool {
        let mut i = 0;
        while i < ops.len() {
            assert!(
                (bit as u32) * (ops[i].width as u32) < 32,
                "pin bit at this field width shifts past the register's 32 bits"
            );
            i += 1;
        }
        true
    }
    check(map.output, bit)
        && check(map.input, bit)
        && check(map.set_high, bit)
        && check(map.set_low, bit)
        && check(map.enable_rising, bit)
        && check(map.enable_falling, bit)
        && check(map.enable_level_high, bit)
        && check(map.enable_level_low, bit)
        && check(map.enable_both, bit)
        && check(map.disable_int, bit)
}

/// Proof that the granted region `R` contains the register group pin `P` lives in, and that the
/// pin's bit fits every field this map could apply at it. An associated const, not an anonymous
/// `const {}`, so it evaluates under `generic_const_exprs`.
trait AssertGpioGrant {
    /// Evaluating this forces the coverage check; referencing it is what makes it run.
    const COVERS: ();
}

impl<P: Routes<Gpio>, R: Mmap> AssertGpioGrant for (P, R) {
    const COVERS: () = {
        let map = <P as Routes<Gpio>>::DATA.map;
        assert!(
            covers::<R>(map.base, map_extent(map)),
            "granted region does not cover this pin's GPIO register group",
        );
        assert!(bit_shift_fits(map, <P as Routes<Gpio>>::DATA.bit));
    };
}

/// The GPIO register block this process was granted in `system.json5`. Holding one is the authority
/// to touch GPIO at all; binding a pin through it proves the grant covers that pin's group.
pub struct GpioBlock<R: Mmap> {
    _region: PhantomData<R>,
}

impl<R: Mmap> GpioBlock<R> {
    /// Consume the granted region — the single gate between an MPU grant and any GPIO access.
    #[must_use]
    pub fn new(_region: Region<R>) -> Self {
        Self {
            _region: PhantomData,
        }
    }
}

/// Direction not yet chosen — what a freshly bound handle carries.
pub struct Unset;

/// The pin has been switched to output.
pub struct Output;

/// The pin has been switched to input.
pub struct Input;

/// Which data-register value an open-drain pin's "active" (driven, not released) state writes.
pub trait OpenDrainMode {
    /// True if driving the pin corresponds to a logical-high request, false if to logical-low.
    const ACTIVE_HIGH: bool;
}

/// `set_high` drives the pin; `set_low` releases it (lets an external pull-down take it low).
pub struct ActiveHigh;

impl OpenDrainMode for ActiveHigh {
    const ACTIVE_HIGH: bool = true;
}

/// `set_high` releases the pin (lets an external pull-up take it high); `set_low` drives it.
pub struct ActiveLow;

impl OpenDrainMode for ActiveLow {
    const ACTIVE_HIGH: bool = false;
}

/// The pin has been switched to open-drain output. Unlike [`Output`], `set_high`/`set_low`
/// toggle *direction* (drive vs. release), never the data register: the data register is
/// written once, to `ODM::ACTIVE_HIGH`'s polarity, when entering this state, so the pin only
/// ever actively pulls toward the one rail that polarity drives.
pub struct OpenDrain<ODM>(PhantomData<ODM>);

/// Which edge or level an input pin raises its interrupt on.
#[derive(Clone, Copy)]
pub enum IntTrigger {
    /// Rising edge.
    Rising,
    /// Falling edge.
    Falling,
    /// High level.
    LevelHigh,
    /// Low level.
    LevelLow,
    /// Both edges.
    Both,
}

/// One GPIO pin's confined access: just the consumed pin token (a ZST). This pin's base, bit, and
/// register map all ride in on const `<P as Routes<Gpio>>::DATA`, so every op folds to this pin's own
/// base and slot — handles for different pins never alias, with no shared block value to carry.
///
/// `R` is the grant the pin was bound through; `D` is the pin's direction: [`Unset`] until
/// [`GpioPin::into_output`] or [`GpioPin::into_input`] switches it, after which the level and
/// interrupt verbs are the ones that direction allows.
pub struct GpioPin<P, R: Mmap, D = Unset> {
    /// The pin's type is the authority (consumed at bind); the handle is a move-only ZST that reads no field.
    _pin: PhantomData<(P, D)>,
    /// The grant this pin was bound through, kept so the chain stays visible in the type.
    _region: PhantomData<R>,
}

impl<P: Routes<Gpio>, R: Mmap> GpioPin<P, R, Unset> {
    /// Bind a (consumed) pin token through a granted block — the token is the authority for the
    /// pin, the block for the registers; base/bit/map ride in on const DATA.
    ///
    /// Binds as [`Unset`]: only [`GpioPin::into_output`] and [`GpioPin::into_input`] mint a directed
    /// handle, so a direction in the type always means the direction register was written.
    pub fn new(_pin: P, _block: &GpioBlock<R>) -> Self {
        let () = <(P, R) as AssertGpioGrant>::COVERS;
        Self {
            _pin: PhantomData,
            _region: PhantomData,
        }
    }
}

impl<P: Routes<Gpio>, R: Mmap, D> GpioPin<P, R, D> {
    /// This pin's register group map — read handles (`int_enable`, `int_status`, `in_level`) hang off it.
    #[must_use]
    const fn map(&self) -> &'static GpioMap {
        <P as Routes<Gpio>>::DATA.map
    }

    /// A transient MMIO accessor over this pin's GPIO base — a compile-time static carried in its map.
    fn mmio(&self) -> Mmio<GpioBlockMarker> {
        // A fixed peripheral address this accessor actually dereferences, not a sentinel that
        // never gets read — with_exposed_provenance is the strict-provenance escape hatch for that.
        Mmio::from_raw(core::ptr::with_exposed_provenance(self.map().base))
    }

    /// Apply a role at this pin's slot: one panic-free RMW per op; conflicts rejected at compile time by [`role_cfgs_ok`].
    ///
    /// The data register's read side samples the live pin level, not the output latch — every
    /// `GpioMap` gives it the same offset as `in_level` (see the per-group maps). RMW against that
    /// would fold in whatever a sibling output pin's line currently reads, not what its latch holds,
    /// and could silently flip that pin's drive to match. Reading `out_level` instead keeps the RMW
    /// sourced from latched state for every bit except the one this op is changing.
    fn apply(&self, role: GpioRole) {
        let bit = <P as Routes<Gpio>>::DATA.bit;
        let map = self.map();
        let block = self.mmio();
        for op in map.role(role) {
            let mask = (1u32 << op.width) - 1;
            // Only `set`/`clr` are kept — `offset` is discarded unbound here (`..`) rather than
            // named, so the only offset this loop ever touches is `op.offset`/`read_offset`, read
            // straight off this pin's static `GpioMap`, never copied into a local first.
            let FieldWrite { set, clr, .. } = field(op.offset, bit * op.width, mask, op.value);
            let read_offset = if op.offset == map.in_level.0 {
                map.out_level.0
            } else {
                op.offset
            };
            // SAFETY: `op.offset` and `read_offset` both come from this pin's own `GpioMap`, whose
            // extent `GpioPin::new` already proved fits the granted region (`AssertGpioGrant::COVERS`,
            // over `map_extent`, which covers every offset this map can ever name).
            #[allow(unsafe_code)]
            unsafe {
                block.write_reg(op.offset, (block.read_reg(read_offset) & !clr) | set);
            }
        }
    }

    /// True if this pin's bit is set in register `reg` (a read handle from this pin's [`GpioMap`]).
    ///
    /// Private: `reg` is any register this pin's own map happens to carry, so a public version
    /// would let one pin read a handle taken from a *different* pin's map (same `Reg<W1C>` type,
    /// wrong register) and compile cleanly. The named accessors below only ever pass this pin's
    /// own `map()`, so that mix-up isn't expressible from outside this file.
    #[must_use]
    fn read<Kind>(&self, reg: Reg<Kind>) -> bool {
        // SAFETY: `reg.0` comes from this pin's own `GpioMap`, whose extent `GpioPin::new` already
        // proved fits the granted region (`AssertGpioGrant::COVERS`, over `map_extent`).
        #[allow(unsafe_code)]
        unsafe {
            self.mmio().read_reg(reg.0) & (1 << <P as Routes<Gpio>>::DATA.bit) != 0
        }
    }

    /// Write-1-to-clear this pin's bit in the w1c register `reg` (typed `W1C`, so only status registers).
    fn ack(&self, reg: Reg<W1C>) {
        // SAFETY: `reg.0` comes from this pin's own `GpioMap`, whose extent `GpioPin::new` already
        // proved fits the granted region (`AssertGpioGrant::COVERS`, over `map_extent`).
        #[allow(unsafe_code)]
        unsafe {
            self.mmio()
                .write_reg(reg.0, 1 << <P as Routes<Gpio>>::DATA.bit);
        }
    }

    /// Switch the pin to output, carrying the direction in its type from here on.
    #[must_use]
    pub fn into_output(self) -> GpioPin<P, R, Output> {
        self.apply(GpioRole::Output);
        GpioPin {
            _pin: PhantomData,
            _region: PhantomData,
        }
    }

    /// Switch the pin to input, carrying the direction in its type from here on.
    #[must_use]
    pub fn into_input(self) -> GpioPin<P, R, Input> {
        self.apply(GpioRole::Input);
        GpioPin {
            _pin: PhantomData,
            _region: PhantomData,
        }
    }

    /// Switch the pin to open-drain output under polarity `ODM`: the data register is written
    /// once here, to `ODM::ACTIVE_HIGH`'s value, then the pin starts released (direction =
    /// input). From here, `set_high`/`set_low` toggle direction, never data again.
    #[must_use]
    pub fn into_open_drain_output<ODM: OpenDrainMode>(self) -> GpioPin<P, R, OpenDrain<ODM>> {
        self.apply(if ODM::ACTIVE_HIGH {
            GpioRole::SetHigh
        } else {
            GpioRole::SetLow
        });
        self.apply(GpioRole::Input);
        GpioPin {
            _pin: PhantomData,
            _region: PhantomData,
        }
    }
}

impl<P: Routes<Gpio>, R: Mmap> GpioPin<P, R, Input> {
    /// Raise this pin's interrupt on `trigger`.
    pub fn enable_interrupt(&self, trigger: IntTrigger) {
        self.apply(match trigger {
            IntTrigger::Rising => GpioRole::EnableRising,
            IntTrigger::Falling => GpioRole::EnableFalling,
            IntTrigger::LevelHigh => GpioRole::EnableLevelHigh,
            IntTrigger::LevelLow => GpioRole::EnableLevelLow,
            IntTrigger::Both => GpioRole::EnableBoth,
        });
    }

    /// Stop this pin raising interrupts.
    pub fn disable_interrupt(&self) {
        self.apply(GpioRole::DisableInt);
    }

    /// True if this pin's interrupt is currently pending.
    #[must_use]
    pub fn int_pending(&self) -> bool {
        self.read(self.map().int_status)
    }

    /// Clear this pin's pending interrupt.
    pub fn clear_int(&self) {
        self.ack(self.map().int_status);
    }

    /// True if this pin's interrupt is currently enabled.
    #[must_use]
    pub fn int_enabled(&self) -> bool {
        self.read(self.map().int_enable)
    }

    /// True if this pin is configured to trigger on both edges.
    #[must_use]
    pub fn sense_both(&self) -> bool {
        self.read(self.map().sense_both)
    }
}

impl<P, R: Mmap> embedded_hal::digital::ErrorType for GpioPin<P, R, Output> {
    type Error = core::convert::Infallible;
}

impl<P: Routes<Gpio>, R: Mmap> embedded_hal::digital::OutputPin for GpioPin<P, R, Output> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.apply(GpioRole::SetHigh);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.apply(GpioRole::SetLow);
        Ok(())
    }
}

impl<P: Routes<Gpio>, R: Mmap> embedded_hal::digital::StatefulOutputPin for GpioPin<P, R, Output> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.read(self.map().out_level))
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        self.is_set_high().map(|high| !high)
    }
}

impl<P, R: Mmap, ODM> embedded_hal::digital::ErrorType for GpioPin<P, R, OpenDrain<ODM>> {
    type Error = core::convert::Infallible;
}

impl<P: Routes<Gpio>, R: Mmap, ODM: OpenDrainMode> embedded_hal::digital::OutputPin
    for GpioPin<P, R, OpenDrain<ODM>>
{
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.apply(if ODM::ACTIVE_HIGH {
            GpioRole::Output
        } else {
            GpioRole::Input
        });
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.apply(if ODM::ACTIVE_HIGH {
            GpioRole::Input
        } else {
            GpioRole::Output
        });
        Ok(())
    }
}

impl<P: Routes<Gpio>, R: Mmap, ODM: OpenDrainMode> embedded_hal::digital::StatefulOutputPin
    for GpioPin<P, R, OpenDrain<ODM>>
{
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        let is_output = self.read(self.map().dir_level);
        Ok(is_output == ODM::ACTIVE_HIGH)
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        self.is_set_high().map(|high| !high)
    }
}

impl<P, R: Mmap> embedded_hal::digital::ErrorType for GpioPin<P, R, Input> {
    type Error = core::convert::Infallible;
}

impl<P: Routes<Gpio>, R: Mmap> embedded_hal::digital::InputPin for GpioPin<P, R, Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.read(self.map().in_level))
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.is_high().map(|high| !high)
    }
}

/// Bind a pin already routed to GPIO by privileged init; touches no SCU register — userspace-safe.
pub fn bind_gpio<P: Routes<Gpio> + Pin, R: Mmap>(pin: P, block: &GpioBlock<R>) -> GpioPin<P, R> {
    GpioPin::new(pin, block)
}

/// A GPIO handle carries its pin's SCU route at the type level, folded to one RMW per register at
/// compile time. Generic over `B`: GPIO's own HAL layer doesn't name a concrete SCU block (that's
/// target-specific), so `B` is whatever block the caller routes this handle through, same as
/// `Route<C, P, B>`.
impl<B: Block, P: Routes<Gpio>, R: Mmap, D> MuxRoutes<B> for GpioPin<P, R, D> {
    const COALESCED: RegBatch<B> = coalesce_routes(&[<P as Routes<Gpio>>::ROUTE]);
}

/// Const-panics if two slots in `slots` share a `(register-group offset, bit)` pair.
pub const fn gpio_slots_unique(slots: &[(u32, u8)]) {
    let mut i = 0;
    while i < slots.len() {
        let mut j = i + 1;
        while j < slots.len() {
            assert!(
                slots[i].0 != slots[j].0 || slots[i].1 != slots[j].1,
                "two pin tokens claim the same GPIO register-group bit"
            );
            j += 1;
        }
        i += 1;
    }
}

/// Proves no two of the named GPIO-capable pins share a physical register-group bit.
///
/// `pins!` makes a second claim on one *token* a compile error (move-only, mint-once), but says
/// nothing about two different tokens whose authored `DATA` happens to name the same physical
/// bit — that typo would compile cleanly otherwise. Listing every GPIO-capable pin a chip's
/// `pins!` block defines here closes that: the slot table is derived from the same `DATA` the
/// pins already carry, so there is nothing to restate or fall out of sync by hand.
#[macro_export]
macro_rules! gpio_slots {
    ($($pin:ty),+ $(,)?) => {
        const _: () = $crate::gpio::gpio_slots_unique(&[
            $((
                <$pin as $crate::resource::Routes<$crate::gpio::Gpio>>::DATA.map.in_level.offset(),
                <$pin as $crate::resource::Routes<$crate::gpio::Gpio>>::DATA.bit,
            )),+
        ]);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MAP: GpioMap = GpioMap {
        base: 0x1000,
        output: &[RegOp::set(0x04)],
        input: &[RegOp::clear(0x04)],
        set_high: &[RegOp::set(0x00)],
        set_low: &[RegOp::clear(0x00)],
        enable_rising: &[],
        enable_falling: &[],
        enable_level_high: &[],
        enable_level_low: &[],
        enable_both: &[],
        disable_int: &[],
        in_level: Reg::new(0x00),
        out_level: Reg::new(0x0c),
        int_enable: Reg::new(0x08),
        int_status: Reg::new(0x10),
        sense_both: Reg::new(0x14),
        dir_level: Reg::new(0x04),
    };

    #[test]
    fn map_extent_covers_the_highest_named_register() {
        // sense_both at 0x14 is the highest offset named anywhere in TEST_MAP; +4 for its width.
        assert_eq!(map_extent(&TEST_MAP), 0x18);
    }

    #[test]
    fn role_cfg_ok_accepts_a_clean_role() {
        role_cfg_ok(&[RegOp::set(0x04)]);
    }

    #[test]
    #[should_panic(expected = "config sets and clears the same bit")]
    fn role_cfg_ok_rejects_a_set_clear_contradiction() {
        role_cfg_ok(&[RegOp::set(0x04), RegOp::clear(0x04)]);
    }

    #[test]
    fn pin_slots_disjoint_accepts_matching_widths_on_a_shared_register() {
        pin_slots_disjoint(&[&[RegOp::set(0x04)], &[RegOp::field(0x04, 1, 0)]]);
    }

    #[test]
    #[should_panic(expected = "pin slots would overlap")]
    fn pin_slots_disjoint_rejects_mismatched_widths_on_a_shared_register() {
        pin_slots_disjoint(&[&[RegOp::field(0x04, 1, 0)], &[RegOp::field(0x04, 2, 0)]]);
    }

    #[test]
    fn bit_shift_fits_accepts_a_bit_within_range() {
        bit_shift_fits(&TEST_MAP, 7);
    }

    #[test]
    #[should_panic(expected = "shifts past the register's 32 bits")]
    fn bit_shift_fits_rejects_a_bit_that_overflows_the_register() {
        // TEST_MAP's role_cfgs are all width-1 fields, so bit 32 is the first one that overflows.
        bit_shift_fits(&TEST_MAP, 32);
    }

    #[test]
    fn gpio_slots_unique_accepts_distinct_slots() {
        gpio_slots_unique(&[(0x000, 0), (0x000, 1), (0x020, 0)]);
    }

    #[test]
    #[should_panic(expected = "two pin tokens claim the same GPIO register-group bit")]
    fn gpio_slots_unique_rejects_a_duplicate_slot() {
        gpio_slots_unique(&[(0x000, 4), (0x000, 4)]);
    }
}
