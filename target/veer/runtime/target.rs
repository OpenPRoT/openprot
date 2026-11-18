// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The OpenPRoT runtime image for the Caliptra MCU.
//!
//! The caliptra-mcu-sw ROM loads this image into MCU SRAM and jumps to it.
//! When the kernel has started, `init_hardware` sets up the hardware that the
//! image uses. `system.json5.tmpl` lists the services of the image.
//!
//! The kernel handles the MCI interrupt. The image enables the interrupt for
//! two kinds of events, and the handler does not return from either:
//!
//! - Caliptra's request for an MCU reset. The handler asks the MCI for the
//!   reset.
//! - The hardware errors in [`FATAL_ERRORS`] and
//!   [`FATAL_AGGREGATED_ERRORS`]. The handler reports a fatal firmware error
//!   to the SoC and stops the MCU.
//!
//! The image does not enable the interrupt for ECC errors in the MCU
//! mailboxes or for the watchdog: it has no mailbox driver, and it stops the
//! watchdog.

#![no_std]
#![no_main]

use arch_riscv::Arch;
use kernel::interrupt_controller::InterruptController as _;
use target_common::{declare_target, TargetInterface};
use veer_handoff::Handoff;
use veer_mci::{AggregatedErrors, ErrorEvents, Events, Mci, Milestones, Notifications};
use veer_wdt::Wdt;
use {codegen as _, console_backend as _, entry as _};

/// The PIC interrupt of the MCI. `system.json5.tmpl` has the same number.
const MCI_IRQ: u32 = 1;

/// The error events of the MCI that stop the image.
const FATAL_ERRORS: ErrorEvents =
    ErrorEvents::MCU_SRAM_DMI_AXI_COLLISION.union(ErrorEvents::INTERNAL);

/// The aggregated fatal errors that stop the image: those of the Caliptra
/// core and of the MCU's DCCM. They are the ones that assert the MCI's fatal
/// error output with the reset value of its mask (`mci_reg.rdl`), which masks
/// the inputs of the lifecycle controller, the fuse controller and the I3C
/// core.
const FATAL_AGGREGATED_ERRORS: AggregatedErrors =
    AggregatedErrors::CALIPTRA.union(AggregatedErrors::MCU);

/// The code that the image writes to `FW_ERROR_FATAL` when a hardware error
/// stops it. The code is in the range that caliptra-mcu-sw keeps for vendors
/// (`McuError::new_vendor`). The image stores the errors in
/// `FW_EXTENDED_ERROR_INFO`: word 0 holds the [`FATAL_ERRORS`] that were
/// pending and word 1 the [`FATAL_AGGREGATED_ERRORS`].
const FATAL_HARDWARE_ERROR: u32 = 0xf000_0001;

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

/// Logs the errors in the MCI's records, if there are any.
fn log_recorded_errors(mci: &Mci) {
    let hw_fatal = mci.recorded_hw_fatal_errors().bits();
    let hw_non_fatal = mci.recorded_hw_non_fatal_errors().bits();
    let aggregated_fatal = mci.recorded_aggregated_fatal_errors().bits();
    let aggregated_non_fatal = mci.recorded_aggregated_non_fatal_errors().bits();
    let fw_fatal = mci.fw_error_fatal();
    let fw_non_fatal = mci.fw_error_non_fatal();
    if hw_fatal | hw_non_fatal | aggregated_fatal | aggregated_non_fatal | fw_fatal | fw_non_fatal
        == 0
    {
        return;
    }
    pw_log::warn!(
        "MCI error records: hardware fatal {:#x}, non-fatal {:#x}, aggregated fatal {:#x}, non-fatal {:#x}, firmware fatal {:#x}, non-fatal {:#x}",
        hw_fatal as u32,
        hw_non_fatal as u32,
        aggregated_fatal as u32,
        aggregated_non_fatal as u32,
        fw_fatal as u32,
        fw_non_fatal as u32,
    );
}

/// Logs why the MCU was reset and which errors the MCI has recorded. Records
/// in the MCI that the kernel has started. Enables the MCI interrupt for
/// Caliptra's request for an MCU reset and for the fatal hardware errors.
fn init_mci() {
    {
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers. The only other code in this image that writes the
        // registers of the driver is `mci_interrupt_handler`, and it does
        // not run while its interrupt is disabled in the PIC.
        let mut mci = unsafe { Mci::new(kernel_config::MCI_BASE) };
        pw_log::info!("MCU reset reason: {}", mci.reset_reason().name() as &str);
        log_recorded_errors(&mci);
        mci.set_milestones(Milestones::FIRMWARE_OS_INITIALIZED);

        // A pending error stays pending through a warm reset, so the fatal
        // errors that are pending here may come from an earlier boot. The
        // image acknowledges them so that they do not stop it. An aggregated
        // error whose input is still asserted becomes pending again.
        let errors = mci.pending_errors() & FATAL_ERRORS;
        let aggregated = mci.pending_aggregated_fatal_errors() & FATAL_AGGREGATED_ERRORS;
        if !errors.is_empty() || !aggregated.is_empty() {
            pw_log::warn!(
                "Fatal errors were pending at start: errors {:#x}, aggregated {:#x}",
                errors.bits() as u32,
                aggregated.bits() as u32,
            );
            mci.acknowledge_errors(errors);
            mci.acknowledge_aggregated_fatal_errors(aggregated);
        }

        mci.enable_interrupts(Events {
            errors: FATAL_ERRORS,
            notifications: Notifications::CPTRA_MCU_RESET_REQUEST,
            aggregated_fatal: FATAL_AGGREGATED_ERRORS,
            ..Events::empty()
        });
    }
    <Arch as kernel::Arch>::InterruptController::enable_interrupt(MCI_IRQ);
}

/// Handles the MCI interrupt. It does not return when an event that the
/// image enables is pending.
///
/// For a fatal hardware error the handler reports a fatal firmware error to
/// the SoC and stops. Caliptra requests an MCU reset when it is about to
/// replace the image in MCU SRAM, and the handler asks the MCI for the reset.
pub fn mci_interrupt_handler<K: kernel::Kernel>(_kernel: K) {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers. `init_mci` dropped its driver before it enabled this
    // interrupt, and no other code in this image writes the registers of the
    // driver.
    let mut mci = unsafe { Mci::new(kernel_config::MCI_BASE) };

    let errors = mci.pending_errors() & FATAL_ERRORS;
    let aggregated = mci.pending_aggregated_fatal_errors() & FATAL_AGGREGATED_ERRORS;
    if !errors.is_empty() || !aggregated.is_empty() {
        // The details are written first, so that they are in place when the
        // code is. The handler does not acknowledge the events.
        mci.set_fw_extended_error_info(&[errors.bits(), aggregated.bits(), 0, 0, 0, 0, 0, 0]);
        mci.set_fw_error_fatal(FATAL_HARDWARE_ERROR);
    } else if mci
        .pending_notifications()
        .contains(Notifications::CPTRA_MCU_RESET_REQUEST)
    {
        // The request must not be pending when the next image enables the
        // interrupt.
        mci.acknowledge_notifications(Notifications::CPTRA_MCU_RESET_REQUEST);
        mci.request_mcu_reset();
    } else {
        return;
    }
    // No other code of this image runs from here on, because interrupts are
    // off in this handler. After a request for a reset, the MCI resets the
    // MCU.
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
