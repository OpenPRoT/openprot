// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::ptr::read_volatile;

#[allow(dead_code)]
pub fn dump_smc_register(addr: u32, count: u32) {
    for i in 0..count {
        let reg_addr = addr + (i * 4);
        let reg = unsafe { read_volatile(reg_addr as *const u32) };

        pw_log::info!("SMC[0x{:08x}] = 0x{:08x}", reg_addr as u32, reg as u32);
    }
}

#[allow(dead_code)]
pub fn dump_smc_read(buf: &[u8], count: u32) {
    let count = core::cmp::min(count as usize, buf.len());
    for i in (0..count).step_by(4) {
        if i + 4 > count {
            break;
        }

        let bytes: [u8; 4] = buf[i..i + 4].try_into().unwrap();
        let value = u32::from_le_bytes(bytes);

        pw_log::info!("[0x{:08x}] = 0x{:08x}", i as u32, value as u32);
    }
}
