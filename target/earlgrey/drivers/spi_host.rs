// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI Host driver for Earlgrey.
//!
//! This driver implements the `embedded-hal` 1.0 SPI traits for the Earlgrey SPI Host controller.

#![no_std]

use util_error::{
    ErrorCode, SPI_GENERIC_FIFO_OVERFLOW, SPI_GENERIC_FIFO_UNDERFLOW, SPI_GENERIC_HARDWARE_ERROR,
    SPI_GENERIC_INVALID_TRANSACTION, SPI_GENERIC_TIMEOUT,
};

/// SPI Host driver error wrapping an [`ErrorCode`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SpiError(pub ErrorCode);

impl SpiError {
    /// Invalid transaction parameters or state.
    pub const INVALID_TRANSACTION: Self = Self(SPI_GENERIC_INVALID_TRANSACTION);
    /// TX FIFO overflow.
    pub const FIFO_OVERFLOW: Self = Self(SPI_GENERIC_FIFO_OVERFLOW);
    /// RX FIFO underflow.
    pub const FIFO_UNDERFLOW: Self = Self(SPI_GENERIC_FIFO_UNDERFLOW);
    /// Operation timed out.
    pub const TIMEOUT: Self = Self(SPI_GENERIC_TIMEOUT);
    /// Hardware error reported by the controller.
    pub const HARDWARE_ERROR: Self = Self(SPI_GENERIC_HARDWARE_ERROR);
}

impl embedded_hal::spi::Error for SpiError {
    fn kind(&self) -> embedded_hal::spi::ErrorKind {
        // Map all errors to Other for now.
        embedded_hal::spi::ErrorKind::Other
    }
}

impl From<ErrorCode> for SpiError {
    fn from(err: ErrorCode) -> Self {
        SpiError(err)
    }
}

impl From<SpiError> for ErrorCode {
    fn from(err: SpiError) -> Self {
        err.0
    }
}

/// Configuration for the SPI Host driver.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SpiConfig {
    /// Clock divider to adjust transfer speed.
    /// Formula: f_sck = f_core / (2 * (clkdiv + 1))
    pub clkdiv: u32,
    /// SPI clock polarity (false for low when idle, true for high when idle).
    pub cpol: bool,
    /// SPI clock phase (false to sample on first edge, true on second edge).
    pub cpha: bool,
    /// Chip Select ID line (0 for CS0, 1 for CS1, etc.)
    pub csid: u32,
}

impl SpiConfig {
    /// Default configuration for SPI Host 0 (SPI0).
    /// SPI0 clock domain is 96MHz. clkdiv 1 provides 24MHz SCK.
    pub const DEFAULT_SPI0: Self = Self {
        clkdiv: 1,
        cpol: false,
        cpha: false,
        csid: 0,
    };

    /// Default configuration for SPI Host 1 (SPI1).
    /// SPI1 clock domain is 48MHz. clkdiv 0 provides 24MHz SCK.
    pub const DEFAULT_SPI1: Self = Self {
        clkdiv: 0,
        cpol: false,
        cpha: false,
        csid: 0,
    };
}

/// Earlgrey SPI Host driver.
pub struct SpiHost {
    mmio: spi_host::RegisterBlock<ureg::RealMmioMut<'static>>,
}

impl SpiHost {
    /// Create a new SpiHost instance using the raw register block.
    ///
    /// # Safety
    ///
    /// The caller must ensure they have exclusive access to the SpiHost peripheral registers
    /// represented by the MMIO block.
    pub unsafe fn new(mmio: spi_host::RegisterBlock<ureg::RealMmioMut<'static>>) -> Self {
        Self { mmio }
    }

    /// Dynamically reconfigure the SPI Host speed, mode, and chip select.
    pub fn configure(&mut self, config: &SpiConfig) -> Result<(), SpiError> {
        self.mmio.configopts().write(|w| {
            w.clkdiv(config.clkdiv)
                .cpol(config.cpol)
                .cpha(config.cpha)
                .fullcyc(true)
                .csnlead(0)
                .csnidle(2)
                .csntrail(1)
        });
        self.mmio.csid().write(|_| config.csid);
        Ok(())
    }

    /// Initialize the SPI Host peripheral.
    ///
    /// Configures the clock, SPI mode (0), CS timings, performs a reset,
    /// and enables the peripheral.
    pub fn init(&mut self, config: &SpiConfig) -> Result<(), SpiError> {
        self.configure(config)?;
        self.mmio.control().write(|w| w.sw_rst(true));
        loop {
            let status = self.mmio.status().read();
            if status.txempty() && status.rxempty() {
                break;
            }
        }
        self.mmio.control().write(|w| w.sw_rst(false).spien(true));
        self.mmio.csid().write(|_| config.csid);

        Ok(())
    }

    fn is_active(&self) -> bool {
        self.mmio.status().read().active()
    }

    fn is_ready(&self) -> bool {
        let status = self.mmio.status().read();
        status.ready() && !status.active()
    }

    /// Send a write command segment.
    fn write_cmd(
        &mut self,
        mut data: &[u8],
        speed: u32,
        final_csaat: bool,
    ) -> Result<(), SpiError> {
        self.mmio.control().modify(|w| w.output_en(true));

        while !data.is_empty() {
            while !self.is_ready() {
                // TODO: we should block here for an interrupt that would signal the hardware is ready.
            }

            let chunk_len = core::cmp::min(data.len(), MAX_TX_CHUNK_LEN);
            let (chunk, rest) = data.split_at(chunk_len);
            data = rest;

            util_regcpy::copy_to_reg_unaligned(&self.mmio.txdata(), chunk);

            let chunk_csaat = !data.is_empty() || final_csaat;

            self.mmio.command().write(|w| {
                w.speed(speed)
                    .csaat(chunk_csaat)
                    .direction(DIR_TXONLY)
                    .len(chunk_len.saturating_sub(1) as u32)
            });
        }
        Ok(())
    }

    /// Send a read command segment and receive data into `dest`.
    fn read_cmd(
        &mut self,
        mut len: usize,
        speed: u32,
        final_csaat: bool,
        mut dest: &mut [u8],
    ) -> Result<(), SpiError> {
        self.mmio.control().modify(|w| w.output_en(true));

        while len > 0 {
            while !self.is_ready() {
                // TODO: we should block here for an interrupt that would signal the hardware is ready.
            }

            let chunk_len = core::cmp::min(len, MAX_RX_CHUNK_LEN);
            len -= chunk_len;

            let chunk_csaat = len > 0 || final_csaat;

            self.mmio.command().write(|w| {
                w.speed(speed)
                    .csaat(chunk_csaat)
                    .direction(DIR_RXONLY)
                    .len(chunk_len.saturating_sub(1) as u32)
            });

            if chunk_len > dest.len() {
                return Err(SpiError::INVALID_TRANSACTION);
            }
            // Split dest into the chunk we want to read now, and the rest for later.
            let (mut chunk_dest, rest) = dest.split_at_mut(chunk_len);
            dest = rest;

            while !chunk_dest.is_empty() {
                let status = self.mmio.status().read();
                let rxqd = status.rxqd() as usize; // rxqd is in 32-bit words
                let bytes_in_fifo = rxqd * 4;

                if bytes_in_fifo > 0 {
                    let drain_len = core::cmp::min(bytes_in_fifo, chunk_dest.len());
                    let (drain_chunk, remaining) = chunk_dest.split_at_mut(drain_len);
                    util_regcpy::copy_from_reg_unaligned(drain_chunk, &self.mmio.rxdata());
                    chunk_dest = remaining;
                }
            }
        }
        Ok(())
    }
}

const MAX_RX_CHUNK_LEN: usize = 256;
const MAX_TX_CHUNK_LEN: usize = 288;
const DIR_RXONLY: u32 = 1;
const DIR_TXONLY: u32 = 2;

impl embedded_hal::spi::ErrorType for SpiHost {
    type Error = SpiError;
}

impl embedded_hal::spi::SpiDevice for SpiHost {
    fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        let op_count = operations.len();
        for (i, op) in operations.iter_mut().enumerate() {
            let is_last = i == op_count - 1;
            let csaat = !is_last;

            match op {
                embedded_hal::spi::Operation::Read(buf) => {
                    self.read_cmd(buf.len(), 0, csaat, buf)?;
                }
                embedded_hal::spi::Operation::Write(buf) => {
                    self.write_cmd(buf, 0, csaat)?;
                }
                embedded_hal::spi::Operation::Transfer(_, _)
                | embedded_hal::spi::Operation::TransferInPlace(_) => {
                    // Full-duplex SPI transfer is not supported by this simple driver yet.
                    return Err(SpiError::INVALID_TRANSACTION);
                }
                embedded_hal::spi::Operation::DelayNs(_) => {
                    // Delay operation is not supported.
                    return Err(SpiError::INVALID_TRANSACTION);
                }
            }
        }

        // Wait for the controller to finish the last command segment.
        while self.is_active() {}
        // Disable output buffers to release the bus.
        self.mmio.control().modify(|w| w.output_en(false));

        Ok(())
    }
}
