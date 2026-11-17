// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Fails the build when one system manifest has a hardware conflict: two
//! apps both claim the same pin, or two processes are both granted the same
//! device register range.
//!
//! Reads `system.json5` directly — not an assembled image — before anything
//! compiles. A pin conflict is read from two `pin_claims_<process>` manifest
//! constants (comma-separated pin names); an MMIO conflict is read straight
//! from each process's own `memory_mappings` (`type: "device"`) — a granted
//! device range *is* the claim for MMIO, unlike a pin, where holding a
//! register-block grant doesn't say which bit within it is actually driven.
//! No separate "mmio claims" constant is needed.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Manifest {
    apps: Vec<App>,
}

#[derive(Deserialize)]
struct App {
    name: String,
    #[serde(default)]
    constants: Vec<ConstConfig>,
    #[serde(default)]
    processes: Vec<Process>,
}

#[derive(Deserialize)]
struct Process {
    name: String,
    #[serde(default)]
    memory_mappings: Vec<MemoryMapping>,
}

#[derive(Deserialize)]
struct MemoryMapping {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    start_address: u64,
    #[serde(default)]
    size_bytes: u64,
}

/// Mirrors `pw_kernel::tooling::system_generator::system_config::ConstConfig`'s wire shape.
/// Only the `String` variant's value is ever read; every other variant exists so a manifest
/// that also declares non-string constants still deserializes instead of failing to parse.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum ConstConfig {
    Bool { name: String, value: bool },
    Char { name: String, value: char },
    U8 { name: String, value: f64 },
    U16 { name: String, value: f64 },
    U32 { name: String, value: f64 },
    U64 { name: String, value: f64 },
    U128 { name: String, value: f64 },
    Usize { name: String, value: f64 },
    I8 { name: String, value: f64 },
    I16 { name: String, value: f64 },
    I32 { name: String, value: f64 },
    I64 { name: String, value: f64 },
    I128 { name: String, value: f64 },
    Isize { name: String, value: f64 },
    String { name: String, value: String },
}

struct MmioClaim {
    label: String,
    start: u64,
    end: u64,
}

/// All pairs, not just adjacent-after-sort: a range that fully contains two later ones would
/// otherwise only ever get compared against its immediate neighbor in start order, undercounting.
fn count_mmio_overlaps(claims: &[MmioClaim]) -> usize {
    let mut conflicts = 0;
    for i in 0..claims.len() {
        for j in (i + 1)..claims.len() {
            let (first, second) = (&claims[i], &claims[j]);
            if first.start < second.end && second.start < first.end {
                eprintln!(
                    "two processes take overlapping MMIO: {} [{:#010x}..{:#010x}) and {} [{:#010x}..{:#010x})",
                    first.label, first.start, first.end, second.label, second.start, second.end,
                );
                conflicts += 1;
            }
        }
    }
    conflicts
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let expect_conflict = args.iter().any(|a| a == "--expect-conflict");
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .expect("usage: manifest_check <system.json5> [--expect-conflict]")
        .clone();

    let text = std::fs::read_to_string(&path).expect("read manifest");
    let manifest: Manifest = serde_json5::from_str(&text).expect("parse manifest");

    let mut conflicts = 0;
    let mut pins_checked = 0;

    // Pin physical name -> the first "app: process" that claimed it.
    let mut claimed_pins: HashMap<String, String> = HashMap::new();

    let mut mmio_claims: Vec<MmioClaim> = Vec::new();

    for app in &manifest.apps {
        for constant in &app.constants {
            let ConstConfig::String { name, value } = constant else {
                continue;
            };
            let Some(process) = name.strip_prefix("pin_claims_") else {
                continue;
            };
            let label = format!("{}: {process}", app.name);
            for pin in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                pins_checked += 1;
                if let Some(prior) = claimed_pins.insert(pin.to_string(), label.clone()) {
                    eprintln!("pin {pin} is driven by both {prior} and {label}");
                    conflicts += 1;
                }
            }
        }

        for process in &app.processes {
            let label = format!("{}: {}", app.name, process.name);
            for mapping in &process.memory_mappings {
                if mapping.ty != "device" {
                    continue;
                }
                mmio_claims.push(MmioClaim {
                    label: format!("{label} ({})", mapping.name),
                    start: mapping.start_address,
                    end: mapping.start_address + mapping.size_bytes,
                });
            }
        }
    }

    conflicts += count_mmio_overlaps(&mmio_claims);

    if expect_conflict {
        if conflicts == 0 {
            eprintln!("{path}: expected a hardware conflict, found none");
            std::process::exit(1);
        }
    } else if conflicts > 0 {
        eprintln!("{path}: {conflicts} manifest-level hardware conflict(s)");
        std::process::exit(1);
    }

    println!(
        "{path}: {pins_checked} pin_claims_<process> pin(s), {} device mapping(s) checked, {conflicts} conflict(s)",
        mmio_claims.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(label: &str, start: u64, end: u64) -> MmioClaim {
        MmioClaim {
            label: label.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn no_overlap_among_disjoint_ranges() {
        let claims = [claim("a", 0x0, 0x10), claim("b", 0x10, 0x20)];
        assert_eq!(count_mmio_overlaps(&claims), 0);
    }

    #[test]
    fn detects_a_simple_overlap() {
        let claims = [claim("a", 0x0, 0x20), claim("b", 0x10, 0x30)];
        assert_eq!(count_mmio_overlaps(&claims), 1);
    }

    #[test]
    fn detects_every_overlap_when_one_range_contains_two_others() {
        // Regression test: comparing only adjacent pairs after sorting by start would check
        // (a, b) and (b, c) but never (a, c) — missing that a fully contains c too.
        let claims = [
            claim("a", 0x0, 0x100),
            claim("b", 0x10, 0x20),
            claim("c", 0x30, 0x40),
        ];
        assert_eq!(count_mmio_overlaps(&claims), 2);
    }
}
