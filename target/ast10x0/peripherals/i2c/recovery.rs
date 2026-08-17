// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Bus recovery implementation

use super::constants::{I2cMasterCommand, I2cMasterStatus, I2cStat};
use super::{constants, controller::Ast1060I2c, error::I2cError};

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Recover the I2C bus from stuck condition
    pub fn recover_bus(&mut self) -> Result<(), I2cError> {
        // Disable master and slave functionality
        let v = self.mmio.i2c.read_reg(constants::I2CC00);
        self.mmio.i2c.write_reg(
            constants::I2CC00,
            v & !(constants::AST_I2CC_MASTER_EN | constants::AST_I2CC_SLAVE_EN),
        );

        // Enable master functionality
        self.mmio.i2c.modify_field(constants::I2CC00, 0, 1, 1);

        // Clear all interrupts
        self.clear_interrupts(I2cMasterStatus::all());

        // Check SDA/SCL state before attempting recovery
        // Only recover if SDA is stuck low while SCL is high
        let line_status = self.mmio.i2c.read_reg(constants::I2CC08);
        if (line_status >> 17) & 1 != 0 || (line_status >> 18) & 1 == 0 {
            // SDA is not stuck low, or SCL is also stuck - can't recover this way
            return Err(I2cError::BusRecoveryFailed);
        }

        // Issue bus recovery command via I2CM18 (command register)
        I2cMasterCommand::recover(&self.mmio.i2c);

        // Wait for recovery completion
        let mut timeout = 100_000; // 100ms
        while timeout > 0 {
            let status = I2cMasterStatus::read(&self.mmio.i2c);

            if status.has(I2cStat::BusRecover) {
                self.clear_interrupts(I2cMasterStatus::flag(I2cStat::BusRecover));

                if status.has(I2cStat::BusRecoverFail) {
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
        let status = I2cMasterStatus::read(&self.mmio.i2c);

        // Check for stuck conditions
        status.has(I2cStat::SclLowTo)
            || status.has(I2cStat::SdaDlTo)
            || status.has(I2cStat::Abnormal)
    }
}
