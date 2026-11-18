// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The OpenPRoT runtime image for the Caliptra MCU.
//!
//! The caliptra-mcu-sw ROM loads this image into MCU SRAM and jumps to it.
//! When the kernel has started, `init_hardware` sets up the hardware that the
//! image uses. `system.json5.tmpl` lists the services of the image.
//!
//! The kernel handles the MCI interrupt. The only event that the image
//! enables is Caliptra's request for an MCU reset. The handler asks the MCI
//! for the reset and does not return.

#![no_std]
#![no_main]

use arch_riscv::Arch;
use kernel::interrupt_controller::InterruptController as _;
use target_common::{declare_target, TargetInterface};
use veer_handoff::Handoff;
use veer_mci::{ErrorEvents, Mci, Milestones, Notifications};
use veer_wdt::Wdt;
use {codegen as _, console_backend as _, entry as _};

/// The PIC interrupt of the MCI. `system.json5.tmpl` has the same number.
const MCI_IRQ: u32 = 1;

/// Sets up the hardware. It runs in the kernel's bootstrap thread, before any
/// service starts.
fn init_hardware() {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers, and no other code in this image uses the watchdog.
    let mut wdt = unsafe { Wdt::new(kernel_config::MCI_BASE) };
    // The ROM leaves the watchdog running, and this image has no code that
    // restarts it.
    wdt.stop();
}

/// Logs what the ROM recorded about this boot in its handoff table.
fn log_handoff() {
    // SAFETY: this code runs in the kernel, which can read DCCM. The ROM
    // reserves the region at `HANDOFF_BASE` for the table, and no code in
    // this image writes it.
    let handoff = match unsafe { Handoff::at(kernel_config::HANDOFF_BASE) } {
        Ok(handoff) => handoff,
        Err(_) => {
            pw_log::warn!("The ROM left no handoff table that this image can read");
            return;
        }
    };
    let boot_type = match handoff.firmware_boot_type() {
        Ok(boot_type) => boot_type.name(),
        Err(_) => "not recorded",
    };
    let capabilities = handoff.rom_capabilities().map_or(0, |c| c.bits());
    pw_log::info!(
        "ROM handoff table {}.{}: boot type {}, ROM capabilities {:#x}",
        handoff.major_version() as u32,
        handoff.minor_version() as u32,
        boot_type as &str,
        capabilities as u32,
    );
}

/// Logs why the MCU was reset, records in the MCI that the kernel has
/// started, and enables the MCI interrupt for Caliptra's request for an MCU
/// reset.
fn init_mci() {
    {
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers. The only other code in this image that writes the
        // registers of the driver is `mci_interrupt_handler`, and it does
        // not run while its interrupt is disabled in the PIC.
        let mut mci = unsafe { Mci::new(kernel_config::MCI_BASE) };
        pw_log::info!("MCU reset reason: {}", mci.reset_reason().name() as &str);
        mci.set_milestones(Milestones::FIRMWARE_OS_INITIALIZED);
        mci.enable_interrupts(ErrorEvents::empty(), Notifications::CPTRA_MCU_RESET_REQUEST);
    }
    <Arch as kernel::Arch>::InterruptController::enable_interrupt(MCI_IRQ);
}

/// Handles the MCI interrupt. Caliptra requests an MCU reset when it is about
/// to replace the image in MCU SRAM. The handler asks the MCI for the reset
/// and does not return.
pub fn mci_interrupt_handler<K: kernel::Kernel>(_kernel: K) {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers. `init_mci` dropped its driver before it enabled this
    // interrupt, and no other code in this image writes the registers of the
    // driver.
    let mut mci = unsafe { Mci::new(kernel_config::MCI_BASE) };
    if !mci
        .pending_notifications()
        .contains(Notifications::CPTRA_MCU_RESET_REQUEST)
    {
        return;
    }
    // The request must not be pending when the next image enables the
    // interrupt.
    mci.acknowledge_notifications(Notifications::CPTRA_MCU_RESET_REQUEST);
    mci.request_mcu_reset();
    // The MCI resets the MCU. No other code of this image runs before the
    // reset, because interrupts are off in this handler.
    loop {
        core::hint::spin_loop();
    }
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "OpenPRoT Caliptra MCU Runtime";

    fn main() -> ! {
        init_hardware();
        log_handoff();
        init_mci();
        pw_log::info!("OpenPRoT Caliptra MCU runtime: kernel started");
        // The bootstrap thread has nothing left to do.
        loop {
            let _ = kernel::sleep_until(Arch, kernel::Instant::MAX);
        }
    }
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);
