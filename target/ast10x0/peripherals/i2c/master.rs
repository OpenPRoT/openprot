// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Master mode operations
//!
//! # Reference Implementation
//!
//! This follows the transfer logic from the original working code:
//! - **aspeed-rust/src/i2c/ast1060_i2c.rs** lines 960-1120
//!   - `aspeed_i2c_read()` - RX command building for byte/buffer/DMA modes
//!   - `aspeed_i2c_write()` - TX command building for byte/buffer/DMA modes
//!   - `i2c_aspeed_transfer()` - Main transfer entry point
//!
//! # Key Register Usage
//!
//! - **i2cm18** (Master Command Register): All command bits written here
//!   - Command: `PKT_EN | pkt_addr(addr) | START_CMD | TX/RX_CMD | BUFF_EN | STOP_CMD`
//!   - Reference: `ast1060_i2c.rs:1024` and `ast1060_i2c.rs:1107`
//!
//! - **i2cc08** (Byte Buffer Register): TX/RX byte data for byte mode
//!   - `tx_byte_buffer()`: Write byte to transmit (`ast1060_i2c.rs:1101`)
//!   - `rx_byte_buffer()`: Read received byte (`ast1060_i2c.rs:790`)
//!
//! - **i2cc0c** (Buffer Size Register): Buffer sizes for buffer mode
//!   - `tx_data_byte_count()`: Set TX count (`ast1060_i2c.rs:1089`)
//!   - `rx_pool_buffer_size()`: Set RX size (`ast1060_i2c.rs:1011`)
//!
//! - **i2cm14** (Interrupt Status Register): Read status, write-to-clear
//!   - Reference: `ast1060_i2c.rs:849-870` (`aspeed_i2c_master_irq`)

use super::constants::{I2cCmd, I2cMasterCommand, I2cMasterStatus, I2cStat};
use super::{constants, controller::Ast1060I2c, error::I2cError, types::I2cXferMode};

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Write bytes to an I2C device
    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
        if bytes.is_empty() {
            return Ok(());
        }

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.write_byte_mode(addr, bytes, true),
            I2cXferMode::BufferMode => self.write_buffer_mode(addr, bytes, true),
            I2cXferMode::DmaMode => self.write_dma_mode(addr, bytes, true),
        }
    }

    /// Read bytes from an I2C device
    pub fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        if buffer.is_empty() {
            return Ok(());
        }

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.read_byte_mode(addr, buffer),
            I2cXferMode::BufferMode => self.read_buffer_mode(addr, buffer),
            I2cXferMode::DmaMode => self.read_dma_mode(addr, buffer),
        }
    }

    /// Write then read with repeated-START (no STOP between phases).
    ///
    /// The write phase omits `STOP_CMD` so the hardware holds SCL low
    /// (clock-stretch) after the last TX ACK. The read's `START_CMD` on a
    /// held bus is interpreted by the hardware as a repeated-START, matching
    /// MCTP/SMBus semantics.
    ///
    /// This works in the polling model because the CPU issues the read command
    /// within microseconds of write completion — well within the clock-stretch
    /// window. Requires `smbus_timeout` disabled (or set long enough) in the
    /// bus config to prevent the hardware releasing the bus before the read
    /// command is issued.
    pub fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        // Write without STOP — bus stays held for repeated-START
        let result = match self.xfer_mode {
            I2cXferMode::ByteMode => self.write_byte_mode(addr, bytes, false),
            I2cXferMode::BufferMode => self.write_buffer_mode(addr, bytes, false),
            I2cXferMode::DmaMode => self.write_dma_mode(addr, bytes, false),
        };
        // Read — START on held bus = repeated-START
        result.and_then(|()| match self.xfer_mode {
            I2cXferMode::ByteMode => self.read_byte_mode(addr, buffer),
            I2cXferMode::BufferMode => self.read_buffer_mode(addr, buffer),
            I2cXferMode::DmaMode => self.read_dma_mode(addr, buffer),
        })
    }

    /// Write in byte mode (for small transfers)
    ///
    /// Uses i2cc08 for TX byte data buffer, i2cm18 for commands.
    /// Only sends START on first byte, STOP on last byte.
    fn write_byte_mode(&mut self, addr: u8, bytes: &[u8], stop: bool) -> Result<(), I2cError> {
        let msg_len = bytes.len();

        // Initialize transfer state
        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = msg_len as u32;
        }
        self.current_xfer_cnt = 0;
        self.completion = false;

        // Clear any previous status
        self.clear_interrupts(I2cMasterStatus::all());

        for (i, &byte) in bytes.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == msg_len - 1;

            // Write data byte to TX byte buffer (i2cc08)
            self.mmio
                .i2c
                .modify_field(constants::I2CC08, 0, 0xff, u32::from(byte));

            // Build command
            let mut cmd = I2cMasterCommand::packet().with(I2cCmd::Tx);

            // Only send START and address on first byte
            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }

            // Send STOP on last byte (omitted when caller wants repeated-START)
            if is_last && stop {
                cmd = cmd.with(I2cCmd::Stop);
            }

            // Issue command to i2cm18
            cmd.issue(&self.mmio.i2c);

            // Wait for completion
            self.completion = false;
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors (read from i2cm14 - interrupt status register)
            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::TxNak) {
                return Err(I2cError::NoAcknowledge);
            }

            self.current_xfer_cnt += 1;
        }

        Ok(())
    }

    /// Read in byte mode
    ///
    /// Uses i2cc08 for RX byte data buffer, i2cm18 for commands.
    /// Only sends START on first byte, NACK+STOP on last byte.
    fn read_byte_mode(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let msg_len = buffer.len();

        // Initialize transfer state
        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = msg_len as u32;
        }
        self.current_xfer_cnt = 0;
        self.completion = false;

        // Clear any previous status
        self.clear_interrupts(I2cMasterStatus::all());

        for (i, byte) in buffer.iter_mut().enumerate() {
            let is_first = i == 0;
            let is_last = i == msg_len - 1;

            // Build command
            let mut cmd = I2cMasterCommand::packet().with(I2cCmd::Rx);

            // Only send START and address on first byte
            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }

            // Send NACK and STOP on last byte
            if is_last {
                cmd = cmd.with(I2cCmd::RxLast).with(I2cCmd::Stop);
            }

            // Issue command to i2cm18
            cmd.issue(&self.mmio.i2c);

            // Wait for completion
            self.completion = false;
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Read data from RX byte buffer (i2cc08)
            *byte = ((self.mmio.i2c.read_reg(constants::I2CC08) >> 8) & 0xff) as u8;

            // Check status (read from i2cm14 - interrupt status register)
            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::TxNak) {
                return Err(I2cError::NoAcknowledge);
            }

            self.current_xfer_cnt += 1;
        }

        Ok(())
    }

    /// Write in buffer mode (optimal for 2-32 bytes)
    ///
    /// Uses hardware buffer for efficient multi-byte transfers.
    /// Single transaction model: START+addr on first chunk only,
    /// subsequent chunks continue the transaction without re-addressing.
    /// Reference: `ast1060_i2c.rs` `do_i2cm_tx()` continuation logic
    fn write_buffer_mode(&mut self, addr: u8, bytes: &[u8], stop: bool) -> Result<(), I2cError> {
        let total_len = bytes.len();
        let mut offset = 0;

        // Initialize transfer state
        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = total_len as u32;
        }
        self.current_xfer_cnt = 0;

        while offset < total_len {
            let chunk_len = core::cmp::min(constants::BUFFER_MODE_SIZE, total_len - offset);
            let chunk = bytes
                .get(offset..offset + chunk_len)
                .ok_or(I2cError::Invalid)?;
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            // Copy data to hardware buffer BEFORE issuing command
            self.copy_to_buffer(chunk)?;

            // Set TX byte count in i2cc0c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.mmio
                .i2c
                .modify_field(constants::I2CC0C, 8, 0x1f, (chunk_len - 1) as u32);

            // Clear interrupts before command
            self.clear_interrupts(I2cMasterStatus::all());
            self.completion = false;

            // Build command based on chunk position
            // First chunk: PKT_EN + addr + START + TX_CMD + TX_BUFF_EN
            // Subsequent chunks: PKT_EN + TX_CMD + TX_BUFF_EN (NO START, NO addr)
            let mut cmd = I2cMasterCommand::packet()
                .with(I2cCmd::Tx)
                .with(I2cCmd::TxBuff);

            // Only send START and address on first chunk
            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }

            // Add STOP on last chunk (omitted when caller wants repeated-START)
            if is_last && stop {
                cmd = cmd.with(I2cCmd::Stop);
            }

            // Issue command to i2cm18
            cmd.issue(&self.mmio.i2c);

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::PktError) {
                if status.has(I2cStat::TxNak) {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            #[allow(clippy::cast_possible_truncation)]
            {
                self.current_xfer_cnt += chunk_len as u32;
            }
            offset += chunk_len;
        }

        Ok(())
    }

    /// Read in buffer mode
    ///
    /// Uses hardware buffer for efficient multi-byte transfers.
    /// Single transaction model: START+addr on first chunk only,
    /// subsequent chunks continue the transaction without re-addressing.
    /// Reference: `ast1060_i2c.rs` `do_i2cm_rx()` lines 762-810
    fn read_buffer_mode(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let total_len = buffer.len();
        let mut offset = 0;

        // Initialize transfer state
        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = total_len as u32;
        }
        self.current_xfer_cnt = 0;

        while offset < total_len {
            let chunk_len = core::cmp::min(constants::BUFFER_MODE_SIZE, total_len - offset);
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            // Set RX buffer size in i2cc0c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.mmio
                .i2c
                .modify_field(constants::I2CC0C, 16, 0x1f, (chunk_len - 1) as u32);

            // Clear interrupts before command
            self.clear_interrupts(I2cMasterStatus::all());
            self.completion = false;

            // Build command based on chunk position
            // First chunk: PKT_EN + addr + START + RX_CMD + RX_BUFF_EN
            // Subsequent chunks: PKT_EN + RX_CMD + RX_BUFF_EN (NO START, NO addr)
            let mut cmd = I2cMasterCommand::packet()
                .with(I2cCmd::Rx)
                .with(I2cCmd::RxBuff);

            // Only send START and address on first chunk
            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }

            // Add NACK and STOP on last chunk
            if is_last {
                cmd = cmd.with(I2cCmd::RxLast).with(I2cCmd::Stop);
            }

            // Issue command to i2cm18
            cmd.issue(&self.mmio.i2c);

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::PktError) {
                if status.has(I2cStat::TxNak) {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            // Copy from hardware buffer AFTER successful transfer
            let chunk = buffer
                .get_mut(offset..offset + chunk_len)
                .ok_or(I2cError::Invalid)?;
            self.copy_from_buffer(chunk)?;

            #[allow(clippy::cast_possible_truncation)]
            {
                self.current_xfer_cnt += chunk_len as u32;
            }
            offset += chunk_len;
        }

        Ok(())
    }

    /// Handle interrupt (process completion status)
    pub fn handle_interrupt(&mut self) -> Result<(), I2cError> {
        let status = I2cMasterStatus::read(&self.mmio.i2c);

        // Check for packet mode completion
        if status.has(I2cStat::PktDone) {
            // Workaround: master/slave packet mode TX_ACK stuck issue.
            // When master gets TX_ACK mid-transaction (no STOP yet) while slave
            // packet mode is active, the slave state machine latches a spurious
            // RX_DONE and will NACK the next master byte. Pulse i2cs28 to clear it.
            // Ref: Zephyr i2c_aspeed.c aspeed_i2c_master_irq() ~line 1284
            if status.has(I2cStat::TxAck) && !status.has(I2cStat::NormalStop) {
                if self.mmio.i2c.read_bit(constants::I2CS28, 16) {
                    let slave_cmd = self.mmio.i2c.read_reg(constants::I2CS28);
                    self.mmio.i2c.write_reg(constants::I2CS28, 0);
                    self.mmio.i2c.write_reg(constants::I2CS28, slave_cmd);
                }
            }

            self.completion = true;
            self.clear_interrupts(I2cMasterStatus::flag(I2cStat::PktDone));

            // Check for errors
            if status.has(I2cStat::PktError) {
                if status.has(I2cStat::TxNak) {
                    return Err(I2cError::NoAcknowledge);
                }
                if status.has(I2cStat::ArbitLoss) {
                    return Err(I2cError::ArbitrationLoss);
                }
                if status.has(I2cStat::Abnormal) {
                    return Err(I2cError::Abnormal);
                }
                return Err(I2cError::Bus);
            }

            return Ok(());
        }

        // Check for byte mode completion
        if status.has(I2cStat::TxAck) || status.has(I2cStat::RxDone) {
            self.completion = true;
            self.clear_interrupts(status);
            return Ok(());
        }

        // Check for errors
        if status.has(I2cStat::TxNak) {
            self.clear_interrupts(status);
            return Err(I2cError::NoAcknowledge);
        }

        if status.has(I2cStat::Abnormal) {
            self.clear_interrupts(status);
            return Err(I2cError::Abnormal);
        }

        if status.has(I2cStat::ArbitLoss) {
            self.clear_interrupts(status);
            return Err(I2cError::ArbitrationLoss);
        }

        if status.has(I2cStat::SclLowTo) {
            self.clear_interrupts(status);
            return Err(I2cError::Timeout);
        }

        Ok(())
    }

    // =========================================================================
    // DMA mode
    //
    // Uses system SRAM (non-cached, caller-allocated) as the I2C DMA buffer.
    // The DMA engine can move up to 4096 bytes in a single START/STOP
    // transaction.
    //
    // Register layout for DMA master TX:
    //   i2cm1c: dmatx_buf_len_byte = (len-1), dmatx_buf_len_wr_enbl_for_cur_write_cmd = 1
    //   i2cm30: sdramdmabuffer_base_addr = physical address of DMA buffer
    // For DMA master RX:
    //   i2cm1c: dmarx_buf_len_byte = (len-1), dmarx_buf_len_wr_enbl_for_cur_write_cmd = 1
    //   i2cm34: sdramdmabuffer_base_addr1 = physical address of DMA buffer
    //
    // Reference: aspeed-rust/src/i2c/ast1060_i2c.rs aspeed_i2c_write/read DmaMode branch
    // =========================================================================

    /// Write in DMA mode (up to 4096 bytes in a single transaction)
    ///
    /// The DMA buffer supplied to [`Ast1060I2c::new_with_dma`] is used as the
    /// staging area. For transfers larger than `DMA_MODE_MAX_SIZE` the data is
    /// chunked into successive START-less continuation transactions (i.e. the bus
    /// is NOT released between chunks).
    fn write_dma_mode(&mut self, addr: u8, bytes: &[u8], stop: bool) -> Result<(), I2cError> {
        let total_len = bytes.len();
        let mut offset = 0;

        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = total_len as u32;
        }
        self.current_xfer_cnt = 0;

        while offset < total_len {
            let chunk_len = core::cmp::min(constants::DMA_MODE_MAX_SIZE, total_len - offset);
            let chunk = bytes
                .get(offset..offset + chunk_len)
                .ok_or(I2cError::Invalid)?;
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            // Copy chunk to master DMA buffer (non-cached SRAM)
            {
                let dma_buf = self
                    .master_dma_buf
                    .as_deref_mut()
                    .ok_or(I2cError::Invalid)?;
                if dma_buf.len() < chunk_len {
                    return Err(I2cError::Invalid);
                }
                dma_buf[..chunk_len].copy_from_slice(chunk);
            }

            let phy_addr = {
                let dma_buf = self.master_dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                dma_buf.as_ptr() as u32
            };

            // Set DMA TX length in i2cm1c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.mmio.i2c.write_reg(
                constants::I2CM1C,
                (((chunk_len - 1) as u32) & 0xfff) | (1 << 15),
            );

            // Set DMA TX buffer base address in i2cm30
            self.mmio
                .i2c
                .write_reg(constants::I2CM30, phy_addr & 0x7fff_ffff);

            self.clear_interrupts(I2cMasterStatus::all());
            self.completion = false;

            // Build command
            let mut cmd = I2cMasterCommand::packet()
                .with(I2cCmd::Tx)
                .with(I2cCmd::TxDma);

            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }
            // Add STOP on last chunk (omitted when caller wants repeated-START)
            if is_last && stop {
                cmd = cmd.with(I2cCmd::Stop);
            }

            cmd.issue(&self.mmio.i2c);

            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::PktError) {
                if status.has(I2cStat::TxNak) {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            #[allow(clippy::cast_possible_truncation)]
            {
                self.current_xfer_cnt += chunk_len as u32;
            }
            offset += chunk_len;
        }

        Ok(())
    }

    /// Read in DMA mode (up to 4096 bytes in a single transaction)
    fn read_dma_mode(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), I2cError> {
        let total_len = buffer.len();
        let mut offset = 0;

        self.current_addr = addr;
        #[allow(clippy::cast_possible_truncation)]
        {
            self.current_len = total_len as u32;
        }
        self.current_xfer_cnt = 0;

        while offset < total_len {
            let chunk_len = core::cmp::min(constants::DMA_MODE_MAX_SIZE, total_len - offset);
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            {
                let dma_buf = self.master_dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                if dma_buf.len() < chunk_len {
                    return Err(I2cError::Invalid);
                }
            }

            let phy_addr = {
                let dma_buf = self.master_dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                dma_buf.as_ptr() as u32
            };

            // Set DMA RX length in i2cm1c (len - 1)
            let cur = self.mmio.i2c.read_reg(constants::I2CM1C);
            #[allow(clippy::cast_possible_truncation)]
            let v = (cur & !((0xfff << 16) | (1 << 31)))
                | ((((chunk_len - 1) as u32) & 0xfff) << 16)
                | (1 << 31);
            self.mmio.i2c.write_reg(constants::I2CM1C, v);

            // Set DMA RX buffer base address in i2cm34
            self.mmio
                .i2c
                .modify_field(constants::I2CM34, 0, 0x7fff_ffff, phy_addr);

            self.clear_interrupts(I2cMasterStatus::all());
            self.completion = false;

            // Build command
            let mut cmd = I2cMasterCommand::packet()
                .with(I2cCmd::Rx)
                .with(I2cCmd::RxDma);

            if is_first {
                cmd = cmd.address(addr).with(I2cCmd::Start);
            }
            if is_last {
                cmd = cmd.with(I2cCmd::RxLast).with(I2cCmd::Stop);
            }

            cmd.issue(&self.mmio.i2c);

            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            let status = I2cMasterStatus::read(&self.mmio.i2c);
            if status.has(I2cStat::PktError) {
                if status.has(I2cStat::TxNak) {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            // Copy from master DMA buffer into caller's buffer
            {
                let dma_buf = self.master_dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                buffer
                    .get_mut(offset..offset + chunk_len)
                    .ok_or(I2cError::Invalid)?
                    .copy_from_slice(dma_buf.get(..chunk_len).ok_or(I2cError::Invalid)?);
            }

            #[allow(clippy::cast_possible_truncation)]
            {
                self.current_xfer_cnt += chunk_len as u32;
            }
            offset += chunk_len;
        }

        Ok(())
    }
}
