// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pure helper logic for SMC address and timing calculations.

use core::convert::TryFrom;

use crate::smc::types::ChipSelect;
use crate::smc::types::FlashConfig;
use crate::smc::types::SmcConfig;
use crate::smc::types::SmcError;

const SMC_WINDOW_SIZE_BYTES: usize = 256 * 1024 * 1024;
const DMA_MAX_TRANSFER_LENGTH: u32 = 0x20_0000; // 32MBytes
pub(crate) const SPI_CTRL_FREQ_MASK: u32 = 0x0F00_0F00;

/// DMA engine's base address for flash-side memory (fmc084 / spi084).
/// DMA flash address = flash_window_base[cs] - SPI_DMA_FLASH_MAP_BASE + cs_offset.
pub(crate) const SPI_DMA_FLASH_MAP_BASE: usize = 0x6000_0000;

/// DMA engine's base address for SRAM-side memory (fmc088 / spi088).
/// DMA DRAM address = physical_sram_addr + SPI_DMA_RAM_MAP_BASE.
pub(crate) const SPI_DMA_RAM_MAP_BASE: u32 = 0x8000_0000;

/// Written to spi080/fmc080 to request the DMA bus (sets DMAReq, bit 31).
/// On SPI1/SPI2 this asserts the hardware arbiter request line.
/// On FMC, bits 20–31 are Reserved — the write is a no-op.
/// Matches aspeed-rust `SPI_DMA_GET_REQ_MAGIC`.
pub(crate) const SPI_DMA_GET_REQ_MAGIC: u32 = 0xaeed_0000;

/// Written to spi080/fmc080 to release the DMA bus grant (DMADiscard).
/// On SPI1/SPI2 this releases the arbiter grant after DMA completes.
/// On FMC, bits 20–31 are Reserved — the write is a no-op.
/// Matches aspeed-rust `SPI_DMA_DISCARD_REQ_MAGIC`.
pub(crate) const SPI_DMA_DISCARD_REQ_MAGIC: u32 = 0xdeea_0000;

/// spi080 bit 31: DMA bus request is pending (SPI1/SPI2 only).
/// Reads as 0 on FMC (Reserved).
pub(crate) const SPI_DMA_REQUEST: u32 = 1 << 31;

/// spi080 bit 30: DMA bus grant is held (SPI1/SPI2 only).
/// Reads as 0 on FMC (Reserved).
pub(crate) const SPI_DMA_GRANT: u32 = 1 << 30;

pub(crate) const SPI_DMA_CALIB_MODE: u32 = 1 << 3;
pub(crate) const SPI_DMA_CALC_CKSUM: u32 = 1 << 2;
pub(crate) const SPI_DMA_ENABLE: u32 = 1 << 0;

/// Validated parameters for a DMA read operation, ready to be written to hardware registers.
///
/// Produced by [`validate_dma_read`] after bounds-checking and address translation.
/// All fields are in the format expected directly by the DMA engine registers.
pub(crate) struct ValidatedDmaRead {
    /// DMA engine flash-side address (written to fmc084 / spi084).
    ///
    /// Computed as `flash_window_base[cs] - SPI_DMA_FLASH_MAP_BASE + cs_offset`.
    pub flash_start: usize,
    /// DMA engine DRAM-side address (written to fmc088 / spi088).
    ///
    /// Must be 4-byte aligned
    pub dram_addr: u32,
    /// Value to write to the DMA length register (transfer length minus one).
    /// [0:24] 0x1F_FFFF:32Mbytes
    pub dma_len_reg: u32,
}

pub(crate) fn flash_capacity_bytes(config: Option<FlashConfig>) -> Result<usize, SmcError> {
    match config {
        Some(config) => (config.capacity_mb as usize)
            .checked_mul(1024 * 1024)
            .ok_or(SmcError::InvalidCapacity),
        None => Ok(0),
    }
}

pub(crate) fn cs_capacity_bytes(config: &SmcConfig, cs: ChipSelect) -> Result<usize, SmcError> {
    let slot = match cs {
        ChipSelect::Cs0 => config.cs0,
        ChipSelect::Cs1 => config.cs1,
    };
    match slot {
        Some(_) => flash_capacity_bytes(slot),
        None => Err(SmcError::InvalidChipSelect),
    }
}

pub(crate) fn total_capacity_bytes(
    cs0: Option<FlashConfig>,
    cs1: Option<FlashConfig>,
) -> Result<usize, SmcError> {
    let cs0_size = flash_capacity_bytes(cs0)?;
    let cs1_size = flash_capacity_bytes(cs1)?;
    let total = cs0_size
        .checked_add(cs1_size)
        .ok_or(SmcError::InvalidCapacity)?;
    if total > SMC_WINDOW_SIZE_BYTES {
        return Err(SmcError::InvalidCapacity);
    }
    Ok(total)
}

pub(crate) fn validate_mapped_range(
    offset: u32,
    len: usize,
    capacity_bytes: usize,
) -> Result<usize, SmcError> {
    let offset = offset as usize;
    let end = offset.checked_add(len).ok_or(SmcError::InvalidCapacity)?;
    if end > capacity_bytes {
        return Err(SmcError::InvalidCapacity);
    }
    Ok(offset)
}

/// Validate a DMA read request and compute DMA register values.
///
/// `flash_window_base`: per-CS AHB window base addresses from `Smc<Ready>`.
/// `cs0_capacity`: byte size of CS0; used to derive which CS the offset falls in.
///
/// The chip select is never accepted as a caller argument. The controller-relative
/// `flash_offset` encodes it implicitly: any offset in `[0, cs0_capacity)` names
/// CS0; any offset in `[cs0_capacity, capacity_bytes)` names CS1. See
/// EVD-20260511-dma-flash-offset-encodes-cs for the full rationale.
pub(crate) fn validate_dma_read(
    flash_offset: u32,
    flash_win_base: usize,
    cs_capacity: usize,
    dram_addr: usize,
    len: u32,
) -> Result<ValidatedDmaRead, SmcError> {
    if len == 0 || len > DMA_MAX_TRANSFER_LENGTH {
        return Err(SmcError::InvalidCapacity);
    }

    // Flash offset must be 4-byte aligned (spec §1.3, matching aspeed-rust).
    if flash_offset & 0x3 != 0 {
        return Err(SmcError::InvalidCapacity);
    }

    // Bounds-check the controller-relative offset.
    let _ = validate_mapped_range(flash_offset, len as usize, cs_capacity)?;

    // Compute the DMA engine's flash address (written to fmc084/spi084).
    // Formula from aspeed-rust fmccontroller.rs::read_dma:
    //   fmc084 = decode_addr[cs].start + op.address.value - SPI_DMA_FLASH_MAP_BASE
    //          = flash_window_base[cs] - SPI_DMA_FLASH_MAP_BASE + cs_offset
    let base = flash_win_base
        .checked_sub(SPI_DMA_FLASH_MAP_BASE)
        .ok_or(SmcError::InvalidCapacity)?;

    let flash_start = base
        .checked_add(flash_offset as usize)
        .ok_or(SmcError::InvalidCapacity)?;

    let dram_addr = u32::try_from(dram_addr).map_err(|_| SmcError::InvalidCapacity)?;
    if dram_addr & 0x3 != 0 {
        return Err(SmcError::InvalidCapacity);
    }

    Ok(ValidatedDmaRead {
        flash_start,
        dram_addr,
        dma_len_reg: len - 1,
    })
}

/// Encode a memory segment into hardware register format.
///
/// Hardware uses 512 KB units for addressing.
pub(crate) fn encode_segment(start: usize, end: usize) -> Result<u32, SmcError> {
    let start_512k = (start >> 19) as u32;
    let end_512k = ((end >> 19) - 1) as u32;

    if start_512k > 0xFFFF || end_512k > 0xFFFF {
        return Err(SmcError::InvalidCapacity);
    }

    Ok((end_512k << 16) | start_512k)
}

/// Calculate AST-compatible SPI clock divider field for CS control registers.
///
/// Ported from aspeed-rust's silicon-tested divisor search logic.
pub(crate) fn spi_freq_div(sysclk_mhz: u32, max_freq_mhz: u32) -> Result<u32, SmcError> {
    if max_freq_mhz == 0 {
        return Err(SmcError::HardwareError);
    }

    let div_arr = [15u32, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 0];

    for i in 0..0x0f {
        for (j, div_val) in div_arr.iter().copied().enumerate() {
            if i == 0 && j == 0 {
                continue;
            }

            let divisor = (j as u32) + 1 + (i * 16);
            let freq = sysclk_mhz / divisor;
            if max_freq_mhz >= freq {
                return Ok((i << 24) | (div_val << 8));
            }
        }
    }

    Ok(0)
}
/// Finds the midpoint of the longest consecutive sequence of 1's in a buffer.
///
/// Returns the midpoint index if the longest run is at least length 4,
/// otherwise returns -1.
///
/// # Arguments
/// * `buf` - slice of bytes (each should be 0 or 1).
pub(crate) fn get_mid_point_of_longest_one(buf: &[u8]) -> i32 {
    let mut start = 0;
    let mut mid_point = 0;
    let mut max_cnt = 0;
    let mut cnt = 0;

    for (i, &val) in buf.iter().enumerate() {
        if val == 1 {
            cnt += 1;
        } else {
            cnt = 0;
            start = i;
        }

        if cnt > max_cnt {
            max_cnt = cnt;
            mid_point = start + (cnt / 2);
        }
    }

    if max_cnt < 4 {
        return -1;
    } else {
        return i32::try_from(mid_point).unwrap();
    }
}

pub(crate) fn spi_calibration_enable(buf: &[u8]) -> Result<bool, SmcError> {
    if buf.len() < 4 {
        return Ok(false);
    }

    let mut valid_count = 0;

    // Process 4 bytes at a time
    for chunk in buf.chunks_exact(4) {
        // Convert 4 bytes to u32 in little-endian order
        let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);

        if word != 0 && word != 0xFFFF_FFFF {
            valid_count += 1;
        }
        if valid_count > 100 {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_segment() {
        let seg = encode_segment(0, 16 * 1024 * 1024).unwrap();
        let start_512k = seg & 0xFFFF;
        let end_512k = (seg >> 16) & 0xFFFF;
        assert_eq!(start_512k, 0);
        assert_eq!(end_512k, 31);
    }

    #[test]
    fn test_encode_segment_8mb_cs0() {
        let seg = encode_segment(0, 8 * 1024 * 1024).unwrap();
        let start_512k = seg & 0xFFFF;
        let end_512k = (seg >> 16) & 0xFFFF;
        assert_eq!(start_512k, 0);
        assert_eq!(end_512k, 15);
    }

    #[test]
    fn test_encode_segment_64mb_cs1_after_8mb_cs0() {
        let start = 8 * 1024 * 1024;
        let end = start + 64 * 1024 * 1024;
        let seg = encode_segment(start, end).unwrap();
        let start_512k = seg & 0xFFFF;
        let end_512k = (seg >> 16) & 0xFFFF;
        assert_eq!(start_512k, 16);
        assert_eq!(end_512k, 143);
    }

    fn decode_divisor(encoded: u32) -> u32 {
        let table = [15u32, 7, 14, 6, 13, 5, 12, 4, 11, 3, 10, 2, 9, 1, 8, 0];
        let hi = (encoded >> 24) & 0x0f;
        let lo = (encoded >> 8) & 0x0f;
        let j = table
            .iter()
            .position(|&v| v == lo)
            .expect("encoded lo nibble must exist in divisor table") as u32;
        j + 1 + (hi * 16)
    }

    #[test]
    fn test_spi_freq_div_25mhz_bound() {
        let encoded = spi_freq_div(200, 25).unwrap();
        let divisor = decode_divisor(encoded);
        assert_eq!(200 / divisor, 25);
        assert!(encoded & !SPI_CTRL_FREQ_MASK == 0);
    }

    #[test]
    fn test_spi_freq_div_50mhz_bound() {
        let encoded = spi_freq_div(200, 50).unwrap();
        let divisor = decode_divisor(encoded);
        assert_eq!(200 / divisor, 50);
        assert!(encoded & !SPI_CTRL_FREQ_MASK == 0);
    }

    #[test]
    fn test_spi_freq_div_rejects_zero_target() {
        assert!(spi_freq_div(200, 0).is_err());
    }

    #[test]
    fn test_segment_overflow() {
        let result = encode_segment(0, 512 * 1024 * 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_total_capacity_overflow() {
        let result = total_capacity_bytes(
            Some(FlashConfig {
                capacity_mb: 128,
                page_size: 256,
                sector_size: 4096,
                block_size: 65536,
                spi_clock_mhz: 25,
            }),
            Some(FlashConfig {
                capacity_mb: 129,
                page_size: 256,
                sector_size: 4096,
                block_size: 65536,
                spi_clock_mhz: 25,
            }),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mapped_range_accepts_exact_fit() {
        let offset = validate_mapped_range(4092, 4, 4096).unwrap();
        assert_eq!(offset, 4092);
    }

    #[test]
    fn test_validate_mapped_range_rejects_overflow() {
        let result = validate_mapped_range(u32::MAX, 8, SMC_WINDOW_SIZE_BYTES);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mapped_range_rejects_out_of_bounds() {
        let result = validate_mapped_range(1024, 1, 1024);
        assert!(result.is_err());
    }

    // FMC window base for tests: 0x8000_0000; SPI_DMA_FLASH_MAP_BASE: 0x6000_0000
    // Expected fmc084 for CS0 offset 0x1000: 0x8000_0000 - 0x6000_0000 + 0x1000 = 0x2000_1000
    const TEST_WINDOW: [usize; 2] = [0x8000_0000, 0x8100_0000]; // CS0=16MB, CS1 starts at +16MB
    const TEST_CS0_CAP: usize = 16 * 1024 * 1024;
    //const TEST_CAP: usize = 16 * 1024 * 1024;

    #[test]
    fn test_validate_dma_read_accepts_valid_request() {
        let validated =
            validate_dma_read(0x1000, TEST_WINDOW, TEST_CS0_CAP, 0x0008_0000, 512).unwrap();
        assert_eq!(validated.flash_start, 0x2000_1000); // 0x8000_0000 - 0x6000_0000 + 0x1000
        assert_eq!(validated.dram_addr, 0x0008_0000);
        assert_eq!(validated.dma_len_reg, 511);
    }

    #[test]
    fn test_validate_dma_read_cs1_region() {
        // Offset falls in CS1 region (past 16 MB CS0) on a 32 MB dual-CS controller.
        const DUAL_WINDOW: [usize; 2] = [0x8000_0000, 0x8100_0000];
        const CS0_CAP: usize = 16 * 1024 * 1024;
        const TOTAL_CAP: usize = 32 * 1024 * 1024;
        let cs1_offset = CS0_CAP + 0x1000;
        let validated =
            validate_dma_read(cs1_offset as u32, DUAL_WINDOW, CS0_CAP, 0x0008_0000, 512).unwrap();
        // fmc084 = 0x8100_0000 - 0x6000_0000 + 0x1000 = 0x2100_1000
        assert_eq!(validated.flash_start, 0x2100_1000);
        assert_eq!(validated.dma_len_reg, 511);
    }

    #[test]
    fn test_validate_dma_read_rejects_zero_length() {
        let result = validate_dma_read(0, TEST_WINDOW, TEST_CS0_CAP, 0x0008_0000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dma_read_rejects_unaligned_flash_offset() {
        let result = validate_dma_read(0x1001, TEST_WINDOW, TEST_CS0_CAP, 0x0008_0000, 256);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dma_read_rejects_unaligned_dram() {
        let result = validate_dma_read(0, TEST_WINDOW, TEST_CS0_CAP, 0x0008_0002, 256);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dma_read_rejects_masked_dram_bits() {
        let result = validate_dma_read(0, TEST_WINDOW, TEST_CS0_CAP, 0x1000_0000, 256);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dma_read_rejects_flash_range_overflow() {
        let result = validate_dma_read(0x00ff_ff00, TEST_WINDOW, TEST_CS0_CAP, 0x0008_0000, 0x200);
        assert!(result.is_err());
    }
}
