// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Generic SMC controller implementation

use core::cell::UnsafeCell;
use core::marker::PhantomData;

use crate::smc::helpers::{
    SMC_WINDOW_SIZE_BYTES, SPI_CTRL_FREQ_MASK, SPI_DMA_CALC_CKSUM, SPI_DMA_CALIB_MODE,
    SPI_DMA_ENABLE, SPI_DMA_RAM_MAP_BASE, encode_fmc_segment, encode_spi_segment,
    get_mid_point_of_longest_one, spi_calibration_enable, spi_freq_div, validate_dma_read,
    validate_mapped_range,
};
use crate::smc::interrupts::{SmcInterrupt, SmcInterruptDecoder};
use crate::smc::registers::SmcRegisters;
use crate::smc::types::*;
use util_sfdp::{FlashGeometry, decode_geometry};

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

/// How a chip select's flash geometry is resolved at init.
///
/// [`Smc::init`] resolves each present CS through the [`SmcInstance`] marker's
/// associated source type. Because an associated type is only code-generated
/// when a marker names it, a build whose markers are all fixed-geometry sources
/// never instantiates the [`Discover`] path, so the SFDP decode code below is
/// never generated.
pub trait GeometrySource {
    /// Produce the flash geometry for `cs`.
    ///
    /// `baseline_ctrl` is the CS control-register value to derive user-mode
    /// transfers from (used only by [`Discover`]).
    fn resolve(
        regs: &SmcRegisters,
        cs: ChipSelect,
        window_base: usize,
        baseline_ctrl: u32,
    ) -> Result<FlashGeometry, SmcError>;
}

impl GeometrySource for Discover {
    fn resolve(
        regs: &SmcRegisters,
        cs: ChipSelect,
        window_base: usize,
        baseline_ctrl: u32,
    ) -> Result<FlashGeometry, SmcError> {
        // NOTE: fixed 256-byte SFDP read. A device whose BFP table extends past
        // offset 256 will fail decode (DeviceNotSupported); grow this if needed.
        let mut image = [0u8; 256];
        transceive_user_raw(
            regs,
            cs,
            window_base,
            baseline_ctrl,
            &[0x5A],
            &[0, 0, 0, 0],
            &mut image,
            TransferMode::Mode111,
        );
        decode_geometry(&image).map_err(|_| SmcError::DeviceNotSupported)
    }
}

/// Geometry-source marker carrying a fixed geometry as a const parameter.
///
/// Use this instead of [`Discover`] for production buils when the wired flash
/// is sure not to change Its `resolve` just returns `G`, so a build that names
/// only `Pinned` never generates the SFDP decode path.
pub struct Pinned<const G: FlashGeometry>;

impl<const G: FlashGeometry> GeometrySource for Pinned<G> {
    fn resolve(
        _regs: &SmcRegisters,
        _cs: ChipSelect,
        _window_base: usize,
        _baseline_ctrl: u32,
    ) -> Result<FlashGeometry, SmcError> {
        Ok(G)
    }
}

/// Type-state marker: controller is constructed but not initialized.
pub struct Uninitialized;

/// Type-state marker: controller has completed hardware initialization.
pub struct Ready;

/// Lifecycle marker for the controller's init state. Empty: config is carried
/// as a compile-time property of the `SmcInstance` type parameter, not stored.
pub trait SmcMode {}

impl SmcMode for Uninitialized {}

impl SmcMode for Ready {}

/// Compile-time-computed controller layout: memory-map config word, per-CS
/// aperture share, window bases, and encoded segment words. Computed inline in
/// the `const` block at the top of `Smc::init` from an [`SmcInstance`]'s const
/// config; an invalid config `panic!`s, turning it into a build error at the
/// instantiation site.
struct SmcLayout {
    conf: u32,
    region_size: usize,
    window_base: [usize; 2],
    cs0_present: bool,
    cs1_present: bool,
    cs0_segment: u32,
    cs1_segment: u32,
}

/// Per-CS state resolved once during [`Smc::init`]
#[derive(Clone, Copy)]
struct ResolvedCs {
    geometry: FlashGeometry,
    normal_read_ctrl: u32,
    window_base: usize,
}

const fn encode_segment_for(
    ctrl: SmcController,
    start: usize,
    end: usize,
) -> Result<u32, SmcError> {
    match ctrl {
        SmcController::Fmc => encode_fmc_segment(start, end),
        SmcController::Spi1 | SmcController::Spi2 => encode_spi_segment(start, end),
    }
}

/// Spin a fixed number of times to let a hardware register settle.
#[inline(never)]
fn loop_delay(spin_cnt: u32) {
    for _ in 0..spin_cnt {
        core::hint::spin_loop();
    }
}

/// Generic Static Memory Controller (SMC).
///
/// `I` names the wired controller and carries its config as compile-time consts
/// ([`SmcInstance`]); `Mode` enforces init ordering. Neither config nor
/// controller id is stored: both are read from `I`, and everything derived from
/// them is computed at compile time in `init`.
pub struct Smc<I: SmcInstance, Mode: SmcMode> {
    regs: SmcRegisters,
    state: SmcState,
    /// Geometry + calibration resolved at init; `None` until `Ready` (and for
    /// any absent chip select).
    cs0_resolved: Option<ResolvedCs>,
    cs1_resolved: Option<ResolvedCs>,
    _i: PhantomData<fn() -> I>,
    _mode: PhantomData<fn() -> Mode>,
}

/// Ergonomic alias for the uninitialized controller handle.
pub type UninitSmc<I> = Smc<I, Uninitialized>;

/// Ergonomic alias for the initialized controller handle.
pub type ReadySmc<I> = Smc<I, Ready>;

impl<I: SmcInstance> Smc<I, Uninitialized> {
    /// Create a new SMC controller instance.
    ///
    /// # Safety
    /// Caller must ensure:
    /// - No other Smc instance exists for this hardware controller
    /// - The controller's base address points to valid hardware
    pub unsafe fn new() -> Result<Self, SmcError> {
        let base = I::CONTROLLER.base_address() as *const _;
        // SAFETY: Caller ensures base address is valid and no other instance exists.
        let regs = unsafe { SmcRegisters::new(base) };

        Ok(Self {
            regs,
            state: SmcState::Idle,
            cs0_resolved: None,
            cs1_resolved: None,
            _i: PhantomData,
            _mode: PhantomData,
        })
    }

    /// Initialize hardware and transition to `Ready` mode.
    pub fn init(self) -> Result<Smc<I, Ready>, SmcError> {
        // Derive the full controller layout from its static config. Splits the
        // 256 MiB aperture evenly across present chip selects for now.
        let layout = const {
            let ctrl = I::CONTROLLER;
            let cfg = I::CONFIG;
            let cs0_present = cfg.cs0.is_some();
            let cs1_present = cfg.cs1.is_some();
            let present = cs0_present as usize + cs1_present as usize;
            if present == 0 {
                panic!("SMC config must configure at least one chip select");
            }
            let region_size = SMC_WINDOW_SIZE_BYTES / present;
            let cs0_size = if cs0_present { region_size } else { 0 };
            let base = ctrl.flash_window_address();
            let window_base = [base, base + cs0_size];

            let mut conf = 0u32;
            if cs0_present {
                conf |= 1 << 16; // CONF_ENABLE_W0
                conf |= 0x2 << 0; // FLASH_TYPE_SPI
            }
            if cs1_present {
                conf |= 1 << 17; // CONF_ENABLE_W1
                conf |= 0x2 << 2; // FLASH_TYPE_SPI
            }

            let cs0_segment = if cs0_present {
                match encode_segment_for(ctrl, 0, cs0_size) {
                    Ok(seg) => seg,
                    Err(_) => panic!("invalid CS0 segment for SMC config"),
                }
            } else {
                0
            };
            let cs1_segment = if cs1_present {
                match encode_segment_for(ctrl, cs0_size, cs0_size + region_size) {
                    Ok(seg) => seg,
                    Err(_) => panic!("invalid CS1 segment for SMC config"),
                }
            } else {
                0
            };

            SmcLayout {
                conf,
                region_size,
                window_base,
                cs0_present,
                cs1_present,
                cs0_segment,
                cs1_segment,
            }
        };

        // 1. Configure flash types and write-enable per CS.
        self.regs.write_config(layout.conf);

        // 2. Set up segment addresses (memory mapping).
        if layout.cs0_present {
            self.regs.write_cs0_segment(layout.cs0_segment);
        }
        if layout.cs1_present {
            self.regs.write_cs1_segment(layout.cs1_segment);
        }

        let mut smc = Smc::<I, Ready> {
            regs: self.regs,
            state: SmcState::Idle,
            cs0_resolved: None,
            cs1_resolved: None,
            _i: PhantomData,
            _mode: PhantomData,
        };

        // 3. Resolve + calibrate each present chip select once.
        if layout.cs0_present {
            smc.cs0_resolved = Some(smc.resolve_cs::<I::Cs0Geometry>(ChipSelect::Cs0, &layout)?);
        }
        if layout.cs1_present {
            smc.cs1_resolved = Some(smc.resolve_cs::<I::Cs1Geometry>(ChipSelect::Cs1, &layout)?);
        }

        Ok(smc)
    }
}

impl<I: SmcInstance> Smc<I, Ready> {
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
        I::CONTROLLER
    }

    /// Get the configured master ID for this controller topology.
    pub fn master_idx(&self) -> u8 {
        I::CONFIG.topology.master_idx()
    }

    /// Build a handle for CS0 from its init-resolved geometry and calibration.
    ///
    /// Returns `SmcError::InvalidChipSelect` if CS0 was not configured.
    pub fn cs0(&mut self) -> Result<Cs<'_>, SmcError> {
        self.build_cs(ChipSelect::Cs0)
    }

    /// Build a handle for CS1 from its init-resolved geometry and calibration.
    ///
    /// Returns `SmcError::InvalidChipSelect` if CS1 was not configured.
    pub fn cs1(&mut self) -> Result<Cs<'_>, SmcError> {
        self.build_cs(ChipSelect::Cs1)
    }

    fn build_cs(&mut self, cs: ChipSelect) -> Result<Cs<'_>, SmcError> {
        let resolved = match cs {
            ChipSelect::Cs0 => self.cs0_resolved,
            ChipSelect::Cs1 => self.cs1_resolved,
        }
        .ok_or(SmcError::InvalidChipSelect)?;

        Ok(Cs {
            regs: &self.regs,
            state: &mut self.state,
            cs,
            window_base: resolved.window_base,
            normal_read_ctrl: resolved.normal_read_ctrl,
            geometry: resolved.geometry,
            controller_id: I::CONTROLLER,
            master_idx: I::CONFIG.topology.master_idx(),
            dma_enabled: I::CONFIG.dma_enabled,
            enable_interrupts: I::CONFIG.enable_interrupts,
        })
    }

    /// Resolve a chip select's geometry and run calibration once, at init.
    ///
    /// Reads the reset-time CS control value as the SFDP baseline, resolves
    /// geometry via `S` (pinned or SFDP discovery), rejects a device that
    /// overflows its aperture share, then calibrates and returns the stored
    /// per-CS state.
    fn resolve_cs<S: GeometrySource>(
        &self,
        cs: ChipSelect,
        layout: &SmcLayout,
    ) -> Result<ResolvedCs, SmcError> {
        let cfg = self.cs_config(cs)?;
        let window_base = layout.window_base[cs as usize];
        // Reset-time CS control value; user-mode transfers (incl. SFDP) derive
        // their frequency bits from this baseline.
        let baseline_ctrl = self.regs.read_cs_ctrl(cs);
        let geometry = S::resolve(&self.regs, cs, window_base, baseline_ctrl)?;

        if geometry.capacity_bytes as usize > layout.region_size {
            return Err(SmcError::InvalidCapacity);
        }

        let normal_read_ctrl = self.calibrate_cs(
            cs,
            cfg.spi_clock_mhz,
            geometry.capacity_bytes as usize,
            window_base,
        )?;

        Ok(ResolvedCs {
            geometry,
            normal_read_ctrl,
            window_base,
        })
    }

    /// Presence check: the configured `FlashConfig` for `cs`, or
    /// `InvalidChipSelect` if the slot was not populated.
    fn cs_config(&self, cs: ChipSelect) -> Result<FlashConfig, SmcError> {
        let slot = match cs {
            ChipSelect::Cs0 => I::CONFIG.cs0,
            ChipSelect::Cs1 => I::CONFIG.cs1,
        };
        slot.ok_or(SmcError::InvalidChipSelect)
    }

    fn poll_blocking_dma_completion(&self, timeout: u32) -> u32 {
        let mut to = timeout;

        while (self.regs.read_dma_status() & DMA_STATUS_RELEVANT_BITS) == 0 {
            if to == 0 {
                return 0;
            }
            to -= 1;
        }
        return to;
    }

    /// Program normal-read command/address width for `cs` and run (or skip)
    /// timing calibration. Returns the final normal-read control value the
    /// handle restores after each user-mode transfer.
    fn calibrate_cs(
        &self,
        cs: ChipSelect,
        spi_clock_mhz: u32,
        capacity: usize,
        window_base: usize,
    ) -> Result<u32, SmcError> {
        let mode: TransferMode = TransferMode::Mode114;
        let dummy: u32 = 0x1;
        let use_4b_addr = spi_nor_uses_4b_addr(capacity);
        let read_opcode = spi_nor_qread_cmd_for_capacity(capacity);
        let read_cmd =
            mode.data_io_bits() | (read_opcode << 16) | (dummy << 6) | ASPEED_SPI_NORMAL_READ;

        self.regs.write_cs_ctrl(cs, read_cmd);
        let addr_width = spi_nor_addr_width_reg(self.regs.read_addr_width(), cs, use_4b_addr);
        self.regs.write_addr_width(addr_width);

        if cs != ChipSelect::Cs0 {
            // CS1 calibration can fault on boards where the secondary flash is
            // not ready for the sweep. Keep CS1 on the fixed timing path and
            // still program its normal-read command/address width above.
            return self.configure_timing(cs, spi_clock_mhz);
        }
        self.timing_calibration(cs, spi_clock_mhz, window_base)
    }

    fn configure_timing(&self, cs: ChipSelect, spi_clock_mhz: u32) -> Result<u32, SmcError> {
        //TODO: need to get this from scu register
        let sysclk_mhz = 200u32;
        let encoded_div = spi_freq_div(sysclk_mhz, spi_clock_mhz)?;

        let reg = self.regs.read_cs_ctrl(cs);
        self.regs
            .write_cs_ctrl(cs, (reg & !SPI_CTRL_FREQ_MASK) | encoded_div);
        Ok(self.regs.read_cs_ctrl(cs))
    }

    fn timing_calibration(
        &self,
        cs: ChipSelect,
        spi_clock_mhz: u32,
        window_base: usize,
    ) -> Result<u32, SmcError> {
        if self.regs.already_calibrated(cs) {
            pw_log::info!("already calibrated");
            return self.configure_timing(cs, spi_clock_mhz);
        }

        //SPI2 work around
        if I::CONFIG.topology.master_idx() != 0 && cs != ChipSelect::Cs0 {
            return self.configure_timing(cs, spi_clock_mhz);
        }
        // TODO: add SPIM config
        /*
         * use the related low frequency to get check calibration data
         * and get golden data.
         */
        let ctrl_val = self.regs.read_cs_ctrl(cs) & (!SPI_CTRL_FREQ_MASK);
        self.regs.write_cs_ctrl(cs, ctrl_val);

        let check_buf = unsafe { &mut *CALIBRATION_SCRATCH.0.get() };
        let window = window_base as *const u8;
        // TODO: configure timing_calibration_start_offset beside be???
        let timing_offset = 0x0;
        let flash_ptr = window.wrapping_add(timing_offset);
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, check_buf.as_mut_ptr(), SPI_CALIB_LEN);
        }

        if !spi_calibration_enable(&check_buf[..])? {
            return self.configure_timing(cs, spi_clock_mhz);
        }

        let gold_checksum = self.spi_dma_checksum(0, 0, window_base);
        self.run_timing_sweep(cs, spi_clock_mhz, gold_checksum, window_base);

        self.configure_timing(cs, spi_clock_mhz)
    }

    fn spi_dma_checksum(&self, div: u32, delay: u32, window_base: usize) -> u32 {
        let timing_offset = 0x0;

        // Request DMA access
        self.regs.acquire_dma_arbiter();

        // Set DMA flash start address
        let flash_addr = window_base + timing_offset;
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

    fn run_timing_sweep(
        &self,
        cs: ChipSelect,
        spi_clock_mhz: u32,
        gold_checksum: u32,
        window_base: usize,
    ) {
        let hclk_masks = [7u32, 14, 6, 13];
        let mut calib_res = [0u8; 6 * 17];
        let mut freq_to_use = spi_clock_mhz;
        let sysclk_div_table = [100u32, 66, 50, 40]; // 200 / [2, 3, 4, 5]

        for (i, &mask) in hclk_masks.iter().enumerate() {
            let freq = *sysclk_div_table.get(i).unwrap_or(&0);
            if freq_to_use < freq {
                continue;
            }

            freq_to_use = freq;

            self.spi_dma_checksum(mask, 0, window_base);

            calib_res.fill(0);

            for hcycle in 0..=5 {
                for delay_ns in 0..=0xf {
                    let reg_val = (1 << 3) | hcycle | (delay_ns << 4);

                    let checksum = self.spi_dma_checksum(mask, reg_val, window_base);

                    let pass = checksum == gold_checksum;
                    let index = (hcycle * 17 + delay_ns) as usize;
                    if let Some(cell) = calib_res.get_mut(index) {
                        *cell = u8::from(pass);
                    }
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

/// Per-chip-select handle vended by [`Smc::cs0`] / [`Smc::cs1`].
///
/// The chip select is baked into the handle, so reads and transfers take no
/// `ChipSelect` argument. The handle borrows the controller exclusively for its
/// lifetime: reads and transfers are `&self`, DMA is `&mut self`, so the
/// compiler enforces one operation on the controller at a time.
pub struct Cs<'a> {
    /// Shared register access: MMIO writes go through `&self`, so a shared
    /// borrow suffices. Kept disjoint from `state` so the handle can stay free
    /// of the controller's `SmcInstance` type parameter.
    regs: &'a SmcRegisters,
    /// Exclusive borrow of the controller's operation state; this is what makes
    /// the handle exclusive (only one `Cs` can exist at a time) and lets DMA
    /// transitions mutate state without threading `Smc<I, Ready>`.
    state: &'a mut SmcState,
    cs: ChipSelect,
    window_base: usize,
    normal_read_ctrl: u32,
    geometry: FlashGeometry,
    /// Copied from `I::CONTROLLER` / `I::CONFIG` at construction so the handle
    /// needs no generic parameter.
    controller_id: SmcController,
    master_idx: u8,
    dma_enabled: bool,
    enable_interrupts: bool,
}

impl Cs<'_> {
    /// The chip select this handle drives.
    pub fn chip_select(&self) -> ChipSelect {
        self.cs
    }

    /// Resolved flash geometry for this chip (SFDP-discovered or pinned).
    pub fn geometry(&self) -> FlashGeometry {
        self.geometry
    }

    /// Flash capacity in bytes for this chip.
    pub fn capacity_bytes(&self) -> usize {
        self.geometry.capacity_bytes as usize
    }

    /// The controller this chip is attached to.
    pub fn controller_id(&self) -> SmcController {
        self.controller_id
    }

    /// The master index of the controller's topology (for SPIM mux routing).
    pub fn master_idx(&self) -> u8 {
        self.master_idx
    }

    /// Perform a programmed I/O read via the memory window.
    ///
    /// Reads directly from the flash memory window. Hardware automatically
    /// converts memory accesses to SPI transactions.
    pub fn read(&self, offset: u32, buf: &mut [u8]) -> Result<usize, SmcError> {
        let offset = validate_mapped_range(offset, buf.len(), self.capacity_bytes())?;
        let flash_ptr = (self.window_base as *const u8).wrapping_add(offset);
        pw_log::debug!(
            "read: offset0x{:08x}, size:0x{:08x}, flash ptr:0x{:08x}",
            offset as u32,
            buf.len() as u32,
            flash_ptr as u32
        );
        // SAFETY: `flash_ptr` is derived from the controller's fixed MMIO flash
        // window via `wrapping_add`; the validated `[offset, offset + buf.len())`
        // range lies within this chip's mapped aperture, and `buf` is a valid,
        // writable destination disjoint from the MMIO window.
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    /// Execute a raw user-mode SPI transfer on this chip.
    ///
    /// The `mode` parameter controls the IO width written to the CS control
    /// register for each phase (cmd / addr+payload / rx).
    pub fn transceive_user(
        &self,
        cmd: &[u8],
        tx_payload: &[u8],
        rx: &mut [u8],
        mode: TransferMode,
    ) -> Result<(), SmcError> {
        if *self.state != SmcState::Idle {
            return Err(SmcError::ControllerNotReady);
        }
        transceive_user_raw(
            self.regs,
            self.cs,
            self.window_base,
            self.normal_read_ctrl,
            cmd,
            tx_payload,
            rx,
            mode,
        );
        Ok(())
    }

    /// Initiate a DMA read operation (non-blocking).
    pub fn dma_read(
        &mut self,
        flash_offset: u32,
        dram_addr: usize,
        len: u32,
    ) -> Result<(), SmcError> {
        if *self.state != SmcState::Idle {
            return Err(SmcError::ControllerNotReady);
        }
        if !self.dma_enabled {
            return Err(SmcError::DmaNotEnabled);
        }
        self.regs.disable_dma();
        loop_delay(0x1000);

        let cs_capacity = self.capacity_bytes();
        let validated =
            validate_dma_read(flash_offset, self.window_base, cs_capacity, dram_addr, len)?;
        pw_log::debug!(
            "flash start: 0x{:08x}, cs_cap: 0x{:08x}, dram_addr: 0x{:08x} len: 0x{:08x} ",
            validated.flash_start as u32,
            cs_capacity as u32,
            validated.dram_addr as u32,
            validated.dma_len_reg as u32
        );

        // Set the CS control register to normal-read mode before programming DMA
        // registers. The DMA engine reads the CSx control register to know which
        // SPI command to issue; it must be in normal-read mode (not user mode)
        // before the kick. Matches aspeed-rust fmccontroller.rs::read_dma.
        let ctrl_val = self.normal_read_ctrl | ASPEED_SPI_NORMAL_READ;
        self.regs.write_cs_ctrl(self.cs, ctrl_val);

        // Acquire the DMA bus arbiter before programming any DMA registers.
        self.regs.acquire_dma_arbiter();
        self.regs.write_dma_flash_addr(validated.flash_start as u32);
        self.regs
            .write_dma_dram_addr(validated.dram_addr + SPI_DMA_RAM_MAP_BASE);
        self.regs.write_dma_len(validated.dma_len_reg);

        // Arm the completion IRQ before kicking DMA (QEMU evaluates the enable
        // once at DMA-done time and won't re-fire if set afterward).
        if self.enable_interrupts {
            self.regs.enable_dma_irq();
        }

        self.regs.kick_dma_read();
        *self.state = SmcState::DmaInFlight;
        Ok(())
    }

    /// Poll for DMA completion without requiring an IRQ.
    ///
    /// Returns `Poll::Pending` while the transfer is still in progress,
    /// `Poll::Ready(Ok(()))` on success, or `Poll::Ready(Err(..))` on failure /
    /// when no DMA is in flight.
    pub fn poll_dma_completion(&mut self) -> core::task::Poll<Result<(), SmcError>> {
        if *self.state != SmcState::DmaInFlight {
            return core::task::Poll::Ready(Err(SmcError::ControllerNotReady));
        }
        let status = self.dma_status();
        if status & DMA_STATUS_RELEVANT_BITS == 0 {
            return core::task::Poll::Pending;
        }
        core::task::Poll::Ready(self.complete_dma(status).map(|_| ()))
    }

    /// Decode and complete an in-flight DMA operation from an IRQ event.
    pub fn handle_dma_irq(&mut self) -> Result<SmcInterrupt, SmcError> {
        self.regs.disable_dma_irq();
        let status = self.dma_status();
        pw_log::info!("SMC handle_dma_irq: status=0x{:08x}", status as u32);
        if status & DMA_STATUS_RELEVANT_BITS == 0 {
            return Err(SmcError::ControllerNotReady);
        }
        self.complete_dma(status)
    }

    /// Read raw DMA/interrupt status register bits (FMC008).
    pub fn dma_status(&self) -> u32 {
        self.regs.read_dma_status()
    }

    /// Clear DMA-related status bits (write-1-to-clear).
    pub fn clear_dma_status(&self, clear_mask: u32) {
        self.regs
            .clear_dma_status(clear_mask & DMA_STATUS_RELEVANT_BITS);
    }

    /// Decode status bits and transition controller state.
    ///
    /// Assumes `status & DMA_STATUS_RELEVANT_BITS != 0`.
    fn complete_dma(&mut self, status: u32) -> Result<SmcInterrupt, SmcError> {
        let relevant = status & DMA_STATUS_RELEVANT_BITS;
        let dma_in_flight = *self.state == SmcState::DmaInFlight;
        let decoded = SmcInterruptDecoder::decode_with_context(status, dma_in_flight);
        self.clear_dma_status(relevant);

        match decoded {
            SmcInterrupt::DmaComplete => {
                self.regs.disable_dma();
                *self.state = SmcState::Idle;
                Ok(decoded)
            }
            SmcInterrupt::DmaError => {
                self.regs.disable_dma();
                *self.state = SmcState::Idle;
                Err(SmcError::DmaAborted)
            }
            SmcInterrupt::CommandAbort => {
                *self.state = SmcState::Faulted;
                Err(SmcError::HardwareError)
            }
            SmcInterrupt::WriteProtected => {
                *self.state = SmcState::Faulted;
                Err(SmcError::WriteProtected)
            }
            SmcInterrupt::Unknown => Err(SmcError::HardwareError),
        }
    }
}

/// Raw user-mode SPI transfer (CS-assert → 3-phase → CS-restore) shared by
/// `Smc<Ready>::transceive_user` and `init`'s SFDP read. Module-private and
/// guardless: assumes segments and the per-CS normal-read snapshot are set.
#[allow(clippy::too_many_arguments)]
fn transceive_user_raw(
    regs: &SmcRegisters,
    cs: ChipSelect,
    window_base: usize,
    normal_read_ctrl: u32,
    cmd: &[u8],
    tx_payload: &[u8],
    rx: &mut [u8],
    mode: TransferMode,
) {
    // Derive user-mode base from the stored normal-read value: preserve
    // frequency bits and replace mode type with ASPEED_SPI_USER.
    let user_base = (normal_read_ctrl & !0x7) | ASPEED_SPI_USER;
    let window = window_base as *mut u32;

    // Assert CS: inactive first, then active (matches aspeed-rust activate_user).
    regs.write_cs_ctrl(cs, user_base | ASPEED_SPI_USER_INACTIVE);
    regs.write_cs_ctrl(cs, user_base);

    // SAFETY: user mode is active; the flash aperture is the hardware-defined
    // byte-stream port for SPI command traffic while user mode is held.
    unsafe {
        // Command phase — always single-wire.
        let cmd_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.cmd_io_bits();
        regs.write_cs_ctrl(cs, cmd_ctrl);
        spi_write_data(window, cmd);

        // Address / TX payload phase.
        let addr_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.addr_io_bits();
        regs.write_cs_ctrl(cs, addr_ctrl);
        spi_write_data(window, tx_payload);

        // RX data phase.
        let data_ctrl = (user_base & SPI_CTRL_IO_MODE_MASK) | mode.data_io_bits();
        regs.write_cs_ctrl(cs, data_ctrl);
        spi_read_data(window as *const u32, rx);
    }

    // Deassert CS, then restore the pre-computed normal-read configuration
    // (matches aspeed-rust deactivate_user restoring cmd_mode[cs].normal_read).
    regs.write_cs_ctrl(cs, user_base | ASPEED_SPI_USER_INACTIVE);
    regs.write_cs_ctrl(cs, normal_read_ctrl);
}

unsafe fn spi_read_data(ahb_addr: *const u32, read_arr: &mut [u8]) {
    let len = read_arr.len();
    let (chunks, remainder) = read_arr.split_at_mut(len - len % 4);

    for (i, chunk) in chunks.chunks_exact_mut(4).enumerate() {
        let word = unsafe { core::ptr::read_volatile(ahb_addr.add(i)) };
        chunk.copy_from_slice(&word.to_le_bytes());
    }

    for (i, cell) in remainder.iter_mut().enumerate() {
        let offset = len - len % 4 + i;
        *cell = unsafe { core::ptr::read_volatile(ahb_addr.cast::<u8>().add(offset)) };
    }
}

unsafe fn spi_write_data(ahb_addr: *mut u32, write_arr: &[u8]) {
    let len = write_arr.len();
    let (chunks, remainder) = write_arr.split_at(len - len % 4);

    for (i, chunk) in chunks.chunks_exact(4).enumerate() {
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        unsafe { core::ptr::write_volatile(ahb_addr.add(i), word) };
    }

    for (i, &val) in remainder.iter().enumerate() {
        let offset = len - len % 4 + i;
        unsafe { core::ptr::write_volatile(ahb_addr.cast::<u8>().add(offset), val) };
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SPI_NOR_4B_READ_THRESHOLD_BYTES, SPI_NOR_CMD_QREAD, SPI_NOR_CMD_QREAD_4B,
        spi_nor_addr_width_reg, spi_nor_qread_cmd_for_capacity,
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
