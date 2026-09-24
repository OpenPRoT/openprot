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
//! - `SpiReady::cs0` / `cs1`: Vend a per-chip [`Cs`] handle
//!
//! For BootSpi (FMC), use the [`crate::smc::fmc`] wrapper.
//! For HostSpi/NormalSpi (SPI1/SPI2), use this wrapper.

use util_region::{Mmap, Region};

use crate::smc::controller::{Cs, ReadySmc, UninitSmc};
use crate::smc::types::{SmcController, SmcError, SmcInstance};

/// SPI handle before hardware initialization.
pub struct SpiUninit<I: SmcInstance> {
    inner: UninitSmc<I>,
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
/// Per-chip operations (read, transfer, DMA) are reached through a [`Cs`] handle
/// vended by [`SpiReady::cs0`] / [`SpiReady::cs1`]; SPIM mux bracketing is layered
/// on top of that handle by [`crate::smc::spi::SpiTransaction`].
pub struct SpiReady<I: SmcInstance> {
    inner: ReadySmc<I>,
}

impl<I: SmcInstance> SpiUninit<I> {
    /// Construct an uninitialized SPI controller for SPI1 or SPI2 from the
    /// regions mapped to it.
    ///
    /// # Topology Requirements
    ///
    /// The SPI wrapper is for HostSpi and NormalSpi topologies only.
    /// BootSpi (FMC) should use the [`crate::smc::fmc`] wrapper.
    pub fn new<Cs0, Cs1>(
        region: Region<I::Regs>,
        cs0_window: Region<Cs0>,
        cs1_window: Region<Cs1>,
    ) -> Result<Self, SmcError>
    where
        Cs0: Mmap,
        Cs1: Mmap,
    {
        // The SPI wrapper is specialized for HostSpi and NormalSpi topologies.
        // FMC (BootSpi topology) uses the FMC wrapper. Enforce that here.
        const {
            assert!(
                !matches!(I::CONTROLLER, SmcController::Fmc),
                "SpiUninit requires an SmcInstance whose CONTROLLER is Spi1 or Spi2"
            );
        }

        let inner = UninitSmc::<I>::new(region, cs0_window, cs1_window)?;
        Ok(Self { inner })
    }

    /// Initialize SPI hardware and transition to ready state.
    pub fn init(self) -> Result<SpiReady<I>, SmcError> {
        Ok(SpiReady {
            inner: self.inner.init()?,
        })
    }
}

impl<I: SmcInstance> SpiReady<I> {
    /// Build a handle for CS0 from its init-resolved geometry.
    pub fn cs0(&mut self) -> Result<Cs<'_>, SmcError> {
        self.inner.cs0()
    }

    /// Build a handle for CS1 from its init-resolved geometry.
    pub fn cs1(&mut self) -> Result<Cs<'_>, SmcError> {
        self.inner.cs1()
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
}
