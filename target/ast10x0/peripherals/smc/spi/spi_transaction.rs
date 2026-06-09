// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Scoped SPI transaction wrapper.

use core::task::Poll;

use super::spi::SpiReady;
use crate::scu::{ScuRegisters, SpiMonitorInstance, SpiMonitorSource, SpimGpioOriVal};
use crate::smc::interrupts::SmcInterrupt;
use crate::smc::types::{ChipSelect, SmcController, SmcError, TransferMode};

#[derive(Clone, Copy)]
struct SpiMuxState {
    spim_present: bool,
    internal_mux_active: bool,
    proprietary_state: Option<SpimGpioOriVal>,
}

/// In-flight SPI transaction state.
///
/// Synchronous helpers hide this state entirely. DMA returns it so the mux can
/// stay enabled until DMA completion.
pub struct SpiTransaction<'a> {
    spi: &'a mut SpiReady,
    previous_mux: Option<SpiMuxState>,
}

impl<'a> SpiTransaction<'a> {
    fn begin(spi: &'a mut SpiReady, spim: Option<SpiMonitorInstance>) -> Result<Self, SmcError> {
        let mut mux_state = SpiMuxState {
            spim_present: spim.is_some(),
            internal_mux_active: false,
            proprietary_state: None,
        };

        if let Some(spim) = spim {
            let scu = unsafe { ScuRegisters::new_global_unlocked() };
            scu.validate_spim_instance(spim)
                .map_err(|_| SmcError::HardwareError)?;

            if spi.master_idx() != 0 {
                scu.set_spim_internal_mux(Self::spim_source(spi)?, spim as u8 + 1)
                    .map_err(|_| SmcError::HardwareError)?;
                mux_state.internal_mux_active = true;
            }

            mux_state.proprietary_state = scu.spim_proprietary_pre_config();
        }

        Ok(Self {
            spi,
            previous_mux: Some(mux_state),
        })
    }

    pub fn read(
        spi: &'a mut SpiReady,
        cs: ChipSelect,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<usize, SmcError> {
        Self::read_with_spim(spi, None, cs, offset, buf)
    }

    pub fn read_with_spim(
        spi: &'a mut SpiReady,
        spim: impl Into<Option<SpiMonitorInstance>>,
        cs: ChipSelect,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<usize, SmcError> {
        let mut txn = Self::begin(spi, spim.into())?;
        let result = txn.spi.read(cs, offset, buf);
        txn.finish_result(result)
    }

    pub fn transceive_user(
        spi: &'a mut SpiReady,
        cs: ChipSelect,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        Self::transceive_user_with_spim(spi, None, cs, cmd, tx_payload, rx, mode)
    }

    pub fn transceive_user_with_spim(
        spi: &'a mut SpiReady,
        spim: impl Into<Option<SpiMonitorInstance>>,
        cs: ChipSelect,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        let mut txn = Self::begin(spi, spim.into())?;
        let result = txn.spi.transceive_user(cs, cmd, tx_payload, rx, mode);
        txn.finish_result(result)
    }

    pub fn dma_read(
        spi: &'a mut SpiReady,
        cs: ChipSelect,
        flash_offset: u32,
        dram_addr: usize,
        len: u32,
    ) -> Result<Self, SmcError> {
        Self::dma_read_with_spim(spi, None, cs, flash_offset, dram_addr, len)
    }

    pub fn dma_read_with_spim(
        spi: &'a mut SpiReady,
        spim: impl Into<Option<SpiMonitorInstance>>,
        cs: ChipSelect,
        flash_offset: u32,
        dram_addr: usize,
        len: u32,
    ) -> Result<Self, SmcError> {
        let txn = Self::begin(spi, spim.into())?;
        txn.spi.dma_read(cs, flash_offset, dram_addr, len)?;
        Ok(txn)
    }

    pub fn poll_dma_completion(&mut self) -> Poll<Result<(), SmcError>> {
        match self.spi.poll_dma_completion() {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => Poll::Ready(result.and_then(|_| self.finish_restore())),
        }
    }

    pub fn handle_dma_irq(&mut self) -> Result<SmcInterrupt, SmcError> {
        let interrupt = self.spi.handle_dma_irq()?;
        self.finish_restore()?;
        Ok(interrupt)
    }

    fn finish_result<T>(&mut self, result: Result<T, SmcError>) -> Result<T, SmcError> {
        let restore = self.finish_restore();
        match (result, restore) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), _) => Err(err),
            (Ok(_), Err(err)) => Err(err),
        }
    }

    fn finish_restore(&mut self) -> Result<(), SmcError> {
        if let Some(state) = self.previous_mux.take() {
            if state.spim_present {
                let scu = unsafe { ScuRegisters::new_global_unlocked() };

                if let Some(proprietary_state) = state.proprietary_state {
                    scu.spim_proprietary_post_config(proprietary_state);
                }

                if state.internal_mux_active {
                    scu.clear_spim_internal_master_route();
                }
            }
        }

        Ok(())
    }

    fn spim_source(spi: &SpiReady) -> Result<SpiMonitorSource, SmcError> {
        match spi.controller_id() {
            SmcController::Spi1 => Ok(SpiMonitorSource::Spi1),
            SmcController::Spi2 => Ok(SpiMonitorSource::Spi2),
            SmcController::Fmc => Err(SmcError::InvalidChipSelect),
        }
    }
}

impl Drop for SpiTransaction<'_> {
    fn drop(&mut self) {
        if let Some(state) = self.previous_mux.take() {
            if state.spim_present {
                let scu = unsafe { ScuRegisters::new_global_unlocked() };

                if let Some(proprietary_state) = state.proprietary_state {
                    scu.spim_proprietary_post_config(proprietary_state);
                }

                if state.internal_mux_active {
                    scu.clear_spim_internal_master_route();
                }
            }
        }
    }
}
