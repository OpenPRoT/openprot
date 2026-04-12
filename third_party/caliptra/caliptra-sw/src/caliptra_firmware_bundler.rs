// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Host tool that bundles a Caliptra FMC ELF + Runtime ELF into a signed
//! `cptra-firmware.bin` functionally equivalent to
//! `caliptra_builder::build_and_sign_image(FMC_WITH_UART, APP_WITH_UART,
//! ImageOptions{ pqc_key_type: LMS, ..default })` from upstream caliptra-sw.
//!
//! This avoids the `caliptra-image-app` tool and the heavy `caliptra-builder`
//! library (both of which either shell out to cargo or need PEM key configs)
//! and instead drives `caliptra-image-gen` directly with the static
//! `VENDOR_CONFIG_KEY_0` + `OWNER_CONFIG` constants from
//! `caliptra-image-fake-keys`, which is what produces the known-good
//! `vendor_pk_hash` that the repo's `caliptra_runner.bzl` hard-codes.
//!
//! Usage:
//!   caliptra_firmware_bundler --fmc <fmc.elf> --runtime <runtime.elf>
//!                             --output <cptra-firmware.bin>
//!
//! # Upstream cross-reference
//!
//! Mirrors `pub fn build_and_sign_image(...)` in
//! `caliptra-sw/builder/src/lib.rs` (around line 489) and the supporting
//! `ElfExecutable` in `caliptra-sw/image/elf/src/lib.rs`. The full upstream
//! pipeline lives in the `caliptra-builder` crate, which we cannot depend on
//! from Bazel because of unresolved upstream deps (`fslock`, `Crypto` trait
//! wiring, `CARGO` env var). Our
//! `//third_party/caliptra/caliptra-sw:caliptra_builder` target is tagged
//! `manual` for that reason. If/when those upstream deps are fixed and
//! `caliptra_builder` becomes buildable, this file should be replaced by a
//! thin wrapper that calls `caliptra_builder::build_and_sign_image()`
//! directly.

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use caliptra_image_crypto::RustCrypto as Crypto;
use caliptra_image_fake_keys::{OWNER_CONFIG, VENDOR_CONFIG_KEY_0};
use caliptra_image_gen::{
    ImageGenerator, ImageGeneratorConfig, ImageGeneratorExecutable,
};
use caliptra_image_types::{FwVerificationPqcKeyType, ImageRevision};
use elf::endian::AnyEndian;
use elf::ElfBytes;

/// Tiny re-implementation of upstream `caliptra_image_elf::ElfExecutable`:
/// just a PT_LOAD aggregator keyed by `p_paddr`, with no padding or digest
/// logic (unlike `elf2rom`). Packs the ELF load segments into a contiguous
/// buffer starting at the lowest `p_paddr`.
struct ElfExecutable {
    version: u32,
    rev: ImageRevision,
    load_addr: u32,
    entry_point: u32,
    content: Vec<u8>,
}

impl ElfExecutable {
    fn from_bytes(elf_bytes: &[u8], version: u32, rev: ImageRevision) -> Result<Self> {
        let elf = ElfBytes::<AnyEndian>::minimal_parse(elf_bytes)
            .map_err(|e| anyhow!("failed to parse ELF: {e}"))?;
        let segments = elf
            .segments()
            .ok_or_else(|| anyhow!("ELF file has no segments"))?;

        let load_addr = segments
            .iter()
            .filter(|s| s.p_type == elf::abi::PT_LOAD)
            .map(|s| s.p_paddr as u32)
            .min()
            .ok_or_else(|| anyhow!("ELF file has no PT_LOAD segments"))?;

        let mut content: Vec<u8> = Vec::new();
        for segment in segments {
            if segment.p_type != elf::abi::PT_LOAD {
                continue;
            }
            let data = elf
                .segment_data(&segment)
                .map_err(|e| anyhow!("failed to read segment: {e}"))?;
            if data.is_empty() {
                continue;
            }
            let seg_addr = segment.p_paddr as u32;
            if seg_addr < load_addr {
                bail!(
                    "segment at 0x{seg_addr:08x} below base 0x{load_addr:08x}"
                );
            }
            let offset = (seg_addr - load_addr) as usize;
            let end = offset + data.len();
            if content.len() < end {
                content.resize(end, 0);
            }
            content[offset..end].copy_from_slice(data);
        }

        let entry_point = elf.ehdr.e_entry as u32;
        Ok(Self {
            version,
            rev,
            load_addr,
            entry_point,
            content,
        })
    }
}

impl ImageGeneratorExecutable for ElfExecutable {
    fn version(&self) -> u32 {
        self.version
    }
    fn rev(&self) -> &ImageRevision {
        &self.rev
    }
    fn load_addr(&self) -> u32 {
        self.load_addr
    }
    fn entry_point(&self) -> u32 {
        self.entry_point
    }
    fn content(&self) -> &Vec<u8> {
        &self.content
    }
    fn size(&self) -> u32 {
        self.content.len() as u32
    }
}

struct Args {
    fmc: PathBuf,
    runtime: PathBuf,
    output: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut fmc: Option<PathBuf> = None;
    let mut runtime: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| anyhow!("flag {flag} requires a value"))?;
        match flag.as_str() {
            "--fmc" => fmc = Some(PathBuf::from(value)),
            "--runtime" => runtime = Some(PathBuf::from(value)),
            "--output" | "--out" => output = Some(PathBuf::from(value)),
            other => bail!("unknown flag: {other}"),
        }
    }
    Ok(Args {
        fmc: fmc.ok_or_else(|| anyhow!("missing --fmc"))?,
        runtime: runtime.ok_or_else(|| anyhow!("missing --runtime"))?,
        output: output.ok_or_else(|| anyhow!("missing --output"))?,
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;

    // Hermetic image revision: upstream's image_revision() honours
    // CALIPTRA_IMAGE_NO_GIT_REVISION and returns this exact placeholder when
    // set. We bake it in unconditionally — this host tool has no git repo to
    // rev-parse anyway.
    let rev: ImageRevision = *b"~~~~~NO_GIT_REVISION";

    let fmc_bytes = fs::read(&args.fmc)
        .with_context(|| format!("failed to read FMC ELF {:?}", args.fmc))?;
    let runtime_bytes = fs::read(&args.runtime)
        .with_context(|| format!("failed to read Runtime ELF {:?}", args.runtime))?;

    let fmc = ElfExecutable::from_bytes(&fmc_bytes, 0, rev)
        .context("failed to pack FMC ELF")?;
    let runtime = ElfExecutable::from_bytes(&runtime_bytes, 0, rev)
        .context("failed to pack Runtime ELF")?;

    let cfg = ImageGeneratorConfig {
        pqc_key_type: FwVerificationPqcKeyType::LMS,
        vendor_config: VENDOR_CONFIG_KEY_0,
        owner_config: Some(OWNER_CONFIG),
        fmc,
        runtime,
        fw_svn: 0,
    };

    let bundle = ImageGenerator::new(Crypto::default())
        .generate(&cfg)
        .map_err(|e| anyhow!("ImageGenerator::generate failed: {e}"))?;

    let bytes = bundle
        .to_bytes()
        .map_err(|e| anyhow!("ImageBundle::to_bytes failed: {e}"))?;
    fs::write(&args.output, &bytes)
        .with_context(|| format!("failed to write {:?}", args.output))?;
    Ok(())
}
