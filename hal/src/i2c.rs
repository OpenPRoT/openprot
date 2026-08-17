// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C capability, owned by openprot: a bus is an SCL role + an SDA role + a controller.
//! Pins carry chip-local facts (approach D); the controller owns routing + config.

use crate::field_mux::{coalesce_routes, Mmio, MuxRoutes, RegBatch};
use crate::resource::{Capability, Routes};

/// I2C-SCL capability: a pin muxable as a controller's clock line, carrying [`I2cData`].
pub struct I2cScl;
impl Capability for I2cScl {
    type Data = I2cData;
}

/// I2C-SDA capability: a pin muxable as a controller's data line, carrying [`I2cData`].
pub struct I2cSda;
impl Capability for I2cSda {
    type Data = I2cData;
}

/// One I2C controller's `(I2C, I2CBUFF)` register bases, type-erased to stay PAC-agnostic.
#[derive(Clone, Copy)]
pub struct I2cCtrlRegs {
    /// Const-comparable controller identity, so a bus's two pins prove they name one controller.
    pub id: usize,
    /// Base of the controller's I2C register block.
    pub i2c: *const (),
    /// Base of the controller's I2CBUFF register block.
    pub buff: *const (),
}

/// One I2C pin's datum: its mux route + the controller it wires to.
#[derive(Clone, Copy)]
pub struct I2cData {
    /// The controller this pin is wired to.
    pub ctrl: I2cCtrlRegs,
}

/// Block-identity marker phantom-typing a controller's I2C applier so it can't stand in for its I2CBUFF applier.
pub struct I2cBlock;
/// Block-identity marker phantom-typing a controller's I2CBUFF applier so it can't stand in for its I2C applier.
pub struct I2cBuffBlock;

/// Const-asserts both pins name one controller — an associated const carries the assert (control flow) a generic const block can't.
trait AssertI2cPins {
    const SAME_CONTROLLER: ();
}

impl<Scl: Routes<I2cScl>, Sda: Routes<I2cSda>> AssertI2cPins for (Scl, Sda) {
    const SAME_CONTROLLER: () = assert!(
        <Scl as Routes<I2cScl>>::DATA.ctrl.id == <Sda as Routes<I2cSda>>::DATA.ctrl.id,
        "SCL and SDA pins name different I2C controllers",
    );
}

/// Mint a controller's `(I2C, I2CBUFF)` appliers from a bus's two pin tokens; const-asserts both name one controller.
pub const fn i2c_mmios<Scl: Routes<I2cScl>, Sda: Routes<I2cSda>>(
    _scl: &Scl,
    _sda: &Sda,
) -> (Mmio<I2cBlock>, Mmio<I2cBuffBlock>) {
    let () = <(Scl, Sda) as AssertI2cPins>::SAME_CONTROLLER;
    (
        Mmio::from_raw(<Scl as Routes<I2cScl>>::DATA.ctrl.i2c as *const u8),
        Mmio::from_raw(<Scl as Routes<I2cScl>>::DATA.ctrl.buff as *const u8),
    )
}

/// A controller that brings up an I2C bus — the chip-side seam. Pins are already muxed by the
/// bus constructor at boot, so this only initializes the controller registers.
pub trait I2cController: Sized {
    /// Configuration consumed while opening (e.g. bus timing).
    type Config;
    /// The ready-to-use driver produced on success.
    type Ready;
    /// Failure returned if hardware bring-up fails.
    type Error: core::fmt::Debug;

    /// Bring up the bus, consuming the controller (open-once).
    fn open(self, config: &Self::Config) -> Result<Self::Ready, Self::Error>;
}

/// An I2C bus: two role-capable pins + a controller, all owned (the bus is their claim).
pub struct I2cBus<Scl, Sda, C> {
    // Held to own the pin claim for the bus's lifetime; never read (routing is done at boot).
    #[allow(dead_code)]
    scl: Scl,
    #[allow(dead_code)]
    sda: Sda,
    ctrl: C,
}

impl<Scl, Sda, C> I2cBus<Scl, Sda, C> {
    /// Token authority: the moved-in pin tokens are the sole right to route these pins; the bus's `MuxRoutes` carries their routes, applied by the SCU `route` applier.
    pub const fn new(scl: Scl, sda: Sda, ctrl: C) -> Self {
        Self { scl, sda, ctrl }
    }
}

/// An I2C bus carries both pins' SCU routes at the type level, folded to one RMW per register at compile time.
impl<Scl: Routes<I2cScl>, Sda: Routes<I2cSda>, C> MuxRoutes for I2cBus<Scl, Sda, C> {
    const COALESCED: RegBatch = coalesce_routes(&[
        <Scl as Routes<I2cScl>>::ROUTE,
        <Sda as Routes<I2cSda>>::ROUTE,
    ]);
}

impl<Scl, Sda, C> I2cBus<Scl, Sda, C>
where
    C: I2cController,
{
    /// Bring up the bus, consuming it (open-once). Pins are already muxed by the bus constructor, and
    /// role capability was enforced at construction (`Routes<I2cScl>`/`Routes<I2cSda>` bounds); returns
    /// the controller's error on bring-up failure.
    pub fn open(self, config: &C::Config) -> Result<C::Ready, C::Error> {
        self.ctrl.open(config)
    }
}
