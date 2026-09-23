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

use util_region::{Mmap, Region};

use crate::smc::controller::{Cs, ReadySmc, UninitSmc};
use crate::smc::types::{SmcController, SmcError, SmcInstance};

/// FMC handle before hardware initialization.
pub struct FmcUninit<I: SmcInstance> {
    inner: UninitSmc<I>,
}

/// FMC handle after hardware initialization.
///
/// Per-chip operations (read, transfer, DMA) are reached through a [`Cs`] handle
/// vended by [`FmcReady::cs0`] / [`FmcReady::cs1`]; the chip select is baked into
/// that handle rather than passed on every call.
pub struct FmcReady<I: SmcInstance> {
    inner: ReadySmc<I>,
}

impl<I: SmcInstance> FmcUninit<I> {
    /// Construct an uninitialized FMC controller from the regions mapped to it.
    pub fn new<Cs0, Cs1>(
        region: Region<I::Regs>,
        cs0_window: Region<Cs0>,
        cs1_window: Region<Cs1>,
    ) -> Result<Self, SmcError>
    where
        Cs0: Mmap,
        Cs1: Mmap,
    {
        const {
            assert!(
                matches!(I::CONTROLLER, SmcController::Fmc),
                "FmcUninit requires an SmcInstance whose CONTROLLER is Fmc"
            );
        }
        let inner = UninitSmc::<I>::new(region, cs0_window, cs1_window)?;
        Ok(Self { inner })
    }

    /// Initialize FMC hardware and transition to ready state.
    pub fn init(self) -> Result<FmcReady<I>, SmcError> {
        Ok(FmcReady {
            inner: self.inner.init()?,
        })
    }
}

impl<I: SmcInstance> FmcReady<I> {
    /// Build a handle for CS0 from its init-resolved geometry.
    pub fn cs0(&mut self) -> Result<Cs<'_>, SmcError> {
        self.inner.cs0()
    }

    /// Build a handle for CS1 from its init-resolved geometry.
    pub fn cs1(&mut self) -> Result<Cs<'_>, SmcError> {
        self.inner.cs1()
    }

    /// Check if FMC is ready for operations.
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    /// Get the controller identifier.
    pub fn controller_id(&self) -> SmcController {
        self.inner.controller_id()
    }

    #[doc(hidden)]
    pub fn test_force_dma_in_flight(&mut self) {
        self.inner.test_force_dma_in_flight();
    }
}
