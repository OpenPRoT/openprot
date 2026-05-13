// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Bus recovery implementation

use super::{constants, controller::Ast1060I2c, error::I2cError};

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Recover the I2C bus from stuck condition
    pub fn recover_bus(&mut self) -> Result<(), I2cError> {
        // Disable master and slave functionality
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_master_fn().clear_bit().enbl_slave_fn().clear_bit());

        // Enable master functionality
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_master_fn().set_bit());

        // Clear all interrupts
        self.clear_interrupts(0xffff_ffff);

        // Check SDA/SCL state before attempting recovery
        // Only recover if SDA is stuck low while SCL is high
        let line_status = self.regs().i2cc08().read();
        if line_status.sampled_sdaline_state().bit() || !line_status.sampled_sclline_state().bit() {
            // SDA is not stuck low, or SCL is also stuck - can't recover this way
            return Err(I2cError::BusRecoveryFailed);
        }

        // Issue bus recovery command via I2CM18 (command register)
        self.regs()
            .i2cm18()
            .modify(|_, w| w.enbl_bus_recover_cmd().set_bit());

        // Wait for recovery completion
        let mut timeout = 100_000; // 100ms
        while timeout > 0 {
            let status = self.regs().i2cm14().read().bits();

            if status & constants::AST_I2CM_BUS_RECOVER != 0 {
                self.clear_interrupts(constants::AST_I2CM_BUS_RECOVER);

                if status & constants::AST_I2CM_BUS_RECOVER_FAIL != 0 {
                    return Err(I2cError::BusRecoveryFailed);
                }

                return Ok(());
            }

            timeout -= 1;
        }

        Err(I2cError::Timeout)
    }

    /// Check if bus recovery is needed
    #[must_use]
    pub fn needs_recovery(&self) -> bool {
        let status = self.regs().i2cm14().read().bits();

        // Check for stuck conditions
        (status & constants::AST_I2CM_SCL_LOW_TO != 0)
            || (status & constants::AST_I2CM_SDA_DL_TO != 0)
            || (status & constants::AST_I2CM_ABNORMAL != 0)
    }
}
