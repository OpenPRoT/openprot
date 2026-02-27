// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use arch_riscv::regs::epmp::{MSeccfg, MSeccfgVal};
use arch_riscv::regs::pmp::{PmpCfgAddressMode, PmpCfgVal, PmpConfig};
use arch_riscv::MemoryConfig;
use memory_config::MemoryConfig as _;

unsafe extern "C" {
    #[link_name = "_code_start"]
    static CODE_START: u8;
    #[link_name = "_code_end"]
    static CODE_END: u8;
    #[link_name = "_kernel_end"]
    static KERNEL_END: u8;

    #[link_name = "_ram_start"]
    static RAM_START: u8;
    #[link_name = "_ram_end"]
    static RAM_END: u8;
}

pub fn init() {
    let mut epmp = unsafe { PmpConfig::<16>::read() };

    // Booting from ROM_EXT, the kernel execution region is a ToR at entries 2-3.
    // We'll place our own code bounds into entries 0-1 and re-write the PMP.
    epmp.addr[0] = &raw const CODE_START as usize >> 2;
    epmp.addr[1] = &raw const CODE_END as usize >> 2;
    epmp.cfg[0] = PmpCfgVal::default();
    epmp.cfg[1] = PmpCfgVal::default()
        .with_r(true)
        .with_x(true)
        .with_l(true)
        .with_a(PmpCfgAddressMode::Tor);

    unsafe { epmp.write() }

    // Set up the next entry to cover the kernel's .rodata.
    epmp.addr[2] = &raw const KERNEL_END as usize >> 2;
    epmp.cfg[2] = PmpCfgVal::default()
        .with_r(true)
        .with_l(true)
        .with_a(PmpCfgAddressMode::Tor);

    // Set up all of RAM as a locked RW NaPOT region.
    // We configure this as the last entry so that this range is a fallback to
    // access all of RAM in the kernel.
    let ram_start = &raw const RAM_START as usize;
    let ram_end = &raw const RAM_END as usize;
    let ram_size = ram_end - ram_start;
    epmp.addr[15] = (ram_start >> 2) | (ram_size - 1) >> 3;
    epmp.cfg[15] = PmpCfgVal::default()
        .with_l(true)
        .with_r(true)
        .with_w(true)
        .with_a(PmpCfgAddressMode::Napot);

    // Write the kernel config into the regsisters before we
    // zero out all of the now-unused entries.
    unsafe { epmp.write() }

    // Now that we've safely applied our kernel configuration, we
    // can zero out the rest of the PMP.
    for i in 3..14 {
        epmp.addr[i] = 0;
        epmp.cfg[i] = PmpCfgVal::default();
    }

    // Write the final ePMP configuration.
    unsafe { epmp.write() }

    // Clear RLB, thus enforcing the L bit.
    // Turn on MML to enable M/U shared regions as documented in Smepmp.
    let sec = MSeccfgVal::default()
        .with_rlb(false)
        .with_mmwp(true)
        .with_mml(true);
    MSeccfg::write(sec);

    // Now that we've configured the locked regions and have configured
    // MSeccfg, write the remaining unlocked kernel-mode ePMP configuration.
    unsafe {
        MemoryConfig::KERNEL_THREAD_MEMORY_CONFIG.write();
    }
    MemoryConfig::KERNEL_THREAD_MEMORY_CONFIG.dump_current();
}
