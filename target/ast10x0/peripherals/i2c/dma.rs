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
//! it arms the engine, and dropping it without [`ArmedDma::commit`] soft-resets
//! the controller automatically — so no transfer error path can leave the
//! engine live. All DMA-lifecycle `unsafe` is confined to this module, holding
//! its own `Copy` of the [`Ast1060I2cRegisters`] façade.

use super::constants;
use super::registers::Ast1060I2cRegisters;

/// Guard for one armed master DMA transaction.
///
/// Constructing an `ArmedDma` programs the DMA length + buffer-base registers
/// (the engine is now a potential AHB bus master). If the guard is dropped
/// without [`commit`](ArmedDma::commit), [`Drop`] soft-resets the controller
/// and waits for the engine to go idle. On the happy path the caller calls
/// [`commit`](ArmedDma::commit) and the drop is a no-op.
#[must_use = "drop tears down the DMA engine; bind it for the transfer's lifetime"]
pub(crate) struct ArmedDma {
    mmio: Ast1060I2cRegisters,
    committed: bool,
}

impl ArmedDma {
    /// Arm a TX DMA transaction: program i2cm1c (len-1) + i2cm30 (base addr).
    pub(crate) fn arm_tx(mmio: Ast1060I2cRegisters, phy_addr: u32, len: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        mmio.i2c().i2cm1c().write(|w| unsafe {
            w.dmatx_buf_len_byte()
                .bits((len - 1) as u16)
                .dmatx_buf_len_wr_enbl_for_cur_write_cmd()
                .set_bit()
        });
        mmio.i2c()
            .i2cm30()
            .write(|w| unsafe { w.sdramdmabuffer_base_addr().bits(phy_addr) });
        Self {
            mmio,
            committed: false,
        }
    }

    /// Arm an RX DMA transaction: program i2cm1c (len-1) + i2cm34 (base addr).
    pub(crate) fn arm_rx(mmio: Ast1060I2cRegisters, phy_addr: u32, len: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        mmio.i2c().i2cm1c().modify(|_, w| unsafe {
            w.dmarx_buf_len_byte()
                .bits((len - 1) as u16)
                .dmarx_buf_len_wr_enbl_for_cur_write_cmd()
                .set_bit()
        });
        mmio.i2c()
            .i2cm34()
            .modify(|_, w| unsafe { w.sdramdmabuffer_base_addr1().bits(phy_addr) });
        Self {
            mmio,
            committed: false,
        }
    }

    /// Transfer completed cleanly (STOP issued); no teardown needed.
    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ArmedDma {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        // No master-only abort exists; soft-reset the controller (datasheet
        // §27.6.8): clear I2CC00 function-control, then restore it. Timing in
        // I2CC04 survives. Then spin until the engine reports idle, bounded so
        // a wedged controller cannot hang the drop.
        //
        // This disables the slave function for the reset window, which is safe
        // here: the i2c-server-runtime backend is master-only and never arms a
        // concurrent slave on this controller.
        let fun_ctrl = self.mmio.i2c().i2cc00().read().bits();
        unsafe {
            self.mmio.i2c().i2cc00().write(|w| w.bits(0));
            self.mmio.i2c().i2cc00().write(|w| w.bits(fun_ctrl));
        }

        let mut timeout = constants::ABORT_TIMEOUT_US;
        while timeout > 0 && self.mmio.i2c().i2cc08().read().bus_busy_status().bit() {
            timeout = timeout.saturating_sub(1);
            core::hint::spin_loop();
        }

        // Clear any latched interrupts from the aborted transaction.
        unsafe {
            self.mmio.i2c().i2cm14().write(|w| w.bits(0xffff_ffff));
        }
    }
}
