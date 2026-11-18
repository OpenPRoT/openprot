// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reads and writes the MCI registers of the emulator or the FPGA with the
//! MCI driver, and takes the MCI interrupt for events that the test triggers.
//!
//! Two instances of the driver must not exist at once. The thread constructs
//! one only while the MCI interrupt is disabled in the PIC, and the interrupt
//! handler constructs its own.
//!
//! The image does not request an MCU reset. After that reset the ROM starts
//! the image in MCU SRAM again without loading it, and the image cannot run
//! a second time: its initialized data keeps the values of the first run.

#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use arch_riscv::Arch;
use entry::exit;
use kernel::interrupt_controller::InterruptController as _;
use kernel_config::MCI_BASE;
use target_common::{declare_target, TargetInterface};
use veer_mci::{ErrorEvents, Mci, Milestones, Notifications, ResetReason};
use {codegen as _, console_backend as _};

type Pic = <Arch as kernel::Arch>::InterruptController;

/// A value for the generic output wires with bits set in both halves.
const OUTPUT_WIRES: u64 = 0x8000_0001_4000_0002;

/// The PIC interrupt of the MCI. `system.json5.tmpl` has the same number.
const MCI_IRQ: u32 = 1;

/// The events whose interrupt the test enables. The handler acknowledges
/// them.
const ENABLED_ERROR: ErrorEvents = ErrorEvents::INTERNAL;
const ENABLED_NOTIFICATION: Notifications = Notifications::MBOX0_ECC_CORRECTED;

/// The events that the test makes pending with their interrupt disabled.
const MASKED_ERROR: ErrorEvents = ErrorEvents::MBOX1_ECC_UNCORRECTED;
const MASKED_NOTIFICATION: Notifications = Notifications::MBOX1_ECC_CORRECTED;

/// The number of iterations that the test spins to let a pending interrupt
/// be taken.
const SPIN_ITERATIONS: u32 = 10_000;

// The handler is the only writer of these, and rv32imc has no atomic
// read-modify-write instructions, so the handler updates them with a load
// and a store.
static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static HANDLED_ERRORS: AtomicU32 = AtomicU32::new(0);
static HANDLED_NOTIFICATIONS: AtomicU32 = AtomicU32::new(0);
static NOTHING_TO_HANDLE: AtomicBool = AtomicBool::new(false);

pub fn mci_interrupt_handler<K: kernel::Kernel>(_kernel: K) {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers. The thread has no driver while this interrupt is enabled in
    // the PIC.
    let mut mci = unsafe { Mci::new(MCI_BASE) };
    let errors = mci.pending_errors() & ENABLED_ERROR;
    let notifications = mci.pending_notifications() & ENABLED_NOTIFICATION;
    if errors.is_empty() && notifications.is_empty() {
        // No event that the test enables is pending, so the handler has
        // nothing to acknowledge and the interrupt would stay asserted. It
        // disables the interrupt for every event so that the thread runs and
        // reports the failure.
        mci.enable_interrupts(ErrorEvents::empty(), Notifications::empty());
        NOTHING_TO_HANDLE.store(true, Ordering::SeqCst);
    }
    mci.acknowledge_errors(errors);
    mci.acknowledge_notifications(notifications);
    HANDLED_ERRORS.store(
        HANDLED_ERRORS.load(Ordering::SeqCst) | errors.bits(),
        Ordering::SeqCst,
    );
    HANDLED_NOTIFICATIONS.store(
        HANDLED_NOTIFICATIONS.load(Ordering::SeqCst) | notifications.bits(),
        Ordering::SeqCst,
    );
    HANDLER_RUNS.store(HANDLER_RUNS.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
}

fn log_state(mci: &Mci) {
    let revision = mci.hw_revision();
    pw_log::info!(
        "MCI {}.{}.{}: reset reason {}, checkpoint {}, milestones {:#x}",
        revision.major as u32,
        revision.minor as u32,
        revision.patch as u32,
        mci.reset_reason().name() as &str,
        mci.boot_checkpoint() as u32,
        mci.milestones().bits() as u32,
    );
    pw_log::info!(
        "MCU SRAM {} KiB, mailbox 0 {} KiB, mailbox 1 {} KiB",
        mci.mcu_sram_size_kib() as u32,
        mci.mbox0_sram_size_kib() as u32,
        mci.mbox1_sram_size_kib() as u32,
    );
}

fn check_registers(mci: &mut Mci) -> Result<(), &'static str> {
    if mci.reset_reason() != ResetReason::FirmwareBoot {
        return Err("the reset reason is not a firmware boot");
    }

    let checkpoint = mci.boot_checkpoint();
    let rom_milestones = mci.milestones();
    if !rom_milestones.contains(Milestones::ROM_STARTED | Milestones::FIRMWARE_BOOT_FLOW_COMPLETE) {
        return Err("the ROM's milestones are not recorded");
    }
    if rom_milestones.contains(Milestones::FIRMWARE_OS_INITIALIZED) {
        return Err("a milestone of the runtime image is recorded before the image set it");
    }
    mci.set_milestones(Milestones::FIRMWARE_OS_INITIALIZED);
    if mci.milestones() != rom_milestones | Milestones::FIRMWARE_OS_INITIALIZED {
        return Err("set_milestones did not add the milestone to the ROM's");
    }
    if mci.boot_checkpoint() != checkpoint {
        return Err("set_milestones changed the checkpoint");
    }

    if mci.security_state().lifecycle().is_none() {
        return Err("the lifecycle state is not valid");
    }
    if mci.hw_revision().major == 0 {
        return Err("the hardware revision has no major version");
    }

    if mci.fw_error_fatal() != 0 {
        return Err("a fatal firmware error is recorded");
    }
    if mci.fw_error_non_fatal() != 0 {
        return Err("a non-fatal firmware error is recorded");
    }
    mci.set_fw_error_non_fatal(0x4d43_0001);
    if mci.fw_error_non_fatal() != 0x4d43_0001 {
        return Err("the non-fatal firmware error did not read back");
    }
    mci.set_fw_error_non_fatal(0);
    if mci.fw_error_non_fatal() != 0 {
        return Err("the non-fatal firmware error was not cleared");
    }

    let info = [1, 2, 3, 4, 5, 6, 7, 8];
    mci.set_fw_extended_error_info(&info);
    if mci.fw_extended_error_info() != info {
        return Err("the extended error information did not read back");
    }
    mci.set_fw_extended_error_info(&[0; 8]);

    mci.set_generic_output_wires(OUTPUT_WIRES);
    if mci.generic_output_wires() != OUTPUT_WIRES {
        return Err("the generic output wires did not read back");
    }
    mci.set_generic_output_wires(0);

    Ok(())
}

/// Runs `f` with a driver. The caller must have the MCI interrupt disabled in
/// the PIC.
fn with_mci<R>(f: impl FnOnce(&mut Mci) -> R) -> R {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers. The only other code that constructs a driver is the
    // interrupt handler, and it does not run while the interrupt is disabled.
    let mut mci = unsafe { Mci::new(MCI_BASE) };
    f(&mut mci)
}

/// Enables the MCI interrupt in the PIC until the handler has run `runs`
/// times in total, or for [`SPIN_ITERATIONS`] iterations of its spin loop.
fn take_interrupts(runs: u32) {
    Pic::enable_interrupt(MCI_IRQ);
    let mut iterations = SPIN_ITERATIONS;
    while HANDLER_RUNS.load(Ordering::SeqCst) < runs && iterations > 0 {
        iterations -= 1;
        core::hint::spin_loop();
    }
    Pic::disable_interrupt(MCI_IRQ);
}

fn check_interrupts() -> Result<(), &'static str> {
    with_mci(|mci| {
        pw_log::info!(
            "Pending at start: errors {:#x}, notifications {:#x}",
            mci.pending_errors().bits() as u32,
            mci.pending_notifications().bits() as u32,
        );
        if mci
            .pending_notifications()
            .contains(Notifications::CPTRA_MCU_RESET_REQUEST)
        {
            return Err("the ROM left Caliptra's request for an MCU reset pending");
        }

        mci.acknowledge_errors(ENABLED_ERROR | MASKED_ERROR);
        mci.acknowledge_notifications(ENABLED_NOTIFICATION | MASKED_NOTIFICATION);
        mci.enable_interrupts(ENABLED_ERROR, ENABLED_NOTIFICATION);
        mci.trigger_errors(MASKED_ERROR);
        mci.trigger_notifications(MASKED_NOTIFICATION);
        if !mci.pending_errors().contains(MASKED_ERROR)
            || !mci.pending_notifications().contains(MASKED_NOTIFICATION)
        {
            return Err("the triggered events are not pending");
        }
        Ok(())
    })?;

    // More runs than the handler makes, so that the interrupt stays enabled
    // for all of the spin loop.
    take_interrupts(u32::MAX);
    if HANDLER_RUNS.load(Ordering::SeqCst) != 0 {
        return Err("events with a disabled interrupt interrupted the MCU");
    }

    with_mci(|mci| mci.trigger_notifications(ENABLED_NOTIFICATION));
    take_interrupts(1);
    if HANDLER_RUNS.load(Ordering::SeqCst) != 1 {
        return Err("the notification did not interrupt the MCU exactly once");
    }
    if HANDLED_NOTIFICATIONS.load(Ordering::SeqCst) != ENABLED_NOTIFICATION.bits()
        || HANDLED_ERRORS.load(Ordering::SeqCst) != 0
    {
        return Err("the handler saw something other than the notification alone");
    }

    with_mci(|mci| mci.trigger_errors(ENABLED_ERROR));
    take_interrupts(2);
    if HANDLER_RUNS.load(Ordering::SeqCst) != 2 {
        return Err("the error event did not interrupt the MCU exactly once");
    }
    if HANDLED_ERRORS.load(Ordering::SeqCst) != ENABLED_ERROR.bits() {
        return Err("the handler did not see the error event");
    }

    // The handler acknowledged the events, so the interrupt is not asserted.
    take_interrupts(u32::MAX);
    if HANDLER_RUNS.load(Ordering::SeqCst) != 2 {
        return Err("the interrupt stayed asserted after the handler ran");
    }
    if NOTHING_TO_HANDLE.load(Ordering::SeqCst) {
        return Err("the handler ran with no enabled event pending");
    }

    with_mci(|mci| {
        // Acknowledging one event leaves the others pending.
        if mci.pending_errors() & (ENABLED_ERROR | MASKED_ERROR) != MASKED_ERROR
            || mci.pending_notifications() & (ENABLED_NOTIFICATION | MASKED_NOTIFICATION)
                != MASKED_NOTIFICATION
        {
            return Err("the handler's acknowledgement changed other pending events");
        }
        mci.acknowledge_errors(MASKED_ERROR);
        mci.acknowledge_notifications(MASKED_NOTIFICATION);
        mci.enable_interrupts(ErrorEvents::empty(), Notifications::empty());
        if mci.pending_errors().intersects(MASKED_ERROR)
            || mci.pending_notifications().intersects(MASKED_NOTIFICATION)
        {
            return Err("the events are pending after they were acknowledged");
        }
        Ok(())
    })
}

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU MCI";

    fn main() -> ! {
        // The MCI interrupt is disabled in the PIC from the start.
        let result = with_mci(|mci| {
            log_state(mci);
            check_registers(mci)
        })
        .and_then(|()| check_interrupts());
        match result {
            Ok(()) => {
                pw_log::info!("PASS");
                exit(0);
            }
            Err(reason) => {
                pw_log::info!("FAIL: {}", reason as &str);
                exit(1);
            }
        }
    }
}

codegen::declare_kernel_interrupt_handlers!();
declare_target!(Target);
