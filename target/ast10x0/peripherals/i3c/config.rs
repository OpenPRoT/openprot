// Licensed under the Apache-2.0 license

//! I3C configuration types
//!
//! Configuration structures for I3C controller and devices.

use core::marker::PhantomData;
use core::sync::atomic::AtomicPtr;
use heapless::Vec;

use super::error::I3cError;
use super::types::{Completion, DevKind};

// =============================================================================
// Target Configuration
// =============================================================================

/// Configuration for I3C target mode
pub struct I3cTargetConfig {
    /// Target flags
    pub flags: u8,
    /// Dynamic address (assigned by controller)
    pub addr: Option<u8>,
    /// Mandatory Data Byte for IBI
    pub mdb: u8,
}

impl I3cTargetConfig {
    /// Create a new target configuration
    #[must_use]
    pub const fn new(flags: u8, addr: Option<u8>, mdb: u8) -> Self {
        Self { flags, addr, mdb }
    }
}

// =============================================================================
// Address Book
// =============================================================================

/// Address allocation and tracking for I3C bus
pub struct AddrBook {
    /// Addresses currently in use
    pub in_use: [bool; 128],
    /// Reserved addresses (not available for allocation)
    pub reserved: [bool; 128],
}

impl Default for AddrBook {
    fn default() -> Self {
        Self::new()
    }
}

impl AddrBook {
    /// Create a new empty address book
    #[must_use]
    pub const fn new() -> Self {
        Self {
            in_use: [false; 128],
            reserved: [false; 128],
        }
    }

    /// Check if an address is free (not in use and not reserved)
    #[inline]
    #[must_use]
    pub fn is_free(&self, addr: u8) -> bool {
        !self.in_use[addr as usize] && !self.reserved[addr as usize]
    }

    /// Reserve default I3C addresses per specification
    ///
    /// Reserves addresses 0-7, 0x7E (broadcast), and addresses that
    /// differ from 0x7E by a single bit.
    pub fn reserve_defaults(&mut self) {
        // Reserve addresses 0-7
        for a in 0usize..=7 {
            self.reserved[a] = true;
        }
        // Reserve broadcast address
        self.reserved[0x7E_usize] = true;
        // Reserve addresses differing from 0x7E by single bit
        for i in 0..=7 {
            let alt = 0x7E ^ (1u8 << i);
            if alt <= 0x7E {
                self.reserved[alt as usize] = true;
            }
        }
    }

    /// Allocate an address starting from the given value
    ///
    /// Returns `Some(addr)` if an address was found, `None` if exhausted.
    pub fn alloc_from(&mut self, start: u8) -> Option<u8> {
        let mut addr = start.max(8);
        while addr < 0x7F {
            if self.is_free(addr) {
                return Some(addr);
            }
            addr += 1;
        }
        None
    }

    /// Mark an address as used or free
    #[inline]
    pub fn mark_use(&mut self, addr: u8, used: bool) {
        if addr != 0 {
            self.in_use[addr as usize] = used;
        }
    }
}

// =============================================================================
// Device Entry
// =============================================================================

/// Entry for a device attached to the I3C bus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceEntry {
    /// Device type (I3C or I2C)
    pub kind: DevKind,
    /// Provisional ID (for I3C devices)
    pub pid: Option<u64>,
    /// Static address (for I2C or SETDASA)
    pub static_addr: u8,
    /// Current dynamic address
    pub dyn_addr: u8,
    /// Desired dynamic address
    pub desired_da: u8,
    /// Bus Characteristics Register
    pub bcr: u8,
    /// Device Characteristics Register
    pub dcr: u8,
    /// Maximum read speed
    pub maxrd: u8,
    /// Maximum write speed
    pub maxwr: u8,
    /// Maximum read length
    pub mrl: u16,
    /// Maximum write length
    pub mwl: u16,
    /// Maximum IBI payload size
    pub max_ibi: u8,
    /// IBI enabled flag
    pub ibi_en: bool,
    /// Position in DAT (Device Address Table)
    pub pos: Option<u8>,
}

impl DeviceEntry {
    /// Create a new I3C device entry
    #[must_use]
    pub const fn new_i3c(pid: u64, desired_da: u8) -> Self {
        Self {
            kind: DevKind::I3c,
            pid: Some(pid),
            static_addr: 0,
            dyn_addr: desired_da,
            desired_da,
            bcr: 0,
            dcr: 0,
            maxrd: 0,
            maxwr: 0,
            mrl: 0,
            mwl: 0,
            max_ibi: 0,
            ibi_en: false,
            pos: None,
        }
    }

    /// Create a new I2C device entry
    #[must_use]
    pub const fn new_i2c(static_addr: u8) -> Self {
        Self {
            kind: DevKind::I2c,
            pid: None,
            static_addr,
            dyn_addr: static_addr,
            desired_da: static_addr,
            bcr: 0,
            dcr: 0,
            maxrd: 0,
            maxwr: 0,
            mrl: 0,
            mwl: 0,
            max_ibi: 0,
            ibi_en: false,
            pos: None,
        }
    }
}

// =============================================================================
// Attached Devices
// =============================================================================

/// Collection of devices attached to the I3C bus
pub struct Attached {
    /// Device entries (max 8 devices)
    pub devices: Vec<DeviceEntry, 8>,
    /// Position-to-index mapping
    pub by_pos: [Option<u8>; 8],
}

impl Default for Attached {
    fn default() -> Self {
        Self::new()
    }
}

impl Attached {
    /// Create a new empty attached devices collection
    #[must_use]
    pub const fn new() -> Self {
        Self {
            devices: Vec::new(),
            by_pos: [None; 8],
        }
    }

    /// Attach a device to the bus
    ///
    /// Returns the device index on success.
    pub fn attach(&mut self, dev: DeviceEntry) -> Result<usize, I3cError> {
        let idx = self.devices.len();
        self.devices.push(dev).map_err(|_| I3cError::NoFreeSlot)?;
        Ok(idx)
    }

    /// Detach a device by its index
    pub fn detach(&mut self, dev_idx: usize) {
        if dev_idx >= self.devices.len() {
            return;
        }

        // Clear position mapping if device had one
        if let Some(pos) = self.devices[dev_idx].pos {
            if let Some(p) = self.by_pos.get_mut(pos as usize) {
                *p = None;
            }
        }

        // Remove device and update position mappings
        self.devices.remove(dev_idx);
        for bp in &mut self.by_pos {
            if let Some(idx) = bp {
                let idx_usize = *idx as usize;
                if idx_usize > dev_idx && idx_usize > 0 {
                    // SAFETY: Saturating subtract to prevent panic on underflow
                    *bp = Some(idx.saturating_sub(1));
                }
            }
        }
    }

    /// Detach a device by its DAT position
    pub fn detach_by_pos(&mut self, pos: usize) {
        if let Some(Some(dev_idx)) = self.by_pos.get(pos) {
            self.detach(*dev_idx as usize);
        }
    }

    /// Get the DAT position of a device by its index
    #[must_use]
    pub fn pos_of(&self, dev_idx: usize) -> Option<u8> {
        let dev_idx_u8 = u8::try_from(dev_idx).ok()?;
        self.by_pos
            .iter()
            .enumerate()
            .find_map(|(pos, &v)| (v == Some(dev_idx_u8)).then_some(pos))
            .and_then(|p| u8::try_from(p).ok())
    }

    /// Find device index by dynamic address
    #[must_use]
    pub fn find_dev_idx_by_addr(&self, da: u8) -> Option<usize> {
        self.devices.iter().position(|d| d.dyn_addr == da)
    }

    /// Get DAT position by dynamic address
    #[must_use]
    pub fn pos_of_addr(&self, da: u8) -> Option<u8> {
        let dev_idx = self.devices.iter().position(|d| d.dyn_addr == da)?;
        self.pos_of(dev_idx)
    }

    /// Get DAT position by PID
    #[must_use]
    pub fn pos_of_pid(&self, pid: u64) -> Option<u8> {
        let dev_idx = self.devices.iter().position(|d| d.pid == Some(pid))?;
        self.pos_of(dev_idx)
    }

    /// Map a DAT position to a device index
    #[inline]
    pub fn map_pos(&mut self, pos: u8, idx: u8) -> bool {
        if let Some(slot) = self.by_pos.get_mut(pos as usize) {
            *slot = Some(idx);
            return true;
        }
        false
    }

    /// Unmap a DAT position
    #[inline]
    pub fn unmap_pos(&mut self, pos: u8) {
        self.by_pos[pos as usize] = None;
    }
}

// =============================================================================
// Common State
// =============================================================================

/// Common state shared across configurations (placeholder)
#[derive(Default)]
pub struct CommonState {
    _phantom: PhantomData<()>,
}

/// Common configuration (placeholder)
#[derive(Default)]
pub struct CommonCfg {
    _phantom: PhantomData<()>,
}

// =============================================================================
// Reset Specification
// =============================================================================

/// Reset line specification
#[derive(Clone, Copy)]
pub struct ResetSpec {
    /// Reset line ID
    pub id: u32,
    /// Whether reset is active high
    pub active_high: bool,
}

// =============================================================================
// Main Configuration
// =============================================================================

/// Main I3C bus configuration
pub struct I3cConfig {
    /// Common higher-level state
    pub common: CommonState,
    /// Target mode configuration (if operating as target)
    pub target_config: Option<I3cTargetConfig>,
    /// Address book for dynamic address management
    pub addrbook: AddrBook,
    /// Collection of attached devices
    pub attached: Attached,

    // Concurrency
    /// Pointer to current transfer in progress
    pub curr_xfer: AtomicPtr<()>,

    // Clock configuration
    /// Core clock frequency in Hz (injected by platform)
    ///
    /// If `None`, hardware implementation may auto-detect or use a default.
    /// Providing this value decouples I3C from SCU/clock tree access.
    pub core_clk_hz: Option<u32>,

    // Timing/PHY parameters (nanoseconds, computed from core_clk_hz)
    /// Core clock period in ns (computed during init)
    pub core_period: u32,
    /// I2C SCL frequency in Hz
    pub i2c_scl_hz: u32,
    /// I3C SCL frequency in Hz
    pub i3c_scl_hz: u32,
    /// I3C push-pull SCL high period in ns
    pub i3c_pp_scl_hi_period_ns: u32,
    /// I3C push-pull SCL low period in ns
    pub i3c_pp_scl_lo_period_ns: u32,
    /// I3C open-drain SCL high period in ns
    pub i3c_od_scl_hi_period_ns: u32,
    /// I3C open-drain SCL low period in ns
    pub i3c_od_scl_lo_period_ns: u32,
    /// SDA TX hold time in ns
    pub sda_tx_hold_ns: u32,
    /// Whether this controller is secondary
    pub is_secondary: bool,

    // Tables/indices
    /// Maximum number of devices
    pub maxdevs: u16,
    /// Bitmap of free DAT positions
    pub free_pos: u32,
    /// Bitmap of devices needing dynamic address
    pub need_da: u32,
    /// Address array for DAT
    pub addrs: [u8; 8],
    /// DCR value
    pub dcr: u32,

    // Target-mode data
    /// Whether SIR (Slave Interrupt Request) is allowed by software
    pub sir_allowed_by_sw: bool,
    /// Completion for target IBI
    pub target_ibi_done: Completion,
    /// Completion for target data transfer
    pub target_data_done: Completion,
}

impl Default for I3cConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl I3cConfig {
    /// Create a new configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            common: CommonState::default(),
            target_config: None,
            addrbook: AddrBook::new(),
            attached: Attached::new(),
            curr_xfer: AtomicPtr::new(core::ptr::null_mut()),
            core_clk_hz: None,
            core_period: 0,
            i2c_scl_hz: 0,
            i3c_scl_hz: 0,
            i3c_pp_scl_hi_period_ns: 0,
            i3c_pp_scl_lo_period_ns: 0,
            i3c_od_scl_hi_period_ns: 0,
            i3c_od_scl_lo_period_ns: 0,
            sda_tx_hold_ns: 0,
            is_secondary: false,
            maxdevs: 8,
            free_pos: 0,
            need_da: 0,
            addrs: [0; 8],
            dcr: 0,
            sir_allowed_by_sw: false,
            target_ibi_done: Completion::new(),
            target_data_done: Completion::new(),
        }
    }

    /// Initialize runtime fields (address book and attached devices)
    pub fn init_runtime_fields(&mut self) {
        self.addrbook = AddrBook::new();
        self.addrbook.reserve_defaults();
        self.attached = Attached::new();
    }

    /// Pick an initial dynamic address for a device
    ///
    /// Tries `desired` first, then `static_addr`, then allocates from pool.
    pub fn pick_initial_da(&mut self, static_addr: u8, desired: u8) -> Option<u8> {
        if desired != 0 && self.addrbook.is_free(desired) {
            self.addrbook.mark_use(desired, true);
            return Some(desired);
        }
        if static_addr != 0 && self.addrbook.is_free(static_addr) {
            self.addrbook.mark_use(static_addr, true);
            return Some(static_addr);
        }
        self.addrbook.alloc_from(8)
    }

    /// Reassign a device's dynamic address
    pub fn reassign_da(&mut self, from: u8, to: u8) -> Result<(), I3cError> {
        if from == to {
            return Ok(());
        }
        if !self.addrbook.is_free(to) {
            return Err(I3cError::AddrInUse);
        }

        self.addrbook.mark_use(from, false);
        self.addrbook.mark_use(to, true);

        if let Some((i, mut e)) = self
            .attached
            .devices
            .iter()
            .enumerate()
            .find_map(|(i, d)| (d.dyn_addr == from).then_some((i, *d)))
        {
            e.dyn_addr = to;
            self.attached.devices[i] = e;
            Ok(())
        } else {
            Err(I3cError::DevNotFound)
        }
    }
}

// =============================================================================
// Builder Pattern for I3cConfig
// =============================================================================

impl I3cConfig {
    /// Set core clock frequency in Hz
    ///
    /// This decouples the I3C driver from SCU/clock tree access.
    /// The platform layer should provide the actual clock rate.
    ///
    /// # I3C Timing Requirements (MIPI I3C Spec v1.1)
    ///
    /// | Mode | Min Clock | Typical | Notes |
    /// |------|-----------|---------|-------|
    /// | SDR | 12.5 `MHz` | 100-200 `MHz` | For 12.5 `MHz` SCL |
    /// | HDR | 25 `MHz` | 100-200 `MHz` | For 25 `MHz` SCL |
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = I3cConfig::new()
    ///     .core_clk_hz(200_000_000)  // 200 MHz from platform
    ///     .i3c_scl_hz(12_500_000);   // 12.5 MHz SCL
    /// ```
    #[must_use]
    pub fn core_clk_hz(mut self, hz: u32) -> Self {
        self.core_clk_hz = Some(hz);
        self
    }

    /// Set I2C SCL frequency
    #[must_use]
    pub fn i2c_scl_hz(mut self, hz: u32) -> Self {
        self.i2c_scl_hz = hz;
        self
    }

    /// Set I3C SCL frequency
    #[must_use]
    pub fn i3c_scl_hz(mut self, hz: u32) -> Self {
        self.i3c_scl_hz = hz;
        self
    }

    /// Set as secondary controller
    #[must_use]
    pub fn secondary(mut self, is_secondary: bool) -> Self {
        self.is_secondary = is_secondary;
        self
    }

    /// Set DCR (Device Characteristics Register)
    #[must_use]
    pub fn dcr(mut self, dcr: u8) -> Self {
        self.dcr = u32::from(dcr);
        self
    }

    /// Set target configuration
    #[must_use]
    pub fn target_config(mut self, config: I3cTargetConfig) -> Self {
        self.target_config = Some(config);
        self
    }

    /// Set I3C Push-Pull SCL high period in ns
    #[must_use]
    pub fn i3c_pp_scl_hi_period_ns(mut self, ns: u32) -> Self {
        self.i3c_pp_scl_hi_period_ns = ns;
        self
    }

    /// Set I3C Push-Pull SCL low period in ns
    #[must_use]
    pub fn i3c_pp_scl_lo_period_ns(mut self, ns: u32) -> Self {
        self.i3c_pp_scl_lo_period_ns = ns;
        self
    }

    /// Set I3C Open-Drain SCL high period in ns
    #[must_use]
    pub fn i3c_od_scl_hi_period_ns(mut self, ns: u32) -> Self {
        self.i3c_od_scl_hi_period_ns = ns;
        self
    }

    /// Set I3C Open-Drain SCL low period in ns
    #[must_use]
    pub fn i3c_od_scl_lo_period_ns(mut self, ns: u32) -> Self {
        self.i3c_od_scl_lo_period_ns = ns;
        self
    }

    /// Set SDA TX hold time in ns
    #[must_use]
    pub fn sda_tx_hold_ns(mut self, ns: u32) -> Self {
        self.sda_tx_hold_ns = ns;
        self
    }
}

// =============================================================================
// Clock Validation
// =============================================================================

/// Minimum core clock for I3C SDR mode (Hz)
/// Required to achieve 12.5 `MHz` SCL with proper timing margins
pub const I3C_MIN_CORE_CLK_SDR: u32 = 12_500_000;

/// Minimum core clock for I3C HDR mode (Hz)
/// Required to achieve 25 `MHz` SCL with proper timing margins
pub const I3C_MIN_CORE_CLK_HDR: u32 = 25_000_000;

/// Maximum supported core clock (Hz)
pub const I3C_MAX_CORE_CLK: u32 = 400_000_000;

impl I3cConfig {
    /// Validate clock configuration
    ///
    /// Checks that the configured clock frequencies are achievable per
    /// MIPI I3C specification timing requirements.
    ///
    /// # Timing Requirements (MIPI I3C Basic Spec v1.1.1)
    ///
    /// | Parameter | SDR Mode | HDR-DDR Mode | Unit |
    /// |-----------|----------|--------------|------|
    /// | fSCL max  | 12.5     | 12.5         | `MHz`  |
    /// | tLOW min  | 32       | 32           | ns   |
    /// | tHIGH min | 32       | 32           | ns   |
    ///
    /// For reliable operation, core clock should be at least 4x the SCL frequency
    /// to allow proper timing register resolution.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if configuration is valid
    /// - `Err(I3cError::InvalidParam)` if configuration is invalid
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = I3cConfig::new()
    ///     .core_clk_hz(200_000_000)
    ///     .i3c_scl_hz(12_500_000);
    ///
    /// config.validate_clock()?;
    /// ```
    pub fn validate_clock(&self) -> Result<(), I3cError> {
        if let Some(core_hz) = self.core_clk_hz {
            // Check core clock range
            if core_hz < I3C_MIN_CORE_CLK_SDR {
                return Err(I3cError::InvalidParam);
            }
            if core_hz > I3C_MAX_CORE_CLK {
                return Err(I3cError::InvalidParam);
            }

            // Check I3C SCL achievability (need ~4x core clock for timing resolution)
            if self.i3c_scl_hz > 0 && core_hz < self.i3c_scl_hz * 4 {
                return Err(I3cError::InvalidParam);
            }

            // Check I2C SCL achievability
            if self.i2c_scl_hz > 0 && core_hz < self.i2c_scl_hz * 4 {
                return Err(I3cError::InvalidParam);
            }
        }

        Ok(())
    }
}
