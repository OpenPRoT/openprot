// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Low-level register access
//!
//! Consolidates all unsafe hardware register access into a single unsafe
//! perimeter, following the AST1060 PAC guide pattern.
//!
//! # Phase 5: Topology Logic Boundary
//!
//! **This trait is pure register abstraction. No topology logic belongs here.**
//!
//! All decisions about when to call these operations, how to interpret results
//! based on controller role, and topology-gated behaviors (decode-range sizing,
//! calibration skip, control register programming per role) live in the
//! controller layer (`controller.rs`), not in backends or this trait.
//!
//! Backends are transport only; they answer "how to read/write hardware registers."
//! The controller layer answers "what to do with the data based on the topology."

use ast1060_pac as device;
use core::marker::PhantomData;

use crate::smc::helpers::{
    SPI_DMA_DISCARD_REQ_MAGIC, SPI_DMA_GET_REQ_MAGIC, SPI_DMA_GRANT, SPI_DMA_REQUEST,
};

/// Safe wrapper around SMC hardware registers
///
/// This struct consolidates all unsafe hardware access. All register operations
/// go through this single point, making it easy to audit safety invariants.
///
/// Register naming follows AST1060 PAC convention: methods are named by their
/// hex offsets (e.g., `fmc000()` for offset 0x00, `fmc080()` for offset 0x80).
pub struct SmcRegisters {
    base: *const device::fmc::RegisterBlock,
    // `*const ()` marker keeps the handle `!Send` and `!Sync`. An `SmcRegisters`
    // represents exclusive ownership of one hardware controller; it must not be
    // shared between threads or moved into another execution context (e.g. an
    // ISR) where it could alias the controller it owns.
    _not_send_sync: PhantomData<*const ()>,
}

impl SmcRegisters {
    /// Create a new register accessor
    ///
    /// # Safety
    /// Caller must ensure:
    /// - `base` points to a valid FMC register block
    /// - Only one SmcRegisters instance exists per register block
    /// - Caller maintains exclusive access (no concurrent mutations)
    pub const unsafe fn new(base: *const device::fmc::RegisterBlock) -> Self {
        Self {
            base,
            _not_send_sync: PhantomData,
        }
    }

    /// Access the register block (single consolidation point for unsafe)
    ///
    /// # Safety
    /// Constructor must have ensured pointer validity and single ownership
    #[inline]
    fn regs(&self) -> &device::fmc::RegisterBlock {
        // SAFETY: Constructor ensures pointer validity and exclusive access.
        // Only one SmcRegisters instance can be created per hardware controller.
        unsafe { &*self.base }
    }

    // ====== Safe read/write wrappers (FMC register methods by offset) ======

    /// FMC000: Configuration register
    pub fn read_config(&self) -> u32 {
        self.regs().fmc000().read().bits()
    }

    pub fn write_config(&self, value: u32) {
        // SAFETY: Value always valid; PAC ensures register width.
        self.regs().fmc000().write(|w| unsafe { w.bits(value) });
    }

    pub fn modify_config<F>(&self, f: F)
    where
        F: FnOnce(&mut u32),
    {
        self.regs().fmc000().modify(|r, w| {
            let mut bits = r.bits();
            f(&mut bits);
            // SAFETY: Caller's function produces valid bits.
            unsafe { w.bits(bits) }
        });
    }

    /// FMC004: 4-byte mode and address width control
    pub fn read_addr_width(&self) -> u32 {
        self.regs().fmc004().read().bits()
    }

    pub fn write_addr_width(&self, value: u32) {
        self.regs().fmc004().write(|w| unsafe { w.bits(value) });
    }

    /// FMC008: DMA status
    pub fn read_dma_status(&self) -> u32 {
        self.regs().fmc008().read().bits()
    }

    /// FMC008: Clear DMA status bits (write-1-to-clear).
    pub fn clear_dma_status(&self, clear_mask: u32) {
        self.regs()
            .fmc008()
            .write(|w| unsafe { w.bits(clear_mask) });
    }

    /// FMC008: Enable DMA interrupt (bit 3, `dmaintenbl`).
    ///
    /// Call at end of each DMA launch, not at init time.
    pub fn enable_dma_irq(&self) {
        self.regs().fmc008().modify(|_, w| w.dmaintenbl().set_bit());
    }

    /// FMC008: Disable DMA interrupt (bit 3, `dmaintenbl`).
    ///
    /// Call at the top of the IRQ handler before processing status bits.
    pub fn disable_dma_irq(&self) {
        self.regs()
            .fmc008()
            .modify(|_, w| w.dmaintenbl().clear_bit());
    }

    /// FMC010: CS0 control register
    pub fn read_cs0_ctrl(&self) -> u32 {
        self.regs().fmc010().read().bits()
    }

    pub fn write_cs0_ctrl(&self, value: u32) {
        self.regs().fmc010().write(|w| unsafe { w.bits(value) });
    }

    /// FMC014: CS1 control register
    pub fn read_cs1_ctrl(&self) -> u32 {
        self.regs().fmc014().read().bits()
    }

    pub fn write_cs1_ctrl(&self, value: u32) {
        self.regs().fmc014().write(|w| unsafe { w.bits(value) });
    }

    /// Dispatch CS control register read by index.
    pub fn read_cs_ctrl(&self, cs: crate::smc::types::ChipSelect) -> u32 {
        match cs {
            crate::smc::types::ChipSelect::Cs0 => self.read_cs0_ctrl(),
            crate::smc::types::ChipSelect::Cs1 => self.read_cs1_ctrl(),
        }
    }

    /// Dispatch CS control register write by index.
    pub fn write_cs_ctrl(&self, cs: crate::smc::types::ChipSelect, value: u32) {
        match cs {
            crate::smc::types::ChipSelect::Cs0 => self.write_cs0_ctrl(value),
            crate::smc::types::ChipSelect::Cs1 => self.write_cs1_ctrl(value),
        }
    }

    /// FMC030: CS0 segment register (memory mapping)
    pub fn read_cs0_segment(&self) -> u32 {
        self.regs().fmc030().read().bits()
    }

    pub fn write_cs0_segment(&self, value: u32) {
        self.regs().fmc030().write(|w| unsafe { w.bits(value) });
    }

    /// FMC034: CS1 segment register (memory mapping)
    pub fn read_cs1_segment(&self) -> u32 {
        self.regs().fmc034().read().bits()
    }

    pub fn write_cs1_segment(&self, value: u32) {
        self.regs().fmc034().write(|w| unsafe { w.bits(value) });
    }

    /// FMC06C: SPI I/O mode register
    pub fn read_spi_mode(&self) -> u32 {
        self.regs().fmc06c().read().bits()
    }

    pub fn write_spi_mode(&self, value: u32) {
        self.regs().fmc06c().write(|w| unsafe { w.bits(value) });
    }

    pub fn modify_spi_mode<F>(&self, f: F)
    where
        F: FnOnce(&mut u32),
    {
        self.regs().fmc06c().modify(|r, w| {
            let mut bits = r.bits();
            f(&mut bits);
            // SAFETY: Caller's function produces valid bits.
            unsafe { w.bits(bits) }
        });
    }

    /// FMC080: DMA control register
    pub fn read_dma_ctrl(&self) -> u32 {
        self.regs().fmc080().read().bits()
    }

    pub fn write_dma_ctrl(&self, value: u32) {
        self.regs().fmc080().write(|w| unsafe { w.bits(value) });
    }

    /// FMC080: Disable DMA — full two-write sequence matching aspeed-rust `dma_disable()`.
    ///
    /// 1. Write `0x0` — deasserts `DMAEnbl` and `DMADirection`.
    /// 2. Write `SPI_DMA_DISCARD_REQ_MAGIC` — releases DMA bus grant on SPI1/SPI2.
    ///    On FMC, bits 20–31 are Reserved; the second write is a no-op.
    pub fn disable_dma(&self) {
        self.regs().fmc080().write(|w| unsafe { w.bits(0x0) });
        self.write_dma_ctrl(SPI_DMA_DISCARD_REQ_MAGIC);
    }

    /// FMC080/SPI080: Acquire the DMA bus arbiter before programming DMA registers.
    ///
    /// Writes `SPI_DMA_GET_REQ_MAGIC` (0xaeed_0000) to assert `DMAReq` (bit 31),
    /// then spins until `DMAGrant` (bit 30) is set.
    ///
    /// On FMC, bits 20–31 of `fmc080` are Reserved (AST1060 PAC: `Reserved0R`).
    /// The write is ignored and bit 31 reads back 0, so the spin condition is
    /// immediately false — no loop is entered. This method is safe to call on
    /// all three controllers without branching on `controller_id`.
    ///
    /// Matches aspeed-rust `spicontroller.rs::read_dma` arbitration sequence.
    pub fn acquire_dma_arbiter(&self) {
        self.write_dma_ctrl(SPI_DMA_GET_REQ_MAGIC);
        if self.read_dma_ctrl() & SPI_DMA_REQUEST != 0 {
            while self.read_dma_ctrl() & SPI_DMA_GRANT == 0 {}
        }
    }

    /// FMC084: DMA flash side start address (`R_DMA_FLASH_ADDR`).
    ///
    /// Receives the flash-side byte offset of the DMA source window.
    /// Matches aspeed-rust `fmc084` / QEMU `R_DMA_FLASH_ADDR`.
    pub fn read_dma_flash_addr(&self) -> u32 {
        self.regs().fmc084().read().bits()
    }

    pub fn write_dma_flash_addr(&self, value: u32) {
        self.regs().fmc084().write(|w| unsafe { w.bits(value) });
    }

    /// FMC088: DMA DRAM/SRAM side start address (`R_DMA_DRAM_ADDR`).
    ///
    /// Receives the DRAM/SRAM destination address for the DMA transfer.
    /// Matches aspeed-rust `fmc088` / QEMU `R_DMA_DRAM_ADDR`.
    pub fn read_dma_dram_addr(&self) -> u32 {
        self.regs().fmc088().read().bits()
    }

    pub fn write_dma_dram_addr(&self, value: u32) {
        self.regs().fmc088().write(|w| unsafe { w.bits(value) });
    }

    /// FMC08C: DMA length register (`R_DMA_LEN`).
    ///
    /// Receives `transfer_length - 1`.
    /// Matches aspeed-rust `fmc08c` / QEMU `R_DMA_LEN`.
    pub fn write_dma_len(&self, value: u32) {
        self.regs().fmc08c().write(|w| unsafe { w.bits(value) });
    }

    /// FMC080: Start a DMA read (flash → DRAM).
    ///
    /// Uses read-modify-write to preserve timing calibration fields (bits 8–19)
    /// while setting `DMAEnbl = 1` and `DMADirection = Read`.
    /// Matches aspeed-rust `fmccontroller.rs::read_dma` kick sequence.
    pub fn kick_dma_read(&self) {
        self.regs().fmc080().modify(|_, w| {
            w.dmaenbl().enable_dma_operation();
            w.dmadirection()
                .read_flash_move_from_flash_to_external_memory()
        });
    }

    /// FMC090: DMA checksum (CRC)
    pub fn read_dma_checksum(&self) -> u32 {
        self.regs().fmc090().read().bits()
    }

    /// FMC094: CS0 calibration status
    pub fn read_cs0_timing_compensation(&self) -> u32 {
        self.regs().fmc094().read().bits()
    }

    /// FMC094: CS0 calibration status
    pub fn write_cs0_timing_compensation(&self, value: u32) {
        self.regs().fmc094().write(|w| unsafe { w.bits(value) });
    }

    /// FMC098: CS1 calibration status
    pub fn read_cs1_timing_compensation(&self) -> u32 {
        self.regs().fmc098().read().bits()
    }

    /// FMC098: CS0 calibration status
    pub fn write_cs1_timing_compensation(&self, value: u32) {
        self.regs().fmc098().write(|w| unsafe { w.bits(value) });
    }

    pub fn write_cs_timing_compensation(&self, cs: crate::smc::types::ChipSelect, value: u32) {
        match cs {
            crate::smc::types::ChipSelect::Cs0 => self.write_cs0_timing_compensation(value),
            crate::smc::types::ChipSelect::Cs1 => self.write_cs1_timing_compensation(value),
        }
    }

    pub fn already_calibrated(&self, cs: crate::smc::types::ChipSelect) -> bool {
        match cs {
            crate::smc::types::ChipSelect::Cs0 => self.read_cs0_timing_compensation() != 0,

            crate::smc::types::ChipSelect::Cs1 => self.read_cs1_timing_compensation() != 0,
        }
    }
}
