// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPIPF address-privilege table programming (SPIPFWA @ 0x0100).
//!
//! Each bit in SPIPFWA[n] grants one 16 KiB block for the selected read/write
//! table (selected via SPIPF000[31:24] magic bytes).

use crate::spimonitor::registers::SpiMonitorRegisters;
use crate::spimonitor::types::{PrivilegeDirection, PrivilegeOp, Result, SpiMonitorError};

/// 16 KiB address-privilege granule.
pub const ACCESS_BLOCK_UNIT: u32 = 16 * 1024;
/// One SPIPFWA register covers 32 × 16 KiB = 512 KiB.
pub const ACCESS_BLOCK_PER_REG: u32 = 32 * ACCESS_BLOCK_UNIT;
/// Maximum flash address space covered by the privilege tables.
pub const ADDR_LIMIT: u32 = 256 * 1024 * 1024;

const SEL_READ_TBL_MAGIC: u32 = 0x52 << 24;
const SEL_WRITE_TBL_MAGIC: u32 = 0x57 << 24;

fn select_addr_priv_table(regs: &SpiMonitorRegisters, direction: PrivilegeDirection) {
    regs.modify_ctrl(|bits| {
        *bits &= 0x00FF_FFFF;
        match direction {
            PrivilegeDirection::Read => *bits |= SEL_READ_TBL_MAGIC,
            PrivilegeDirection::Write => *bits |= SEL_WRITE_TBL_MAGIC,
        }
    });
}

fn adjusted_addr_len(addr: u32, len: u32) -> (u32, u32) {
    if len == 0 {
        return (addr, 0);
    }
    let mut adjusted_len = len;
    let mut aligned_addr = addr;
    if !addr.is_multiple_of(ACCESS_BLOCK_UNIT) {
        adjusted_len += addr % ACCESS_BLOCK_UNIT;
        aligned_addr = (addr / ACCESS_BLOCK_UNIT) * ACCESS_BLOCK_UNIT;
    }
    adjusted_len = adjusted_len.div_ceil(ACCESS_BLOCK_UNIT) * ACCESS_BLOCK_UNIT;
    (aligned_addr, adjusted_len)
}

/// Program a contiguous address range in the read or write privilege table.
pub fn configure_address_privilege(
    regs: &SpiMonitorRegisters,
    direction: PrivilegeDirection,
    op: PrivilegeOp,
    addr: u32,
    len: u32,
) -> Result<()> {
    if addr >= ADDR_LIMIT {
        return Err(SpiMonitorError::InvalidRegion);
    }
    if len == 0 || addr.saturating_add(len) > ADDR_LIMIT {
        return Err(SpiMonitorError::InvalidRegion);
    }

    let (aligned_addr, adjusted_len) = adjusted_addr_len(addr, len);
    if adjusted_len == 0 {
        return Ok(());
    }

    let mut reg_off = (aligned_addr / ACCESS_BLOCK_PER_REG) as usize;
    let mut bit_off = ((aligned_addr % ACCESS_BLOCK_PER_REG) / ACCESS_BLOCK_UNIT) as u32;
    let mut total_bit_num = adjusted_len / ACCESS_BLOCK_UNIT;

    select_addr_priv_table(regs, direction);

    while total_bit_num > 0 {
        if bit_off > 31 {
            bit_off = 0;
            reg_off += 1;
        }

        if bit_off == 0 && total_bit_num >= 32 {
            let word = match op {
                PrivilegeOp::Enable => 0xFFFF_FFFF,
                PrivilegeOp::Disable => 0,
            };
            regs.write_addr_filter_slot(reg_off, word);
            reg_off += 1;
            total_bit_num -= 32;
        } else {
            let mut reg_val = regs.read_addr_filter_slot(reg_off);
            match op {
                PrivilegeOp::Enable => reg_val |= 1 << bit_off,
                PrivilegeOp::Disable => reg_val &= !(1 << bit_off),
            }
            regs.write_addr_filter_slot(reg_off, reg_val);
            bit_off += 1;
            total_bit_num -= 1;
        }
    }

    Ok(())
}

/// Enable full-flash read and write in the addr-priv tables.
pub fn configure_full_flash_rw(regs: &SpiMonitorRegisters) -> Result<()> {
    configure_address_privilege(regs, PrivilegeDirection::Read, PrivilegeOp::Enable, 0, ADDR_LIMIT)?;
    configure_address_privilege(regs, PrivilegeDirection::Write, PrivilegeOp::Enable, 0, ADDR_LIMIT)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjusted_addr_len_aligns_to_16k() {
        let (addr, len) = adjusted_addr_len(0x1000, 0x2000);
        assert_eq!(addr, 0);
        assert_eq!(len, 0x3000);
    }
}
