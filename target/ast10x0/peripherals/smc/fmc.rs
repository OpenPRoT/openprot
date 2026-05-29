// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! FMC-specialized wrapper around the generic SMC controller.
//!
//! # FMC vs SPI1/SPI2
//!
//! This module provides an abstraction **exclusively for the FMC (Firmware Memory Controller)**
//! block. The FMC connects directly to flash without SPI-monitor interception.
//!
//! **SPI1 and SPI2** (application SPI controllers that route through SPIPF monitor) are
//! handled by the separate [`crate::smc::spi`] module, not here.
//!
//! - **FMC**: Single dedicated flash controller, no SPI-monitor support, boot-time device
//! - **SPI1/SPI2**: Multi-instance application controllers with optional SPIPF monitoring
//!
//! See [`crate::smc`] module-level documentation for the full taxonomy.

use crate::smc::controller::{ReadySmc, UninitSmc};
use crate::smc::interrupts::SmcInterrupt;
use crate::smc::types::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, TransferMode,
};

/// FMC handle before hardware initialization.
pub struct FmcUninit {
    inner: UninitSmc,
}

/// FMC handle after hardware initialization.
pub struct FmcReady {
    inner: ReadySmc,
}

impl FmcUninit {
    /// Construct an uninitialized FMC controller.
    ///
    /// # Safety
    /// Caller must ensure unique ownership of the FMC hardware block.
    pub unsafe fn new(mut config: SmcConfig) -> Result<Self, SmcError> {
        config.controller_id = SmcController::Fmc;
        // SAFETY: Caller upholds controller ownership requirements.
        let inner = unsafe { UninitSmc::new(config)? };
        Ok(Self { inner })
    }

    /// Initialize FMC hardware and transition to ready state.
    pub fn init(self) -> Result<FmcReady, SmcError> {
        Ok(FmcReady {
            inner: self.inner.init()?,
        })
    }
}

impl FmcReady {
    /// Perform a programmed I/O read via the FMC flash window.
    pub fn read(&self, cs: ChipSelect, offset: u32, buf: &mut [u8]) -> Result<usize, SmcError> {
        self.inner.read(cs, offset, buf)
    }

    /// Initiate a DMA read operation.
    pub fn dma_read(
        &mut self,
        cs: ChipSelect,
        flash_offset: u32,
        dram_addr: usize,
        len: u32,
    ) -> Result<(), SmcError> {
        self.inner.dma_read(cs, flash_offset, dram_addr, len)
    }

    /// Read raw DMA/interrupt status bits from FMC008.
    pub fn dma_status(&self) -> u32 {
        self.inner.dma_status()
    }

    /// Clear DMA-related status bits in FMC008 (write-1-to-clear).
    pub fn clear_dma_status(&self, clear_mask: u32) {
        self.inner.clear_dma_status(clear_mask)
    }

    /// Handle DMA completion/error from IRQ status and finalize controller state.
    pub fn handle_dma_irq(&mut self) -> Result<SmcInterrupt, SmcError> {
        self.inner.handle_dma_irq()
    }

    /// Poll for DMA completion without requiring an IRQ. See `Smc::poll_dma_completion`.
    pub fn poll_dma_completion(&mut self) -> core::task::Poll<Result<(), SmcError>> {
        self.inner.poll_dma_completion()
    }

    /// Check if FMC is ready for operations.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Program memory-mapped SPI NOR read mode for the selected chip select.
    pub fn spi_nor_read_init(&mut self, cs: ChipSelect) -> Result<(), SmcError> {
        self.inner.spi_nor_read_init(cs)
    }

    #[doc(hidden)]
    pub fn test_force_dma_in_flight(&mut self) {
        self.inner.test_force_dma_in_flight();
    }

    /// Return configured flash capacity in bytes.
    pub fn capacity_bytes(&self) -> Result<usize, SmcError> {
        self.inner.capacity_bytes()
    }

    /// Return configured flash capacity in bytes for the given chip select.
    pub fn cs_capacity_bytes(&self, cs: ChipSelect) -> Result<usize, SmcError> {
        self.inner.cs_capacity_bytes(cs)
    }

    /// Return the configured `FlashConfig` for the requested chip select.
    pub fn cs_config(&self, cs: ChipSelect) -> Result<FlashConfig, SmcError> {
        self.inner.cs_config(cs)
    }

    /// Execute a raw user-mode SPI transfer on the selected FMC chip select.
    ///
    /// `cs` selects CS0 or CS1; `mode` controls the per-phase IO width.
    /// Returns `SmcError::InvalidChipSelect` if CS1 is requested but not configured.
    pub fn transceive_user(
        &self,
        cs: ChipSelect,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        self.inner.transceive_user(cs, cmd, tx_payload, rx, mode)
    }

    /// Convenience wrapper: execute a user-mode transfer on CS0.
    pub fn transceive_user_cs0(
        &self,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        self.inner
            .transceive_user(ChipSelect::Cs0, cmd, tx_payload, rx, mode)
    }

    /// Access the underlying generic ready controller.
    pub fn as_inner(&self) -> &ReadySmc {
        &self.inner
    }

    /// Mutable access to the underlying generic ready controller.
    pub fn as_inner_mut(&mut self) -> &mut ReadySmc {
        &mut self.inner
    }
}
