// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I2C Slave/Target Mode Implementation
//!
//! This module provides slave (target) mode functionality for the AST1060 I2C controllers.
//! In slave mode, the controller responds to requests from an external I2C master.

use super::I2cXferMode;

use super::constants::I2cMasterStatus;
use super::{constants, controller::Ast1060I2c, error::I2cError};

/// Hardware buffer size (32 bytes / 8 DWORDs)
const BUFFER_SIZE: usize = 32;

/// Maximum slave receive buffer size (hardware limitation)
pub const SLAVE_BUFFER_SIZE: usize = 256;

/// Slave RX DMA enable bit in slave command register (i2cs28 bit 9).
///
/// When set, the hardware writes received bytes into the DMA buffer pointed to
/// by i2cs38/i2cs3c instead of the 32-byte FIFO. Supports up to 4096-byte transfers.
const AST_I2CS_RX_DMA_EN: u32 = 1 << 9;

/// Slave mode configuration
#[derive(Debug, Clone, Copy)]
pub struct SlaveConfig {
    /// Primary slave address (7-bit)
    pub address: u8,
    /// Enable packet mode for slave
    pub packet_mode: bool,
    /// Use buffer mode (32 bytes) vs byte mode (1 byte)
    pub buffer_mode: bool,
}

impl SlaveConfig {
    /// Create a new slave configuration
    pub fn new(address: u8) -> Result<Self, I2cError> {
        if address > 0x7F {
            return Err(I2cError::InvalidAddress);
        }

        Ok(Self {
            address,
            packet_mode: true, // Recommended for performance
            buffer_mode: true, // Recommended for performance
        })
    }
}

/// Slave mode events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaveEvent {
    /// Master is requesting to read from us (we need to send data)
    ReadRequest,
    /// Master is writing to us (we're receiving data)
    WriteRequest,
    /// Data received from master
    DataReceived { len: usize },
    /// Data sent to master
    DataSent { len: usize },
    /// Data received from master and send data to master (combined event)
    DataReceivedAndSent { rx_len: usize, tx_len: usize },
    /// Stop condition received
    Stop,
}

/// Slave mode data buffer for application-level buffering
pub struct SlaveBuffer {
    data: [u8; SLAVE_BUFFER_SIZE],
    len: usize,
}

impl Default for SlaveBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SlaveBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: [0u8; SLAVE_BUFFER_SIZE],
            len: 0,
        }
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
    }

    pub fn set_len(&mut self, len: usize) {
        self.len = len.min(SLAVE_BUFFER_SIZE);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn write(&mut self, data: &[u8]) -> usize {
        let to_copy = data.len().min(SLAVE_BUFFER_SIZE);
        self.data[..to_copy].copy_from_slice(&data[..to_copy]);
        self.len = to_copy;
        to_copy
    }
}

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    #[inline]
    fn slave_rx_len(&self) -> usize {
        if self.xfer_mode == I2cXferMode::DmaMode {
            ((self.mmio.i2c.read_reg(constants::I2CS4C) >> 16) & 0x1fff) as usize
        } else {
            // Hardware includes the I2C address byte in the buffer count (packet mode,
            // I2CC00 bit 20). Report the full count including that byte, consistent
            // with what slave_read returns.
            ((self.mmio.i2c.read_reg(constants::I2CC0C) >> 24) & 0x3f) as usize
        }
    }

    /// Arm slave receive path based on transfer mode.
    fn arm_slave_receive(&mut self, cmd: &mut u32) {
        if self.xfer_mode == I2cXferMode::DmaMode {
            if let Some(dma_buf) = self.slave_dma_buf.as_deref_mut() {
                let dma_addr = dma_buf.as_mut_ptr() as u32;
                let dma_len = u16::try_from(dma_buf.len().min(4096) - 1).unwrap_or(u16::MAX);
                self.mmio.i2c.write_reg(constants::I2CS4C, 0);
                self.mmio.i2c.write_reg(constants::I2CS38, dma_addr);
                self.mmio.i2c.write_reg(constants::I2CS3C, dma_addr);
                self.mmio.i2c.write_reg(
                    constants::I2CS2C,
                    ((u32::from(dma_len) & 0xfff) << 16) | (1 << 31),
                );
                *cmd |= AST_I2CS_RX_DMA_EN;
            } else {
                *cmd |= constants::AST_I2CS_RX_BUFF_EN;
                self.mmio.i2c.write_reg(
                    constants::I2CC0C,
                    (u32::from(constants::I2C_BUF_SIZE - 1) & 0x1f) << 16,
                );
            }
        } else if self.xfer_mode == I2cXferMode::BufferMode {
            *cmd |= constants::AST_I2CS_RX_BUFF_EN;
            self.mmio.i2c.write_reg(
                constants::I2CC0C,
                (u32::from(constants::I2C_BUF_SIZE - 1) & 0x1f) << 16,
            );
        } else {
            *cmd &= !constants::AST_I2CS_PKT_MODE_EN;
        }
    }

    /// Configure the controller for slave mode
    pub fn configure_slave(&mut self, config: &SlaveConfig) -> Result<(), I2cError> {
        // Disable master mode while the slave registers are programmed so the
        // shared controller is quiescent during setup; its prior state is
        // restored at the end so slave-only callers keep master off.
        let master_was_enabled = self.mmio.i2c.read_bit(constants::I2CC00, 0);
        self.mmio.i2c.modify_field(constants::I2CC00, 0, 1, 0);

        // Set slave address
        self.mmio.i2c.write_reg(
            constants::I2CS40,
            (u32::from(config.address) & 0x7f) | (1 << 7),
        );

        // Clear slave interrupts
        self.clear_slave_interrupts();

        // Enable slave mode and save address byte in packet mode (I2CC00 bit 20)
        // This makes the hardware include the destination address byte in the receive buffer
        // which is required for MCTP-over-SMBus (DSP0237) packet format.
        let v = self.mmio.i2c.read_reg(constants::I2CC00);
        self.mmio.i2c.write_reg(
            constants::I2CC00,
            v | constants::AST_I2CC_SLAVE_EN | constants::AST_I2CC_SLAVE_PKT_SAVE_ADDR,
        );

        // Configure slave mode
        let mut cmd = 0u32;

        if config.packet_mode {
            cmd |= constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_ACTIVE_ALL;
        }

        if self.xfer_mode == I2cXferMode::BufferMode {
            cmd |= constants::AST_I2CS_RX_BUFF_EN;
            self.mmio.i2c.write_reg(
                constants::I2CC0C,
                (u32::from(constants::I2C_BUF_SIZE - 1) & 0x1f) << 16,
            );
        } else if self.xfer_mode == I2cXferMode::DmaMode {
            if let Some(dma_buf) = self.slave_dma_buf.as_deref_mut() {
                // Arm slave DMA: point hardware at the non-cached buffer and enable RX_DMA.
                // i2cs38/i2cs3c hold the physical DMA buffer address (same address in
                // both registers — the hardware uses both for different address widths).
                // i2cs2c sets the DMA receive length and enables the length register.
                let dma_addr = dma_buf.as_mut_ptr() as u32;
                let dma_len = u16::try_from(dma_buf.len().min(4096) - 1).unwrap_or(u16::MAX);
                self.mmio.i2c.write_reg(constants::I2CS38, dma_addr);
                self.mmio.i2c.write_reg(constants::I2CS3C, dma_addr);
                self.mmio.i2c.write_reg(
                    constants::I2CS2C,
                    ((u32::from(dma_len) & 0xfff) << 16) | (1 << 31),
                );
                cmd |= AST_I2CS_RX_DMA_EN;
            } else {
                // No DMA buffer provided — fall back to buffer mode.
                cmd |= constants::AST_I2CS_RX_BUFF_EN;
                self.mmio.i2c.write_reg(
                    constants::I2CC0C,
                    (u32::from(constants::I2C_BUF_SIZE - 1) & 0x1f) << 16,
                );
            }
        } else {
            cmd &= !constants::AST_I2CS_PKT_MODE_EN;
        }

        // Set slave command register
        self.mmio.i2c.write_reg(constants::I2CS28, cmd);

        // Enable slave interrupts
        self.enable_slave_interrupts();

        // Restore master mode to its prior state: dual master+slave callers
        // (e.g. MCTP) get it back on; slave-only callers keep it off.
        if master_was_enabled {
            self.mmio.i2c.modify_field(constants::I2CC00, 0, 1, 1);
        }

        Ok(())
    }

    /// Enable slave mode interrupts
    fn enable_slave_interrupts(&mut self) {
        let mut mask = constants::AST_I2CS_PKT_DONE | constants::AST_I2CS_INACTIVE_TO;
        if self.xfer_mode == I2cXferMode::BufferMode || self.xfer_mode == I2cXferMode::DmaMode {
            mask |= constants::AST_I2CM_ABNORMAL
                | constants::AST_I2CM_NORMAL_STOP
                | constants::AST_I2CM_RX_DONE
                | constants::AST_I2CM_TX_ACK;
        }

        self.mmio.i2c.write_reg(constants::I2CS20, mask);
    }

    /// Clear slave mode interrupts
    fn clear_slave_interrupts(&mut self) {
        self.mmio.i2c.write_reg(constants::I2CS24, 0xFFFF_FFFF);
        let _ = self.mmio.i2c.read_reg(constants::I2CS24);
    }

    /// Enable slave mode (re-enable after disable)
    ///
    /// This re-enables slave mode and interrupts without reconfiguring the address.
    /// Use `configure_slave()` for initial setup, this for re-enabling after `disable_slave()`.
    pub fn enable_slave(&mut self) {
        // Enable slave mode
        self.mmio.i2c.modify_field(constants::I2CC00, 1, 1, 1);

        // Enable slave interrupts
        self.enable_slave_interrupts();
    }

    /// Disable slave mode
    pub fn disable_slave(&mut self) {
        // Disable interrupts
        self.mmio.i2c.write_reg(constants::I2CS20, 0);

        // Clear interrupts
        self.clear_slave_interrupts();

        // Disable slave mode
        self.mmio.i2c.modify_field(constants::I2CC00, 1, 1, 0);
    }

    /// Check if slave has received data
    #[must_use]
    pub fn slave_has_data(&self) -> bool {
        let status = self.mmio.i2c.read_reg(constants::I2CS24);
        (status & constants::AST_I2CS_RX_DONE) != 0
    }

    /// Read data received in slave mode
    pub fn slave_read(&mut self, buffer: &mut [u8]) -> Result<usize, I2cError> {
        // Get receive length from buffer length register
        if self.xfer_mode == I2cXferMode::BufferMode {
            // AST_I2CC_SLAVE_PKT_SAVE_ADDR (I2CC00 bit 20) deposits the I2C
            // address byte at buffer offset 0. Include it in the returned data;
            // callers that need the full SMBus frame (e.g. MctpI2cEncap::decode)
            // depend on it being present.
            let raw = ((self.mmio.i2c.read_reg(constants::I2CC0C) >> 24) & 0x3f) as usize;
            let to_read = raw.min(buffer.len()).min(BUFFER_SIZE);

            let mut tmp = [0u8; BUFFER_SIZE];
            self.copy_from_buffer(&mut tmp[..to_read])?;
            buffer[..to_read].copy_from_slice(&tmp[..to_read]);

            // Re-enable RX buffer
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_RX_BUFF_EN;
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);

            Ok(to_read)
        } else if self.xfer_mode == I2cXferMode::DmaMode {
            // DMA mode: the hardware has already DMA'd into `self.dma_buf`.
            // AST_I2CC_SLAVE_PKT_SAVE_ADDR deposits the address byte at dma_buf[0];
            // include it in the returned data (matches buffer-mode treatment above).
            let hw_len = ((self.mmio.i2c.read_reg(constants::I2CS4C) >> 16) & 0x1fff) as usize;
            let to_read = hw_len.min(buffer.len());

            if let Some(dma_buf) = self.slave_dma_buf.as_deref() {
                let src_len = to_read.min(dma_buf.len());
                if let (Some(src), Some(dst)) = (dma_buf.get(..src_len), buffer.get_mut(..src_len))
                {
                    dst.copy_from_slice(src);
                }
            }

            // Re-arm slave DMA for next receive
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            if let Some(dma_buf) = self.slave_dma_buf.as_deref_mut() {
                let dma_addr = dma_buf.as_mut_ptr() as u32;
                let dma_len = u16::try_from(dma_buf.len().min(4096) - 1).unwrap_or(u16::MAX);
                self.mmio.i2c.write_reg(constants::I2CS4C, 0);
                self.mmio.i2c.write_reg(constants::I2CS38, dma_addr);
                self.mmio.i2c.write_reg(constants::I2CS3C, dma_addr);
                self.mmio.i2c.write_reg(
                    constants::I2CS2C,
                    ((u32::from(dma_len) & 0xfff) << 16) | (1 << 31),
                );
                cmd |= AST_I2CS_RX_DMA_EN;
            } else {
                cmd |= constants::AST_I2CS_RX_BUFF_EN;
            }
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);

            Ok(to_read)
        } else {
            // byte mode
            let byte = ((self.mmio.i2c.read_reg(constants::I2CC08) >> 8) & 0xff) as u8;
            if let Some(slot) = buffer.get_mut(0) {
                *slot = byte;
            }

            let cmd = constants::AST_I2CS_ACTIVE_ALL;
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);

            self.clear_slave_interrupts();
            Ok(1)
        }
    }

    /// Write data to send in slave mode (in response to read request)
    pub fn slave_write(&mut self, data: &[u8]) -> Result<usize, I2cError> {
        if data.is_empty() {
            return Ok(0);
        }

        if self.xfer_mode == I2cXferMode::BufferMode {
            let to_write = data.len().min(BUFFER_SIZE);

            // Copy data to buffer
            self.copy_to_buffer(&data[..to_write])?;

            // Set transfer length
            #[allow(clippy::cast_possible_truncation)]
            self.mmio
                .i2c
                .write_reg(constants::I2CC0C, ((to_write as u32 - 1) & 0x1f) << 8);

            // Arm TX and keep RX armed in one atomic i2cs28 write.
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_TX_BUFF_EN | constants::AST_I2CS_RX_BUFF_EN;
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);
            Ok(to_write)
        } else if self.xfer_mode == I2cXferMode::DmaMode {
            // Slave TX always uses the 32-byte hardware FIFO, even in DMA mode.
            // The DMA buffer is reserved exclusively for slave RX — writing TX data
            // into dma_buf would alias the RX path. This matches base's
            // slave_set_response() pattern (buffs.buff(0) + TX_BUFF_EN).
            let to_write = data.len().min(BUFFER_SIZE);
            self.copy_to_buffer(&data[..to_write])?;
            #[allow(clippy::cast_possible_truncation)]
            self.mmio
                .i2c
                .write_reg(constants::I2CC0C, ((to_write as u32 - 1) & 0x1f) << 8);

            // Arm TX via FIFO and keep RX DMA armed in one atomic i2cs28 write.
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_TX_BUFF_EN | AST_I2CS_RX_DMA_EN;
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);

            Ok(to_write)
        } else {
            // byte mode
            let cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_TX_CMD;
            self.mmio
                .i2c
                .write_reg(constants::I2CC08, u32::from(data[0]));
            self.mmio.i2c.write_reg(constants::I2CS28, cmd);
            self.clear_slave_interrupts();

            Ok(1)
        }
    }

    /// Handle slave mode interrupt
    #[allow(clippy::too_many_lines)]
    pub fn handle_slave_interrupt(&mut self) -> Option<SlaveEvent> {
        let status = self.mmio.i2c.read_reg(constants::I2CS24);

        if status == 0 {
            // Master status register i2cm14 retains bits after a master operation
            // and keeps the shared IRQ line asserted. Clear it here to stop the storm.
            let m14 = I2cMasterStatus::read(&self.mmio.i2c);
            if m14.raw() != 0 {
                m14.clear(&self.mmio.i2c);
            }
            return None;
        }

        // Check for errors first
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            return None;
        }

        if (status & constants::AST_I2CS_PKT_DONE) != 0 {
            let mut cmd: u32 = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            self.mmio
                .i2c
                .write_reg(constants::I2CS24, constants::AST_I2CS_PKT_DONE);
            let sts = status & (!(constants::AST_I2CS_PKT_DONE | constants::AST_I2CS_PKT_ERROR));
            if sts == constants::AST_I2CS_SLAVE_MATCH
                || sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_RX_DONE
            {
                // S: Sw
                return Some(SlaveEvent::WriteRequest);
            } else if sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_WAIT_RX_DMA
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_WAIT_RX_DMA
            {
                // S: Sw|D
                self.arm_slave_receive(&mut cmd);
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                return Some(SlaveEvent::DataReceived {
                    len: self.slave_rx_len(),
                });
            } else if sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_STOP {
                // S: Sw|P
                self.arm_slave_receive(&mut cmd);
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                return Some(SlaveEvent::Stop);
            } else if sts == constants::AST_I2CS_RX_DONE | constants::AST_I2CS_STOP
                || sts == constants::AST_I2CS_RX_DONE | constants::AST_I2CS_WAIT_RX_DMA
                || sts
                    == constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_WAIT_RX_DMA
                        | constants::AST_I2CS_STOP
                || sts
                    == constants::AST_I2CS_RX_DONE_NAK
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_STOP
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_STOP
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_WAIT_RX_DMA
                        | constants::AST_I2CS_STOP
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE_NAK
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_STOP
            {
                // S: (Sw)|D|(P)
                return Some(SlaveEvent::DataReceived {
                    len: self.slave_rx_len(),
                });
            } else if sts == constants::AST_I2CS_RX_DONE | constants::AST_I2CS_WAIT_TX_DMA
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_WAIT_TX_DMA
            {
                // S: rx_done | wait_tx
                return Some(SlaveEvent::DataReceivedAndSent {
                    rx_len: self.slave_rx_len(),
                    tx_len: (((self.mmio.i2c.read_reg(constants::I2CC0C) >> 8) & 0x1f) + 1)
                        as usize,
                });
            } else if sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_WAIT_TX_DMA {
                // S: Sw | wait_tx
                return Some(SlaveEvent::DataSent {
                    len: (((self.mmio.i2c.read_reg(constants::I2CC0C) >> 8) & 0x1f) + 1) as usize,
                });
            } else if sts == constants::AST_I2CS_WAIT_TX_DMA {
                // S: wait_tx
                return Some(SlaveEvent::DataSent {
                    len: (((self.mmio.i2c.read_reg(constants::I2CC0C) >> 8) & 0x1f) + 1) as usize,
                });
            } else if sts == constants::AST_I2CS_TX_NAK | constants::AST_I2CS_STOP
                || sts == constants::AST_I2CS_STOP
                || sts
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_TX_NAK
                        | constants::AST_I2CS_STOP
            {
                // S: (Sr) (TX_NAK)|P — master read completed with NAK then STOP
                self.arm_slave_receive(&mut cmd);
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                return Some(SlaveEvent::Stop);
            } else {
                // TODO packet slave sts
            }
        } else {
            //byte irq
            let cmd: u32 = constants::AST_I2CS_ACTIVE_ALL;

            if status
                == constants::AST_I2CS_SLAVE_MATCH
                    | constants::AST_I2CS_RX_DONE
                    | constants::AST_I2CS_WAIT_RX_DMA
            {
                // S: Sw|D
                let _byte_data = ((self.mmio.i2c.read_reg(constants::I2CC08) >> 8) & 0xff) as u8;
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                self.mmio.i2c.write_reg(constants::I2CS24, status);
                return Some(SlaveEvent::WriteRequest);
            } else if status
                == constants::AST_I2CS_SLAVE_MATCH
                    | constants::AST_I2CS_RX_DONE
                    | constants::AST_I2CS_WAIT_RX_DMA
                    | constants::AST_I2CS_STOP
                    | constants::AST_I2CS_TX_NAK
                || status
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_RX_DONE
                        | constants::AST_I2CS_WAIT_RX_DMA
                        | constants::AST_I2CS_STOP
            {
                // S: Sw|D|P
                let _byte_data = ((self.mmio.i2c.read_reg(constants::I2CC08) >> 8) & 0xff) as u8;
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                self.mmio.i2c.write_reg(constants::I2CS24, status);
                return Some(SlaveEvent::WriteRequest);
            } else if status == constants::AST_I2CS_RX_DONE | constants::AST_I2CS_WAIT_RX_DMA {
                // S: rD
                return Some(SlaveEvent::DataReceived { len: 1 });
            } else if status
                == constants::AST_I2CS_SLAVE_MATCH
                    | constants::AST_I2CS_RX_DONE
                    | constants::AST_I2CS_WAIT_TX_DMA
            {
                // S: Sr|D
                // received one byte
                let _byte_data = ((self.mmio.i2c.read_reg(constants::I2CC08) >> 8) & 0xff) as u8;
                return Some(SlaveEvent::DataSent { len: 1 });
            } else if status == constants::AST_I2CS_TX_ACK | constants::AST_I2CS_WAIT_TX_DMA {
                // S: tD
                return Some(SlaveEvent::DataSent { len: 1 });
            } else if status == constants::AST_I2CS_STOP
                || status == constants::AST_I2CS_STOP | constants::AST_I2CS_TX_NAK
                || status
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_STOP
                        | constants::AST_I2CS_TX_NAK
                || status
                    == constants::AST_I2CS_SLAVE_MATCH
                        | constants::AST_I2CS_WAIT_RX_DMA
                        | constants::AST_I2CS_STOP
                        | constants::AST_I2CS_TX_NAK
            {
                // S: P
                self.mmio.i2c.write_reg(constants::I2CS28, cmd);
                self.mmio.i2c.write_reg(constants::I2CS24, status);
                return Some(SlaveEvent::Stop);
            }
            // TODO byte slave sts
        }
        None
    }
}
