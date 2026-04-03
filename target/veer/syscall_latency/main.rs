// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0
#![no_main]
#![no_std]

use kernel_config::{KernelConfig, KernelConfigInterface};
use pw_status::Result;
//use registers::rv_timer::RvTimer;
use userspace::{entry, syscall};

#[cfg(feature = "silicon")]
const IO_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "fpga")]
const IO_CLOCK_HZ: u64 = 6_000_000;
#[cfg(feature = "emulator")]
const IO_CLOCK_HZ: u64 = 1_000_000;

#[inline(always)]
fn mtime_value() -> u64 {
    let mtime = core::ptr::with_exposed_provenance::<u64>(0x21000000 + 0xe4);
    unsafe { mtime.read_volatile() }
}

fn measure_nop_syscall(n: usize) -> Result<()> {
    let mut total = 0u64;
    for _ in 0..n {
        let t0 = mtime_value();
        syscall::debug_nop()?;
        let t1 = mtime_value();
        total += t1 - t0;
    }
    pw_log::info!(
        "Performed {} syscalls in {} rv_timer ticks.",
        n as usize,
        total as u64,
    );
    let average = total / (n as u64);
    pw_log::info!("Average latency: {} rv_timer ticks", average as u64);

    let cpu_clocks = average * KernelConfig::SYSTEM_CLOCK_HZ / IO_CLOCK_HZ;
    pw_log::info!("Average latency: {} cpu clocks", cpu_clocks as u64);
    Ok(())
}

#[entry]
fn entry() -> ! {
    let result = measure_nop_syscall(100);
    syscall::debug_shutdown(result).unwrap();
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("PANIC: syscall latency");
    loop {}
}
