// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Times, in CPU cycles, draining a received I3C frame from the RX queue, the way
//! caliptra-mcu-sw times things: with the `mcycle` CSR.
//!
//! The host sends frames of several sizes (see `host_test.rs`), one at a time,
//! after this firmware logs that it is ready for it. Frames alternate between two
//! ways of draining:
//!
//! * `driver`: the real `CaliptraI3cTarget::read_frame`, which pops the
//!   descriptor and then, for every word, pops the data port and stores the word
//!   into the buffer.
//! * `raw`: the descriptor pop and one pop of the data port per word, with
//!   nothing stored. This is the cost of the register reads alone.
//!
//! The difference between the two for the same size is the driver's own work per
//! word. Each result is one log line; the image asserts nothing about it.
//!
//! The image has no apps, so nothing handles the I3C interrupt: the frames are
//! found by polling, and the driver's `enable` is not called.

#![no_std]
#![no_main]

use core::hint::black_box;
use core::ptr::with_exposed_provenance;

use caliptra_i3c_target::{CaliptraI3cTarget, Reg};
use entry::exit;
use kernel_config::I3C_BASE;
use openprot_hal_blocking::i3c_hardware::I3cTarget;
use target_common::{declare_target, TargetInterface};
use {codegen as _, console_backend as _};

/// Frames to receive one at a time. The host sends this many (`FRAMES` in
/// `host_test.rs`).
const FRAMES: u32 = 24;

/// Then bursts: the host sends `BURST_LEN` full-size frames with no pause between
/// them, and this drains them back to back without logging in between, as the
/// i3c service does. A second frame that arrives while the first is being read
/// out finds the RX queue still full, and the core cuts it short and flags it.
/// (`BURSTS` and `BURST_LEN` in `host_test.rs`.)
const BURSTS: u32 = 5;
const BURST_LEN: usize = 4;

/// The 64-bit `mcycle` counter, read so that a carry between `mcycleh` and
/// `mcycle` cannot tear it.
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

fn read(reg: Reg) -> u32 {
    let addr = with_exposed_provenance::<u32>(I3C_BASE + reg.offset());
    // SAFETY: `addr` is a register of the I3C block, and this code runs in the
    // kernel, which can access it. The block is not used by anything else here.
    unsafe { addr.read_volatile() }
}

/// Drain one frame with the driver. Returns the frame length, the cycles the
/// call that found it took, and the error field of its RX descriptor (0 for a
/// frame that arrived whole, 1 when the core's RX queue ran out of room).
fn drain_driver(target: &mut CaliptraI3cTarget, buf: &mut [u8; 256]) -> (usize, u64, u32) {
    loop {
        let start = mcycle();
        let frame = target.read_frame(buf);
        let cycles = mcycle() - start;
        if let Ok(Some(len)) = frame {
            let err = target.take_rx_error().map_or(0, |desc| desc >> 28);
            return (len, cycles, err);
        }
    }
}

/// Drain one frame with register reads only: the descriptor, then one pop of the
/// data port per word, discarding the words.
fn drain_raw() -> (usize, u64, u32) {
    // A frame is there when the descriptor queue has an entry or the interrupt
    // status says so. The driver accepts either, because the emulator model has
    // no depth register and the FPGA's status bit can drop while a descriptor is
    // still queued. Nothing else raises an interrupt status bit in this image.
    while read(Reg::TtiDescQueueDepth) & 0xff == 0 && read(Reg::TtiInterruptStatus) == 0 {
        core::hint::spin_loop();
    }
    let start = mcycle();
    let desc = read(Reg::TtiRxDescPort);
    let len = (desc & 0xffff) as usize;
    let mut sink = 0u32;
    for _ in 0..len.div_ceil(4) {
        sink ^= read(Reg::TtiRxDataPort);
    }
    let cycles = mcycle() - start;
    black_box(sink);
    (len, cycles, desc >> 28)
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU RX drain timing";

    fn main() -> ! {
        // SAFETY: this code runs in the kernel, which can access the I3C block,
        // and this image has no other user of it.
        let mut target = unsafe { CaliptraI3cTarget::new() };
        let mut buf = [0u8; 256];

        for i in 0..FRAMES {
            pw_log::info!("rx_timing: ready #{};", i as u32);
            let (mode, (len, cycles, err)) = if i % 2 == 0 {
                ("driver", drain_driver(&mut target, &mut buf))
            } else {
                ("raw", drain_raw())
            };
            let words = len.div_ceil(4);
            pw_log::info!(
                "rx_timing: frame {} {} len {} words {} cycles {} cycles-per-word {} err {}",
                i as u32,
                mode as &str,
                len as u32,
                words as u32,
                cycles as u64,
                (cycles / words.max(1) as u64) as u64,
                err as u32
            );
        }

        for burst in 0..BURSTS {
            pw_log::info!("rx_timing: burst ready #{};", burst as u32);
            let mut results = [(0usize, 0u64, 0u32); BURST_LEN];
            // No logging until every frame is drained.
            for result in results.iter_mut() {
                *result = drain_driver(&mut target, &mut buf);
            }
            for (n, (len, cycles, err)) in results.iter().enumerate() {
                pw_log::info!(
                    "rx_timing: burst {} frame {} len {} cycles {} err {}",
                    burst as u32,
                    n as u32,
                    *len as u32,
                    *cycles as u64,
                    *err as u32
                );
            }
        }

        pw_log::info!("PASS");
        exit(0);
    }
}

declare_target!(Target);
