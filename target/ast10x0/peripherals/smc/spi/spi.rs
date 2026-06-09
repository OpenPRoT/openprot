// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI1/SPI2-specialized wrapper around the generic SMC controller.
//!
//! # Architecture: Wrapper vs. Controller
//!
//! This module provides an ergonomic construction API for SPI1/SPI2 controllers.
//! **All topology-aware behavior gating (decode-range sizing, calibration skip,
//! role-specific control programming) lives in the generic controller layer**
//! (`controller.rs`), not here.
//!
//! The wrapper's role is **construction and delegation only**:
//! - `SpiUninit::new()`: Type-check that controller_id is SPI1 or SPI2 (not FMC)
//! - `SpiReady` methods: Delegate to inner controller operations
//!
//! For BootSpi (FMC), use `Smc<FmcRegisterBackend>` directly.
//! For HostSpi/NormalSpi (SPI1/SPI2), use this wrapper.

use crate::smc::controller::{ReadySmc, UninitSmc};
use crate::smc::interrupts::SmcInterrupt;
use crate::smc::types::{
    ChipSelect, FlashConfig, SmcConfig, SmcController, SmcError, TransferMode,
};

/// SPI handle before hardware initialization.
pub struct SpiUninit {
    inner: UninitSmc,
}

/// SPI handle after hardware initialization.
///
/// # Topology Gating
///
/// This wrapper delegates all operations to the inner controller. Topology-specific
/// behavior (decode-range sizing, calibration skip, role-dependent control register
/// programming) is handled by the controller layer, not here. This keeps the wrapper
/// thin and the topology logic centralized.
///
/// # Example
///
/// ```ignore
/// let mut spi = unsafe { SpiUninit::new(SmcController::Spi1, config)? };
/// let spi = spi.init()?;  // Topology-gated behaviors in controller.init()
/// spi.read(0, &mut buf)?;
/// ```
pub struct SpiReady {
    inner: ReadySmc,
}

impl SpiUninit {
    /// Construct an uninitialized SPI controller for SPI1 or SPI2.
    ///
    /// # Topology Requirements
    ///
    /// The SPI wrapper is for HostSpi and NormalSpi topologies only.
    /// BootSpi (FMC) should use the generic Smc<FmcRegisterBackend> directly.
    ///
    /// # Safety
    /// Caller must ensure unique ownership of the selected SPI hardware block.
    pub unsafe fn new(
        controller_id: SmcController,
        mut config: SmcConfig,
    ) -> Result<Self, SmcError> {
        // Phase 3: Topology-aware SPI construction check.
        //
        // The SPI wrapper is specialized for HostSpi and NormalSpi topologies.
        // FMC (BootSpi topology) uses the generic controller with FmcRegisterBackend.
        // This check enforces that constraint at construction time.
        match controller_id {
            SmcController::Fmc => return Err(SmcError::InvalidChipSelect),
            SmcController::Spi1 | SmcController::Spi2 => {}
        }

        config.controller_id = controller_id;
        // SAFETY: Caller upholds controller ownership requirements.
        let inner = unsafe { UninitSmc::new(config)? };
        Ok(Self { inner })
    }

    /// Initialize SPI hardware and transition to ready state.
    pub fn init(self) -> Result<SpiReady, SmcError> {
        Ok(SpiReady {
            inner: self.inner.init()?,
        })
    }
}

impl SpiReady {
    /// Perform a programmed I/O read via the SPI flash window.
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

    /// Get the controller identifier.
    pub fn controller_id(&self) -> SmcController {
        self.inner.controller_id()
    }

    /// Get the configured master ID for this controller topology.
    pub fn master_idx(&self) -> u8 {
        self.inner.master_idx()
    }

    /// Check if SPI controller is ready for operations.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Program memory-mapped SPI NOR read mode for the selected chip select.
    pub fn spi_nor_read_init(&mut self, cs: ChipSelect) -> Result<(), SmcError> {
        self.inner.spi_nor_read_init(cs)
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

    /// Execute a raw user-mode SPI transfer on the selected SPI chip select.
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
