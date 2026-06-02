// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Generic SMC controller implementation

use core::cell::UnsafeCell;
use core::marker::PhantomData;

use crate::smc::helpers::{
    encode_fmc_segment, encode_spi_segment, flash_capacity_bytes, get_mid_point_of_longest_one,
    spi_calibration_enable, spi_freq_div, total_capacity_bytes, validate_dma_read,
    validate_mapped_range, SPI_CTRL_FREQ_MASK, SPI_DMA_CALC_CKSUM, SPI_DMA_CALIB_MODE,
    SPI_DMA_ENABLE, SPI_DMA_RAM_MAP_BASE,
};
use crate::smc::interrupts::{SmcInterrupt, SmcInterruptDecoder};
use crate::smc::registers::SmcRegisters;
use crate::smc::types::*;

/// Internal controller state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SmcState {
    /// Controller is initialized and idle — no operation in progress.
    Idle,
    /// A DMA transfer has been kicked and is in progress.
    DmaInFlight,
    /// Controller encountered an unrecoverable hardware fault.
    Faulted,
}

const ASPEED_SPI_USER: u32 = 0x3;
const ASPEED_SPI_USER_INACTIVE: u32 = 0x4;
const ASPEED_SPI_NORMAL_READ: u32 = 0x1;
pub const SPI_NOR_CMD_QREAD: u32 = 0x6B;
pub const SPI_NOR_CMD_QREAD_4B: u32 = 0x6C;
const SPI_NOR_4B_READ_THRESHOLD_BYTES: usize = 16 * 1024 * 1024;
const SPI_NOR_ADDR_WIDTH_MASK: u32 = 0x11;
const DMA_STATUS_RELEVANT_BITS: u32 = (1 << 11) | (1 << 10) | (1 << 9);
/// Mask for bits that are not IO mode or mode-type fields — preserves
/// frequency divisor and other config bits across per-phase ctrl writes.
const SPI_CTRL_IO_MODE_MASK: u32 = !0x7000_0000;
const SPI_CALIB_LEN: usize = 0x400;

struct CalibrationScratch(UnsafeCell<[u8; SPI_CALIB_LEN]>);

// Calibration runs during controller initialization with exclusive controller
// ownership, so this scratch buffer is not accessed concurrently.
unsafe impl Sync for CalibrationScratch {}

static CALIBRATION_SCRATCH: CalibrationScratch =
    CalibrationScratch(UnsafeCell::new([0; SPI_CALIB_LEN]));

const fn spi_nor_qread_cmd_for_capacity(capacity_bytes: usize) -> u32 {
    if capacity_bytes > SPI_NOR_4B_READ_THRESHOLD_BYTES {
        SPI_NOR_CMD_QREAD_4B
    } else {
        SPI_NOR_CMD_QREAD
    }
}

const fn spi_nor_uses_4b_addr(capacity_bytes: usize) -> bool {
    capacity_bytes > SPI_NOR_4B_READ_THRESHOLD_BYTES
}

const fn spi_nor_addr_width_mask(cs: ChipSelect) -> u32 {
    SPI_NOR_ADDR_WIDTH_MASK << (cs as u32)
}

const fn spi_nor_addr_width_reg(current: u32, cs: ChipSelect, use_4b: bool) -> u32 {
    let mask = spi_nor_addr_width_mask(cs);
    if use_4b {
        current | mask
    } else {
        current & !mask
    }
}

/// Type-state marker: controller is constructed but not initialized.
pub struct Uninitialized;

/// Type-state marker: controller has completed hardware initialization.
pub struct Ready;

/// Generic Static Memory Controller (SMC)
///
/// The `Mode` type parameter enforces init ordering at compile time.
pub struct Smc<Mode> {
    regs: SmcRegisters,
    controller_id: SmcController,
    config: SmcConfig,
    state: SmcState,
    /// Per-CS normal-read control register values stored at init time.
    /// Indexed by `ChipSelect as usize`. Restored unconditionally after every
    /// user-mode transaction, matching aspeed-rust `deactivate_user()` behavior.
    normal_read_ctrl: [u32; 2],
    /// Per-CS AHB flash window base addresses.
    /// CS0 starts at `controller_id.flash_window_address()`;
    /// CS1 starts immediately after the CS0 segment.
    flash_window_base: [usize; 2],
    _mode: PhantomData<fn() -> Mode>,
}

/// Ergonomic alias for the uninitialized controller handle.
pub type UninitSmc = Smc<Uninitialized>;

/// Ergonomic alias for the initialized controller handle.
pub type ReadySmc = Smc<Ready>;

impl Smc<Uninitialized> {
    /// Create a new SMC controller instance.
    ///
    /// # Safety
    /// Caller must ensure:
    /// - No other Smc instance exists for this hardware controller
    /// - The controller's base address points to valid hardware
    pub unsafe fn new(config: SmcConfig) -> Result<Self, SmcError> {
        if config.cs0.is_none() && config.cs1.is_none() {
            return Err(SmcError::InvalidCapacity);
        }

        let base = config.controller_id.base_address() as *const _;
        // SAFETY: Caller ensures base address is valid and no other instance exists.
        let regs = unsafe { SmcRegisters::new(base) };

        Ok(Self {
            regs,
            controller_id: config.controller_id,
            config,
            state: SmcState::Idle,
            normal_read_ctrl: [0; 2],
            flash_window_base: [0; 2],
            _mode: PhantomData,
        })
    }

    /// Initialize hardware and transition to `Ready` mode.
    pub fn init(self) -> Result<Smc<Ready>, SmcError> {
        // Phase 3: Topology-aware initialization
        //
        // The SmcTopology enum encodes the controller's role and master_idx:
        // - BootSpi { master_idx }: Boot firmware path (typically FMC, master_idx=0)
        // - HostSpi { master_idx }: Host BMC SPI path (typically SPI1, master_idx=0)
        // - NormalSpi { master_idx }: Normal user SPI path (typically SPI2, master_idx=2)
        //
        // Topology gates behavior in setup_segments() and configure_timing():
        // The topology is consulted via self.config.topology.

        // 1. Configure flash types and write-enable per CS
        let mut conf = 0u32;
        if self.config.cs0.is_some() {
            conf |= 1 << 16; // CONF_ENABLE_W0
            conf |= 0x2 << 0; // FLASH_TYPE_SPI
        }
        if self.config.cs1.is_some() {
            conf |= 1 << 17; // CONF_ENABLE_W1
            conf |= 0x2 << 2; // FLASH_TYPE_SPI
        }
        self.regs.write_config(conf);

        // 2. Set up segment addresses (memory mapping)
        Self::setup_segments(&self)?;

        // Snapshot per-CS normal-read control register values after all init writes.
        // CS1 value is captured even if cs1 is None (safe: register read is harmless).
        let cs0_normal_read = self.regs.read_cs0_ctrl();
        let cs1_normal_read = self.regs.read_cs1_ctrl();

        // Compute per-CS AHB flash window base addresses.
        let base = self.controller_id.flash_window_address();
        let cs0_size = flash_capacity_bytes(self.config.cs0).unwrap_or(0);
        let flash_window_base = [base, base + cs0_size];

        Ok(Smc {
            regs: self.regs,
            controller_id: self.controller_id,
            config: self.config,
            state: SmcState::Idle,
            normal_read_ctrl: [cs0_normal_read, cs1_normal_read],
            flash_window_base,
            _mode: PhantomData,
        })
    }
    fn encode_segment(&self, start: usize, end: usize) -> Result<u32, SmcError> {
        match self.config.controller_id {
            SmcController::Fmc => encode_fmc_segment(start, end),
            SmcController::Spi1 | SmcController::Spi2 => encode_spi_segment(start, end),
        }
    }

    fn setup_segments(&self) -> Result<(), SmcError> {
        // Decode-range sizing is topology-aware.
        //
        // For BootSpi (FMC, master_idx=0): Full decode range from configured capacity.
        //   Used for boot firmware; exclusive access to flash; no shared-bus concerns.
        //
        // For HostSpi / NormalSpi when master_idx != 0: Potential shared-bus topology.
        //   When multiple masters multiplex a single SPI flash, decode ranges may need
        //   to be restricted. Phase 3+ may implement decode_range_reinit logic keyed
        //   on config.topology.master_idx() to prevent collisions.
        //
        // For now, all topologies use the full capacity from FlashConfig.
        // Phase 3+: add conditional decode-range sizing based on topology + master_idx.

        let cs0_size = flash_capacity_bytes(self.config.cs0)?;
        let cs1_size = flash_capacity_bytes(self.config.cs1)?;
        total_capacity_bytes(self.config.cs0, self.config.cs1)?;

        if cs0_size > 0 {
            let seg = self.encode_segment(0, cs0_size)?;
            self.regs.write_cs0_segment(seg);
        }

        if cs1_size > 0 {
            let seg = self.encode_segment(cs0_size, cs0_size + cs1_size)?;
            self.regs.write_cs1_segment(seg);
        }

        Ok(())
    }
}

impl Smc<Ready> {
    /// Perform a programmed I/O read via memory window.
    ///
    /// Reads directly from the flash memory window. Hardware automatically
    /// converts memory accesses to SPI transactions.
    pub fn read(&self, cs: ChipSelect, offset: u32, buf: &mut [u8]) -> Result<usize, SmcError> {
        let cs_config = self.cs_config(cs)?;
        let cs_capacity = flash_capacity_bytes(Some(cs_config))?;
        let cs_idx = cs as usize;
        let window = self.flash_window_base[cs_idx] as *const u8;
        let offset = validate_mapped_range(offset, buf.len(), cs_capacity)?;
        let flash_ptr = window.wrapping_add(offset);
        pw_log::debug!(
            "read: offset0x{:08x}, size:0x{:08x}, flash ptr:0x{:08x}",
            offset as u32,
            buf.len() as u32,
            flash_ptr as u32
        );
        // SAFETY: `flash_ptr` is derived from the controller's fixed MMIO flash
        // window using `wrapping_add`, which avoids imposing Rust allocation
        // provenance rules on the raw address arithmetic itself. The actual read
        // below requires the requested `[offset, offset + buf.len())` range to be
        // backed by the controller's mapped flash aperture, and `buf` provides a
        // valid, writable destination that does not overlap this MMIO window.
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), buf.len());
        }

        Ok(buf.len())
    }
    #[inline(never)]
    pub fn loop_delay(spin_cnt: u32) {
        for _ in 0..spin_cnt {
            core::hint::spin_loop();
        }
    }
    /// Initiate a DMA read operation (non-blocking).
    pub fn dma_read(
        &mut self,
        cs: ChipSelect,
        flash_offset: u32,
        dram_addr: usize,
        len: u32,
    ) -> Result<(), SmcError> {
        if self.state != SmcState::Idle {
            return Err(SmcError::ControllerNotReady);
        }
        if !self.config.dma_enabled {
            return Err(SmcError::DmaNotEnabled);
        }
        if cs == ChipSelect::Cs1 && self.config.cs1.is_none() {
            return Err(SmcError::InvalidChipSelect);
        }
        self.regs.disable_dma();
        Self::loop_delay(0x1000);

        let cs_config = self.cs_config(cs)?;
        let cs_capacity = flash_capacity_bytes(Some(cs_config))?;
        pw_log::debug!(
            "flash_offset: 0x{:08x}, cs_cap: 0x{:08x}",
            flash_offset as u32,
            cs_capacity as u32
        );
        let cs_idx = cs as usize;

        let validated = validate_dma_read(
            flash_offset,
            self.flash_window_base[cs_idx],
            cs_capacity,
            dram_addr,
            len,
        )?;
        pw_log::debug!(
            "flash start: 0x{:08x}, cs_cap: 0x{:08x}, dram_addr: 0x{:08x} len: 0x{:08x} ",
            validated.flash_start as u32,
            cs_capacity as u32,
            validated.dram_addr as u32,
            validated.dma_len_reg as u32
        );

        // Set CS0 control register to normal-read mode before programming DMA
        // registers. The DMA engine reads the CSx control register to know which
        // SPI command to issue; it must be in normal-read mode (not user mode)
        // before the kick. Matches aspeed-rust fmccontroller.rs::read_dma
        // ctrl construction: preserve frequency bits, set ASPEED_SPI_NORMAL_READ.
        let cs_idx = cs as usize;
        let ctrl_val = self.normal_read_ctrl[cs_idx] | ASPEED_SPI_NORMAL_READ;
        self.regs.write_cs_ctrl(cs, ctrl_val);

        // Acquire the DMA bus arbiter before programming any DMA registers.
        // On SPI1/SPI2: writes SPI_DMA_GET_REQ_MAGIC and spins until DMAGrant
        // (bit 30 of spi080) is set. On FMC: bits 20–31 are Reserved — the write
        // is a no-op and the spin condition is immediately false. Safe to call
        // unconditionally on all controllers, matching aspeed-rust's approach.
        self.regs.acquire_dma_arbiter();
        pw_log::debug!("acquired dma bus arbiter");
        // Program DMA registers in the order used by aspeed-rust fmccontroller.rs::read_dma:
        //   fmc084 = flash side DMA address (R_DMA_FLASH_ADDR)
        //            = flash_window_base[cs] - SPI_DMA_FLASH_MAP_BASE + cs_offset
        //            (computed in validate_dma_read)
        //   fmc088 = DRAM/SRAM destination address (R_DMA_DRAM_ADDR)
        //            = physical_sram_addr + SPI_DMA_RAM_MAP_BASE
        //   fmc08c = transfer length - 1 (R_DMA_LEN)
        self.regs.write_dma_flash_addr(validated.flash_start as u32);
        self.regs
            .write_dma_dram_addr(validated.dram_addr + SPI_DMA_RAM_MAP_BASE);
        self.regs.write_dma_len(validated.dma_len_reg);

        // Enable the completion IRQ before kicking DMA. QEMU evaluates
        // INTR_CTRL_DMA_EN exactly once at DMA-done time
        // (`aspeed_smc_dma_done` in qemu/hw/ssi/aspeed_smc.c) and won't
        // re-fire the IRQ if the bit is set after the fact; aspeed-rust
        // arms the IRQ before starting DMA for the same reason
        // (`spicontroller.rs::read_dma`).
        if self.config.enable_interrupts {
            pw_log::debug!("enable dma irq");
            self.regs.enable_dma_irq();
        }

        // Kick DMA via read-modify-write to preserve timing calibration
        // bits (fmc080 bits 8-19), matching aspeed-rust fmccontroller.rs::read_dma.
        pw_log::debug!("start dma read...");
        self.regs.kick_dma_read();
        self.state = SmcState::DmaInFlight;
        Ok(())
    }

    /// Read raw DMA/interrupt status register bits (FMC008).
    pub fn dma_status(&self) -> u32 {
        self.regs.read_dma_status()
    }

    /// Clear DMA-related status bits in the status register (FMC008).
    ///
    /// `clear_mask` is write-1-to-clear and should contain only relevant bits.
    pub fn clear_dma_status(&self, clear_mask: u32) {
        self.regs
            .clear_dma_status(clear_mask & DMA_STATUS_RELEVANT_BITS);
    }

    /// Decode status bits and transition controller state.
    ///
    /// Called by both `handle_dma_irq` (IRQ-driven) and `poll_dma_completion`
    /// (polling). Assumes `status & DMA_STATUS_RELEVANT_BITS != 0`.
    fn complete_dma(&mut self, status: u32) -> Result<SmcInterrupt, SmcError> {
        let relevant = status & DMA_STATUS_RELEVANT_BITS;
        let dma_in_flight = self.state == SmcState::DmaInFlight;
        let decoded = SmcInterruptDecoder::decode_with_context(status, dma_in_flight);
        self.clear_dma_status(relevant);

        match decoded {
            SmcInterrupt::DmaComplete => {
                self.regs.disable_dma();
                self.state = SmcState::Idle;
                Ok(decoded)
            }
            SmcInterrupt::DmaError => {
                self.regs.disable_dma();
                self.state = SmcState::Idle;
                Err(SmcError::DmaAborted)
            }
            SmcInterrupt::CommandAbort => {
                self.state = SmcState::Faulted;
                Err(SmcError::HardwareError)
            }
            SmcInterrupt::WriteProtected => {
                self.state = SmcState::Faulted;
                Err(SmcError::WriteProtected)
            }
            SmcInterrupt::Unknown => Err(SmcError::HardwareError),
        }
    }

    /// Decode and complete an in-flight DMA operation from an IRQ event.
    ///
    /// Returns the decoded interrupt cause when a completion/error event was
    /// observed and processed. If no relevant status bits are set, returns
    /// `SmcError::ControllerNotReady` to indicate no completion work was found.
    pub fn handle_dma_irq(&mut self) -> Result<SmcInterrupt, SmcError> {
        self.regs.disable_dma_irq();
        let status = self.dma_status();
        pw_log::info!("SMC handle_dma_irq: status=0x{:08x}", status as u32);
        if status & DMA_STATUS_RELEVANT_BITS == 0 {
            return Err(SmcError::ControllerNotReady);
        }
        self.complete_dma(status)
    }

    /// Poll for DMA completion without requiring an IRQ.
    ///
    /// Returns `Poll::Pending` while the transfer is still in progress.
    /// Returns `Poll::Ready(Ok(()))` on success or `Poll::Ready(Err(SmcError))`
    /// on failure. Returns `Poll::Ready(Err(SmcError::ControllerNotReady))` if
    /// no DMA is in flight.
    ///
    /// Suitable for spin-poll loops in contexts where `enable_interrupts` is
    /// false (e.g., QEMU tests without an IRQ handler):
    /// ```ignore
    /// loop {
    ///     match controller.poll_dma_completion() {
    ///         Poll::Ready(result) => break result,
    ///         Poll::Pending => {}
    ///     }
    /// }
    /// ```
    pub fn poll_dma_completion(&mut self) -> core::task::Poll<Result<(), SmcError>> {
        if self.state != SmcState::DmaInFlight {
            return core::task::Poll::Ready(Err(SmcError::ControllerNotReady));
        }
        let status = self.dma_status();
        if status & DMA_STATUS_RELEVANT_BITS == 0 {
            return core::task::Poll::Pending;
        }
        core::task::Poll::Ready(self.complete_dma(status).map(|_| ()))
    }

    pub fn poll_blocking_dma_completion(&self, timeout: u32) -> u32 {
        let mut to = timeout;

        while (self.regs.read_dma_status() & DMA_STATUS_RELEVANT_BITS) == 0 {
            to -= 1;

            if to == 0 {
                return 0;
            }
        }
        return to;
    }
    /// Check if controller is ready for operations.
    pub fn is_ready(&self) -> bool {
        self.state == SmcState::Idle
    }

    #[doc(hidden)]
    pub fn test_force_dma_in_flight(&mut self) {
        self.state = SmcState::DmaInFlight;
    }

    /// Get the controller identifier.
    pub fn controller_id(&self) -> SmcController {
        self.controller_id
    }

    /// Return configured total flash capacity for this controller in bytes.
    pub fn capacity_bytes(&self) -> Result<usize, SmcError> {
        total_capacity_bytes(self.config.cs0, self.config.cs1)
    }

    /// Return configured flash capacity in bytes for the given chip select.
    ///
    /// Returns `SmcError::InvalidChipSelect` if the slot was not populated
    /// at construction time. Used by the device facade to bounds-check
    /// per-CS reads and to compute per-CS controller-window offsets.
    pub fn cs_capacity_bytes(&self, cs: ChipSelect) -> Result<usize, SmcError> {
        crate::smc::helpers::cs_capacity_bytes(&self.config, cs)
    }

    /// Return the configured `FlashConfig` for the requested chip select.
    ///
    /// Returns `SmcError::InvalidChipSelect` if the slot was not populated at
    /// construction time. Used by device-facade constructors to validate the
    /// caller-supplied `FlashConfig` against the per-CS configuration the
    /// controller was actually initialized with.
    pub fn cs_config(&self, cs: ChipSelect) -> Result<FlashConfig, SmcError> {
        let slot = match cs {
            ChipSelect::Cs0 => self.config.cs0,
            ChipSelect::Cs1 => self.config.cs1,
        };
        slot.ok_or(SmcError::InvalidChipSelect)
    }

    /// Execute a raw user-mode SPI transfer on CS0 for this controller.
    ///
    /// The `mode` parameter controls the IO width written to the CS control
    /// register for each phase (cmd / addr+payload / rx), matching the
    /// per-phase register update pattern used by aspeed-rust's
    /// `spi_nor_transceive_user()`.
    pub fn transceive_user(
        &self,
        cs: ChipSelect,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        if self.state != SmcState::Idle {
            return Err(SmcError::ControllerNotReady);
        }
        if cs == ChipSelect::Cs1 && self.config.cs1.is_none() {
            return Err(SmcError::InvalidChipSelect);
        }

        let cs_idx = cs as usize;
        // Derive user-mode base from the stored normal-read value: preserve
        // frequency bits and replace mode type with ASPEED_SPI_USER.
        let user_base = (self.normal_read_ctrl[cs_idx] & !0x7) | ASPEED_SPI_USER;
        let window = self.flash_window_base[cs_idx] as *mut u32;

        // Assert CS: inactive first, then active (matches aspeed-rust activate_user).
        self.regs
            .write_cs_ctrl(cs, user_base | ASPEED_SPI_USER_INACTIVE);
        self.regs.write_cs_ctrl(cs, user_base);

        // SAFETY: user mode is active; the flash aperture is the hardware-defined
        // byte-stream port for SPI command traffic while user mode is held.
        unsafe {
            // Command phase — always single-wire.
            let cmd_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.cmd_io_bits();
            self.regs.write_cs_ctrl(cs, cmd_ctrl);
            spi_write_data(window, cmd);

            // Address / TX payload phase.
            let addr_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.addr_io_bits();
            self.regs.write_cs_ctrl(cs, addr_ctrl);
            spi_write_data(window, tx_payload);

            // RX data phase.
            let data_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.data_io_bits();
            self.regs.write_cs_ctrl(cs, data_ctrl);
            spi_read_data(window as *const u32, rx);
        }

        // Deassert CS, then restore the pre-computed normal-read configuration
        // (matches aspeed-rust deactivate_user restoring cmd_mode[cs].normal_read).
        self.regs
            .write_cs_ctrl(cs, user_base | ASPEED_SPI_USER_INACTIVE);
        self.regs.write_cs_ctrl(cs, self.normal_read_ctrl[cs_idx]);
        Ok(())
    }

    //
    // MMIO access:: nor read init
    //
    //TODO: call from nordevice layer instead
    pub fn spi_nor_read_init(&mut self, cs: ChipSelect) -> Result<(), SmcError> {
        let mode: TransferMode = TransferMode::Mode114;
        let dummy: u32 = 0x1;
        let cs_idx = cs as usize;
        let cs_capacity = self.cs_capacity_bytes(cs)?;
        let use_4b_addr = spi_nor_uses_4b_addr(cs_capacity);
        let read_opcode = spi_nor_qread_cmd_for_capacity(cs_capacity);
        //pw_log::info!("=== spi_read_init()===");
        let read_cmd =
            mode.data_io_bits() | (read_opcode << 16) | (dummy << 6) | ASPEED_SPI_NORMAL_READ;

        self.regs.write_cs_ctrl(cs, read_cmd);
        let addr_width = spi_nor_addr_width_reg(self.regs.read_addr_width(), cs, use_4b_addr);
        self.regs.write_addr_width(addr_width);
        self.normal_read_ctrl[cs_idx] = read_cmd;
        if cs != ChipSelect::Cs0 {
            // CS1 calibration can fault on boards where the secondary FMC flash
            // is not ready for the calibration sweep. Keep CS1 on the same
            // fixed timing path used after calibration and still program its
            // normal-read command/address width above.
            return self.configure_timing(cs, self.cs_config(cs)?.spi_clock_mhz);
        }
        self.timing_calibration(cs)
    }

    fn configure_timing(&mut self, cs: ChipSelect, spi_clock_mhz: u32) -> Result<(), SmcError> {
        // Timing calibration is topology-aware.
        //
        // For BootSpi (FMC, master_idx=0): Full calibration sweep recommended.
        //   Boot firmware has exclusive access; full timing margin is priority.
        //
        // For HostSpi / NormalSpi when master_idx != 0: Shared-bus topology.
        //   When a secondary master shares the flash bus, calibration on CS1 may need
        //   to be skipped to avoid interfering with the primary master's calibration.
        //   Phase 3+: gate calibration logic on config.topology.master_idx().
        //
        // For now, all topologies use a single divider lookup; no HCLK sweep.
        // Phase 3+: add conditional calibration logic per topology and master_idx.
        // pw_log::info!("=== configure_timing()===");
        //TODO: need to get this from scu register
        let sysclk_mhz = 200u32;
        let encoded_div = spi_freq_div(sysclk_mhz, spi_clock_mhz)?;

        let cs_idx = cs as usize;
        let reg = self.regs.read_cs_ctrl(cs);
        self.regs
            .write_cs_ctrl(cs, (reg & !SPI_CTRL_FREQ_MASK) | encoded_div);
        self.normal_read_ctrl[cs_idx] &= (!SPI_CTRL_FREQ_MASK) | encoded_div;

        Ok(())
    }

    fn timing_calibration(&mut self, cs: ChipSelect) -> Result<(), SmcError> {
        let cs_cfg = self.cs_config(cs)?;
        let cs_idx = cs as usize;

        if self.regs.already_calibrated(cs) {
            pw_log::info!("already calibrated");
            return self.configure_timing(cs, cs_cfg.spi_clock_mhz);
        }

        //SPI2 work around
        if self.config.topology.master_idx() != 0 && cs_idx != 0 {
            return self.configure_timing(cs, cs_cfg.spi_clock_mhz);
        }
        // TODO: add SPIM config
        /*
         * use the related low frequency to get check calibration data
         * and get golden data.
         */
        let ctrl_val = self.regs.read_cs_ctrl(cs) & (!SPI_CTRL_FREQ_MASK);
        self.regs.write_cs_ctrl(cs, ctrl_val);

        let check_buf = unsafe { &mut *CALIBRATION_SCRATCH.0.get() };
        let window = self.flash_window_base[cs_idx] as *const u8;
        // TODO: configure timing_calibration_start_offset beside be???
        let timing_offset = 0x0;
        let flash_ptr = window.wrapping_add(timing_offset);
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, check_buf.as_mut_ptr(), SPI_CALIB_LEN);
        }

        if !spi_calibration_enable(&check_buf[..])? {
            return self.configure_timing(cs, cs_cfg.spi_clock_mhz);
        }

        let gold_checksum = self.spi_dma_checksum(cs, 0, 0);
        self.run_timing_sweep(cs, gold_checksum);

        self.configure_timing(cs, cs_cfg.spi_clock_mhz)
    }

    fn spi_dma_checksum(&mut self, cs: ChipSelect, div: u32, delay: u32) -> u32 {
        let timing_offset = 0x0;

        // Request DMA access
        self.regs.acquire_dma_arbiter();

        // Set DMA flash start address
        let cs_idx = cs as usize;
        let flash_addr = self.flash_window_base[cs_idx] + timing_offset;
        self.regs.write_dma_flash_addr(flash_addr as u32);
        // Set DMA length
        self.regs.write_dma_len(SPI_CALIB_LEN as u32);

        // Configure DMA control register
        let ctrl_val = SPI_DMA_ENABLE
            | SPI_DMA_CALC_CKSUM
            | SPI_DMA_CALIB_MODE
            | (delay << 0x8)
            | ((div & 0xf) << 16);
        self.regs.write_dma_ctrl(ctrl_val);

        // Wait until DMA done
        if self.poll_blocking_dma_completion(0x1000) == 0 {
            pw_log::info!("dma timeout!");
        }

        // Read checksum result
        // disable dma will clear the checksum
        let checksum = self.regs.read_dma_checksum();
        // Clear DMA control and discard request
        self.regs.disable_dma();

        return checksum;
    }

    fn run_timing_sweep(&mut self, cs: ChipSelect, gold_checksum: u32) {
        let hclk_masks = [7u32, 14, 6, 13];
        let mut calib_res = [0u8; 6 * 17];
        let cs_cfg = self.cs_config(cs);
        let mut freq_to_use = cs_cfg.unwrap().spi_clock_mhz;
        let sysclk_mhz = 200u32;

        for (i, &mask) in hclk_masks.iter().enumerate() {
            let div = u32::try_from(i).unwrap() + 2;
            if freq_to_use < sysclk_mhz / div {
                continue;
            }

            freq_to_use = sysclk_mhz / div;

            self.spi_dma_checksum(cs, mask, 0);

            calib_res.fill(0);

            for hcycle in 0..=5 {
                for delay_ns in 0..=0xf {
                    let reg_val = (1 << 3) | hcycle | (delay_ns << 4);

                    let checksum = self.spi_dma_checksum(cs, mask, reg_val);

                    let pass = checksum == gold_checksum;
                    let index = (hcycle * 17 + delay_ns) as usize;
                    calib_res[index] = u8::from(pass);
                }
            } //hcycle

            let calib_point = get_mid_point_of_longest_one(&calib_res);
            if calib_point >= 0 {
                let hcycle = (calib_point as u32 / 17) as u32;
                let delay_ns = (calib_point as u32 % 17) as u32;
                let final_delay = ((1 << 3) | hcycle | (delay_ns << 4)) << (i * 8);

                pw_log::info!(
                    "Final hcycle: {}, delay_ns: {} final_delay0x{:08x}",
                    hcycle as u32,
                    delay_ns as u32,
                    final_delay as u32
                );

                self.regs.write_cs_timing_compensation(cs, final_delay);
                return;
            } else {
                pw_log::info!("Cannot get good calibration point.");
            }
        }
    } // run_timing_sweep
}

unsafe fn spi_read_data(ahb_addr: *const u32, read_arr: &mut [u8]) {
    let len = read_arr.len();
    let mut index = 0usize;

    while index + 4 <= len {
        let word = unsafe { core::ptr::read_volatile(ahb_addr.add(index / 4)) };
        read_arr[index..index + 4].copy_from_slice(&word.to_le_bytes());
        index += 4;
    }

    while index < len {
        read_arr[index] = unsafe { core::ptr::read_volatile(ahb_addr.cast::<u8>().add(index)) };
        index += 1;
    }
}

unsafe fn spi_write_data(ahb_addr: *mut u32, write_arr: &[u8]) {
    let len = write_arr.len();
    let mut index = 0usize;

    while index + 4 <= len {
        let word = u32::from_le_bytes([
            write_arr[index],
            write_arr[index + 1],
            write_arr[index + 2],
            write_arr[index + 3],
        ]);
        unsafe { core::ptr::write_volatile(ahb_addr.add(index / 4), word) };
        index += 4;
    }

    while index < len {
        unsafe { core::ptr::write_volatile(ahb_addr.cast::<u8>().add(index), write_arr[index]) };
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        spi_nor_addr_width_reg, spi_nor_qread_cmd_for_capacity, SPI_NOR_4B_READ_THRESHOLD_BYTES,
        SPI_NOR_CMD_QREAD, SPI_NOR_CMD_QREAD_4B,
    };
    use crate::smc::types::ChipSelect;

    #[test]
    fn qread_command_uses_3b_at_or_below_16mib() {
        assert_eq!(
            spi_nor_qread_cmd_for_capacity(1024 * 1024),
            SPI_NOR_CMD_QREAD
        );
        assert_eq!(
            spi_nor_qread_cmd_for_capacity(SPI_NOR_4B_READ_THRESHOLD_BYTES),
            SPI_NOR_CMD_QREAD
        );
    }

    #[test]
    fn qread_command_uses_4b_above_16mib() {
        assert_eq!(
            spi_nor_qread_cmd_for_capacity(SPI_NOR_4B_READ_THRESHOLD_BYTES + 1),
            SPI_NOR_CMD_QREAD_4B
        );
    }

    #[test]
    fn addr_width_register_sets_only_selected_cs_for_4b() {
        assert_eq!(spi_nor_addr_width_reg(0, ChipSelect::Cs0, true), 0x11);
        assert_eq!(spi_nor_addr_width_reg(0, ChipSelect::Cs1, true), 0x22);
    }

    #[test]
    fn addr_width_register_clears_only_selected_cs_for_3b() {
        assert_eq!(
            spi_nor_addr_width_reg(0x2a33, ChipSelect::Cs0, false),
            0x2a22
        );
        assert_eq!(
            spi_nor_addr_width_reg(0x2a33, ChipSelect::Cs1, false),
            0x2a11
        );
    }
}
