// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Transfer mode implementation
//!
//! Note: The `start_transfer`, `start_byte_mode`, and `start_buffer_mode` functions
//! are kept for potential future use or testing. The main byte/buffer mode logic
//! is now in master.rs with inline command building for better control.

use super::constants::{I2cCmd, I2cMasterCommand, I2cMasterStatus};
use super::{constants, controller::Ast1060I2c, error::I2cError, types::I2cXferMode};

#[allow(dead_code)]
impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Start a transfer (common setup for byte/buffer modes)
    ///
    /// Note: Currently unused - byte/buffer mode functions in master.rs
    /// handle command building directly for better control.
    pub(crate) fn start_transfer(
        &mut self,
        addr: u8,
        is_read: bool,
        len: usize,
    ) -> Result<(), I2cError> {
        if len == 0 || len > 255 {
            return Err(I2cError::Invalid);
        }

        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = len as u32;
        }
        self.current_xfer_cnt = 0;
        self.completion = false;

        // Clear any previous status
        self.clear_interrupts(I2cMasterStatus::all());

        match self.xfer_mode {
            I2cXferMode::ByteMode => {
                self.start_byte_mode(addr, is_read, len);
                Ok(())
            }
            I2cXferMode::BufferMode => self.start_buffer_mode(addr, is_read, len),
            // DMA mode transfers are initiated directly in master.rs (write_dma_mode / read_dma_mode)
            I2cXferMode::DmaMode => Err(super::error::I2cError::Invalid),
        }
    }

    /// Start transfer in byte mode
    ///
    /// Byte mode uses packet mode with single-byte transfers.
    /// Command register is i2cm18, data register is i2cc08.
    fn start_byte_mode(&mut self, addr: u8, is_read: bool, len: usize) {
        // Build command: packet mode + address + start
        let mut cmd = I2cMasterCommand::packet().address(addr).with(I2cCmd::Start);

        if is_read {
            cmd = cmd.with(I2cCmd::Rx);
            // For last byte (or single byte), send NACK and STOP
            if len == 1 {
                cmd = cmd.with(I2cCmd::RxLast).with(I2cCmd::Stop);
            }
        } else {
            cmd = cmd.with(I2cCmd::Tx);
            // For last byte (or single byte), send STOP
            if len == 1 {
                cmd = cmd.with(I2cCmd::Stop);
            }
        }

        // Issue command to i2cm18 (Master Command Register)
        cmd.issue(&self.mmio.i2c);
    }

    /// Start transfer in buffer mode (up to 32 bytes)
    ///
    /// Buffer mode uses packet mode with hardware buffer for multi-byte transfers.
    /// All command bits go to i2cm18 in a single write.
    fn start_buffer_mode(&mut self, addr: u8, is_read: bool, len: usize) -> Result<(), I2cError> {
        if len > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        // Configure buffer size in i2cc0c before issuing command
        #[allow(clippy::cast_possible_truncation)]
        if is_read {
            // Set RX buffer size (len - 1)
            self.mmio
                .i2c
                .modify_field(constants::I2CC0C, 16, 0x1f, (len - 1) as u32);
        } else {
            // Set TX byte count (len - 1)
            self.mmio
                .i2c
                .modify_field(constants::I2CC0C, 8, 0x1f, (len - 1) as u32);
        }

        // Build command: PKT_EN + address + START + TX/RX + BUFF_EN + STOP
        let mut cmd = I2cMasterCommand::packet().address(addr).with(I2cCmd::Start);

        if is_read {
            cmd = cmd
                .with(I2cCmd::Rx)
                .with(I2cCmd::RxBuff)
                .with(I2cCmd::RxLast);
        } else {
            cmd = cmd.with(I2cCmd::Tx).with(I2cCmd::TxBuff);
        }

        // Add stop for last chunk
        cmd = cmd.with(I2cCmd::Stop);

        // Issue command to i2cm18 (Master Command Register) - single write
        cmd.issue(&self.mmio.i2c);

        Ok(())
    }

    /// Copy data to hardware buffer (for writes)
    pub(crate) fn copy_to_buffer(&mut self, data: &[u8]) -> Result<(), I2cError> {
        if data.len() > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        let mut idx = 0;

        while idx < data.len() {
            // Pack bytes into DWORD (little-endian)
            let mut dword: u32 = 0;
            for byte_pos in 0..4 {
                if let Some(&byte) = data.get(idx + byte_pos) {
                    dword |= u32::from(byte) << (byte_pos * 8);
                }
            }

            let dword_idx = idx / 4;
            if dword_idx >= 8 {
                return Err(I2cError::Invalid);
            }

            // Write to buffer register array (AST1060 has 8 DWORDs = 32 bytes)
            self.mmio.buff.write_reg((dword_idx as u32) * 4, dword);

            idx += 4;
        }

        Ok(())
    }

    /// Copy data from hardware buffer (for reads)
    pub(crate) fn copy_from_buffer(&self, data: &mut [u8]) -> Result<(), I2cError> {
        if data.len() > constants::BUFFER_MODE_SIZE {
            return Err(I2cError::Invalid);
        }

        let mut idx = 0;

        while idx < data.len() {
            let dword_idx = idx / 4;
            if dword_idx >= 8 {
                return Err(I2cError::Invalid);
            }

            // Read from buffer register array
            let dword = self.mmio.buff.read_reg((dword_idx as u32) * 4);

            // Extract bytes from DWORD (little-endian)
            for byte_pos in 0..4 {
                if let Some(slot) = data.get_mut(idx + byte_pos) {
                    *slot = ((dword >> (byte_pos * 8)) & 0xFF) as u8;
                }
            }

            idx += 4;
        }

        Ok(())
    }
}
