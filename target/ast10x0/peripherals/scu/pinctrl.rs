// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SCU Pin Control (Pinctrl) for multi-function pin configuration.
//!
//! Provides register-level access to configure AST1060 pins for different
//! functions (I2C, I3C, GPIO, etc.) via SCU multiplex registers.

use super::ScuRegisters;
use paste::paste;

/// Describes a single pin configuration operation.
#[derive(Clone, Copy, Debug)]
pub struct PinctrlPin {
    /// SCU register offset (0x410, 0x414, 0x690, etc.)
    pub offset: u32,
    /// Bit position within the register (0-31)
    pub bit: u32,
    /// true = clear bit, false = set bit
    pub clear: bool,
}

/// Macro to generate paired pin constants for set/clear operations.
macro_rules! gen_pin_pairs {
    ($reg_name:ident, $offset:expr, $bit:expr) => {
        paste! {
            pub const [<PIN_ $reg_name _ $bit>]: PinctrlPin = PinctrlPin {
                offset: $offset,
                bit: $bit,
                clear: false,
            };

            pub const [<CLR_PIN_ $reg_name _ $bit>]: PinctrlPin = PinctrlPin {
                offset: $offset,
                bit: $bit,
                clear: true,
            };
        }
    };
}

// Generate individual pin constants for each SCU register and bit position
paste! {
    gen_pin_pairs!(SCU410, 0x410, 0);
    gen_pin_pairs!(SCU410, 0x410, 1);
    gen_pin_pairs!(SCU410, 0x410, 2);
    gen_pin_pairs!(SCU410, 0x410, 3);
    gen_pin_pairs!(SCU410, 0x410, 4);
    gen_pin_pairs!(SCU410, 0x410, 5);
    gen_pin_pairs!(SCU410, 0x410, 6);
    gen_pin_pairs!(SCU410, 0x410, 7);
    gen_pin_pairs!(SCU410, 0x410, 8);
    gen_pin_pairs!(SCU410, 0x410, 9);
    gen_pin_pairs!(SCU410, 0x410, 10);
    gen_pin_pairs!(SCU410, 0x410, 11);
    gen_pin_pairs!(SCU410, 0x410, 12);
    gen_pin_pairs!(SCU410, 0x410, 13);
    gen_pin_pairs!(SCU410, 0x410, 14);
    gen_pin_pairs!(SCU410, 0x410, 15);
    gen_pin_pairs!(SCU410, 0x410, 16);
    gen_pin_pairs!(SCU410, 0x410, 17);
    gen_pin_pairs!(SCU410, 0x410, 18);
    gen_pin_pairs!(SCU410, 0x410, 19);
    gen_pin_pairs!(SCU410, 0x410, 20);
    gen_pin_pairs!(SCU410, 0x410, 21);
    gen_pin_pairs!(SCU410, 0x410, 22);
    gen_pin_pairs!(SCU410, 0x410, 23);
    gen_pin_pairs!(SCU410, 0x410, 24);
    gen_pin_pairs!(SCU410, 0x410, 25);
    gen_pin_pairs!(SCU410, 0x410, 26);
    gen_pin_pairs!(SCU410, 0x410, 27);
    gen_pin_pairs!(SCU410, 0x410, 28);
    gen_pin_pairs!(SCU410, 0x410, 29);
    gen_pin_pairs!(SCU410, 0x410, 30);
    gen_pin_pairs!(SCU410, 0x410, 31);

    gen_pin_pairs!(SCU414, 0x414, 0);
    gen_pin_pairs!(SCU414, 0x414, 1);
    gen_pin_pairs!(SCU414, 0x414, 2);
    gen_pin_pairs!(SCU414, 0x414, 3);
    gen_pin_pairs!(SCU414, 0x414, 4);
    gen_pin_pairs!(SCU414, 0x414, 5);
    gen_pin_pairs!(SCU414, 0x414, 6);
    gen_pin_pairs!(SCU414, 0x414, 7);
    gen_pin_pairs!(SCU414, 0x414, 8);
    gen_pin_pairs!(SCU414, 0x414, 9);
    gen_pin_pairs!(SCU414, 0x414, 10);
    gen_pin_pairs!(SCU414, 0x414, 11);
    gen_pin_pairs!(SCU414, 0x414, 12);
    gen_pin_pairs!(SCU414, 0x414, 13);
    gen_pin_pairs!(SCU414, 0x414, 14);
    gen_pin_pairs!(SCU414, 0x414, 15);
    gen_pin_pairs!(SCU414, 0x414, 16);
    gen_pin_pairs!(SCU414, 0x414, 17);
    gen_pin_pairs!(SCU414, 0x414, 18);
    gen_pin_pairs!(SCU414, 0x414, 19);
    gen_pin_pairs!(SCU414, 0x414, 20);
    gen_pin_pairs!(SCU414, 0x414, 21);
    gen_pin_pairs!(SCU414, 0x414, 22);
    gen_pin_pairs!(SCU414, 0x414, 23);
    gen_pin_pairs!(SCU414, 0x414, 24);
    gen_pin_pairs!(SCU414, 0x414, 25);
    gen_pin_pairs!(SCU414, 0x414, 26);
    gen_pin_pairs!(SCU414, 0x414, 27);
    gen_pin_pairs!(SCU414, 0x414, 28);
    gen_pin_pairs!(SCU414, 0x414, 29);
    gen_pin_pairs!(SCU414, 0x414, 30);
    gen_pin_pairs!(SCU414, 0x414, 31);

    gen_pin_pairs!(SCU418, 0x418, 0);
    gen_pin_pairs!(SCU418, 0x418, 1);
    gen_pin_pairs!(SCU418, 0x418, 2);
    gen_pin_pairs!(SCU418, 0x418, 3);
    gen_pin_pairs!(SCU418, 0x418, 4);
    gen_pin_pairs!(SCU418, 0x418, 5);
    gen_pin_pairs!(SCU418, 0x418, 6);
    gen_pin_pairs!(SCU418, 0x418, 7);
    gen_pin_pairs!(SCU418, 0x418, 8);
    gen_pin_pairs!(SCU418, 0x418, 9);
    gen_pin_pairs!(SCU418, 0x418, 10);
    gen_pin_pairs!(SCU418, 0x418, 11);
    gen_pin_pairs!(SCU418, 0x418, 12);
    gen_pin_pairs!(SCU418, 0x418, 13);
    gen_pin_pairs!(SCU418, 0x418, 14);
    gen_pin_pairs!(SCU418, 0x418, 15);
    gen_pin_pairs!(SCU418, 0x418, 16);
    gen_pin_pairs!(SCU418, 0x418, 17);
    gen_pin_pairs!(SCU418, 0x418, 18);
    gen_pin_pairs!(SCU418, 0x418, 19);
    gen_pin_pairs!(SCU418, 0x418, 20);
    gen_pin_pairs!(SCU418, 0x418, 21);
    gen_pin_pairs!(SCU418, 0x418, 22);
    gen_pin_pairs!(SCU418, 0x418, 23);
    gen_pin_pairs!(SCU418, 0x418, 24);
    gen_pin_pairs!(SCU418, 0x418, 25);
    gen_pin_pairs!(SCU418, 0x418, 26);
    gen_pin_pairs!(SCU418, 0x418, 27);
    gen_pin_pairs!(SCU418, 0x418, 28);
    gen_pin_pairs!(SCU418, 0x418, 29);
    gen_pin_pairs!(SCU418, 0x418, 30);
    gen_pin_pairs!(SCU418, 0x418, 31);

    gen_pin_pairs!(SCU41C, 0x41C, 0);
    gen_pin_pairs!(SCU41C, 0x41C, 1);
    gen_pin_pairs!(SCU41C, 0x41C, 2);
    gen_pin_pairs!(SCU41C, 0x41C, 3);
    gen_pin_pairs!(SCU41C, 0x41C, 4);
    gen_pin_pairs!(SCU41C, 0x41C, 5);
    gen_pin_pairs!(SCU41C, 0x41C, 6);
    gen_pin_pairs!(SCU41C, 0x41C, 7);
    gen_pin_pairs!(SCU41C, 0x41C, 8);
    gen_pin_pairs!(SCU41C, 0x41C, 9);
    gen_pin_pairs!(SCU41C, 0x41C, 10);
    gen_pin_pairs!(SCU41C, 0x41C, 11);
    gen_pin_pairs!(SCU41C, 0x41C, 12);
    gen_pin_pairs!(SCU41C, 0x41C, 13);
    gen_pin_pairs!(SCU41C, 0x41C, 14);
    gen_pin_pairs!(SCU41C, 0x41C, 15);
    gen_pin_pairs!(SCU41C, 0x41C, 16);
    gen_pin_pairs!(SCU41C, 0x41C, 17);
    gen_pin_pairs!(SCU41C, 0x41C, 18);
    gen_pin_pairs!(SCU41C, 0x41C, 19);
    gen_pin_pairs!(SCU41C, 0x41C, 20);
    gen_pin_pairs!(SCU41C, 0x41C, 21);
    gen_pin_pairs!(SCU41C, 0x41C, 22);
    gen_pin_pairs!(SCU41C, 0x41C, 23);
    gen_pin_pairs!(SCU41C, 0x41C, 24);
    gen_pin_pairs!(SCU41C, 0x41C, 25);
    gen_pin_pairs!(SCU41C, 0x41C, 26);
    gen_pin_pairs!(SCU41C, 0x41C, 27);
    gen_pin_pairs!(SCU41C, 0x41C, 28);
    gen_pin_pairs!(SCU41C, 0x41C, 29);
    gen_pin_pairs!(SCU41C, 0x41C, 30);
    gen_pin_pairs!(SCU41C, 0x41C, 31);

    gen_pin_pairs!(SCU430, 0x430, 0);
    gen_pin_pairs!(SCU430, 0x430, 1);
    gen_pin_pairs!(SCU430, 0x430, 2);
    gen_pin_pairs!(SCU430, 0x430, 3);
    gen_pin_pairs!(SCU430, 0x430, 4);
    gen_pin_pairs!(SCU430, 0x430, 5);
    gen_pin_pairs!(SCU430, 0x430, 6);
    gen_pin_pairs!(SCU430, 0x430, 7);
    gen_pin_pairs!(SCU430, 0x430, 8);
    gen_pin_pairs!(SCU430, 0x430, 9);
    gen_pin_pairs!(SCU430, 0x430, 10);
    gen_pin_pairs!(SCU430, 0x430, 11);
    gen_pin_pairs!(SCU430, 0x430, 12);
    gen_pin_pairs!(SCU430, 0x430, 13);
    gen_pin_pairs!(SCU430, 0x430, 14);
    gen_pin_pairs!(SCU430, 0x430, 15);
    gen_pin_pairs!(SCU430, 0x430, 16);
    gen_pin_pairs!(SCU430, 0x430, 17);
    gen_pin_pairs!(SCU430, 0x430, 18);
    gen_pin_pairs!(SCU430, 0x430, 19);
    gen_pin_pairs!(SCU430, 0x430, 20);
    gen_pin_pairs!(SCU430, 0x430, 21);
    gen_pin_pairs!(SCU430, 0x430, 22);
    gen_pin_pairs!(SCU430, 0x430, 23);
    gen_pin_pairs!(SCU430, 0x430, 24);
    gen_pin_pairs!(SCU430, 0x430, 25);
    gen_pin_pairs!(SCU430, 0x430, 26);
    gen_pin_pairs!(SCU430, 0x430, 27);
    gen_pin_pairs!(SCU430, 0x430, 28);
    gen_pin_pairs!(SCU430, 0x430, 29);
    gen_pin_pairs!(SCU430, 0x430, 30);
    gen_pin_pairs!(SCU430, 0x430, 31);

    gen_pin_pairs!(SCU434, 0x434, 0);
    gen_pin_pairs!(SCU434, 0x434, 1);
    gen_pin_pairs!(SCU434, 0x434, 2);
    gen_pin_pairs!(SCU434, 0x434, 3);
    gen_pin_pairs!(SCU434, 0x434, 4);
    gen_pin_pairs!(SCU434, 0x434, 5);
    gen_pin_pairs!(SCU434, 0x434, 6);
    gen_pin_pairs!(SCU434, 0x434, 7);
    gen_pin_pairs!(SCU434, 0x434, 8);
    gen_pin_pairs!(SCU434, 0x434, 9);
    gen_pin_pairs!(SCU434, 0x434, 10);
    gen_pin_pairs!(SCU434, 0x434, 11);
    gen_pin_pairs!(SCU434, 0x434, 12);
    gen_pin_pairs!(SCU434, 0x434, 13);
    gen_pin_pairs!(SCU434, 0x434, 14);
    gen_pin_pairs!(SCU434, 0x434, 15);
    gen_pin_pairs!(SCU434, 0x434, 16);
    gen_pin_pairs!(SCU434, 0x434, 17);
    gen_pin_pairs!(SCU434, 0x434, 18);
    gen_pin_pairs!(SCU434, 0x434, 19);
    gen_pin_pairs!(SCU434, 0x434, 20);
    gen_pin_pairs!(SCU434, 0x434, 21);
    gen_pin_pairs!(SCU434, 0x434, 22);
    gen_pin_pairs!(SCU434, 0x434, 23);
    gen_pin_pairs!(SCU434, 0x434, 24);
    gen_pin_pairs!(SCU434, 0x434, 25);
    gen_pin_pairs!(SCU434, 0x434, 26);
    gen_pin_pairs!(SCU434, 0x434, 27);
    gen_pin_pairs!(SCU434, 0x434, 28);
    gen_pin_pairs!(SCU434, 0x434, 29);
    gen_pin_pairs!(SCU434, 0x434, 30);
    gen_pin_pairs!(SCU434, 0x434, 31);

    gen_pin_pairs!(SCU4B0, 0x4B0, 0);
    gen_pin_pairs!(SCU4B0, 0x4B0, 1);
    gen_pin_pairs!(SCU4B0, 0x4B0, 2);
    gen_pin_pairs!(SCU4B0, 0x4B0, 3);
    gen_pin_pairs!(SCU4B0, 0x4B0, 4);
    gen_pin_pairs!(SCU4B0, 0x4B0, 5);
    gen_pin_pairs!(SCU4B0, 0x4B0, 6);
    gen_pin_pairs!(SCU4B0, 0x4B0, 7);
    gen_pin_pairs!(SCU4B0, 0x4B0, 8);
    gen_pin_pairs!(SCU4B0, 0x4B0, 9);
    gen_pin_pairs!(SCU4B0, 0x4B0, 10);
    gen_pin_pairs!(SCU4B0, 0x4B0, 11);
    gen_pin_pairs!(SCU4B0, 0x4B0, 12);
    gen_pin_pairs!(SCU4B0, 0x4B0, 13);
    gen_pin_pairs!(SCU4B0, 0x4B0, 14);
    gen_pin_pairs!(SCU4B0, 0x4B0, 15);
    gen_pin_pairs!(SCU4B0, 0x4B0, 16);
    gen_pin_pairs!(SCU4B0, 0x4B0, 17);
    gen_pin_pairs!(SCU4B0, 0x4B0, 18);
    gen_pin_pairs!(SCU4B0, 0x4B0, 19);
    gen_pin_pairs!(SCU4B0, 0x4B0, 20);
    gen_pin_pairs!(SCU4B0, 0x4B0, 21);
    gen_pin_pairs!(SCU4B0, 0x4B0, 22);
    gen_pin_pairs!(SCU4B0, 0x4B0, 23);
    gen_pin_pairs!(SCU4B0, 0x4B0, 24);
    gen_pin_pairs!(SCU4B0, 0x4B0, 25);
    gen_pin_pairs!(SCU4B0, 0x4B0, 26);
    gen_pin_pairs!(SCU4B0, 0x4B0, 27);
    gen_pin_pairs!(SCU4B0, 0x4B0, 28);
    gen_pin_pairs!(SCU4B0, 0x4B0, 29);
    gen_pin_pairs!(SCU4B0, 0x4B0, 30);
    gen_pin_pairs!(SCU4B0, 0x4B0, 31);

    gen_pin_pairs!(SCU4B4, 0x4B4, 0);
    gen_pin_pairs!(SCU4B4, 0x4B4, 1);
    gen_pin_pairs!(SCU4B4, 0x4B4, 2);
    gen_pin_pairs!(SCU4B4, 0x4B4, 3);
    gen_pin_pairs!(SCU4B4, 0x4B4, 4);
    gen_pin_pairs!(SCU4B4, 0x4B4, 5);
    gen_pin_pairs!(SCU4B4, 0x4B4, 6);
    gen_pin_pairs!(SCU4B4, 0x4B4, 7);
    gen_pin_pairs!(SCU4B4, 0x4B4, 8);
    gen_pin_pairs!(SCU4B4, 0x4B4, 9);
    gen_pin_pairs!(SCU4B4, 0x4B4, 10);
    gen_pin_pairs!(SCU4B4, 0x4B4, 11);
    gen_pin_pairs!(SCU4B4, 0x4B4, 12);
    gen_pin_pairs!(SCU4B4, 0x4B4, 13);
    gen_pin_pairs!(SCU4B4, 0x4B4, 14);
    gen_pin_pairs!(SCU4B4, 0x4B4, 15);
    gen_pin_pairs!(SCU4B4, 0x4B4, 16);
    gen_pin_pairs!(SCU4B4, 0x4B4, 17);
    gen_pin_pairs!(SCU4B4, 0x4B4, 18);
    gen_pin_pairs!(SCU4B4, 0x4B4, 19);
    gen_pin_pairs!(SCU4B4, 0x4B4, 20);
    gen_pin_pairs!(SCU4B4, 0x4B4, 21);
    gen_pin_pairs!(SCU4B4, 0x4B4, 22);
    gen_pin_pairs!(SCU4B4, 0x4B4, 23);
    gen_pin_pairs!(SCU4B4, 0x4B4, 24);
    gen_pin_pairs!(SCU4B4, 0x4B4, 25);
    gen_pin_pairs!(SCU4B4, 0x4B4, 26);
    gen_pin_pairs!(SCU4B4, 0x4B4, 27);
    gen_pin_pairs!(SCU4B4, 0x4B4, 28);
    gen_pin_pairs!(SCU4B4, 0x4B4, 29);
    gen_pin_pairs!(SCU4B4, 0x4B4, 30);
    gen_pin_pairs!(SCU4B4, 0x4B4, 31);

    gen_pin_pairs!(SCU4B8, 0x4B8, 0);
    gen_pin_pairs!(SCU4B8, 0x4B8, 1);
    gen_pin_pairs!(SCU4B8, 0x4B8, 2);
    gen_pin_pairs!(SCU4B8, 0x4B8, 3);
    gen_pin_pairs!(SCU4B8, 0x4B8, 4);
    gen_pin_pairs!(SCU4B8, 0x4B8, 5);
    gen_pin_pairs!(SCU4B8, 0x4B8, 6);
    gen_pin_pairs!(SCU4B8, 0x4B8, 7);
    gen_pin_pairs!(SCU4B8, 0x4B8, 8);
    gen_pin_pairs!(SCU4B8, 0x4B8, 9);
    gen_pin_pairs!(SCU4B8, 0x4B8, 10);
    gen_pin_pairs!(SCU4B8, 0x4B8, 11);
    gen_pin_pairs!(SCU4B8, 0x4B8, 12);
    gen_pin_pairs!(SCU4B8, 0x4B8, 13);
    gen_pin_pairs!(SCU4B8, 0x4B8, 14);
    gen_pin_pairs!(SCU4B8, 0x4B8, 15);
    gen_pin_pairs!(SCU4B8, 0x4B8, 16);
    gen_pin_pairs!(SCU4B8, 0x4B8, 17);
    gen_pin_pairs!(SCU4B8, 0x4B8, 18);
    gen_pin_pairs!(SCU4B8, 0x4B8, 19);
    gen_pin_pairs!(SCU4B8, 0x4B8, 20);
    gen_pin_pairs!(SCU4B8, 0x4B8, 21);
    gen_pin_pairs!(SCU4B8, 0x4B8, 22);
    gen_pin_pairs!(SCU4B8, 0x4B8, 23);
    gen_pin_pairs!(SCU4B8, 0x4B8, 24);
    gen_pin_pairs!(SCU4B8, 0x4B8, 25);
    gen_pin_pairs!(SCU4B8, 0x4B8, 26);
    gen_pin_pairs!(SCU4B8, 0x4B8, 27);
    gen_pin_pairs!(SCU4B8, 0x4B8, 28);
    gen_pin_pairs!(SCU4B8, 0x4B8, 29);
    gen_pin_pairs!(SCU4B8, 0x4B8, 30);
    gen_pin_pairs!(SCU4B8, 0x4B8, 31);

    gen_pin_pairs!(SCU4BC, 0x4BC, 0);
    gen_pin_pairs!(SCU4BC, 0x4BC, 1);
    gen_pin_pairs!(SCU4BC, 0x4BC, 2);
    gen_pin_pairs!(SCU4BC, 0x4BC, 3);
    gen_pin_pairs!(SCU4BC, 0x4BC, 4);
    gen_pin_pairs!(SCU4BC, 0x4BC, 5);
    gen_pin_pairs!(SCU4BC, 0x4BC, 6);
    gen_pin_pairs!(SCU4BC, 0x4BC, 7);
    gen_pin_pairs!(SCU4BC, 0x4BC, 8);
    gen_pin_pairs!(SCU4BC, 0x4BC, 9);
    gen_pin_pairs!(SCU4BC, 0x4BC, 10);
    gen_pin_pairs!(SCU4BC, 0x4BC, 11);
    gen_pin_pairs!(SCU4BC, 0x4BC, 12);
    gen_pin_pairs!(SCU4BC, 0x4BC, 13);
    gen_pin_pairs!(SCU4BC, 0x4BC, 14);
    gen_pin_pairs!(SCU4BC, 0x4BC, 15);
    gen_pin_pairs!(SCU4BC, 0x4BC, 16);
    gen_pin_pairs!(SCU4BC, 0x4BC, 17);
    gen_pin_pairs!(SCU4BC, 0x4BC, 18);
    gen_pin_pairs!(SCU4BC, 0x4BC, 19);
    gen_pin_pairs!(SCU4BC, 0x4BC, 20);
    gen_pin_pairs!(SCU4BC, 0x4BC, 21);
    gen_pin_pairs!(SCU4BC, 0x4BC, 22);
    gen_pin_pairs!(SCU4BC, 0x4BC, 23);
    gen_pin_pairs!(SCU4BC, 0x4BC, 24);
    gen_pin_pairs!(SCU4BC, 0x4BC, 25);
    gen_pin_pairs!(SCU4BC, 0x4BC, 26);
    gen_pin_pairs!(SCU4BC, 0x4BC, 27);
    gen_pin_pairs!(SCU4BC, 0x4BC, 28);
    gen_pin_pairs!(SCU4BC, 0x4BC, 29);
    gen_pin_pairs!(SCU4BC, 0x4BC, 30);
    gen_pin_pairs!(SCU4BC, 0x4BC, 31);

    gen_pin_pairs!(SCU690, 0x690, 0);
    gen_pin_pairs!(SCU690, 0x690, 1);
    gen_pin_pairs!(SCU690, 0x690, 2);
    gen_pin_pairs!(SCU690, 0x690, 3);
    gen_pin_pairs!(SCU690, 0x690, 4);
    gen_pin_pairs!(SCU690, 0x690, 5);
    gen_pin_pairs!(SCU690, 0x690, 6);
    gen_pin_pairs!(SCU690, 0x690, 7);
    gen_pin_pairs!(SCU690, 0x690, 8);
    gen_pin_pairs!(SCU690, 0x690, 9);
    gen_pin_pairs!(SCU690, 0x690, 10);
    gen_pin_pairs!(SCU690, 0x690, 11);
    gen_pin_pairs!(SCU690, 0x690, 12);
    gen_pin_pairs!(SCU690, 0x690, 13);
    gen_pin_pairs!(SCU690, 0x690, 14);
    gen_pin_pairs!(SCU690, 0x690, 15);
    gen_pin_pairs!(SCU690, 0x690, 16);
    gen_pin_pairs!(SCU690, 0x690, 17);
    gen_pin_pairs!(SCU690, 0x690, 18);
    gen_pin_pairs!(SCU690, 0x690, 19);
    gen_pin_pairs!(SCU690, 0x690, 20);
    gen_pin_pairs!(SCU690, 0x690, 21);
    gen_pin_pairs!(SCU690, 0x690, 22);
    gen_pin_pairs!(SCU690, 0x690, 23);
    gen_pin_pairs!(SCU690, 0x690, 24);
    gen_pin_pairs!(SCU690, 0x690, 25);
    gen_pin_pairs!(SCU690, 0x690, 26);
    gen_pin_pairs!(SCU690, 0x690, 27);
    gen_pin_pairs!(SCU690, 0x690, 28);
    gen_pin_pairs!(SCU690, 0x690, 29);
    gen_pin_pairs!(SCU690, 0x690, 30);
    gen_pin_pairs!(SCU690, 0x690, 31);

    gen_pin_pairs!(SCU694, 0x694, 0);
    gen_pin_pairs!(SCU694, 0x694, 1);
    gen_pin_pairs!(SCU694, 0x694, 2);
    gen_pin_pairs!(SCU694, 0x694, 3);
    gen_pin_pairs!(SCU694, 0x694, 4);
    gen_pin_pairs!(SCU694, 0x694, 5);
    gen_pin_pairs!(SCU694, 0x694, 6);
    gen_pin_pairs!(SCU694, 0x694, 7);
    gen_pin_pairs!(SCU694, 0x694, 8);
    gen_pin_pairs!(SCU694, 0x694, 9);
    gen_pin_pairs!(SCU694, 0x694, 10);
    gen_pin_pairs!(SCU694, 0x694, 11);
    gen_pin_pairs!(SCU694, 0x694, 12);
    gen_pin_pairs!(SCU694, 0x694, 13);
    gen_pin_pairs!(SCU694, 0x694, 14);
    gen_pin_pairs!(SCU694, 0x694, 15);
    gen_pin_pairs!(SCU694, 0x694, 16);
    gen_pin_pairs!(SCU694, 0x694, 17);
    gen_pin_pairs!(SCU694, 0x694, 18);
    gen_pin_pairs!(SCU694, 0x694, 19);
    gen_pin_pairs!(SCU694, 0x694, 20);
    gen_pin_pairs!(SCU694, 0x694, 21);
    gen_pin_pairs!(SCU694, 0x694, 22);
    gen_pin_pairs!(SCU694, 0x694, 23);
    gen_pin_pairs!(SCU694, 0x694, 24);
    gen_pin_pairs!(SCU694, 0x694, 25);
    gen_pin_pairs!(SCU694, 0x694, 26);
    gen_pin_pairs!(SCU694, 0x694, 27);
    gen_pin_pairs!(SCU694, 0x694, 28);
    gen_pin_pairs!(SCU694, 0x694, 29);
    gen_pin_pairs!(SCU694, 0x694, 30);
    gen_pin_pairs!(SCU694, 0x694, 31);

    gen_pin_pairs!(SCU698, 0x698, 0);
    gen_pin_pairs!(SCU698, 0x698, 1);
    gen_pin_pairs!(SCU698, 0x698, 2);
    gen_pin_pairs!(SCU698, 0x698, 3);
    gen_pin_pairs!(SCU698, 0x698, 4);
    gen_pin_pairs!(SCU698, 0x698, 5);
    gen_pin_pairs!(SCU698, 0x698, 6);
    gen_pin_pairs!(SCU698, 0x698, 7);
    gen_pin_pairs!(SCU698, 0x698, 8);
    gen_pin_pairs!(SCU698, 0x698, 9);
    gen_pin_pairs!(SCU698, 0x698, 10);
    gen_pin_pairs!(SCU698, 0x698, 11);
    gen_pin_pairs!(SCU698, 0x698, 12);
    gen_pin_pairs!(SCU698, 0x698, 13);
    gen_pin_pairs!(SCU698, 0x698, 14);
    gen_pin_pairs!(SCU698, 0x698, 15);
    gen_pin_pairs!(SCU698, 0x698, 16);
    gen_pin_pairs!(SCU698, 0x698, 17);
    gen_pin_pairs!(SCU698, 0x698, 18);
    gen_pin_pairs!(SCU698, 0x698, 19);
    gen_pin_pairs!(SCU698, 0x698, 20);
    gen_pin_pairs!(SCU698, 0x698, 21);
    gen_pin_pairs!(SCU698, 0x698, 22);
    gen_pin_pairs!(SCU698, 0x698, 23);
    gen_pin_pairs!(SCU698, 0x698, 24);
    gen_pin_pairs!(SCU698, 0x698, 25);
    gen_pin_pairs!(SCU698, 0x698, 26);
    gen_pin_pairs!(SCU698, 0x698, 27);
    gen_pin_pairs!(SCU698, 0x698, 28);
    gen_pin_pairs!(SCU698, 0x698, 29);
    gen_pin_pairs!(SCU698, 0x698, 30);
    gen_pin_pairs!(SCU698, 0x698, 31);

    gen_pin_pairs!(SCU69C, 0x69C, 0);
    gen_pin_pairs!(SCU69C, 0x69C, 1);
    gen_pin_pairs!(SCU69C, 0x69C, 2);
    gen_pin_pairs!(SCU69C, 0x69C, 3);
    gen_pin_pairs!(SCU69C, 0x69C, 4);
    gen_pin_pairs!(SCU69C, 0x69C, 5);
    gen_pin_pairs!(SCU69C, 0x69C, 6);
    gen_pin_pairs!(SCU69C, 0x69C, 7);
    gen_pin_pairs!(SCU69C, 0x69C, 8);
    gen_pin_pairs!(SCU69C, 0x69C, 9);
    gen_pin_pairs!(SCU69C, 0x69C, 10);
    gen_pin_pairs!(SCU69C, 0x69C, 11);
    gen_pin_pairs!(SCU69C, 0x69C, 12);
    gen_pin_pairs!(SCU69C, 0x69C, 13);
    gen_pin_pairs!(SCU69C, 0x69C, 14);
    gen_pin_pairs!(SCU69C, 0x69C, 15);
    gen_pin_pairs!(SCU69C, 0x69C, 16);
    gen_pin_pairs!(SCU69C, 0x69C, 17);
    gen_pin_pairs!(SCU69C, 0x69C, 18);
    gen_pin_pairs!(SCU69C, 0x69C, 19);
    gen_pin_pairs!(SCU69C, 0x69C, 20);
    gen_pin_pairs!(SCU69C, 0x69C, 21);
    gen_pin_pairs!(SCU69C, 0x69C, 22);
    gen_pin_pairs!(SCU69C, 0x69C, 23);
    gen_pin_pairs!(SCU69C, 0x69C, 24);
    gen_pin_pairs!(SCU69C, 0x69C, 25);
    gen_pin_pairs!(SCU69C, 0x69C, 26);
    gen_pin_pairs!(SCU69C, 0x69C, 27);
    gen_pin_pairs!(SCU69C, 0x69C, 28);
    gen_pin_pairs!(SCU69C, 0x69C, 29);
    gen_pin_pairs!(SCU69C, 0x69C, 30);
    gen_pin_pairs!(SCU69C, 0x69C, 31);

    gen_pin_pairs!(SCU6B0, 0x6B0, 0);
    gen_pin_pairs!(SCU6B0, 0x6B0, 1);
    gen_pin_pairs!(SCU6B0, 0x6B0, 2);
    gen_pin_pairs!(SCU6B0, 0x6B0, 3);
    gen_pin_pairs!(SCU6B0, 0x6B0, 4);
    gen_pin_pairs!(SCU6B0, 0x6B0, 5);
    gen_pin_pairs!(SCU6B0, 0x6B0, 6);
    gen_pin_pairs!(SCU6B0, 0x6B0, 7);
    gen_pin_pairs!(SCU6B0, 0x6B0, 8);
    gen_pin_pairs!(SCU6B0, 0x6B0, 9);
    gen_pin_pairs!(SCU6B0, 0x6B0, 10);
    gen_pin_pairs!(SCU6B0, 0x6B0, 11);
    gen_pin_pairs!(SCU6B0, 0x6B0, 12);
    gen_pin_pairs!(SCU6B0, 0x6B0, 13);
    gen_pin_pairs!(SCU6B0, 0x6B0, 14);
    gen_pin_pairs!(SCU6B0, 0x6B0, 15);
    gen_pin_pairs!(SCU6B0, 0x6B0, 16);
    gen_pin_pairs!(SCU6B0, 0x6B0, 17);
    gen_pin_pairs!(SCU6B0, 0x6B0, 18);
    gen_pin_pairs!(SCU6B0, 0x6B0, 19);
    gen_pin_pairs!(SCU6B0, 0x6B0, 20);
    gen_pin_pairs!(SCU6B0, 0x6B0, 21);
    gen_pin_pairs!(SCU6B0, 0x6B0, 22);
    gen_pin_pairs!(SCU6B0, 0x6B0, 23);
    gen_pin_pairs!(SCU6B0, 0x6B0, 24);
    gen_pin_pairs!(SCU6B0, 0x6B0, 25);
    gen_pin_pairs!(SCU6B0, 0x6B0, 26);
    gen_pin_pairs!(SCU6B0, 0x6B0, 27);
    gen_pin_pairs!(SCU6B0, 0x6B0, 28);
    gen_pin_pairs!(SCU6B0, 0x6B0, 29);
    gen_pin_pairs!(SCU6B0, 0x6B0, 30);
    gen_pin_pairs!(SCU6B0, 0x6B0, 31);
}

/// I2C1 pin group: SCL2/SDA2 mux selection on SCU414[30:31].
///
/// The SVD names these EnblSCL2FnPin/EnblSDA2FnPin, but they correspond to
/// PAC peripheral I2c1 (controller 1, base 0x7e7b_0100). The SVD uses
/// 1-based naming where "2" means "second bus", matching PAC I2c1.
pub const PINCTRL_I2C1: &[PinctrlPin] = &[PIN_SCU414_30, PIN_SCU414_31];

/// I2C2 pin group: SCL3/SDA3 mux selection on SCU418[0:1].
///
/// The SVD names these EnblSCL3FnPin/EnblSDA3FnPin, corresponding to
/// PAC peripheral I2c2 (controller 2, base 0x7e7b_0180). On the Test
/// Harness board, these pins are exposed on J15 and used for inter-device
/// I2C communication between the two AST1060 daughter cards.
pub const PINCTRL_I2C2: &[PinctrlPin] = &[PIN_SCU418_0, PIN_SCU418_1];

/// GPIOA pin groups: GPIO mux selection.
pub const PINCTRL_GPIOA0: &[PinctrlPin] = &[CLR_PIN_SCU410_0, CLR_PIN_SCU4B0_0, CLR_PIN_SCU690_0];
pub const PINCTRL_GPIOA1: &[PinctrlPin] = &[CLR_PIN_SCU410_1, CLR_PIN_SCU4B0_1, CLR_PIN_SCU690_1];
pub const PINCTRL_GPIOA2: &[PinctrlPin] = &[CLR_PIN_SCU410_2, CLR_PIN_SCU4B0_2, CLR_PIN_SCU690_2];
pub const PINCTRL_GPIOA3: &[PinctrlPin] = &[CLR_PIN_SCU410_3, CLR_PIN_SCU4B0_3, CLR_PIN_SCU690_3];
pub const PINCTRL_GPIOA4: &[PinctrlPin] = &[CLR_PIN_SCU410_4, CLR_PIN_SCU4B0_4, CLR_PIN_SCU690_4];
pub const PINCTRL_GPIOA5: &[PinctrlPin] = &[CLR_PIN_SCU410_5, CLR_PIN_SCU4B0_5, CLR_PIN_SCU690_5];
pub const PINCTRL_GPIOA6: &[PinctrlPin] = &[CLR_PIN_SCU410_6, CLR_PIN_SCU4B0_6, CLR_PIN_SCU690_6];
pub const PINCTRL_GPIOA7: &[PinctrlPin] = &[CLR_PIN_SCU410_7, CLR_PIN_SCU4B0_7, CLR_PIN_SCU690_7];

/// GPIOB pin groups: GPIO mux selection.
pub const PINCTRL_GPIOB0: &[PinctrlPin] = &[CLR_PIN_SCU410_8, CLR_PIN_SCU4B0_8, CLR_PIN_SCU690_8];
pub const PINCTRL_GPIOB1: &[PinctrlPin] = &[CLR_PIN_SCU410_9, CLR_PIN_SCU4B0_9, CLR_PIN_SCU690_9];
pub const PINCTRL_GPIOB2: &[PinctrlPin] =
    &[CLR_PIN_SCU410_10, CLR_PIN_SCU4B0_10, CLR_PIN_SCU690_10];
pub const PINCTRL_GPIOB3: &[PinctrlPin] =
    &[CLR_PIN_SCU410_11, CLR_PIN_SCU4B0_11, CLR_PIN_SCU690_11];
pub const PINCTRL_GPIOB4: &[PinctrlPin] =
    &[CLR_PIN_SCU410_12, CLR_PIN_SCU4B0_12, CLR_PIN_SCU690_12];
pub const PINCTRL_GPIOB5: &[PinctrlPin] =
    &[CLR_PIN_SCU410_13, CLR_PIN_SCU4B0_13, CLR_PIN_SCU690_13];
pub const PINCTRL_GPIOB6: &[PinctrlPin] =
    &[CLR_PIN_SCU410_14, CLR_PIN_SCU4B0_14, CLR_PIN_SCU690_14];
pub const PINCTRL_GPIOB7: &[PinctrlPin] =
    &[CLR_PIN_SCU410_15, CLR_PIN_SCU4B0_15, CLR_PIN_SCU690_15];

/// GPIOC pin groups: GPIO mux selection.
pub const PINCTRL_GPIOC0: &[PinctrlPin] =
    &[CLR_PIN_SCU410_16, CLR_PIN_SCU4B0_16, CLR_PIN_SCU690_16];
pub const PINCTRL_GPIOC1: &[PinctrlPin] =
    &[CLR_PIN_SCU410_17, CLR_PIN_SCU4B0_17, CLR_PIN_SCU690_17];
pub const PINCTRL_GPIOC2: &[PinctrlPin] =
    &[CLR_PIN_SCU410_18, CLR_PIN_SCU4B0_18, CLR_PIN_SCU690_18];
pub const PINCTRL_GPIOC3: &[PinctrlPin] =
    &[CLR_PIN_SCU410_19, CLR_PIN_SCU4B0_19, CLR_PIN_SCU690_19];
pub const PINCTRL_GPIOC4: &[PinctrlPin] =
    &[CLR_PIN_SCU410_20, CLR_PIN_SCU4B0_20, CLR_PIN_SCU690_20];
pub const PINCTRL_GPIOC5: &[PinctrlPin] =
    &[CLR_PIN_SCU410_21, CLR_PIN_SCU4B0_21, CLR_PIN_SCU690_21];
pub const PINCTRL_GPIOC6: &[PinctrlPin] =
    &[CLR_PIN_SCU410_22, CLR_PIN_SCU4B0_22, CLR_PIN_SCU690_22];
pub const PINCTRL_GPIOC7: &[PinctrlPin] =
    &[CLR_PIN_SCU410_23, CLR_PIN_SCU4B0_23, CLR_PIN_SCU690_23];

/// GPIOD pin groups: GPIO mux selection.
pub const PINCTRL_GPIOD0: &[PinctrlPin] =
    &[CLR_PIN_SCU410_24, CLR_PIN_SCU4B0_24, CLR_PIN_SCU690_24];
pub const PINCTRL_GPIOD1: &[PinctrlPin] =
    &[CLR_PIN_SCU410_25, CLR_PIN_SCU4B0_25, CLR_PIN_SCU690_25];
pub const PINCTRL_GPIOD2: &[PinctrlPin] =
    &[CLR_PIN_SCU410_26, CLR_PIN_SCU4B0_26, CLR_PIN_SCU690_26];
pub const PINCTRL_GPIOD3: &[PinctrlPin] =
    &[CLR_PIN_SCU410_27, CLR_PIN_SCU4B0_27, CLR_PIN_SCU690_27];
pub const PINCTRL_GPIOD4: &[PinctrlPin] =
    &[CLR_PIN_SCU410_28, CLR_PIN_SCU4B0_28, CLR_PIN_SCU690_28];
pub const PINCTRL_GPIOD5: &[PinctrlPin] =
    &[CLR_PIN_SCU410_29, CLR_PIN_SCU4B0_29, CLR_PIN_SCU690_29];
pub const PINCTRL_GPIOD6: &[PinctrlPin] =
    &[CLR_PIN_SCU410_30, CLR_PIN_SCU4B0_30, CLR_PIN_SCU690_30];
pub const PINCTRL_GPIOD7: &[PinctrlPin] =
    &[CLR_PIN_SCU410_31, CLR_PIN_SCU4B0_31, CLR_PIN_SCU690_31];

/// GPIOE pin groups: GPIO mux selection.
pub const PINCTRL_GPIOE0: &[PinctrlPin] = &[CLR_PIN_SCU414_0, CLR_PIN_SCU4B4_0, CLR_PIN_SCU694_0];
pub const PINCTRL_GPIOE1: &[PinctrlPin] = &[CLR_PIN_SCU414_1, CLR_PIN_SCU4B4_1, CLR_PIN_SCU694_1];
pub const PINCTRL_GPIOE2: &[PinctrlPin] = &[CLR_PIN_SCU414_2, CLR_PIN_SCU4B4_2, CLR_PIN_SCU694_2];
pub const PINCTRL_GPIOE3: &[PinctrlPin] = &[CLR_PIN_SCU414_3, CLR_PIN_SCU4B4_3, CLR_PIN_SCU694_3];
pub const PINCTRL_GPIOE4: &[PinctrlPin] = &[CLR_PIN_SCU414_4, CLR_PIN_SCU4B4_4, CLR_PIN_SCU694_4];
pub const PINCTRL_GPIOE5: &[PinctrlPin] = &[CLR_PIN_SCU414_5, CLR_PIN_SCU4B4_5, CLR_PIN_SCU694_5];
pub const PINCTRL_GPIOE6: &[PinctrlPin] = &[CLR_PIN_SCU414_6, CLR_PIN_SCU4B4_6, CLR_PIN_SCU694_6];
pub const PINCTRL_GPIOE7: &[PinctrlPin] = &[CLR_PIN_SCU414_7, CLR_PIN_SCU4B4_7, CLR_PIN_SCU694_7];

/// GPIOF pin groups: GPIO mux selection.
pub const PINCTRL_GPIOF0: &[PinctrlPin] = &[CLR_PIN_SCU414_8, CLR_PIN_SCU4B4_8, CLR_PIN_SCU694_8];
pub const PINCTRL_GPIOF1: &[PinctrlPin] = &[CLR_PIN_SCU414_9, CLR_PIN_SCU4B4_9, CLR_PIN_SCU694_9];
pub const PINCTRL_GPIOF2: &[PinctrlPin] =
    &[CLR_PIN_SCU414_10, CLR_PIN_SCU4B4_10, CLR_PIN_SCU694_10];
pub const PINCTRL_GPIOF3: &[PinctrlPin] =
    &[CLR_PIN_SCU414_11, CLR_PIN_SCU4B4_11, CLR_PIN_SCU694_11];
pub const PINCTRL_GPIOF4: &[PinctrlPin] =
    &[CLR_PIN_SCU414_12, CLR_PIN_SCU4B4_12, CLR_PIN_SCU694_12];
pub const PINCTRL_GPIOF5: &[PinctrlPin] =
    &[CLR_PIN_SCU414_13, CLR_PIN_SCU4B4_13, CLR_PIN_SCU694_13];
pub const PINCTRL_GPIOF6: &[PinctrlPin] =
    &[CLR_PIN_SCU414_14, CLR_PIN_SCU4B4_14, CLR_PIN_SCU694_14];
pub const PINCTRL_GPIOF7: &[PinctrlPin] =
    &[CLR_PIN_SCU414_15, CLR_PIN_SCU4B4_15, CLR_PIN_SCU694_15];

/// GPIOG pin groups: GPIO mux selection.
pub const PINCTRL_GPIOG0: &[PinctrlPin] =
    &[CLR_PIN_SCU414_16, CLR_PIN_SCU4B4_16, CLR_PIN_SCU694_16];
pub const PINCTRL_GPIOG1: &[PinctrlPin] =
    &[CLR_PIN_SCU414_17, CLR_PIN_SCU4B4_17, CLR_PIN_SCU694_17];
pub const PINCTRL_GPIOG2: &[PinctrlPin] =
    &[CLR_PIN_SCU414_18, CLR_PIN_SCU4B4_18, CLR_PIN_SCU694_18];
pub const PINCTRL_GPIOG3: &[PinctrlPin] =
    &[CLR_PIN_SCU414_19, CLR_PIN_SCU4B4_19, CLR_PIN_SCU694_19];
pub const PINCTRL_GPIOG4: &[PinctrlPin] =
    &[CLR_PIN_SCU414_20, CLR_PIN_SCU4B4_20, CLR_PIN_SCU694_20];
pub const PINCTRL_GPIOG5: &[PinctrlPin] =
    &[CLR_PIN_SCU414_21, CLR_PIN_SCU4B4_21, CLR_PIN_SCU694_21];
pub const PINCTRL_GPIOG6: &[PinctrlPin] =
    &[CLR_PIN_SCU414_22, CLR_PIN_SCU4B4_22, CLR_PIN_SCU694_22];
pub const PINCTRL_GPIOG7: &[PinctrlPin] =
    &[CLR_PIN_SCU414_23, CLR_PIN_SCU4B4_23, CLR_PIN_SCU694_23];

/// GPIOH pin groups: GPIO mux selection.
pub const PINCTRL_GPIOH0: &[PinctrlPin] =
    &[CLR_PIN_SCU414_24, CLR_PIN_SCU4B4_24, CLR_PIN_SCU694_24];
pub const PINCTRL_GPIOH1: &[PinctrlPin] =
    &[CLR_PIN_SCU414_25, CLR_PIN_SCU4B4_25, CLR_PIN_SCU694_25];
pub const PINCTRL_GPIOH2: &[PinctrlPin] =
    &[CLR_PIN_SCU414_26, CLR_PIN_SCU4B4_26, CLR_PIN_SCU694_26];
pub const PINCTRL_GPIOH3: &[PinctrlPin] =
    &[CLR_PIN_SCU414_27, CLR_PIN_SCU4B4_27, CLR_PIN_SCU694_27];
pub const PINCTRL_GPIOH4: &[PinctrlPin] =
    &[CLR_PIN_SCU414_28, CLR_PIN_SCU4B4_28, CLR_PIN_SCU694_28];
pub const PINCTRL_GPIOH5: &[PinctrlPin] =
    &[CLR_PIN_SCU414_29, CLR_PIN_SCU4B4_29, CLR_PIN_SCU694_29];
pub const PINCTRL_GPIOH6: &[PinctrlPin] = &[CLR_PIN_SCU414_30];
pub const PINCTRL_GPIOH7: &[PinctrlPin] = &[CLR_PIN_SCU414_31];

/// GPIOI pin groups: GPIO mux selection.
pub const PINCTRL_GPIOI0: &[PinctrlPin] = &[CLR_PIN_SCU418_0];
pub const PINCTRL_GPIOI1: &[PinctrlPin] = &[CLR_PIN_SCU418_1];
pub const PINCTRL_GPIOI2: &[PinctrlPin] = &[CLR_PIN_SCU418_2];
pub const PINCTRL_GPIOI3: &[PinctrlPin] = &[CLR_PIN_SCU418_3];
pub const PINCTRL_GPIOI4: &[PinctrlPin] = &[CLR_PIN_SCU418_4];
pub const PINCTRL_GPIOI5: &[PinctrlPin] = &[CLR_PIN_SCU418_5];
pub const PINCTRL_GPIOI6: &[PinctrlPin] = &[CLR_PIN_SCU418_6];
pub const PINCTRL_GPIOI7: &[PinctrlPin] = &[CLR_PIN_SCU418_7];

/// GPIOJ pin groups: GPIO mux selection.
pub const PINCTRL_GPIOJ0: &[PinctrlPin] = &[CLR_PIN_SCU418_8, CLR_PIN_SCU4B8_8];
pub const PINCTRL_GPIOJ1: &[PinctrlPin] = &[CLR_PIN_SCU418_9, CLR_PIN_SCU4B8_9];
pub const PINCTRL_GPIOJ2: &[PinctrlPin] = &[CLR_PIN_SCU418_10, CLR_PIN_SCU4B8_10];
pub const PINCTRL_GPIOJ3: &[PinctrlPin] = &[CLR_PIN_SCU418_11, CLR_PIN_SCU4B8_11];
pub const PINCTRL_GPIOJ4: &[PinctrlPin] = &[CLR_PIN_SCU418_12, CLR_PIN_SCU4B8_12];
pub const PINCTRL_GPIOJ5: &[PinctrlPin] = &[CLR_PIN_SCU418_13, CLR_PIN_SCU4B8_13];
pub const PINCTRL_GPIOJ6: &[PinctrlPin] = &[CLR_PIN_SCU418_14, CLR_PIN_SCU4B8_14];
pub const PINCTRL_GPIOJ7: &[PinctrlPin] = &[CLR_PIN_SCU418_15, CLR_PIN_SCU4B8_15];

/// GPIOK pin groups: GPIO mux selection.
pub const PINCTRL_GPIOK0: &[PinctrlPin] = &[CLR_PIN_SCU418_16, CLR_PIN_SCU4B8_16];
pub const PINCTRL_GPIOK1: &[PinctrlPin] = &[CLR_PIN_SCU418_17, CLR_PIN_SCU4B8_17];
pub const PINCTRL_GPIOK2: &[PinctrlPin] = &[CLR_PIN_SCU418_18, CLR_PIN_SCU4B8_18];
pub const PINCTRL_GPIOK3: &[PinctrlPin] = &[CLR_PIN_SCU418_19, CLR_PIN_SCU4B8_19];
pub const PINCTRL_GPIOK4: &[PinctrlPin] = &[CLR_PIN_SCU418_20, CLR_PIN_SCU4B8_20];
pub const PINCTRL_GPIOK5: &[PinctrlPin] = &[CLR_PIN_SCU418_21, CLR_PIN_SCU4B8_21];
pub const PINCTRL_GPIOK6: &[PinctrlPin] = &[CLR_PIN_SCU418_22, CLR_PIN_SCU4B8_22];
pub const PINCTRL_GPIOK7: &[PinctrlPin] = &[CLR_PIN_SCU418_23, CLR_PIN_SCU4B8_23];

/// GPIOL pin groups: GPIO mux selection.
pub const PINCTRL_GPIOL0: &[PinctrlPin] = &[CLR_PIN_SCU418_24];
pub const PINCTRL_GPIOL1: &[PinctrlPin] = &[CLR_PIN_SCU418_25];
pub const PINCTRL_GPIOL2: &[PinctrlPin] = &[CLR_PIN_SCU418_26];
pub const PINCTRL_GPIOL3: &[PinctrlPin] = &[CLR_PIN_SCU418_27];
pub const PINCTRL_GPIOL4: &[PinctrlPin] =
    &[CLR_PIN_SCU418_28, CLR_PIN_SCU4B8_28, CLR_PIN_SCU698_28];
pub const PINCTRL_GPIOL5: &[PinctrlPin] =
    &[CLR_PIN_SCU418_29, CLR_PIN_SCU4B8_29, CLR_PIN_SCU698_29];
pub const PINCTRL_GPIOL6: &[PinctrlPin] =
    &[CLR_PIN_SCU418_30, CLR_PIN_SCU4B8_30, CLR_PIN_SCU698_30];
pub const PINCTRL_GPIOL7: &[PinctrlPin] =
    &[CLR_PIN_SCU418_31, CLR_PIN_SCU4B8_31, CLR_PIN_SCU698_31];

/// GPIOM pin groups: GPIO mux selection.
pub const PINCTRL_GPIOM0: &[PinctrlPin] = &[CLR_PIN_SCU41C_0];
pub const PINCTRL_GPIOM1: &[PinctrlPin] = &[CLR_PIN_SCU41C_1];
pub const PINCTRL_GPIOM2: &[PinctrlPin] = &[CLR_PIN_SCU41C_2];
pub const PINCTRL_GPIOM3: &[PinctrlPin] = &[CLR_PIN_SCU41C_3];
pub const PINCTRL_GPIOM4: &[PinctrlPin] = &[CLR_PIN_SCU41C_4];
pub const PINCTRL_GPIOM5: &[PinctrlPin] = &[CLR_PIN_SCU41C_5];

/// GPION pin groups: GPIO mux selection.
pub const PINCTRL_GPION0: &[PinctrlPin] = &[CLR_PIN_SCU41C_8];
pub const PINCTRL_GPION1: &[PinctrlPin] = &[CLR_PIN_SCU41C_9, CLR_PIN_SCU4BC_9, CLR_PIN_SCU69C_9];
pub const PINCTRL_GPION2: &[PinctrlPin] = &[CLR_PIN_SCU41C_10];
pub const PINCTRL_GPION3: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_11, CLR_PIN_SCU4BC_11, CLR_PIN_SCU69C_11];
pub const PINCTRL_GPION4: &[PinctrlPin] = &[CLR_PIN_SCU41C_12, CLR_PIN_SCU4BC_12];
pub const PINCTRL_GPION5: &[PinctrlPin] = &[CLR_PIN_SCU41C_13, CLR_PIN_SCU4BC_13];
pub const PINCTRL_GPION6: &[PinctrlPin] = &[CLR_PIN_SCU41C_14, CLR_PIN_SCU4BC_14];
pub const PINCTRL_GPION7: &[PinctrlPin] = &[CLR_PIN_SCU41C_15, CLR_PIN_SCU4BC_15];

/// GPIOO pin groups: GPIO mux selection.
pub const PINCTRL_GPIOO0: &[PinctrlPin] = &[CLR_PIN_SCU41C_16, CLR_PIN_SCU4BC_16];
pub const PINCTRL_GPIOO1: &[PinctrlPin] = &[CLR_PIN_SCU41C_17, CLR_PIN_SCU4BC_17];
pub const PINCTRL_GPIOO2: &[PinctrlPin] = &[CLR_PIN_SCU41C_18, CLR_PIN_SCU4BC_18];
pub const PINCTRL_GPIOO3: &[PinctrlPin] = &[CLR_PIN_SCU41C_19, CLR_PIN_SCU4BC_19];
pub const PINCTRL_GPIOO4: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_20, CLR_PIN_SCU4BC_20, CLR_PIN_SCU69C_20];
pub const PINCTRL_GPIOO5: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_21, CLR_PIN_SCU4BC_21, CLR_PIN_SCU69C_21];
pub const PINCTRL_GPIOO6: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_22, CLR_PIN_SCU4BC_22, CLR_PIN_SCU69C_22];
pub const PINCTRL_GPIOO7: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_23, CLR_PIN_SCU4BC_23, CLR_PIN_SCU69C_23];

/// GPIOP pin groups: GPIO mux selection.
pub const PINCTRL_GPIOP0: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_24, CLR_PIN_SCU4BC_24, CLR_PIN_SCU69C_24];
pub const PINCTRL_GPIOP1: &[PinctrlPin] = &[CLR_PIN_SCU41C_25, CLR_PIN_SCU4BC_25];
pub const PINCTRL_GPIOP2: &[PinctrlPin] = &[CLR_PIN_SCU41C_26, CLR_PIN_SCU4BC_26];
pub const PINCTRL_GPIOP3: &[PinctrlPin] = &[CLR_PIN_SCU41C_27, CLR_PIN_SCU4BC_27];
pub const PINCTRL_GPIOP4: &[PinctrlPin] = &[CLR_PIN_SCU41C_28, CLR_PIN_SCU4BC_28];
pub const PINCTRL_GPIOP5: &[PinctrlPin] = &[CLR_PIN_SCU41C_29, CLR_PIN_SCU4BC_29];
pub const PINCTRL_GPIOP6: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_30, CLR_PIN_SCU4BC_30, CLR_PIN_SCU69C_30];
pub const PINCTRL_GPIOP7: &[PinctrlPin] =
    &[CLR_PIN_SCU41C_31, CLR_PIN_SCU4BC_31, CLR_PIN_SCU69C_31];

/// GPIOQ pin groups: GPIO mux selection.
pub const PINCTRL_GPIOQ0: &[PinctrlPin] = &[CLR_PIN_SCU430_0, CLR_PIN_SCU6B0_0];
pub const PINCTRL_GPIOQ1: &[PinctrlPin] = &[CLR_PIN_SCU430_1, CLR_PIN_SCU6B0_1];
pub const PINCTRL_GPIOQ2: &[PinctrlPin] = &[CLR_PIN_SCU430_2, CLR_PIN_SCU6B0_2];
pub const PINCTRL_GPIOQ3: &[PinctrlPin] = &[CLR_PIN_SCU430_3, CLR_PIN_SCU6B0_3];
pub const PINCTRL_GPIOQ4: &[PinctrlPin] = &[CLR_PIN_SCU430_4, CLR_PIN_SCU6B0_4];

/// GPIOR pin groups: GPIO mux selection.
pub const PINCTRL_GPIOR2: &[PinctrlPin] = &[CLR_PIN_SCU430_10];
pub const PINCTRL_GPIOR3: &[PinctrlPin] = &[CLR_PIN_SCU430_11];

/// GPIOS pin groups: GPIO mux selection.
pub const PINCTRL_GPIOS2: &[PinctrlPin] = &[CLR_PIN_SCU430_17];
pub const PINCTRL_GPIOS3: &[PinctrlPin] = &[CLR_PIN_SCU430_18];

/// GPIOT pin groups: GPIO mux selection.
pub const PINCTRL_GPIOT0: &[PinctrlPin] = &[PIN_SCU430_24];
pub const PINCTRL_GPIOT1: &[PinctrlPin] = &[PIN_SCU430_25];
pub const PINCTRL_GPIOT2: &[PinctrlPin] = &[PIN_SCU430_26];
pub const PINCTRL_GPIOT3: &[PinctrlPin] = &[PIN_SCU430_27];
pub const PINCTRL_GPIOT4: &[PinctrlPin] = &[PIN_SCU430_28];
pub const PINCTRL_GPIOT5: &[PinctrlPin] = &[PIN_SCU430_29];
pub const PINCTRL_GPIOT6: &[PinctrlPin] = &[PIN_SCU430_30];
pub const PINCTRL_GPIOT7: &[PinctrlPin] = &[PIN_SCU430_31];

/// GPIOU pin groups: GPIO mux selection.
pub const PINCTRL_GPIOU0: &[PinctrlPin] = &[PIN_SCU434_0];
pub const PINCTRL_GPIOU1: &[PinctrlPin] = &[PIN_SCU434_1];
pub const PINCTRL_GPIOU2: &[PinctrlPin] = &[PIN_SCU434_2];
pub const PINCTRL_GPIOU3: &[PinctrlPin] = &[PIN_SCU434_3];
pub const PINCTRL_GPIOU4: &[PinctrlPin] = &[PIN_SCU434_4];
pub const PINCTRL_GPIOU5: &[PinctrlPin] = &[PIN_SCU434_5];
pub const PINCTRL_GPIOU6: &[PinctrlPin] = &[PIN_SCU434_6];
pub const PINCTRL_GPIOU7: &[PinctrlPin] = &[PIN_SCU434_7];

/// FMC quad-SPI pin group: mux selection on SCU430[10:11].
pub const PINCTRL_FMC_QUAD: &[PinctrlPin] = &[PIN_SCU430_10, PIN_SCU430_11];

/// SPI1 quad.
pub const PINCTRL_SPI1_QUAD: &[PinctrlPin] = &[PIN_SCU430_17, PIN_SCU430_18];

/// SPI2 quad.
pub const PINCTRL_SPI2_QUAD: &[PinctrlPin] = &[
    PIN_SCU41C_30,
    PIN_SCU41C_31,
    PIN_SCU430_0,
    PIN_SCU430_1,
    PIN_SCU430_2,
    PIN_SCU430_3,
    PIN_SCU430_4,
];

/// SPIM1
pub const PINCTRL_SPIM1_DEFAULT: &[PinctrlPin] = &[
    // CSIN
    CLR_PIN_SCU410_0,
    CLR_PIN_SCU4B0_0,
    PIN_SCU690_0,
    // CLKIN
    CLR_PIN_SCU410_1,
    CLR_PIN_SCU4B0_1,
    PIN_SCU690_1,
    // MOSIIN
    CLR_PIN_SCU410_2,
    CLR_PIN_SCU4B0_2,
    PIN_SCU690_2,
    // MISOIN
    CLR_PIN_SCU410_3,
    CLR_PIN_SCU4B0_3,
    PIN_SCU690_3,
    // IO2IN
    CLR_PIN_SCU410_4,
    CLR_PIN_SCU4B0_4,
    PIN_SCU690_4,
    // IO3IN
    CLR_PIN_SCU410_5,
    CLR_PIN_SCU4B0_5,
    PIN_SCU690_5,
    // CSOUT
    CLR_PIN_SCU410_6,
    CLR_PIN_SCU4B0_6,
    PIN_SCU690_6,
];
/// SPIM2
pub const PINCTRL_SPIM2_DEFAULT: &[PinctrlPin] = &[
    // CSIN
    CLR_PIN_SCU410_14,
    CLR_PIN_SCU4B0_14,
    PIN_SCU690_14,
    // CLKIN
    CLR_PIN_SCU410_15,
    CLR_PIN_SCU4B0_15,
    PIN_SCU690_15,
    // MOSIIN
    CLR_PIN_SCU410_16,
    CLR_PIN_SCU4B0_16,
    PIN_SCU690_16,
    // MISOIN
    CLR_PIN_SCU410_17,
    CLR_PIN_SCU4B0_17,
    PIN_SCU690_17,
    // IO2IN
    CLR_PIN_SCU410_18,
    CLR_PIN_SCU4B0_18,
    PIN_SCU690_18,
    // IO3IN
    CLR_PIN_SCU410_19,
    CLR_PIN_SCU4B0_19,
    PIN_SCU690_19,
    // CSOUT
    CLR_PIN_SCU410_20,
    CLR_PIN_SCU4B0_20,
    PIN_SCU690_20,
];
/// SPIM3
pub const PINCTRL_SPIM3_DEFAULT: &[PinctrlPin] = &[
    // CSIN
    CLR_PIN_SCU410_28,
    CLR_PIN_SCU4B0_28,
    PIN_SCU690_28,
    // CLKIN
    CLR_PIN_SCU410_29,
    CLR_PIN_SCU4B0_29,
    PIN_SCU690_29,
    // MOSIIN
    CLR_PIN_SCU410_30,
    CLR_PIN_SCU4B0_30,
    PIN_SCU690_30,
    // MISOIN
    CLR_PIN_SCU410_31,
    CLR_PIN_SCU4B0_31,
    PIN_SCU690_31,
    // IO2IN
    CLR_PIN_SCU414_0,
    CLR_PIN_SCU4B4_0,
    PIN_SCU694_0,
    // IO3IN
    CLR_PIN_SCU414_1,
    CLR_PIN_SCU4B4_1,
    PIN_SCU694_1,
    // CSOUT
    CLR_PIN_SCU414_2,
    CLR_PIN_SCU4B4_2,
    PIN_SCU694_2,
    // CLKOUT
    CLR_PIN_SCU414_3,
    CLR_PIN_SCU4B4_3,
    PIN_SCU694_3,
    // MOSIOUT
    CLR_PIN_SCU414_4,
    CLR_PIN_SCU4B4_4,
    PIN_SCU694_4,
    // MISOOUT
    CLR_PIN_SCU414_5,
    CLR_PIN_SCU4B4_5,
    PIN_SCU694_5,
    // IO2OUT
    CLR_PIN_SCU414_6,
    CLR_PIN_SCU4B4_6,
    PIN_SCU694_6,
    // IO3OUT
    CLR_PIN_SCU414_7,
    CLR_PIN_SCU4B4_7,
    PIN_SCU694_7,
];
/// SPIM4
pub const PINCTRL_SPIM4_DEFAULT: &[PinctrlPin] = &[
    // CSIN
    CLR_PIN_SCU414_10,
    CLR_PIN_SCU4B4_10,
    PIN_SCU694_10,
    // CLKIN
    CLR_PIN_SCU414_11,
    CLR_PIN_SCU4B4_11,
    PIN_SCU694_11,
    // MOSIIN
    CLR_PIN_SCU414_12,
    CLR_PIN_SCU4B4_12,
    PIN_SCU694_12,
    // MISOIN
    CLR_PIN_SCU414_13,
    CLR_PIN_SCU4B4_13,
    PIN_SCU694_13,
    // IO2IN
    CLR_PIN_SCU414_14,
    CLR_PIN_SCU4B4_14,
    PIN_SCU694_14,
    // IO3IN
    CLR_PIN_SCU414_15,
    CLR_PIN_SCU4B4_15,
    PIN_SCU694_15,
    // CSOUT
    CLR_PIN_SCU414_16,
    CLR_PIN_SCU4B4_16,
    PIN_SCU694_16,
    // CLKOUT
    CLR_PIN_SCU414_17,
    CLR_PIN_SCU4B4_17,
    PIN_SCU694_17,
    // MOSIOUT
    CLR_PIN_SCU414_18,
    CLR_PIN_SCU4B4_18,
    PIN_SCU694_18,
    // MISOOUT
    CLR_PIN_SCU414_19,
    CLR_PIN_SCU4B4_19,
    PIN_SCU694_19,
    // IO2OUT
    CLR_PIN_SCU414_20,
    CLR_PIN_SCU4B4_20,
    PIN_SCU694_20,
    // IO3OUT
    CLR_PIN_SCU414_21,
    CLR_PIN_SCU4B4_21,
    PIN_SCU694_21,
];

/// SGPIOM (Serial GPIO Master) pin group: mux selection on SCU41C[8:11].
///
/// Enables the four SGPIOM serial pins:
///   - bit 8  `sgpmclk` — serial clock out
///   - bit 9  `sgpmld`  — load/latch out
///   - bit 10 `sgpmo`   — serial data out
///   - bit 11 `sgpmi`   — serial data in
///
/// Mux bits derived from the AST10x0 datasheet / Zephyr `ast10x0-pinctrl.dtsi`
/// (`pin_sgpmclk/ld/o/i` = `SIG_DESC_SET(0x41C, 8..11)`).
pub const PINCTRL_SGPIOM: &[PinctrlPin] =
    &[PIN_SCU41C_8, PIN_SCU41C_9, PIN_SCU41C_10, PIN_SCU41C_11];

/// Macro to safely modify a register bit (set or clear).
macro_rules! modify_reg {
    ($reg:expr, $bit:expr, $clear:expr) => {{
        let mut val: u32 = $reg.read().bits();
        if $clear {
            val &= !(1 << $bit);
        } else {
            val |= (1 << $bit);
        }
        $reg.write(|w| unsafe { w.bits(val) });
    }};
}

impl ScuRegisters {
    /// Apply a pinctrl group configuration.
    ///
    /// Iterates through pin descriptors and applies each to the corresponding
    /// SCU register offset and bit position.
    ///
    /// # Example
    /// ```no_run
    /// # use ast10x0_peripherals::scu::{ScuRegisters, pinctrl};
    /// # unsafe {
    /// let scu = ScuRegisters::new_global();
    /// scu.apply_pinctrl_group(&[pinctrl::CLR_PIN_SCU41C_0]);
    /// # }
    /// ```
    pub fn apply_pinctrl_group(&self, pins: &[PinctrlPin]) {
        let regs = self.regs();
        for pin in pins {
            match pin.offset {
                0x410 => modify_reg!(regs.scu410(), pin.bit, pin.clear),
                0x414 => modify_reg!(regs.scu414(), pin.bit, pin.clear),
                0x418 => modify_reg!(regs.scu418(), pin.bit, pin.clear),
                0x41C => modify_reg!(regs.scu41c(), pin.bit, pin.clear),
                0x430 => modify_reg!(regs.scu430(), pin.bit, pin.clear),
                0x434 => modify_reg!(regs.scu434(), pin.bit, pin.clear),
                0x4B0 => modify_reg!(regs.scu4b0(), pin.bit, pin.clear),
                0x4B4 => modify_reg!(regs.scu4b4(), pin.bit, pin.clear),
                0x4B8 => modify_reg!(regs.scu4b8(), pin.bit, pin.clear),
                0x4BC => modify_reg!(regs.scu4bc(), pin.bit, pin.clear),
                0x690 => modify_reg!(regs.scu690(), pin.bit, pin.clear),
                0x694 => modify_reg!(regs.scu694(), pin.bit, pin.clear),
                0x698 => modify_reg!(regs.scu698(), pin.bit, pin.clear),
                0x69C => modify_reg!(regs.scu69c(), pin.bit, pin.clear),
                0x6B0 => modify_reg!(regs.scu6b0(), pin.bit, pin.clear),
                _ => {} // Unknown offset, silently ignore
            }
        }
    }
}
