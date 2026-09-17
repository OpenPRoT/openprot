// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! RAII teardown for in-flight master DMA transactions.
//!
//! A master DMA transfer arms the engine as an AHB bus master pointed at a
//! buffer in shared `.ram_nc`. If the transfer times out, returning an error
//! without stopping the engine leaves it free to keep writing into that buffer
//! (issue #359). The AST1060 has no master-only abort; the only teardown is a
//! controller soft-reset (datasheet §27.6.8).
//!
//! [`ArmedDma`] makes that teardown a property of the type system: constructing
//! it arms the engine, and its [`Drop`] soft-resets the controller
//! unconditionally. [`ArmedDma::run`] drives the transaction to completion and
//! *consumes* the guard on success (defusing the teardown by forgetting it), so
//! a committed transaction is no longer a droppable `ArmedDma` — "committed" is
//! the absence of the value, not a runtime flag. Every error path returns
//! before that point and therefore drops a live guard; there is no state in
//! which the teardown can be skipped by mistake.
//!
//! The guard borrows the controller mutably for the whole armed window, so a
//! second transaction cannot be armed while one is live — exclusivity of the
//! engine is the borrow checker's, not a comment's.

use super::constants::{self, I2cMasterCommand, I2cMasterStatus};
use super::controller::Ast1060I2c;
use super::error::I2cError;

/// Guard for one armed master DMA transaction.
///
/// Constructing an `ArmedDma` programs the DMA length + buffer-base registers
/// (the engine is now a potential AHB bus master). [`Drop`] soft-resets the
/// controller and waits for the engine to go idle. [`run`](ArmedDma::run)
/// issues the command and forgets the guard once the engine has quiesced
/// normally — there is no "committed" flag, the committed state is simply the
/// guard no longer existing.
#[must_use = "drop tears down the DMA engine; bind it for the transfer's lifetime"]
pub(crate) struct ArmedDma<'i, 'b, Y: FnMut(u32)> {
    i2c: &'i mut Ast1060I2c<'b, Y>,
}

impl<'i, 'b, Y: FnMut(u32)> ArmedDma<'i, 'b, Y> {
    /// Arm a TX DMA transaction: program i2cm1c (len-1) + i2cm30 (base addr).
    pub(crate) fn arm_tx(i2c: &'i mut Ast1060I2c<'b, Y>, phy_addr: u32, len: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        i2c.mmio
            .i2c
            .write_reg(constants::I2CM1C, (((len - 1) as u32) & 0xfff) | (1 << 15));
        i2c.mmio
            .i2c
            .write_reg(constants::I2CM30, phy_addr & 0x7fff_ffff);
        Self { i2c }
    }

    /// Arm an RX DMA transaction: program i2cm1c (len-1) + i2cm34 (base addr).
    pub(crate) fn arm_rx(i2c: &'i mut Ast1060I2c<'b, Y>, phy_addr: u32, len: usize) -> Self {
        let cur = i2c.mmio.i2c.read_reg(constants::I2CM1C);
        #[allow(clippy::cast_possible_truncation)]
        let v =
            (cur & !((0xfff << 16) | (1 << 31))) | ((((len - 1) as u32) & 0xfff) << 16) | (1 << 31);
        i2c.mmio.i2c.write_reg(constants::I2CM1C, v);
        i2c.mmio
            .i2c
            .modify_field(constants::I2CM34, 0, 0x7fff_ffff, phy_addr);
        Self { i2c }
    }

    /// Issue `cmd` on i2cm18 and wait for the engine to quiesce. On success the
    /// guard is forgotten, so no teardown runs; on timeout it drops live here
    /// and soft-resets the controller.
    pub(crate) fn run(self, cmd: I2cMasterCommand) -> Result<(), I2cError> {
        self.i2c.clear_interrupts(I2cMasterStatus::all());
        self.i2c.completion = false;

        cmd.issue(&self.i2c.mmio.i2c);

        match self.i2c.wait_completion(constants::DEFAULT_TIMEOUT_US) {
            Ok(()) => {
                core::mem::forget(self);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl<Y: FnMut(u32)> Drop for ArmedDma<'_, '_, Y> {
    fn drop(&mut self) {
        // Reached only for an *uncommitted* guard: `run` forgets the value on
        // the success path, so a committed transaction never drops here.
        // Teardown is therefore unconditional.
        //
        // No master-only abort exists; soft-reset the controller (datasheet
        // §27.6.8): clear I2CC00 function-control, then restore it. Timing in
        // I2CC04 survives. Then spin until the engine reports idle, bounded so
        // a wedged controller cannot hang the drop.
        //
        // This disables the slave function for the reset window, which is safe
        // here: the i2c-server-runtime backend is master-only and never arms a
        // concurrent slave on this controller.
        let fun_ctrl = self.i2c.mmio.i2c.read_reg(constants::I2CC00);
        self.i2c.mmio.i2c.write_reg(constants::I2CC00, 0);
        self.i2c.mmio.i2c.write_reg(constants::I2CC00, fun_ctrl);

        let mut timeout = constants::ABORT_TIMEOUT_US;
        while timeout > 0
            && self
                .i2c
                .mmio
                .i2c
                .read_bit(constants::I2CC08, constants::AST_I2CC_BUS_BUSY_BIT)
        {
            timeout = timeout.saturating_sub(1);
            core::hint::spin_loop();
        }

        // Clear any latched interrupts from the aborted transaction.
        I2cMasterStatus::all().clear(&self.i2c.mmio.i2c);
    }
}
