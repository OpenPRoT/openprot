// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I2C Slave/Target Mode Implementation
//!
//! This module provides slave (target) mode functionality for the AST1060 I2C controllers.
//! In slave mode, the controller responds to requests from an external I2C master.

use super::I2cXferMode;

use super::{constants, controller::Ast1060I2c, error::I2cError};

/// Hardware buffer size (32 bytes / 8 DWORDs)
const BUFFER_SIZE: usize = 32;

/// Maximum slave receive buffer size (hardware limitation)
pub const SLAVE_BUFFER_SIZE: usize = 256;

/// Write-1-only control for the slave command register (i2cs28 bit 31).
///
/// With this bit set the write asserts just the bits that are 1 and leaves the
/// rest of I2CS28 as it was, so re-asserting the buffer-enable does not rewrite
/// the active-address selection (18:17) or packet-mode enable (16) underneath a
/// parked transaction.
const AST_I2CS_CMD_W1T: u32 = 1 << 31;

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
            self.regs().i2cs4c().read().dmarx_actual_len_byte().bits() as usize
        } else {
            // Hardware includes the I2C address byte in the buffer count (packet mode,
            // I2CC00 bit 20). Report the full count including that byte, consistent
            // with what slave_read returns.
            self.regs()
                .i2cc0c()
                .read()
                .actual_rxd_pool_buffer_size()
                .bits() as usize
        }
    }

    /// Arm slave receive path based on transfer mode.
    fn arm_slave_receive(&mut self, cmd: &mut u32) {
        if self.xfer_mode == I2cXferMode::DmaMode {
            if let Some(dma_buf) = self.slave_dma_buf.as_deref() {
                let dma_addr = dma_buf.as_ptr() as u32;
                let dma_len = u16::try_from(dma_buf.len().min(4096) - 1).unwrap_or(u16::MAX);
                unsafe {
                    self.regs().i2cs4c().write(|w| w.bits(0));
                    self.regs().i2cs38().write(|w| w.bits(dma_addr));
                    self.regs().i2cs3c().write(|w| w.bits(dma_addr));
                    self.regs().i2cs2c().write(|w| {
                        w.dmarx_buf_len_byte()
                            .bits(dma_len)
                            .dmarx_buf_len_wr_enbl_for_cur_cmd()
                            .set_bit()
                    });
                }
                *cmd |= AST_I2CS_RX_DMA_EN;
            } else {
                *cmd |= constants::AST_I2CS_RX_BUFF_EN;
                self.regs().i2cc0c().write(|w| unsafe {
                    w.rx_pool_buffer_size().bits(constants::I2C_BUF_SIZE - 1)
                });
            }
        } else if self.xfer_mode == I2cXferMode::BufferMode {
            *cmd |= constants::AST_I2CS_RX_BUFF_EN;
            self.regs()
                .i2cc0c()
                .write(|w| unsafe { w.rx_pool_buffer_size().bits(constants::I2C_BUF_SIZE - 1) });
        } else {
            *cmd &= !constants::AST_I2CS_PKT_MODE_EN;
        }
    }

    /// Configure the controller for slave mode
    pub fn configure_slave(&mut self, config: &SlaveConfig) -> Result<(), I2cError> {
        // Disable master mode while the slave registers are programmed so the
        // shared controller is quiescent during setup; its prior state is
        // restored at the end so slave-only callers keep master off.
        let master_was_enabled = self.regs().i2cc00().read().enbl_master_fn().bit();
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_master_fn().clear_bit());

        // Set slave address
        self.regs().i2cs40().write(|w| unsafe {
            w.slave_dev_addr1()
                .bits(config.address)
                .enbl_slave_dev_addr1only_for_new_reg_mode()
                .bit(true)
        });

        // Clear slave interrupts
        self.clear_slave_interrupts();

        // Enable slave mode and save address byte in packet mode (I2CC00 bit 20)
        // This makes the hardware include the destination address byte in the receive buffer
        // which is required for MCTP-over-SMBus (DSP0237) packet format.
        self.regs().i2cc00().modify(|r, w| unsafe {
            w.bits(
                r.bits() | constants::AST_I2CC_SLAVE_EN | constants::AST_I2CC_SLAVE_PKT_SAVE_ADDR,
            )
        });

        // Configure slave mode
        let mut cmd = 0u32;

        if config.packet_mode {
            cmd |= constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_ACTIVE_ALL;
        }

        // Reuse the exact same arm sequence every re-arm uses, instead of a
        // hand-duplicated copy that can quietly drift out of sync with it.
        self.arm_slave_receive(&mut cmd);

        // Set slave command register
        unsafe {
            self.regs().i2cs28().write(|w| w.bits(cmd));
        }

        // The first frame is paid for by this setup, so it can park and resume
        // like any other.
        self.slave_rx_credit = true;

        // Enable slave interrupts
        self.enable_slave_interrupts();

        // Restore master mode to its prior state: dual master+slave callers
        // (e.g. MCTP) get it back on; slave-only callers keep it off.
        if master_was_enabled {
            self.regs()
                .i2cc00()
                .modify(|_, w| w.enbl_master_fn().set_bit());
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

        unsafe {
            self.regs().i2cs20().write(|w| w.bits(mask));
        }
    }

    /// Clear slave mode interrupts
    fn clear_slave_interrupts(&mut self) {
        unsafe {
            self.regs().i2cs24().write(|w| w.bits(0xFFFF_FFFF));
            let _ = self.regs().i2cs24().read().bits();
        }
    }

    /// Enable slave mode (re-enable after disable)
    ///
    /// This re-enables slave mode and interrupts without reconfiguring the address.
    /// Use `configure_slave()` for initial setup, this for re-enabling after `disable_slave()`.
    pub fn enable_slave(&mut self) {
        // Enable slave mode
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_slave_fn().set_bit());

        // Enable slave interrupts
        self.enable_slave_interrupts();
    }

    /// Disable slave mode
    pub fn disable_slave(&mut self) {
        // Disable interrupts
        unsafe {
            self.regs().i2cs20().write(|w| w.bits(0));
        }

        // Clear interrupts
        self.clear_slave_interrupts();

        // Disable slave mode
        self.regs()
            .i2cc00()
            .modify(|_, w| w.enbl_slave_fn().clear_bit());
    }

    /// Check if slave has received data
    #[must_use]
    pub fn slave_has_data(&self) -> bool {
        let status = self.regs().i2cs24().read().bits();
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
            let raw = self
                .regs()
                .i2cc0c()
                .read()
                .actual_rxd_pool_buffer_size()
                .bits() as usize;
            let to_read = raw.min(buffer.len()).min(BUFFER_SIZE);

            let mut tmp = [0u8; BUFFER_SIZE];
            self.copy_from_buffer(&mut tmp[..to_read])?;
            buffer[..to_read].copy_from_slice(&tmp[..to_read]);

            // Deliberately not re-armed here: see `rearm_slave_receive()`.
            Ok(to_read)
        } else if self.xfer_mode == I2cXferMode::DmaMode {
            // DMA mode: the hardware has already DMA'd into `self.dma_buf`.
            // AST_I2CC_SLAVE_PKT_SAVE_ADDR deposits the address byte at dma_buf[0];
            // include it in the returned data (matches buffer-mode treatment above).

            let hw_len = self.regs().i2cs4c().read().dmarx_actual_len_byte().bits() as usize;
            let to_read = hw_len.min(buffer.len());

            // The CM4F cache is write-through and covers this buffer, so the
            // CPU's own earlier stores here satisfy reads and hide what the DMA
            // engine wrote. Invalidate the covering entries first. SCUA50 cannot
            // mark the region uncacheable: it is only writable while the CM4F is
            // held in reset, so it keeps its all-cacheable reset value.
            if let Some(dma_buf) = self.slave_dma_buf.as_deref() {
                let base = dma_buf.as_ptr() as u32;
                let span = to_read.min(dma_buf.len()) as u32;
                // SAFETY: SCUA54/SCUA58 are granted to this process. The cache is
                // taken down around the invalidate, matching `entry.rs`'s
                // init_cache sequence (disable, invalidate, re-enable) — the
                // W1T invalidate does not take effect on a live cache.
                unsafe {
                    let scu = &*ast1060_pac::Scu::ptr();
                    scu.scua58().write(|w| w.bits(0));
                    let mut off = 0u32;
                    loop {
                        let addr = (base.wrapping_add(off)) & 0x7ff;
                        scu.scua54().write(|w| w.bits((1 << 31) | (addr << 16)));
                        if off >= span {
                            break;
                        }
                        off = off.wrapping_add(4);
                    }
                    scu.scua58().write(|w| w.bits(1));
                }
            }

            if let Some(dma_buf) = self.slave_dma_buf.as_deref() {
                let src_len = to_read.min(dma_buf.len());
                if let (Some(src), Some(dst)) = (dma_buf.get(..src_len), buffer.get_mut(..src_len))
                {
                    dst.copy_from_slice(src);
                }
            }

            // Clear the DMA length status now that these bytes are copied out.
            // One frame can raise two completion events; since the re-arm is
            // deferred, the second would otherwise still see this length and
            // deliver the same bytes twice.
            // SAFETY: writing I2CS4C clears it, per the datasheet.
            unsafe {
                self.regs().i2cs4c().write(|w| w.bits(0));
            }

            // Deliberately not re-armed here: see `rearm_slave_receive()`.
            Ok(to_read)
        } else {
            // byte mode
            let byte = self.regs().i2cc08().read().rx_byte_buffer().bits();
            if let Some(slot) = buffer.get_mut(0) {
                *slot = byte;
            }

            let cmd = constants::AST_I2CS_ACTIVE_ALL;
            self.regs().i2cs28().write(|w| unsafe { w.bits(cmd) });

            self.clear_slave_interrupts();
            Ok(1)
        }
    }

    /// Re-arm the slave receive path after a drained frame has actually been
    /// consumed (not just copied into `slave_read`'s caller-supplied buffer).
    ///
    /// Deferring this — rather than re-arming immediately after `slave_read`
    /// drains — is what holds the receive path unarmed until the consumer has
    /// actually taken the frame. With no buffer prepared the next frame is
    /// ACKed and then holds SCL low ("slave wait for FW"), and preparing the
    /// buffer here releases it: the datasheet's `slave wait for FW -> RxD -> P`.
    pub fn rearm_slave_receive(&mut self) {
        if self.xfer_mode != I2cXferMode::BufferMode && self.xfer_mode != I2cXferMode::DmaMode {
            return;
        }

        // The consumer has taken a frame, so grant permission for one more. A
        // parked frame spends this on its address packet and then on its data;
        // nothing else may arm the receive.
        self.slave_rx_credit = true;

        // A parked transaction has already been accepted by the hardware.
        // Re-preparing its buffer drops it and admits the frame after it, so in
        // that state only the enable below is re-asserted. The base and length
        // are unchanged anyway — skipping the writes costs nothing.
        let parked = (self.regs().i2cs24().read().bits() & constants::AST_I2CS_WAIT_RX_DMA) != 0;

        // Clear the finished frame's status while the slave is still parked with
        // no RX command. After the command below lands, a clear truncates the
        // frame it just released.
        // SAFETY: PKT_DONE is write-1-to-clear.
        unsafe {
            self.regs()
                .i2cs24()
                .write(|w| w.bits(constants::AST_I2CS_PKT_DONE));
        }

        // Step 8 of the slave programming sequence: prepare the Rx buffer.
        let enable = if self.xfer_mode == I2cXferMode::DmaMode {
            if let Some(dma_buf) = self.slave_dma_buf.as_deref() {
                let dma_addr = dma_buf.as_ptr() as u32;
                let dma_len = u16::try_from(dma_buf.len().min(4096) - 1).unwrap_or(u16::MAX);
                // SAFETY: I2CS3C is the slave DMA *Rx* base (I2CS38 is Tx, and
                // is left alone). I2CS2C[31] is the Rx-length write enable, so
                // only the Rx length field is updated.
                if !parked {
                    unsafe {
                        self.regs().i2cs4c().write(|w| w.bits(0));
                        self.regs().i2cs3c().write(|w| w.bits(dma_addr));
                        self.regs().i2cs2c().write(|w| {
                            w.dmarx_buf_len_byte()
                                .bits(dma_len)
                                .dmarx_buf_len_wr_enbl_for_cur_cmd()
                                .set_bit()
                        });
                    }
                }
                AST_I2CS_RX_DMA_EN
            } else {
                constants::AST_I2CS_RX_BUFF_EN
            }
        } else {
            if !parked {
                self.regs().i2cc0c().write(|w| unsafe {
                    w.rx_pool_buffer_size().bits(constants::I2C_BUF_SIZE - 1)
                });
            }
            constants::AST_I2CS_RX_BUFF_EN
        };

        self.enable_slave_interrupts();

        // Step 9: the hardware consumes I2CS28[9] on every completed packet, so
        // it has to be re-asserted. This write is what releases the parked frame
        // and ends the clock stretch, so it comes last.
        // SAFETY: W1T write asserting a single command bit.
        unsafe {
            self.regs()
                .i2cs28()
                .write(|w| w.bits(AST_I2CS_CMD_W1T | enable));
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
            self.regs()
                .i2cc0c()
                .write(|w| unsafe { w.tx_data_byte_count().bits((to_write - 1) as u8) });

            // Arm TX and keep RX armed in one atomic i2cs28 write.
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_TX_BUFF_EN | constants::AST_I2CS_RX_BUFF_EN;
            unsafe {
                self.regs().i2cs28().write(|w| w.bits(cmd));
            }
            Ok(to_write)
        } else if self.xfer_mode == I2cXferMode::DmaMode {
            // Slave TX always uses the 32-byte hardware FIFO, even in DMA mode.
            // The DMA buffer is reserved exclusively for slave RX — writing TX data
            // into dma_buf would alias the RX path. This matches base's
            // slave_set_response() pattern (buffs.buff(0) + TX_BUFF_EN).
            let to_write = data.len().min(BUFFER_SIZE);
            self.copy_to_buffer(&data[..to_write])?;
            #[allow(clippy::cast_possible_truncation)]
            self.regs()
                .i2cc0c()
                .write(|w| unsafe { w.tx_data_byte_count().bits((to_write - 1) as u8) });

            // Arm TX via FIFO and keep RX DMA armed in one atomic i2cs28 write.
            let mut cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            cmd |= constants::AST_I2CS_TX_BUFF_EN | AST_I2CS_RX_DMA_EN;
            unsafe {
                self.regs().i2cs28().write(|w| w.bits(cmd));
            }

            Ok(to_write)
        } else {
            // byte mode
            let cmd = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_TX_CMD;
            unsafe {
                self.regs()
                    .i2cc08()
                    .write(|w| w.tx_byte_buffer().bits(data[0]));
                self.regs().i2cs28().write(|w| w.bits(cmd));
            }
            self.clear_slave_interrupts();

            Ok(1)
        }
    }

    /// Handle slave mode interrupt
    #[allow(clippy::too_many_lines)]
    pub fn handle_slave_interrupt(&mut self) -> Option<SlaveEvent> {
        let status = self.regs().i2cs24().read().bits();

        if status == 0 {
            // Master status register i2cm14 retains bits after a master operation
            // and keeps the shared IRQ line asserted. Clear it here to stop the storm.
            let m14 = self.regs().i2cm14().read().bits();
            if m14 != 0 {
                unsafe {
                    self.regs().i2cm14().write(|w| w.bits(m14));
                }
            }
            return None;
        }

        // A slave inactive timeout drops the RX arming, so restart slave mode
        // and re-arm receive. A timeout also sets PKT_ERROR, so this check
        // comes first.
        if (status & constants::AST_I2CS_INACTIVE_TO) != 0 {
            self.regs()
                .i2cc00()
                .modify(|_, w| w.enbl_slave_fn().clear_bit());
            let mut cmd: u32 = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            self.arm_slave_receive(&mut cmd);
            self.regs()
                .i2cc00()
                .modify(|_, w| w.enbl_slave_fn().set_bit());
            // SAFETY: `cmd` (built above) sets only writable I2CS28 bits.
            unsafe {
                self.regs().i2cs28().write(|w| w.bits(cmd));
            }
            // Slave mode requires the command before the status clear.
            self.clear_slave_interrupts();
            self.enable_slave_interrupts();
            return None;
        }

        // Check for errors first
        if (status & constants::AST_I2CS_PKT_ERROR) != 0 {
            self.clear_slave_interrupts();
            return None;
        }

        // remove unnecessary bits from the status
        let status =
            status & !(constants::AST_I2CS_SLAVE_PENDING | constants::AST_I2CS_SLAVE_ADDR1_NAK);

        if (status & constants::AST_I2CS_PKT_DONE) != 0 {
            let mut cmd: u32 = constants::AST_I2CS_ACTIVE_ALL | constants::AST_I2CS_PKT_MODE_EN;
            // Parked waiting for an RX buffer: hold the bus. Masking keeps the
            // level-triggered IRQ quiet without clearing the status, and only
            // `rearm_slave_receive` arms again — one consumer drain, one frame.
            if (status & constants::AST_I2CS_WAIT_RX_DMA) != 0 {
                unsafe {
                    self.regs()
                        .i2cs20()
                        .write(|w| w.bits(constants::AST_I2CS_INACTIVE_TO));
                }
                // Without STOP nothing has completed: this is the address phase,
                // and the arm the consumer just spent went on its 1-byte address
                // packet. The data needs an arm of its own. That is a
                // continuation of the frame already paid for, not a new one.
                if (status & constants::AST_I2CS_STOP) == 0 {
                    if self.slave_rx_credit {
                        let enable = if self.xfer_mode == I2cXferMode::DmaMode
                            && self.slave_dma_buf.is_some()
                        {
                            AST_I2CS_RX_DMA_EN
                        } else {
                            constants::AST_I2CS_RX_BUFF_EN
                        };
                        // SAFETY: PKT_DONE is write-1-to-clear; the I2CS28 write
                        // is W1T asserting a single command bit.
                        unsafe {
                            self.regs()
                                .i2cs24()
                                .write(|w| w.bits(constants::AST_I2CS_PKT_DONE));
                            self.regs()
                                .i2cs28()
                                .write(|w| w.bits(AST_I2CS_CMD_W1T | enable));
                        }
                        self.slave_rx_credit = false;
                        self.enable_slave_interrupts();
                    }
                    return None;
                }
            } else {
                unsafe {
                    self.regs()
                        .i2cs24()
                        .write(|w| w.bits(constants::AST_I2CS_PKT_DONE));
                }
            }
            // A frame ended here, so any unspent grant expires with it: the next
            // frame has to be paid for by its own drain.
            if (status & constants::AST_I2CS_STOP) != 0 {
                self.slave_rx_credit = false;
            }
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
                // S: Sw — address phase with no STOP, already held above.
                return None;
            } else if sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_STOP {
                // S: Sw|P
                self.arm_slave_receive(&mut cmd);
                unsafe {
                    self.regs().i2cs28().write(|w| w.bits(cmd));
                }
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
                    tx_len: usize::from(
                        self.regs().i2cc0c().read().tx_data_byte_count().bits() + 1,
                    ),
                });
            } else if sts == constants::AST_I2CS_SLAVE_MATCH | constants::AST_I2CS_WAIT_TX_DMA {
                // S: Sw | wait_tx
                return Some(SlaveEvent::DataSent {
                    len: usize::from(self.regs().i2cc0c().read().tx_data_byte_count().bits() + 1),
                });
            } else if sts == constants::AST_I2CS_WAIT_TX_DMA {
                // S: wait_tx
                return Some(SlaveEvent::DataSent {
                    len: usize::from(self.regs().i2cc0c().read().tx_data_byte_count().bits() + 1),
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
                unsafe {
                    self.regs().i2cs28().write(|w| w.bits(cmd));
                }
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
                let _byte_data = self.regs().i2cc08().read().rx_byte_buffer().bits();
                self.regs().i2cs28().write(|w| unsafe { w.bits(cmd) });
                self.regs().i2cs24().write(|w| unsafe { w.bits(status) });
                self.regs().i2cs24().read().bits();
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
                let _byte_data = self.regs().i2cc08().read().rx_byte_buffer().bits();
                self.regs().i2cs28().write(|w| unsafe { w.bits(cmd) });
                self.regs().i2cs24().write(|w| unsafe { w.bits(status) });
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
                let _byte_data = self.regs().i2cc08().read().rx_byte_buffer().bits();
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
                self.regs().i2cs28().write(|w| unsafe { w.bits(cmd) });
                self.regs().i2cs24().write(|w| unsafe { w.bits(status) });
                return Some(SlaveEvent::Stop);
            }
            // TODO byte slave sts
        }
        None
    }
}
