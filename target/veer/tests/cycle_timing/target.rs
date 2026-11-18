// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Measures, in CPU cycles, what a register read costs on the emulator or the
//! FPGA, the way caliptra-mcu-sw times things: with the `mcycle` CSR, never
//! through a clock constant.
//!
//! 1. Clock calibration. `mcycle` and the MCI `mtime` are read around a spin
//!    loop, which gives the CPU cycles per `mtime` tick. The kernel config
//!    assumes `mtime` ticks at the CPU clock (`MTIME_HZ = SYSTEM_CLOCK_HZ`), and
//!    `syscall_latency` assumes it ticks at a slower I/O clock; this says which.
//! 2. Read latency. N reads of an I3C TTI register, of `mtime` (an MCI register),
//!    and of a word of RAM, each as cycles per iteration, next to an empty loop so
//!    the cost of the read is the difference. A pop of the RX data FIFO is
//!    one read of the same kind as the I3C one.
//!
//! The image has no apps: this runs in the kernel, in M-mode, where `mcycle` is
//! readable. It prints results and exits PASS; it asserts nothing about them.

#![no_std]
#![no_main]

use core::hint::black_box;
use core::ptr::with_exposed_provenance;

use caliptra_i3c_target::Reg;
use entry::exit;
use kernel_config::{I3C_BASE, MTIME_REGISTER};
use target_common::{declare_target, TargetInterface};
use {codegen as _, console_backend as _};

/// Reads per measurement.
const READS: u32 = 1000;

/// How long the calibration loop spins, in CPU cycles. About a quarter of a
/// second at 20 MHz, so the low word of `mtime` cannot wrap.
const CALIBRATION_CYCLES: u64 = 5_000_000;

/// The 64-bit `mcycle` counter. `mcycleh` is read before and after `mcycle`, and
/// retried if it moved, so a carry between the two reads cannot tear the value.
fn mcycle() -> u64 {
    loop {
        let hi: u32;
        let lo: u32;
        let hi2: u32;
        // SAFETY: `mcycle` and `mcycleh` are read-only counters in M-mode, where
        // this code runs, and the reads have no side effects.
        unsafe {
            core::arch::asm!("csrr {}, mcycleh", out(reg) hi, options(nomem, nostack));
            core::arch::asm!("csrr {}, mcycle", out(reg) lo, options(nomem, nostack));
            core::arch::asm!("csrr {}, mcycleh", out(reg) hi2, options(nomem, nostack));
        }
        if hi == hi2 {
            return (u64::from(hi) << 32) | u64::from(lo);
        }
    }
}

/// The low word of the MCI `mtime`.
fn mtime() -> u32 {
    // SAFETY: `MTIME_REGISTER` is a readable MCI register, and this code runs in
    // the kernel, which can access it.
    unsafe { with_exposed_provenance::<u32>(MTIME_REGISTER).read_volatile() }
}

fn calibrate() {
    let (c0, t0) = (mcycle(), mtime());
    while mcycle() - c0 < CALIBRATION_CYCLES {
        core::hint::spin_loop();
    }
    let (c1, t1) = (mcycle(), mtime());
    let cycles = c1 - c0;
    let ticks = u64::from(t1.wrapping_sub(t0));
    pw_log::info!("calibration: {} mcycle cycles", cycles as u64);
    pw_log::info!("calibration: {} mtime ticks", ticks as u64);
    if ticks != 0 {
        // Cycles per tick to three decimals, so 3.333 reads as 3333.
        pw_log::info!(
            "calibration: {} milli-cycles per mtime tick",
            (cycles * 1000 / ticks) as u64
        );
    }
}

/// Time `READS` calls of `read`, and print the total and the average.
fn time_reads(name: &str, mut read: impl FnMut() -> u32) {
    let mut sink = 0u32;
    let start = mcycle();
    for _ in 0..READS {
        sink = sink.wrapping_add(black_box(read()));
    }
    let total = mcycle() - start;
    black_box(sink);
    pw_log::info!(
        "{}: {} cycles for {} reads, {} milli-cycles per read",
        name as &str,
        total as u64,
        READS as u32,
        (total * 1000 / u64::from(READS)) as u64
    );
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU cycle timing";

    fn main() -> ! {
        calibrate();

        // The loop with nothing read: what each measurement below costs in
        // overhead, so the cost of a read is the difference.
        time_reads("empty loop", || 0);

        // The register the I3C driver polls for a queued frame. Reading it has no
        // side effect, unlike popping a FIFO port.
        let i3c_status =
            with_exposed_provenance::<u32>(I3C_BASE + Reg::TtiDescQueueDepth.offset());
        // SAFETY (each read below): the address is a readable register in the
        // I3C block, and this code runs in the kernel.
        time_reads("i3c tti_desc_queue_depth", || unsafe {
            i3c_status.read_volatile()
        });
        time_reads("mci mtime", mtime);

        static RAM_WORD: u32 = 0x1234_5678;
        let ram = &RAM_WORD as *const u32;
        // SAFETY: `ram` points at a live static.
        time_reads("ram word", || unsafe { ram.read_volatile() });

        pw_log::info!("PASS");
        exit(0);
    }
}

declare_target!(Target);
