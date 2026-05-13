// Licensed under the Apache-2.0 license

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

use super::{constants, controller::Ast1060I2c, error::I2cError, types::I2cXferMode};

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Write bytes to an I2C device
    pub fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
        if bytes.is_empty() {
            return Ok(());
        }

        match self.xfer_mode {
            I2cXferMode::ByteMode => self.write_byte_mode(addr, bytes),
            I2cXferMode::BufferMode => self.write_buffer_mode(addr, bytes),
            I2cXferMode::DmaMode => self.write_dma_mode(addr, bytes),
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

    /// Write then read (combined transaction)
    pub fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), I2cError> {
        // Write phase
        self.write(addr, bytes)?;

        // Read phase
        self.read(addr, buffer)?;

        Ok(())
    }

    /// Write in byte mode (for small transfers)
    ///
    /// Uses i2cc08 for TX byte data buffer, i2cm18 for commands.
    /// Only sends START on first byte, STOP on last byte.
    fn write_byte_mode(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
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
        self.clear_interrupts(0xffff_ffff);

        for (i, &byte) in bytes.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == msg_len - 1;

            // Write data byte to TX byte buffer (i2cc08)
            self.regs()
                .i2cc08()
                .modify(|_, w| unsafe { w.tx_byte_buffer().bits(byte) });

            // Build command
            let mut cmd = constants::AST_I2CM_PKT_EN | constants::AST_I2CM_TX_CMD;

            // Only send START and address on first byte
            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }

            // Send STOP on last byte
            if is_last {
                cmd |= constants::AST_I2CM_STOP_CMD;
            }

            // Issue command to i2cm18
            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            // Wait for completion
            self.completion = false;
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors (read from i2cm14 - interrupt status register)
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_TX_NAK != 0 {
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
        self.clear_interrupts(0xffff_ffff);

        for (i, byte) in buffer.iter_mut().enumerate() {
            let is_first = i == 0;
            let is_last = i == msg_len - 1;

            // Build command
            let mut cmd = constants::AST_I2CM_PKT_EN | constants::AST_I2CM_RX_CMD;

            // Only send START and address on first byte
            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }

            // Send NACK and STOP on last byte
            if is_last {
                cmd |= constants::AST_I2CM_RX_CMD_LAST | constants::AST_I2CM_STOP_CMD;
            }

            // Issue command to i2cm18
            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            // Wait for completion
            self.completion = false;
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Read data from RX byte buffer (i2cc08)
            *byte = self.regs().i2cc08().read().rx_byte_buffer().bits();

            // Check status (read from i2cm14 - interrupt status register)
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_TX_NAK != 0 {
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
    fn write_buffer_mode(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
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
            let chunk = &bytes[offset..offset + chunk_len];
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            // Copy data to hardware buffer BEFORE issuing command
            self.copy_to_buffer(chunk)?;

            // Set TX byte count in i2cc0c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.regs()
                .i2cc0c()
                .modify(|_, w| unsafe { w.tx_data_byte_count().bits((chunk_len - 1) as u8) });

            // Clear interrupts before command
            self.clear_interrupts(0xffff_ffff);
            self.completion = false;

            // Build command based on chunk position
            // First chunk: PKT_EN + addr + START + TX_CMD + TX_BUFF_EN
            // Subsequent chunks: PKT_EN + TX_CMD + TX_BUFF_EN (NO START, NO addr)
            let mut cmd = constants::AST_I2CM_PKT_EN
                | constants::AST_I2CM_TX_CMD
                | constants::AST_I2CM_TX_BUFF_EN;

            // Only send START and address on first chunk
            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }

            // Add STOP on last chunk
            if is_last {
                cmd |= constants::AST_I2CM_STOP_CMD;
            }

            // Issue command to i2cm18
            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
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
            self.regs()
                .i2cc0c()
                .modify(|_, w| unsafe { w.rx_pool_buffer_size().bits((chunk_len - 1) as u8) });

            // Clear interrupts before command
            self.clear_interrupts(0xffff_ffff);
            self.completion = false;

            // Build command based on chunk position
            // First chunk: PKT_EN + addr + START + RX_CMD + RX_BUFF_EN
            // Subsequent chunks: PKT_EN + RX_CMD + RX_BUFF_EN (NO START, NO addr)
            let mut cmd = constants::AST_I2CM_PKT_EN
                | constants::AST_I2CM_RX_CMD
                | constants::AST_I2CM_RX_BUFF_EN;

            // Only send START and address on first chunk
            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }

            // Add NACK and STOP on last chunk
            if is_last {
                cmd |= constants::AST_I2CM_RX_CMD_LAST | constants::AST_I2CM_STOP_CMD;
            }

            // Issue command to i2cm18
            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            // Wait for completion
            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            // Check for errors
            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            // Copy from hardware buffer AFTER successful transfer
            let chunk = &mut buffer[offset..offset + chunk_len];
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
        let status = self.regs().i2cm14().read().bits();

        // Check for packet mode completion
        if status & constants::AST_I2CM_PKT_DONE != 0 {
            self.completion = true;
            self.clear_interrupts(constants::AST_I2CM_PKT_DONE);

            // Check for errors
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                if status & constants::AST_I2CM_ARBIT_LOSS != 0 {
                    return Err(I2cError::ArbitrationLoss);
                }
                if status & constants::AST_I2CM_ABNORMAL != 0 {
                    return Err(I2cError::Abnormal);
                }
                return Err(I2cError::Bus);
            }

            return Ok(());
        }

        // Check for byte mode completion
        if status & (constants::AST_I2CM_TX_ACK | constants::AST_I2CM_RX_DONE) != 0 {
            self.completion = true;
            self.clear_interrupts(status);
            return Ok(());
        }

        // Check for errors
        if status & constants::AST_I2CM_TX_NAK != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::NoAcknowledge);
        }

        if status & constants::AST_I2CM_ABNORMAL != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::Abnormal);
        }

        if status & constants::AST_I2CM_ARBIT_LOSS != 0 {
            self.clear_interrupts(status);
            return Err(I2cError::ArbitrationLoss);
        }

        if status & constants::AST_I2CM_SCL_LOW_TO != 0 {
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
    fn write_dma_mode(&mut self, addr: u8, bytes: &[u8]) -> Result<(), I2cError> {
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
            let chunk = &bytes[offset..offset + chunk_len];
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= total_len;

            // Copy chunk to DMA buffer (non-cached SRAM)
            {
                let dma_buf = self.dma_buf.as_deref_mut().ok_or(I2cError::Invalid)?;
                if dma_buf.len() < chunk_len {
                    return Err(I2cError::Invalid);
                }
                dma_buf[..chunk_len].copy_from_slice(chunk);
            }

            let phy_addr = {
                let dma_buf = self.dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                dma_buf.as_ptr() as u32
            };

            // Set DMA TX length in i2cm1c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.regs().i2cm1c().write(|w| unsafe {
                w.dmatx_buf_len_byte()
                    .bits((chunk_len - 1) as u16)
                    .dmatx_buf_len_wr_enbl_for_cur_write_cmd()
                    .set_bit()
            });

            // Set DMA TX buffer base address in i2cm30
            self.regs()
                .i2cm30()
                .write(|w| unsafe { w.sdramdmabuffer_base_addr().bits(phy_addr) });

            self.clear_interrupts(0xffff_ffff);
            self.completion = false;

            // Build command
            let mut cmd = constants::AST_I2CM_PKT_EN
                | constants::AST_I2CM_TX_CMD
                | constants::AST_I2CM_TX_DMA_EN;

            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }
            if is_last {
                cmd |= constants::AST_I2CM_STOP_CMD;
            }

            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
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
                let dma_buf = self.dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                if dma_buf.len() < chunk_len {
                    return Err(I2cError::Invalid);
                }
            }

            let phy_addr = {
                let dma_buf = self.dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                dma_buf.as_ptr() as u32
            };

            // Set DMA RX length in i2cm1c (len - 1)
            #[allow(clippy::cast_possible_truncation)]
            self.regs().i2cm1c().modify(|_, w| unsafe {
                w.dmarx_buf_len_byte()
                    .bits((chunk_len - 1) as u16)
                    .dmarx_buf_len_wr_enbl_for_cur_write_cmd()
                    .set_bit()
            });

            // Set DMA RX buffer base address in i2cm34
            self.regs()
                .i2cm34()
                .modify(|_, w| unsafe { w.sdramdmabuffer_base_addr1().bits(phy_addr) });

            self.clear_interrupts(0xffff_ffff);
            self.completion = false;

            // Build command
            let mut cmd = constants::AST_I2CM_PKT_EN
                | constants::AST_I2CM_RX_CMD
                | constants::AST_I2CM_RX_DMA_EN;

            if is_first {
                cmd |= constants::ast_i2cm_pkt_addr(addr) | constants::AST_I2CM_START_CMD;
            }
            if is_last {
                cmd |= constants::AST_I2CM_RX_CMD_LAST | constants::AST_I2CM_STOP_CMD;
            }

            self.regs().i2cm18().write(|w| unsafe { w.bits(cmd) });

            self.wait_completion(constants::DEFAULT_TIMEOUT_US)?;

            let status = self.regs().i2cm14().read().bits();
            if status & constants::AST_I2CM_PKT_ERROR != 0 {
                if status & constants::AST_I2CM_TX_NAK != 0 {
                    return Err(I2cError::NoAcknowledge);
                }
                return Err(I2cError::Abnormal);
            }

            // Copy from DMA buffer into caller's buffer
            {
                let dma_buf = self.dma_buf.as_deref().ok_or(I2cError::Invalid)?;
                buffer[offset..offset + chunk_len].copy_from_slice(&dma_buf[..chunk_len]);
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
