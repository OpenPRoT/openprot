// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Writes a flash image for a QEMU test from a board's slot layout.
//!
//! The layout comes in as arguments but is built with the same
//! `Region`/`Slot`/`ImageLayout` constructors the board tables use, so a
//! layout this tool accepts is one the orchestrator would accept: slot ids
//! unique, no two regions overlapping, no zero-length region, nothing past
//! the end of the offset space. Re-deriving those rules here would let a
//! test image drift from what the firmware believes.
//!
//! Everything the layout does not name stays 0xFF, so the image looks like
//! erased flash with images written into it.
//!
//! ```text
//! flash-image --flash-size 0x4000000 \
//!     --slot 0:0x0:0x100000=slot_a.bin \
//!     --slot 1:0x100000:0x100000=slot_b.bin \
//!     --golden 0x200000:0x100000=golden.bin \
//!     --output cs1.img
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use orchestrator_config::{Golden, ImageLayout, Region, Slot, SlotId};

/// Erased flash. Everything no image covers keeps this value.
const ERASED: u8 = 0xFF;

/// One `--slot` or `--golden` argument: where the image goes and what
/// fills it.
struct Placement {
    region: Region,
    payload: PathBuf,
}

/// `base:len=path`, with base and len in decimal or 0x hex.
fn parse_region(spec: &str) -> Result<Placement, String> {
    let (region, path) = spec
        .split_once('=')
        .ok_or_else(|| format!("{spec}: expected base:len=path"))?;
    let (base, len) = region
        .split_once(':')
        .ok_or_else(|| format!("{spec}: expected base:len=path"))?;
    Ok(Placement {
        region: Region::new(parse_u32(base)?, parse_u32(len)?),
        payload: PathBuf::from(path),
    })
}

/// `id:base:len=path`, the slot id in front of a region spec.
fn parse_slot(spec: &str) -> Result<(SlotId, Placement), String> {
    let (id, rest) = spec
        .split_once(':')
        .ok_or_else(|| format!("{spec}: expected id:base:len=path"))?;
    let id: u8 = id
        .parse()
        .map_err(|_| format!("{id}: slot id must be 0 to 255"))?;
    Ok((SlotId(id), parse_region(rest)?))
}

fn parse_u32(text: &str) -> Result<u32, String> {
    let parsed = match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => text.parse(),
    };
    parsed.map_err(|_| format!("{text}: not a 32-bit number"))
}

struct Args {
    flash_size: u32,
    slots: Vec<(SlotId, Placement)>,
    golden: Option<Placement>,
    output: PathBuf,
}

fn parse_args(argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut flash_size = None;
    let mut slots = Vec::new();
    let mut golden = None;
    let mut output = None;

    let mut argv = argv.peekable();
    while let Some(flag) = argv.next() {
        let mut value = || {
            argv.next()
                .ok_or_else(|| format!("{flag}: missing its value"))
        };
        match flag.as_str() {
            "--flash-size" => flash_size = Some(parse_u32(&value()?)?),
            "--slot" => slots.push(parse_slot(&value()?)?),
            "--golden" => golden = Some(parse_region(&value()?)?),
            "--output" => output = Some(PathBuf::from(value()?)),
            other => return Err(format!("{other}: unknown argument")),
        }
    }

    Ok(Args {
        flash_size: flash_size.ok_or("--flash-size is required")?,
        slots,
        golden,
        output: output.ok_or("--output is required")?,
    })
}

/// Builds the image. The layout is constructed first so a bad one is
/// rejected before any file is read.
fn build(args: &Args) -> Result<Vec<u8>, String> {
    // ImageLayout borrows the slots for 'static, matching board tables
    // that declare them as consts. A build tool runs once and exits, so
    // leaking the one list it makes costs nothing.
    let declared: Vec<Slot> = args
        .slots
        .iter()
        .map(|(id, placement)| Slot::new(*id, placement.region))
        .collect();
    let layout = ImageLayout::new(
        Box::leak(declared.into_boxed_slice()),
        args.golden.as_ref().map(|g| Golden::new(g.region)),
    );

    // Slot ids are unique by now, so a payload per id is unambiguous.
    let payloads: BTreeMap<u8, &PathBuf> = args
        .slots
        .iter()
        .map(|(id, placement)| (id.0, &placement.payload))
        .collect();

    let mut image = vec![ERASED; args.flash_size as usize];
    let mut place = |region: Region, payload: &PathBuf, what: &str| {
        let bytes = fs::read(payload).map_err(|e| format!("{}: {e}", payload.display()))?;
        if bytes.len() > region.len() as usize {
            return Err(format!(
                "{}: {} bytes does not fit the {}-byte {what}",
                payload.display(),
                bytes.len(),
                region.len()
            ));
        }
        let base = region.base() as usize;
        let end = base + bytes.len();
        if region.end() as usize > image.len() {
            return Err(format!(
                "{what} ends at {:#x}, past the {:#x}-byte flash",
                region.end(),
                image.len()
            ));
        }
        image[base..end].copy_from_slice(&bytes);
        Ok(())
    };

    for slot in layout.slots() {
        let payload = payloads[&slot.id().0];
        place(slot.region(), payload, "slot")?;
    }
    if let (Some(golden), Some(placement)) = (layout.golden(), args.golden.as_ref()) {
        place(golden.region(), &placement.payload, "golden image")?;
    }

    Ok(image)
}

fn main() -> ExitCode {
    // skip(1) drops argv[0], which is the part semgrep's rule is about:
    // the caller chooses it and it need not name anything real. Nothing
    // here reads it.
    // nosemgrep: rust.lang.security.args.args
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("flash-image: {e}");
            return ExitCode::FAILURE;
        }
    };

    let image = match build(&args) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("flash-image: {e}");
            return ExitCode::FAILURE;
        }
    };

    match fs::write(&args.output, &image) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("flash-image: {}: {e}", args.output.display());
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
