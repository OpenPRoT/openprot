// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra Subsystem register definitions.
//!
//! Re-exports the generated firmware register modules from caliptra-mcu-sw so
//! peripheral drivers in `target/veer/peripherals/` have a single import path.

#![no_std]

pub use caliptra_mcu_registers_generated::axicdma;
pub use caliptra_mcu_registers_generated::defines;
pub use caliptra_mcu_registers_generated::doe_mbox;
pub use caliptra_mcu_registers_generated::el2_pic_ctrl;
pub use caliptra_mcu_registers_generated::fuses;
pub use caliptra_mcu_registers_generated::i3c;
pub use caliptra_mcu_registers_generated::lc_ctrl;
pub use caliptra_mcu_registers_generated::mbox;
pub use caliptra_mcu_registers_generated::mci;
pub use caliptra_mcu_registers_generated::otp_ctrl;
pub use caliptra_mcu_registers_generated::primary_flash_ctrl;
pub use caliptra_mcu_registers_generated::secondary_flash_ctrl;
pub use caliptra_mcu_registers_generated::sha512_acc;
pub use caliptra_mcu_registers_generated::soc;
