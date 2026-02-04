#![no_main]
#![no_std]

use kernel_config::{KernelConfig, KernelConfigInterface};
use pw_status::Result;
use registers::rv_timer::RvTimer;
use userspace::{entry, syscall};

#[cfg(feature = "silicon")]
const IO_CLOCK_HZ: u64 = 24_000_000;
#[cfg(feature = "fpga")]
const IO_CLOCK_HZ: u64 = 6_000_000;
#[cfg(feature = "verilator")]
const IO_CLOCK_HZ: u64 = 125_000;

#[inline(always)]
fn rv_timer_value(rv_timer: &RvTimer) -> u64 {
    let regs = rv_timer.regs();
    loop {
        let hi1 = regs.timer_v_upper0().read();
        let low = regs.timer_v_lower0().read();
        let hi2 = regs.timer_v_upper0().read();
        if hi1 == hi2 {
            return ((hi1 as u64) << 32) | (low as u64);
        }
    }
}

fn measure_nop_syscall(rv_timer: &RvTimer, n: usize) -> Result<()> {
    let mut total = 0u64;
    for _ in 0..n {
        let t0 = rv_timer_value(&rv_timer);
        syscall::debug_nop()?;
        let t1 = rv_timer_value(&rv_timer);
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
    let rv_timer = unsafe { RvTimer::new() };
    let result = measure_nop_syscall(&rv_timer, 100);
    syscall::debug_shutdown(result).unwrap();
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("PANIC: syscall latency");
    loop {}
}
