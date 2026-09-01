// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end check of external interrupt delivery on VeeR.
//!
//! The emulated core vectors machine-external interrupts through the MEIVT
//! redirect table (`handle_external_int` in the Caliptra CPU model reads
//! `*(MEIVT + 4 * claimid)` and jumps there), so an interrupt that reaches
//! `flash_error_handler` proves the table was programmed with a usable trap
//! vector.
//!
//! The primary flash controller is used as the interrupt source because it is
//! always instantiated by the emulator and needs no host-side driving. The PIC
//! has no software trigger (`VeerPic::trigger_interrupt` panics), so the shared
//! `pw_kernel/tests/interrupts` body cannot be reused here.

#![no_std]
#![no_main]

use core::ptr::with_exposed_provenance_mut;
use core::sync::atomic::{AtomicBool, Ordering};

use arch_riscv::Arch;
use entry::exit;
use target_common::{declare_target, TargetInterface};
use {codegen as _, console_backend as _};

// Primary flash controller, emulator memory map (PRIMARY_FLASH_CTRL_ADDR).
const FLASH_CTRL_BASE: usize = 0x2000_8000;
const INT_STATE: usize = FLASH_CTRL_BASE;
const INT_ENABLE: usize = FLASH_CTRL_BASE + 0x04;
const CONTROL: usize = FLASH_CTRL_BASE + 0x14;

const INT_ERROR: u32 = 1 << 0;
// FlControl: Start at bit 0, Op at bits 2:1. Op 3 is reserved, so the
// controller fails the operation with InvalidOp and raises the error interrupt.
const CONTROL_START_INVALID_OP: u32 = 0b111;

// Must match the IRQ number in system.json5.
const TEST_IRQ: u32 = 19;

// rv32imc has no A extension, so plain load/store rather than a counter.
static IRQ_FIRED: AtomicBool = AtomicBool::new(false);

pub fn flash_error_handler<K: kernel::Kernel>(_kernel: K) {
    // Write-one-to-clear the error status; this also drops the controller's
    // interrupt line, so the handler is not re-entered.
    unsafe { with_exposed_provenance_mut::<u32>(INT_STATE).write_volatile(INT_ERROR) };
    IRQ_FIRED.store(true, Ordering::SeqCst);
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU Kernel Interrupts";

    fn main() -> ! {
        use kernel::interrupt_controller::InterruptController;

        <Arch as kernel::Arch>::InterruptController::enable_interrupt(TEST_IRQ);

        unsafe {
            with_exposed_provenance_mut::<u32>(INT_ENABLE).write_volatile(INT_ERROR);
            with_exposed_provenance_mut::<u32>(CONTROL).write_volatile(CONTROL_START_INVALID_OP);
        }

        let mut timeout = 1_000_000;
        while !IRQ_FIRED.load(Ordering::SeqCst) && timeout > 0 {
            timeout -= 1;
            core::hint::spin_loop();
        }

        if !IRQ_FIRED.load(Ordering::SeqCst) {
            pw_log::info!("FAIL: no external interrupt delivered");
            exit(1);
        }

        pw_log::info!("PASS");
        exit(0);
    }
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);
