// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI NOR facade with shared logic for FMC and SPI1/SPI2 backends.

use core::ops::FnMut;
use core::result::Result;
use core::result::Result::{Err, Ok};

use crate::smc::fmc::FmcReady;
use crate::smc::spi::SpiReady;
use crate::smc::types::{AddressWidth, ChipSelect, FlashConfig, SmcError, TransferMode};

/// Build a command byte array: opcode followed by address bytes selected by
/// `width`. Returns a fixed-size buffer and the valid length.
///
/// The caller slices `&buf[..len]` to get the exact command bytes.
/// This mirrors aspeed-rust's explicit `AddressWidth` selection rather than
/// relying on implicit `to_be_bytes()[1..]` slicing.
fn encode_addr_cmd(opcode: u8, offset: u32, width: AddressWidth) -> ([u8; 5], usize) {
    let be = offset.to_be_bytes();
    match width {
        AddressWidth::None => {
            let mut buf = [0u8; 5];
            buf[0] = opcode;
            (buf, 1)
        }
        AddressWidth::ThreeByte => {
            let mut buf = [0u8; 5];
            buf[0] = opcode;
            buf[1] = be[1];
            buf[2] = be[2];
            buf[3] = be[3];
            (buf, 4)
        }
        AddressWidth::FourByte => {
            let mut buf = [0u8; 5];
            buf[0] = opcode;
            buf[1] = be[0];
            buf[2] = be[1];
            buf[3] = be[2];
            buf[4] = be[3];
            (buf, 5)
        }
    }
}

/// Minimal SPI NOR flash device API.
pub trait SpiNorFlashDevice {
    /// Read bytes from flash at `offset` into `buf`.
    fn read(&self, offset: u32, buf: &mut [u8]) -> Result<usize, SmcError>;

    /// Return configured flash capacity in bytes.
    fn capacity_bytes(&self) -> Result<usize, SmcError>;

    /// Erase one sector at `offset`.
    fn erase_sector(&mut self, offset: u32) -> Result<(), SmcError>;

    /// Program one page at `offset`.
    fn program_page(&mut self, offset: u32, data: &[u8]) -> Result<usize, SmcError>;

    /// Verify flash content against `expected` bytes.
    fn verify(&self, offset: u32, expected: &[u8]) -> Result<bool, SmcError>;

    /// Read status register.
    fn status(&self) -> Result<u8, SmcError>;

    /// Read JEDEC ID bytes (manufacturer + 2-byte device ID).
    fn jedec_id(&self) -> Result<[u8; 3], SmcError>;
}

/// Standard SPI NOR opcodes used by Phase 3B operations.
pub mod commands {
    pub const WRITE_ENABLE: u8 = 0x06;
    pub const ERASE_SECTOR_4K: u8 = 0x20;
    pub const ERASE_SECTOR_4K_4B: u8 = 0x21;
    pub const PAGE_PROGRAM: u8 = 0x02;
    pub const PAGE_PROGRAM_4B: u8 = 0x12;
    pub const READ_FAST: u8 = 0x0B;
    pub const READ_FAST_4B: u8 = 0x0C;
    pub const READ_STATUS: u8 = 0x05;
    pub const READ_ID: u8 = 0x9F;
}

const STATUS_WIP: u8 = 0x01;
const WRITE_COMPLETE_MAX_POLLS: u32 = 1_000_000;
const STATUS_POLL_SPIN_DELAY: u32 = 64;

/// Addressing policy for SPI NOR command transactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlashAddressingPolicy {
    /// Use 3-byte addressing command set.
    ThreeByteOnly,
    /// Use 4-byte addressing command set.
    FourByteCommands,
}

impl FlashAddressingPolicy {
    pub const fn addr_width(self) -> AddressWidth {
        match self {
            Self::ThreeByteOnly => AddressWidth::ThreeByte,
            Self::FourByteCommands => AddressWidth::FourByte,
        }
    }
}

/// SPI NOR opcode profile selected by addressing policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlashCommandProfile {
    pub read_fast: u8,
    pub page_program: u8,
    pub erase_sector_4k: u8,
    pub read_status: u8,
    pub write_enable: u8,
}

impl FlashCommandProfile {
    const fn for_addressing(policy: FlashAddressingPolicy) -> Self {
        match policy {
            FlashAddressingPolicy::ThreeByteOnly => Self {
                read_fast: commands::READ_FAST,
                page_program: commands::PAGE_PROGRAM,
                erase_sector_4k: commands::ERASE_SECTOR_4K,
                read_status: commands::READ_STATUS,
                write_enable: commands::WRITE_ENABLE,
            },
            FlashAddressingPolicy::FourByteCommands => Self {
                read_fast: commands::READ_FAST_4B,
                page_program: commands::PAGE_PROGRAM_4B,
                erase_sector_4k: commands::ERASE_SECTOR_4K_4B,
                read_status: commands::READ_STATUS,
                write_enable: commands::WRITE_ENABLE,
            },
        }
    }
}

/// Compare `expected` against bytes produced by `read`, in chunks of at most
/// `chunk` bytes. `read(offset, dst)` fills `dst` with bytes at the given
/// device-local offset. Returns `Ok(true)` on full equality, `Ok(false)` on the
/// first mismatch, and propagates any error from `read`. Bounded scratch use:
/// the comparison is performed in stack-resident chunks (`chunk` bytes, capped
/// at 256) regardless of `expected.len()`.
fn compare_chunked<F>(
    mut read: F,
    offset: u32,
    expected: &[u8],
    chunk: usize,
) -> Result<bool, SmcError>
where
    F: FnMut(u32, &mut [u8]) -> Result<usize, SmcError>,
{
    let mut scratch = [0u8; 256];
    let step = core::cmp::min(chunk, scratch.len());
    if step == 0 {
        return Err(SmcError::InvalidCapacity);
    }
    let mut cursor = 0usize;
    while cursor < expected.len() {
        let n = core::cmp::min(step, expected.len() - cursor);
        let chunk_offset = offset
            .checked_add(cursor as u32)
            .ok_or(SmcError::InvalidCapacity)?;
        read(chunk_offset, &mut scratch[..n])?;
        if scratch[..n] != expected[cursor..cursor + n] {
            return Ok(false);
        }
        cursor += n;
    }
    Ok(true)
}

fn poll_delay() {
    for _ in 0..STATUS_POLL_SPIN_DELAY {
        core::hint::spin_loop();
    }
}

enum FlashBackend<'a> {
    Fmc(&'a FmcReady),
    Spi(&'a SpiReady),
}

/// Decoded JEDEC identifier returned by `READ_ID` (`0x9F`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JedecId {
    pub manufacturer: u8,
    pub memory_type: u8,
    pub capacity_code: u8,
}

impl JedecId {
    #[must_use]
    pub const fn from_bytes(raw: [u8; 3]) -> Self {
        Self {
            manufacturer: raw[0],
            memory_type: raw[1],
            capacity_code: raw[2],
        }
    }

    #[must_use]
    pub const fn as_bytes(self) -> [u8; 3] {
        [self.manufacturer, self.memory_type, self.capacity_code]
    }
}

fn expect_jedec_match(actual: JedecId, expected: JedecId) -> Result<JedecId, SmcError> {
    if actual != expected {
        return Err(SmcError::HardwareError);
    }
    Ok(actual)
}

/// Wrapper-aware SPI NOR flash facade.
pub struct SpiNorFlash<'a> {
    backend: FlashBackend<'a>,
    // Validated metadata for Phase 3B alignment/policy checks.
    cfg: FlashConfig,
    /// Chip select this flash device sits on.
    cs: ChipSelect,
    /// IO mode used for all SPI command transactions (WREN, RDSR, PP, SE).
    /// Defaults to `Mode111` (single-wire cmd/addr/data).
    cmd_mode: TransferMode,
    /// Addressing policy selected for SPI NOR command construction.
    addressing_policy: FlashAddressingPolicy,
    /// Opcodes selected for the current addressing policy.
    command_profile: FlashCommandProfile,
}

impl<'a> SpiNorFlash<'a> {
    /// Build a flash facade from an initialized FMC controller wrapper.
    pub fn from_fmc(fmc: &'a mut FmcReady, cfg: FlashConfig) -> Result<Self, SmcError> {
        Self::from_fmc_cs(fmc, cfg, ChipSelect::Cs0)
    }

    /// Build a flash facade from an initialized FMC controller wrapper with explicit CS.
    pub fn from_fmc_cs(
        fmc: &'a mut FmcReady,
        cfg: FlashConfig,
        cs: ChipSelect,
    ) -> Result<Self, SmcError> {
        let addressing_policy = Self::default_addressing_for_cfg(cfg);
        Self::validate_capacity_cfg(cfg, fmc.cs_config(cs)?)?;
        Ok(Self {
            backend: FlashBackend::Fmc(fmc),
            cfg,
            cs,
            cmd_mode: TransferMode::Mode111,
            addressing_policy,
            command_profile: FlashCommandProfile::for_addressing(addressing_policy),
        })
    }

    /// Build a flash facade from an initialized SPI1/SPI2 controller wrapper.
    pub fn from_spi(spi: &'a mut SpiReady, cfg: FlashConfig) -> Result<Self, SmcError> {
        Self::from_spi_cs(spi, cfg, ChipSelect::Cs0)
    }

    /// Build a flash facade from an initialized SPI1/SPI2 controller wrapper with explicit CS.
    pub fn from_spi_cs(
        spi: &'a mut SpiReady,
        cfg: FlashConfig,
        cs: ChipSelect,
    ) -> Result<Self, SmcError> {
        let addressing_policy = Self::default_addressing_for_cfg(cfg);
        Self::validate_capacity_cfg(cfg, spi.cs_config(cs)?)?;
        Ok(Self {
            backend: FlashBackend::Spi(spi),
            cfg,
            cs,
            cmd_mode: TransferMode::Mode111,
            addressing_policy,
            command_profile: FlashCommandProfile::for_addressing(addressing_policy),
        })
    }

    /// Override the IO mode used for all SPI command transactions.
    ///
    /// Applies to WREN, RDSR, PAGE_PROGRAM, and SECTOR_ERASE paths.
    /// The memory-mapped read path always uses the segment-routed AHB window
    /// and is unaffected by this setting.
    pub fn with_cmd_mode(mut self, mode: TransferMode) -> Self {
        self.cmd_mode = mode;
        self
    }

    /// Override the addressing policy used for command-profile selection.
    pub fn with_addressing_policy(mut self, policy: FlashAddressingPolicy) -> Self {
        self.addressing_policy = policy;
        self.command_profile = FlashCommandProfile::for_addressing(policy);
        self
    }

    /// Read and decode the three-byte JEDEC ID tuple.
    pub fn jedec(&self) -> Result<JedecId, SmcError> {
        Ok(JedecId::from_bytes(self.read_jedec_id_impl()?))
    }

    /// Read JEDEC ID and require an exact match.
    pub fn expect_jedec(&self, expected: JedecId) -> Result<JedecId, SmcError> {
        expect_jedec_match(self.jedec()?, expected)
    }

    /// Program an arbitrary-length buffer by issuing page-sized writes.
    ///
    /// The starting `offset` must be page-aligned. The final chunk may be
    /// shorter than a full page.
    pub fn program(&mut self, offset: u32, data: &[u8]) -> Result<usize, SmcError> {
        if data.is_empty() {
            return Ok(0);
        }

        let page_size = self.cfg.page_size as usize;
        if page_size == 0 || (offset as usize) % page_size != 0 {
            return Err(SmcError::InvalidCapacity);
        }
        self.validate_range(offset, data.len())?;

        let mut written = 0usize;
        while written < data.len() {
            let chunk_len = core::cmp::min(page_size, data.len() - written);
            let chunk_offset = offset
                .checked_add(written as u32)
                .ok_or(SmcError::InvalidCapacity)?;
            self.program_page(chunk_offset, &data[written..written + chunk_len])?;
            written += chunk_len;
        }

        Ok(written)
    }

    /// Erase all sectors intersecting the requested byte range.
    ///
    /// The erased interval is rounded outward to sector boundaries.
    pub fn erase_range(&mut self, offset: u32, len: usize) -> Result<(), SmcError> {
        if len == 0 {
            return Ok(());
        }

        self.validate_range(offset, len)?;

        let sector_size = self.cfg.sector_size as usize;
        if sector_size == 0 {
            return Err(SmcError::InvalidCapacity);
        }

        let start = (offset as usize / sector_size) * sector_size;
        let end = (offset as usize)
            .checked_add(len)
            .ok_or(SmcError::InvalidCapacity)?;
        let end_aligned = end
            .checked_add(sector_size - 1)
            .ok_or(SmcError::InvalidCapacity)?
            / sector_size
            * sector_size;

        let mut current = start;
        while current < end_aligned {
            self.erase_sector(current as u32)?;
            current += sector_size;
        }

        Ok(())
    }

    /// Perform a correctness-first flash update of the requested region.
    ///
    /// This helper erases all sectors intersecting the target range, programs
    /// the supplied bytes page-by-page, then verifies the written contents.
    /// Returns the number of bytes programmed on success.
    pub fn update_region(&mut self, offset: u32, data: &[u8]) -> Result<usize, SmcError> {
        if data.is_empty() {
            return Ok(0);
        }

        self.erase_range(offset, data.len())?;
        let written = self.program(offset, data)?;
        if !self.verify(offset, data)? {
            return Err(SmcError::HardwareError);
        }

        Ok(written)
    }

    fn validate_capacity_cfg(cfg: FlashConfig, expected: FlashConfig) -> Result<(), SmcError> {
        if cfg != expected {
            return Err(SmcError::InvalidCapacity);
        }
        Ok(())
    }

    fn default_addressing_for_cfg(cfg: FlashConfig) -> FlashAddressingPolicy {
        if cfg.capacity_mb > 16 {
            FlashAddressingPolicy::FourByteCommands
        } else {
            FlashAddressingPolicy::ThreeByteOnly
        }
    }

    pub fn addr_width(&self) -> AddressWidth {
        self.addressing_policy.addr_width()
    }

    pub fn command_profile(&self) -> FlashCommandProfile {
        self.command_profile
    }

    /// Validate a device-local offset before handing it to the controller.
    ///
    /// `FmcReady::read` / `SpiReady::read` already select the per-CS AHB
    /// window from the chip-select argument, so offsets stay CS-local here.
    fn device_to_controller_offset(&self, device_offset: u32) -> Result<u32, SmcError> {
        let cs_cap = self.capacity_bytes()?;
        if (device_offset as usize) >= cs_cap {
            return Err(SmcError::InvalidCapacity);
        }
        Ok(device_offset)
    }

    fn validate_range(&self, offset: u32, len: usize) -> Result<(), SmcError> {
        let start = offset as usize;
        let end = start.checked_add(len).ok_or(SmcError::InvalidCapacity)?;
        if end > self.capacity_bytes()? {
            return Err(SmcError::InvalidCapacity);
        }
        Ok(())
    }

    fn validate_sector_erase(&self, offset: u32) -> Result<(), SmcError> {
        let sector_size = self.cfg.sector_size as usize;
        if sector_size == 0 || (offset as usize) % sector_size != 0 {
            return Err(SmcError::InvalidCapacity);
        }
        self.validate_range(offset, sector_size)
    }

    fn validate_page_program(&self, offset: u32, data: &[u8]) -> Result<(), SmcError> {
        let page_size = self.cfg.page_size as usize;
        if page_size == 0 || data.is_empty() || data.len() > page_size {
            return Err(SmcError::InvalidCapacity);
        }
        if (offset as usize) % page_size != 0 {
            return Err(SmcError::InvalidCapacity);
        }
        self.validate_range(offset, data.len())
    }

    fn issue_command(&mut self, cmd: &[u8], payload: &[u8]) -> Result<(), SmcError> {
        let cs = self.cs;
        let mode = self.cmd_mode;
        match &self.backend {
            FlashBackend::Fmc(fmc) => fmc.transceive_user(cs, cmd, payload, &mut [], mode),
            FlashBackend::Spi(spi) => spi.transceive_user(cs, cmd, payload, &mut [], mode),
        }
    }

    fn read_status_impl(&self) -> Result<u8, SmcError> {
        let cs = self.cs;
        let mode = self.cmd_mode;
        let opcode = self.command_profile().read_status;
        let mut status = [0u8; 1];
        match &self.backend {
            FlashBackend::Fmc(fmc) => fmc.transceive_user(cs, &[opcode], &[], &mut status, mode)?,
            FlashBackend::Spi(spi) => spi.transceive_user(cs, &[opcode], &[], &mut status, mode)?,
        }
        Ok(status[0])
    }

    fn read_jedec_id_impl(&self) -> Result<[u8; 3], SmcError> {
        let cs = self.cs;
        let mode = self.cmd_mode;
        let mut id = [0u8; 3];
        match &self.backend {
            FlashBackend::Fmc(fmc) => {
                fmc.transceive_user(cs, &[commands::READ_ID], &[], &mut id, mode)?
            }
            FlashBackend::Spi(spi) => {
                spi.transceive_user(cs, &[commands::READ_ID], &[], &mut id, mode)?
            }
        }
        Ok(id)
    }

    fn write_enable(&mut self) -> Result<(), SmcError> {
        let profile = self.command_profile();
        self.issue_command(&[profile.write_enable], &[])
    }

    fn wait_write_complete(&self, max_polls: u32) -> Result<(), SmcError> {
        let mut polls = 0u32;
        while polls < max_polls {
            let sr = self.read_status_impl()?;
            if (sr & STATUS_WIP) == 0 {
                return Ok(());
            }
            poll_delay();
            polls += 1;
        }
        Err(SmcError::Timeout)
    }
}

impl SpiNorFlashDevice for SpiNorFlash<'_> {
    fn read(&self, offset: u32, buf: &mut [u8]) -> Result<usize, SmcError> {
        // Bounds-check against the selected CS's capacity, then translate to
        // the controller-window address before issuing the segment-routed read.
        self.validate_range(offset, buf.len())?;
        let translated = self.device_to_controller_offset(offset)?;
        let cs = self.cs;
        //TODO: need to add dma_read variant if buf.len() is above some threshold
        // and dma is enabled
        match &self.backend {
            FlashBackend::Fmc(fmc) => fmc.read(cs, translated, buf),
            FlashBackend::Spi(spi) => spi.read(cs, translated, buf),
        }
    }

    fn capacity_bytes(&self) -> Result<usize, SmcError> {
        match &self.backend {
            FlashBackend::Fmc(fmc) => fmc.cs_capacity_bytes(self.cs),
            FlashBackend::Spi(spi) => spi.cs_capacity_bytes(self.cs),
        }
    }

    fn erase_sector(&mut self, offset: u32) -> Result<(), SmcError> {
        self.validate_sector_erase(offset)?;

        let profile = self.command_profile();
        let width = self.addr_width();
        self.write_enable()?;
        let (cmd, len) = encode_addr_cmd(profile.erase_sector_4k, offset, width);
        self.issue_command(&cmd[..len], &[])?;
        self.wait_write_complete(WRITE_COMPLETE_MAX_POLLS)
    }

    fn program_page(&mut self, offset: u32, data: &[u8]) -> Result<usize, SmcError> {
        self.validate_page_program(offset, data)?;

        let profile = self.command_profile();
        let width = self.addr_width();
        self.write_enable()?;
        let (cmd, len) = encode_addr_cmd(profile.page_program, offset, width);
        self.issue_command(&cmd[..len], data)?;
        self.wait_write_complete(WRITE_COMPLETE_MAX_POLLS)?;
        Ok(data.len())
    }

    fn verify(&self, offset: u32, expected: &[u8]) -> Result<bool, SmcError> {
        self.validate_range(offset, expected.len())?;
        compare_chunked(|o, b| self.read(o, b), offset, expected, 256)
    }

    fn status(&self) -> Result<u8, SmcError> {
        self.read_status_impl()
    }

    fn jedec_id(&self) -> Result<[u8; 3], SmcError> {
        self.read_jedec_id_impl()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        commands, compare_chunked, encode_addr_cmd, expect_jedec_match, FlashAddressingPolicy,
        FlashCommandProfile, JedecId, SpiNorFlash,
    };
    use crate::smc::types::{AddressWidth, FlashConfig, SmcError};

    #[test]
    fn encode_addr_cmd_none_emits_opcode_only() {
        let (buf, len) = encode_addr_cmd(0x06, 0x1234_5678, AddressWidth::None);
        assert_eq!(len, 1);
        assert_eq!(&buf[..len], &[0x06]);
    }

    #[test]
    fn encode_addr_cmd_three_byte_emits_low_24_bits() {
        let (buf, len) = encode_addr_cmd(0x20, 0x1234_5678, AddressWidth::ThreeByte);
        assert_eq!(len, 4);
        assert_eq!(&buf[..len], &[0x20, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn encode_addr_cmd_four_byte_emits_full_32_bits() {
        let (buf, len) = encode_addr_cmd(0x13, 0x1234_5678, AddressWidth::FourByte);
        assert_eq!(len, 5);
        assert_eq!(&buf[..len], &[0x13, 0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn command_profile_three_byte_uses_legacy_opcodes() {
        let profile = FlashCommandProfile::for_addressing(FlashAddressingPolicy::ThreeByteOnly);
        assert_eq!(profile.read_fast, commands::READ_FAST);
        assert_eq!(profile.page_program, commands::PAGE_PROGRAM);
        assert_eq!(profile.erase_sector_4k, commands::ERASE_SECTOR_4K);
        assert_eq!(profile.read_status, commands::READ_STATUS);
        assert_eq!(profile.write_enable, commands::WRITE_ENABLE);
    }

    #[test]
    fn command_profile_four_byte_uses_4b_opcodes() {
        let profile = FlashCommandProfile::for_addressing(FlashAddressingPolicy::FourByteCommands);
        assert_eq!(profile.read_fast, commands::READ_FAST_4B);
        assert_eq!(profile.page_program, commands::PAGE_PROGRAM_4B);
        assert_eq!(profile.erase_sector_4k, commands::ERASE_SECTOR_4K_4B);
        assert_eq!(profile.read_status, commands::READ_STATUS);
        assert_eq!(profile.write_enable, commands::WRITE_ENABLE);
    }

    #[test]
    fn default_addressing_policy_derives_from_capacity() {
        assert_eq!(
            SpiNorFlash::default_addressing_for_cfg(FlashConfig {
                capacity_mb: 8,
                page_size: 256,
                sector_size: 4096,
                block_size: 65536,
                spi_clock_mhz: 25,
            }),
            FlashAddressingPolicy::ThreeByteOnly
        );

        assert_eq!(
            SpiNorFlash::default_addressing_for_cfg(FlashConfig {
                capacity_mb: 32,
                page_size: 256,
                sector_size: 4096,
                block_size: 65536,
                spi_clock_mhz: 25,
            }),
            FlashAddressingPolicy::FourByteCommands
        );
    }

    fn fixture_1kb() -> [u8; 1024] {
        let mut out = [0u8; 1024];
        let mut i = 0;
        while i < out.len() {
            out[i] = (i as u8).wrapping_mul(31).wrapping_add(7);
            i += 1;
        }
        out
    }

    #[test]
    fn compare_chunked_matches_buffer_above_scratch_size() {
        let expected = fixture_1kb();
        let backing = expected;
        let read = |offset: u32, dst: &mut [u8]| -> Result<usize, SmcError> {
            let off = offset as usize;
            dst.copy_from_slice(&backing[off..off + dst.len()]);
            Ok(dst.len())
        };
        assert_eq!(compare_chunked(read, 0, &expected, 256), Ok(true));
    }

    #[test]
    fn compare_chunked_detects_mismatch_in_later_chunk() {
        let expected = fixture_1kb();
        let mut backing = expected;
        backing[800] = backing[800].wrapping_add(1);
        let read = |offset: u32, dst: &mut [u8]| -> Result<usize, SmcError> {
            let off = offset as usize;
            dst.copy_from_slice(&backing[off..off + dst.len()]);
            Ok(dst.len())
        };
        assert_eq!(compare_chunked(read, 0, &expected, 256), Ok(false));
    }

    #[test]
    fn compare_chunked_propagates_read_error() {
        let expected = fixture_1kb();
        let read =
            |_offset: u32, _dst: &mut [u8]| -> Result<usize, SmcError> { Err(SmcError::Timeout) };
        assert_eq!(
            compare_chunked(read, 0, &expected, 256),
            Err(SmcError::Timeout)
        );
    }

    #[test]
    fn expect_jedec_match_returns_actual_on_exact_match() {
        let jedec = JedecId::from_bytes([0xEF, 0x40, 0x18]);
        assert_eq!(expect_jedec_match(jedec, jedec), Ok(jedec));
    }

    #[test]
    fn expect_jedec_match_rejects_mismatch() {
        let expected = JedecId::from_bytes([0xEF, 0x40, 0x18]);
        let actual = JedecId::from_bytes([0xC2, 0x20, 0x19]);
        assert_eq!(
            expect_jedec_match(actual, expected),
            Err(SmcError::HardwareError)
        );
    }
}
