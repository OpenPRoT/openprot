// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! This SoC's GPIO register groups as data: one `GpioMap` per pin-group (ABCD/EFGH/IJKL), each a
//! datasheet offset table. Role composition is shared; only the per-group offsets differ.

use crate::scu::pins::GPIO_BASE;
use openprot_hal::gpio::{role_cfgs_ok, GpioMap, Reg, RegOp};

// Stamp one pin-group's `GpioMap` from its datasheet offsets; role composition is fixed.
macro_rules! gpio_group {
    (
        dataval: $dataval:expr, dir: $dir:expr, int_en: $int_en:expr,
        sen0: $sen0:expr, sen1: $sen1:expr, senboth: $senboth:expr,
        int_sts: $int_sts:expr, in_level: $in_level:expr, out_read: $out_read:expr $(,)?
    ) => {
        GpioMap {
            base: GPIO_BASE,
            output: &[RegOp::set($dir)],
            input: &[RegOp::clear($dir)],
            set_high: &[RegOp::set($dataval)],
            set_low: &[RegOp::clear($dataval)],
            enable_rising: &[
                RegOp::set($sen0),
                RegOp::clear($sen1),
                RegOp::clear($senboth),
                RegOp::set($int_en),
            ],
            enable_falling: &[
                RegOp::clear($sen0),
                RegOp::clear($sen1),
                RegOp::clear($senboth),
                RegOp::set($int_en),
            ],
            enable_level_high: &[
                RegOp::set($sen0),
                RegOp::set($sen1),
                RegOp::clear($senboth),
                RegOp::set($int_en),
            ],
            enable_level_low: &[
                RegOp::clear($sen0),
                RegOp::set($sen1),
                RegOp::clear($senboth),
                RegOp::set($int_en),
            ],
            // Both-edge sets only the override register, then arms — enable last, never armed mid-select.
            enable_both: &[RegOp::set($senboth), RegOp::set($int_en)],
            disable_int: &[RegOp::clear($int_en)],
            in_level: Reg::new($in_level),
            out_level: Reg::new($out_read),
            int_enable: Reg::new($int_en),
            int_status: Reg::new($int_sts),
            sense_both: Reg::new($senboth),
        }
    };
}

/// GPIO group ABCD (GPIOA–GPIOD): write latch at 0x000, input sampled at 0x000, latch read at 0x0c0.
pub const ABCD: GpioMap = gpio_group!(
    dataval: 0x000, dir: 0x004, int_en: 0x008,
    sen0: 0x00c, sen1: 0x010, senboth: 0x014, int_sts: 0x018, in_level: 0x000, out_read: 0x0c0,
);

/// GPIO group EFGH (GPIOE–GPIOH): input-read offset unverified; reads the data reg (no test reads it).
pub const EFGH: GpioMap = gpio_group!(
    dataval: 0x020, dir: 0x024, int_en: 0x028,
    sen0: 0x02c, sen1: 0x030, senboth: 0x034, int_sts: 0x038, in_level: 0x020, out_read: 0x0c4,
);

/// GPIO group IJKL (GPIOI–GPIOL): int_en breaks stride (0x098); input-read reads the data reg.
pub const IJKL: GpioMap = gpio_group!(
    dataval: 0x070, dir: 0x074, int_en: 0x098,
    sen0: 0x09c, sen1: 0x0a0, senboth: 0x0a4, int_sts: 0x0a8, in_level: 0x070, out_read: 0x0c8,
);

/// Reject a contradictory or unevenly-slotted role in any group at compile time — zero runtime cost.
const _: () = assert!(
    role_cfgs_ok(&ABCD) && role_cfgs_ok(&EFGH) && role_cfgs_ok(&IJKL),
    "an AST GPIO group has a self-conflicting or unevenly-slotted role"
);
