// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Standalone reimplementation of caliptra-sw's `elf2rom()` (from
//! `caliptra-sw/builder/src/lib.rs`). Converts a linked Caliptra ROM ELF into
//! a raw 0x18000-byte ROM image, patching the `CALIPTRA_ROM_INFO` struct with
//! the SHA256 digest over the preceding bytes so that the on-chip
//! `rom_integrity_test` passes at boot.
//!
//! Usage: `caliptra_rom_packager <input_elf> <output_bin>`
//!
//! # Upstream cross-reference
//!
//! Mirrors `pub fn elf2rom(elf_bytes: &[u8]) -> io::Result<Vec<u8>>` in
//! `caliptra-sw/builder/src/lib.rs` (around line 396). That function lives in
//! the `caliptra-builder` crate, which we cannot depend on from Bazel because
//! of unresolved upstream deps (`fslock`, `Crypto` trait wiring, `CARGO` env
//! var). Our `//third_party/caliptra/caliptra-sw:caliptra_builder` target is
//! tagged `manual` for that reason. If/when those upstream deps are fixed and
//! `caliptra_builder` becomes buildable, this file should be replaced by a
//! thin wrapper that calls `caliptra_builder::elf2rom()` directly.

use std::env;
use std::fs;

use anyhow::{anyhow, bail, Context, Result};
use elf::endian::LittleEndian;
use elf::ElfBytes;
use sha2::{Digest, Sha256};

const ROM_SIZE: usize = 0x18000;

// Byte layout of `caliptra_image_types::RomInfo` (repr(C), #[derive(IntoBytes)]):
//   offset 0..32   : sha256_digest: [u32; 8]  (LE on host)
//   offset 32..52  : revision:      [u8; 20]
//   offset 52..56  : flags:         u32        (LE)
//   offset 56..58  : version:       u16        (LE)
//   offset 58..60  : rsvd:          u16        (LE, padding)
// Total: 60 bytes.
const ROM_INFO_SIZE: usize = 60;

// Matches upstream `builder::version::get_rom_version()`
// (ROM_VERSION_MAJOR=2, MINOR=0, PATCH=0).
const ROM_VERSION: u16 = (2 & 0x1F) << 11;

// Matches the "CALIPTRA_IMAGE_NO_GIT_REVISION" fallback in upstream
// `builder::image_revision()`.
const NO_GIT_REVISION: [u8; 20] = *b"~~~~~NO_GIT_REVISION";

/// SHA256 with upstream's "word-reversed" preprocessing, producing the exact
/// bytes that upstream writes into `RomInfo::sha256_digest` (which is
/// `[u32; 8]` serialized via zerocopy on a little-endian host).
///
/// Upstream `sha256_word_reversed`:
///   - Split input into 4-byte little-endian u32 words.
///   - `word.swap_bytes().to_le_bytes()` feeds the bytes in reverse per word.
///   - Finalize, then interpret each 4-byte group of the raw digest as a
///     big-endian u32 (`to_hw_format`), producing `[u32; 8]`.
///
/// When that `[u32; 8]` is later serialized with zerocopy on a little-endian
/// host, each u32 is written as little-endian bytes — which reverses each
/// 4-byte group of the raw digest a second time. So the net on-disk bytes are:
/// for each 4-byte group of SHA256(preprocessed_input), reverse the group.
fn rom_info_digest_bytes(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let word_count = input.len() / 4;
    let mut swapped = [0u8; 4];
    for i in 0..word_count {
        let chunk = &input[i * 4..i * 4 + 4];
        swapped[0] = chunk[3];
        swapped[1] = chunk[2];
        swapped[2] = chunk[1];
        swapped[3] = chunk[0];
        hasher.update(swapped);
    }
    let raw: [u8; 32] = hasher.finalize().into();
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4] = raw[i * 4 + 3];
        out[i * 4 + 1] = raw[i * 4 + 2];
        out[i * 4 + 2] = raw[i * 4 + 1];
        out[i * 4 + 3] = raw[i * 4];
    }
    out
}

fn elf2rom(elf_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut result = vec![0u8; ROM_SIZE];
    let elf = ElfBytes::<LittleEndian>::minimal_parse(elf_bytes)
        .map_err(|e| anyhow!("failed to parse Caliptra ROM ELF: {e}"))?;

    let segments = elf
        .segments()
        .ok_or_else(|| anyhow!("Caliptra ROM ELF has no program headers"))?;
    for segment in segments {
        if segment.p_type != elf::abi::PT_LOAD {
            continue;
        }
        let len = segment.p_filesz as usize;
        if len == 0 {
            continue;
        }
        let file_offset = segment.p_offset as usize;
        let mem_offset = segment.p_paddr as usize;
        let src = elf_bytes
            .get(file_offset..file_offset + len)
            .ok_or_else(|| anyhow!("segment at file offset 0x{file_offset:x} out of bounds"))?;
        let dst = result.get_mut(mem_offset..mem_offset + len).ok_or_else(|| {
            anyhow!(
                "segment at 0x{mem_offset:x}..0x{:x} exceeds ROM region 0x0..0x{:x}",
                mem_offset + len,
                ROM_SIZE
            )
        })?;
        dst.copy_from_slice(src);
    }

    // Locate CALIPTRA_ROM_INFO via the ELF symbol table. It is defined by the
    // ROM linker script (`rom.ld`) at the start of the `.rom_info` section and
    // is the anchor over which the on-chip integrity test hashes.
    let (symtab, strtab) = elf
        .symbol_table()
        .map_err(|e| anyhow!("failed to read symbol table: {e}"))?
        .ok_or_else(|| anyhow!("Caliptra ROM ELF has no symbol table"))?;
    let rom_info_sym = symtab
        .iter()
        .find(|sym| {
            strtab
                .get(sym.st_name as usize)
                .map(|n| n == "CALIPTRA_ROM_INFO")
                .unwrap_or(false)
        })
        .ok_or_else(|| anyhow!("CALIPTRA_ROM_INFO symbol not found in ROM ELF"))?;
    let rom_info_start = rom_info_sym.st_value as usize;
    if rom_info_start % 4 != 0 {
        bail!("CALIPTRA_ROM_INFO @ 0x{rom_info_start:x} is not 4-byte aligned");
    }

    // Build RomInfo exactly as upstream does. Only `sha256_digest` is checked
    // by `rom_integrity_test` at boot; other fields are informational.
    let digest_bytes = rom_info_digest_bytes(&result[0..rom_info_start]);
    let rom_info_end = rom_info_start
        .checked_add(ROM_INFO_SIZE)
        .ok_or_else(|| anyhow!("rom_info end overflow"))?;
    let dst = result
        .get_mut(rom_info_start..rom_info_end)
        .ok_or_else(|| anyhow!("no space in ROM for CALIPTRA_ROM_INFO"))?;
    let mut rom_info_bytes = [0u8; ROM_INFO_SIZE];
    rom_info_bytes[0..32].copy_from_slice(&digest_bytes);
    rom_info_bytes[32..52].copy_from_slice(&NO_GIT_REVISION);
    // flags = 0 (already zero)
    rom_info_bytes[56..58].copy_from_slice(&ROM_VERSION.to_le_bytes());
    // rsvd = 0 (already zero)
    dst.copy_from_slice(&rom_info_bytes);

    Ok(result)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        bail!(
            "usage: {} <input_elf> <output_bin>",
            args.first().map(String::as_str).unwrap_or("caliptra_rom_packager")
        );
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let elf_bytes = fs::read(input_path)
        .with_context(|| format!("failed to read Caliptra ROM ELF {input_path}"))?;
    let rom = elf2rom(&elf_bytes)?;
    fs::write(output_path, &rom)
        .with_context(|| format!("failed to write Caliptra ROM bin {output_path}"))?;
    Ok(())
}
