// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Target-agnostic JEDEC JESD216 (SFDP) decoder.
//!
//! Pure byte logic: callers perform the SPI reads (opcode `0x5A`) and hand the
//! bytes here. Transcribed from opentitanlib's `spiflash/sfdp.rs`, cross-checked
//! against Zephyr's `jesd216_bfp_*` helpers, with `no_std` byte reads and
//! `util_error` codes in place of `std::io`/`thiserror`.

#![no_std]

use util_error::{
    ErrorCode, FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY, FLASH_GENERIC_SFDP_INVALID_SIGNATURE,
    FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND, FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT,
    FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV,
    FLASH_GENERIC_SFDP_UNSUPPORTED_PARAMS_MAJOR_REV,
};

/// SFDP signature: ASCII "SFDP" read little-endian from the first dword.
pub const SFDP_SIGNATURE: u32 = 0x5044_4653;
/// Length in bytes of the SFDP header.
pub const SFDP_HEADER_LEN: usize = 8;
/// Length in bytes of one parameter header.
pub const PARAM_HEADER_LEN: usize = 8;
/// The only header/parameter major revision this decoder understands.
pub const SUPPORTED_MAJOR_REV: u8 = 1;
/// Parameter-header ID (LSB/MSB) of the mandatory JEDEC Basic Flash table.
pub const BFP_ID_LSB: u8 = 0x00;
pub const BFP_ID_MSB: u8 = 0xFF;
/// Maximum number of erase types a BFP table defines (DW8/DW9).
pub const NUM_ERASE_TYPES: usize = 4;

/// Read a little-endian u32 from a 4-byte window at `off`.
const fn le_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

/// Extract a `size`-bit field at `offset` from `word` (size < 32).
const fn field(word: u32, offset: u32, size: u32) -> u32 {
    (word >> offset) & ((1u32 << size) - 1)
}

/// The 8-byte SFDP header: signature, revision, and parameter-header count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfdpHeader {
    pub signature: u32,
    pub minor: u8,
    pub major: u8,
    /// Number of parameter headers minus one (as stored on the wire).
    pub nph: u8,
}

impl SfdpHeader {
    /// Parse the SFDP header from the first [`SFDP_HEADER_LEN`] bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.len() < SFDP_HEADER_LEN {
            return Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT);
        }
        let signature = le_u32(bytes, 0);
        if signature != SFDP_SIGNATURE {
            return Err(FLASH_GENERIC_SFDP_INVALID_SIGNATURE);
        }
        let major = bytes[5];
        if major != SUPPORTED_MAJOR_REV {
            return Err(FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV);
        }
        Ok(Self {
            signature,
            minor: bytes[4],
            major,
            nph: bytes[6],
        })
    }

    /// Number of parameter headers that follow the SFDP header.
    pub fn num_param_headers(&self) -> usize {
        self.nph as usize + 1
    }
}

/// One 8-byte parameter header: identifies a table and points at its dwords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterHeader {
    pub id_lsb: u8,
    pub minor: u8,
    pub major: u8,
    /// Table length in 32-bit dwords.
    pub dwords: u8,
    /// 24-bit byte offset of the table within SFDP space.
    pub pointer: u32,
    pub id_msb: u8,
}

impl ParameterHeader {
    /// Parse one parameter header from an 8-byte window.
    pub fn parse(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.len() < PARAM_HEADER_LEN {
            return Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT);
        }
        let major = bytes[2];
        if major != SUPPORTED_MAJOR_REV {
            return Err(FLASH_GENERIC_SFDP_UNSUPPORTED_PARAMS_MAJOR_REV);
        }
        let pointer = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], 0]);
        Ok(Self {
            id_lsb: bytes[0],
            minor: bytes[1],
            major,
            dwords: bytes[3],
            pointer,
            id_msb: bytes[7],
        })
    }

    /// True for the mandatory JEDEC Basic Flash Parameter table.
    pub fn is_basic_flash(&self) -> bool {
        self.id_lsb == BFP_ID_LSB && self.id_msb == BFP_ID_MSB
    }
}

/// A single supported erase operation from BFP DW8/DW9.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EraseType {
    /// Erase opcode.
    pub opcode: u8,
    /// Erased size in bytes (`2^exp`).
    pub size: u32,
}

/// Decoded Basic Flash Parameter table.
///
/// Holds a borrow of the raw table bytes; accessors decode fields on demand.
#[derive(Clone, Copy, Debug)]
pub struct BasicFlashParams<'a> {
    data: &'a [u8],
    dwords: usize,
}

impl<'a> BasicFlashParams<'a> {
    /// Parse the BFP table. `data` must hold at least the density dword (DW2).
    pub fn parse(data: &'a [u8]) -> Result<Self, ErrorCode> {
        let dwords = data.len() / 4;
        if dwords < 2 {
            return Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT);
        }
        Ok(Self { data, dwords })
    }

    /// Read the 1-based dword `idx` if present.
    fn dword(&self, idx: usize) -> Option<u32> {
        if idx == 0 || idx > self.dwords {
            return None;
        }
        Some(le_u32(self.data, (idx - 1) * 4))
    }

    /// Device density in bits, decoded from DW2 (JESD216 rule).
    pub fn density_bits(&self) -> Result<u64, ErrorCode> {
        let dw2 = self
            .dword(2)
            .ok_or(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT)?;
        if dw2 & (1 << 31) != 0 {
            let exp = field(dw2, 0, 31);
            if exp >= 64 {
                return Err(FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY);
            }
            Ok(1u64 << exp)
        } else {
            Ok(1u64 + dw2 as u64)
        }
    }

    /// Device capacity in bytes (density / 8).
    pub fn capacity_bytes(&self) -> Result<u64, ErrorCode> {
        Ok(self.density_bits()? / 8)
    }

    /// Programmable page size in bytes from DW11; defaults to 256 pre-JESD216A.
    pub fn page_size(&self) -> u32 {
        match self.dword(11) {
            Some(dw11) => 1u32 << field(dw11, 4, 4),
            None => 256,
        }
    }

    /// Decode erase type `idx` (1..=4) from DW8/DW9; `None` if unused/absent.
    pub fn erase_type(&self, idx: usize) -> Option<EraseType> {
        if idx == 0 || idx > NUM_ERASE_TYPES {
            return None;
        }
        // Types 1,2 live in DW8; types 3,4 in DW9. Even indices are the upper half.
        let dw = self.dword(8 + (idx - 1) / 2)?;
        let half = if idx % 2 == 0 { dw >> 16 } else { dw };
        let exp = (half & 0xFF) as u8;
        if exp == 0 {
            return None;
        }
        Some(EraseType {
            opcode: ((half >> 8) & 0xFF) as u8,
            size: 1u32 << exp,
        })
    }

    /// Smallest supported erase (the erasable "sector"), if any.
    pub fn smallest_erase(&self) -> Option<EraseType> {
        (1..=NUM_ERASE_TYPES)
            .filter_map(|i| self.erase_type(i))
            .min_by_key(|e| e.size)
    }

    /// Largest supported erase (the erasable "block"), if any.
    pub fn largest_erase(&self) -> Option<EraseType> {
        (1..=NUM_ERASE_TYPES)
            .filter_map(|i| self.erase_type(i))
            .max_by_key(|e| e.size)
    }
}

/// Geometry a controller needs to configure a flash device, all derived from
/// SFDP. Fields SFDP does not encode (e.g. desired clock) are the caller's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlashGeometry {
    pub capacity_bytes: u64,
    pub page_size: u32,
    pub sector_size: u32,
    pub block_size: u32,
}

/// Locate the JEDEC Basic Flash parameter header, given the already-read SFDP
/// `header` (>= 8 bytes) and the `param_headers` block (`num_param_headers` * 8
/// bytes). The returned `pointer`/`dwords` tell the caller what BFP bytes to
/// read next; this is pure decode, no I/O.
pub fn find_bfp_header(header: &[u8], param_headers: &[u8]) -> Result<ParameterHeader, ErrorCode> {
    let hdr = SfdpHeader::parse(header)?;
    let count = hdr.num_param_headers();
    if param_headers.len() < count * PARAM_HEADER_LEN {
        return Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT);
    }
    for i in 0..count {
        let ph = ParameterHeader::parse(&param_headers[i * PARAM_HEADER_LEN..])?;
        if ph.is_basic_flash() {
            return Ok(ph);
        }
    }
    Err(FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND)
}

/// Assemble [`FlashGeometry`] from a parsed BFP table.
pub fn geometry_from_bfp(bfp: &BasicFlashParams) -> Result<FlashGeometry, ErrorCode> {
    let capacity_bytes = bfp.capacity_bytes()?;
    let sector = bfp
        .smallest_erase()
        .ok_or(FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND)?;
    let block = bfp
        .largest_erase()
        .ok_or(FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND)?;
    Ok(FlashGeometry {
        capacity_bytes,
        page_size: bfp.page_size(),
        sector_size: sector.size,
        block_size: block.size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each fixture below is a hand-built SFDP image whose every decoded field is
    // justified against JEDEC JESD216 (the SFDP standard)
    // Only the DWORDs this decoder reads are meaningful
    // (DW2 density, DW8/DW9 erase types, DW11 page size); DWORDs it never touches
    // are left zero. Encodings used below, per JESD216 Basic Flash Parameter table:
    //   * DW2 density: if bit 31 is clear, density_bits = value + 1 (linear form);
    //     if bit 31 is set, density_bits = 1 << (value & 0x7FFF_FFFF) (power-of-two).
    //     capacity_bytes = density_bits / 8.
    //   * DW11 page size: bits [7:4] are an exponent; page_size = 1 << exp. Absent
    //     (table shorter than 11 dwords) it defaults to 256 (pre-JESD216A rule).
    //   * DW8/DW9 erase types: each 16-bit half is `opcode << 8 | size_exponent`,
    //     erased size = 1 << exponent; exponent 0 marks an unused slot. Smallest
    //     erase -> sector_size, largest -> block_size.

    /// Write the 8-byte SFDP header: signature, revision, parameter-header count.
    fn write_header(buf: &mut [u8], nph_minus_1: u8) {
        buf[0..4].copy_from_slice(&SFDP_SIGNATURE.to_le_bytes());
        buf[4] = 0x06; // minor rev (not decoded beyond major)
        buf[5] = SUPPORTED_MAJOR_REV; // major rev = 1
        buf[6] = nph_minus_1; // number of parameter headers minus one
        buf[7] = 0xFF; // reserved
    }

    /// Build one 8-byte parameter header pointing at `dwords` of table at `ptr`.
    fn param_header(id_lsb: u8, id_msb: u8, dwords: u8, ptr: u32) -> [u8; 8] {
        let p = ptr.to_le_bytes();
        [
            id_lsb,
            0x06,
            SUPPORTED_MAJOR_REV,
            dwords,
            p[0],
            p[1],
            p[2],
            id_msb,
        ]
    }

    /// Write little-endian `dwords` into `buf` starting at byte offset `at`.
    fn put_dwords(buf: &mut [u8], at: usize, dwords: &[u32]) {
        for (i, w) in dwords.iter().enumerate() {
            buf[at + i * 4..at + i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
    }

    /// Drive the full caller path from a raw image: parse the header, walk the
    /// parameter headers to the BFP, then read its table at the advertised pointer.
    fn decode_geometry(image: &[u8]) -> Result<FlashGeometry, ErrorCode> {
        let header = &image[0..SFDP_HEADER_LEN];
        let hdr = SfdpHeader::parse(header)?;
        let count = hdr.num_param_headers();
        let params = &image[SFDP_HEADER_LEN..SFDP_HEADER_LEN + count * PARAM_HEADER_LEN];
        let bfp = find_bfp_header(header, params)?;
        let start = bfp.pointer as usize;
        let end = start + bfp.dwords as usize * 4;
        geometry_from_bfp(&BasicFlashParams::parse(&image[start..end])?)
    }

    #[test]
    fn linear_16mib_page256_three_erases() {
        // 128 Mbit part, linear density: DW2 = 128 Mbit - 1 = 0x07FF_FFFF ->
        // capacity 16 MiB. 256-byte page, 4K/32K/64K erases (common W25Q128 shape).
        let mut img = [0u8; 60];
        write_header(&mut img, 0); // one parameter header
        img[8..16].copy_from_slice(&param_header(BFP_ID_LSB, BFP_ID_MSB, 11, 16));
        put_dwords(
            &mut img,
            16,
            &[
                0,
                0x07FF_FFFF, // DW2  density: 128 Mbit - 1 (linear)
                0,
                0,
                0,
                0,
                0,
                0x520F_200C, // DW8  type1 4K/0x20, type2 32K/0x52
                0x0000_D810, // DW9  type3 64K/0xD8, type4 unused
                0,
                0x0000_0080, // DW11 page exp 8 -> 256
            ],
        );
        let g = decode_geometry(&img).unwrap();
        assert_eq!(g.capacity_bytes, 16 * 1024 * 1024);
        assert_eq!(g.page_size, 256);
        assert_eq!(g.sector_size, 4096); // smallest erase
        assert_eq!(g.block_size, 65536); // largest erase
    }

    #[test]
    fn pow2_4gbit_page512_two_erases() {
        // 4 Gbit part exercising the power-of-two density form: DW2 bit 31 set,
        // exp = 32 -> 2^32 bits = 512 MiB. Varies the page size (512) and omits
        // the 32K erase, so DW8's upper half is the "unused" slot (exponent 0)
        // between two real types.
        let mut img = [0u8; 60];
        write_header(&mut img, 0);
        img[8..16].copy_from_slice(&param_header(BFP_ID_LSB, BFP_ID_MSB, 11, 16));
        put_dwords(
            &mut img,
            16,
            &[
                0,
                0x8000_0020, // DW2  density: bit31 set, exp 32 -> 4 Gbit
                0,
                0,
                0,
                0,
                0,
                0x0000_200C, // DW8  type1 4K/0x20, type2 unused
                0x0000_D810, // DW9  type3 64K/0xD8, type4 unused
                0,
                0x0000_0090, // DW11 page exp 9 -> 512
            ],
        );
        let g = decode_geometry(&img).unwrap();
        assert_eq!(g.capacity_bytes, 512 * 1024 * 1024);
        assert_eq!(g.page_size, 512);
        assert_eq!(g.sector_size, 4096);
        assert_eq!(g.block_size, 65536);
    }

    #[test]
    fn skips_vendor_header_and_defaults_page() {
        // 32 Mbit part behind a non-BFP vendor header: the decoder must skip
        // header 0, find the BFP in header 1, and follow its 0x20 pointer. The BFP
        // is only 9 dwords (no DW11), so page size falls back to the default 256.
        let mut img = [0u8; 72];
        write_header(&mut img, 1); // two parameter headers
        img[8..16].copy_from_slice(&param_header(0x81, 0x00, 4, 0)); // vendor, ignored
        img[16..24].copy_from_slice(&param_header(BFP_ID_LSB, BFP_ID_MSB, 9, 0x20));
        put_dwords(
            &mut img,
            0x20,
            &[
                0,
                0x01FF_FFFF, // DW2  density: 32 Mbit - 1 -> 4 MiB
                0,
                0,
                0,
                0,
                0,
                0x520F_200C, // DW8  type1 4K/0x20, type2 32K/0x52
                0x0000_D810, // DW9  type3 64K/0xD8
            ],
        );
        let g = decode_geometry(&img).unwrap();
        assert_eq!(g.capacity_bytes, 4 * 1024 * 1024);
        assert_eq!(g.page_size, 256); // no DW11 -> default
        assert_eq!(g.sector_size, 4096);
        assert_eq!(g.block_size, 65536);
    }

    #[test]
    fn bad_signature_rejected() {
        let mut img = [0u8; 60];
        write_header(&mut img, 0);
        img[0] = 0; // corrupt "SFDP"
        assert_eq!(
            SfdpHeader::parse(&img[0..8]),
            Err(FLASH_GENERIC_SFDP_INVALID_SIGNATURE)
        );
    }

    #[test]
    fn unsupported_major_rev_rejected() {
        let mut img = [0u8; 60];
        write_header(&mut img, 0);
        img[5] = 2; // major rev != SUPPORTED_MAJOR_REV
        assert_eq!(
            SfdpHeader::parse(&img[0..8]),
            Err(FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV)
        );
    }

    #[test]
    fn header_too_short_rejected() {
        assert_eq!(
            SfdpHeader::parse(&[0u8; 4]),
            Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT)
        );
    }

    #[test]
    fn no_bfp_header_rejected() {
        let mut img = [0u8; 60];
        write_header(&mut img, 0);
        img[8..16].copy_from_slice(&param_header(0x81, 0x00, 4, 16)); // vendor only
        assert_eq!(
            find_bfp_header(&img[0..8], &img[8..16]),
            Err(FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND)
        );
    }

    #[test]
    fn invalid_pow2_density_rejected() {
        // DW2 bit 31 set with exponent >= 64 is not a representable density.
        let mut img = [0u8; 60];
        write_header(&mut img, 0);
        img[8..16].copy_from_slice(&param_header(BFP_ID_LSB, BFP_ID_MSB, 2, 16));
        put_dwords(&mut img, 16, &[0, 0x8000_0040]); // exp 64
        assert_eq!(
            decode_geometry(&img),
            Err(FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY)
        );
    }
}
