// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reads and writes the MCI registers of the emulator or the FPGA with the
//! MCI driver, and takes the MCI interrupt for events that the test triggers.
//!
//! The emulator does not model the aggregated errors, so the test triggers
//! them only on the FPGA.
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
use veer_mci::{
    AggregatedErrors, ErrorEvents, Events, Mci, Milestones, Notifications, ResetReason,
};
use {codegen as _, console_backend as _};

type Pic = <Arch as kernel::Arch>::InterruptController;

/// A value for the generic output wires with bits set in both halves.
const OUTPUT_WIRES: u64 = 0x8000_0001_4000_0002;

/// The PIC interrupt of the MCI. `system.json5.tmpl` has the same number.
const MCI_IRQ: u32 = 1;

/// Whether the aggregated errors work on the platform. The emulator does not
/// model them: <https://github.com/chipsalliance/caliptra-mcu-sw/issues/2378>.
const HAS_AGGREGATED_ERRORS: bool = cfg!(feature = "fpga");

/// The events whose interrupt the test enables. The handler acknowledges
/// them. Its aggregated errors are inputs that the subsystem ties to 0.
const ENABLED: Events = Events {
    errors: ErrorEvents::INTERNAL,
    notifications: Notifications::MBOX0_ECC_CORRECTED,
    aggregated_fatal: AggregatedErrors::from_bits_retain(1 << 30),
    aggregated_non_fatal: AggregatedErrors::from_bits_retain(1 << 30),
};

/// The events that the test makes pending with their interrupt disabled.
const MASKED: Events = Events {
    errors: ErrorEvents::MBOX1_ECC_UNCORRECTED,
    notifications: Notifications::MBOX1_ECC_CORRECTED,
    aggregated_fatal: AggregatedErrors::from_bits_retain(1 << 31),
    aggregated_non_fatal: AggregatedErrors::from_bits_retain(1 << 31),
};

/// The number of iterations that the test spins to let a pending interrupt
/// be taken.
const SPIN_ITERATIONS: u32 = 10_000;

// The handler is the only writer of these, and rv32imc has no atomic
// read-modify-write instructions, so the handler updates them with a load
// and a store.
static HANDLER_RUNS: AtomicU32 = AtomicU32::new(0);
static HANDLED_ERRORS: AtomicU32 = AtomicU32::new(0);
static HANDLED_NOTIFICATIONS: AtomicU32 = AtomicU32::new(0);
static HANDLED_AGGREGATED_FATAL: AtomicU32 = AtomicU32::new(0);
static HANDLED_AGGREGATED_NON_FATAL: AtomicU32 = AtomicU32::new(0);
static NOTHING_TO_HANDLE: AtomicBool = AtomicBool::new(false);

/// The pending events of every group that are also in `of`.
fn pending(mci: &Mci, of: Events) -> Events {
    Events {
        errors: mci.pending_errors() & of.errors,
        notifications: mci.pending_notifications() & of.notifications,
        aggregated_fatal: mci.pending_aggregated_fatal_errors() & of.aggregated_fatal,
        aggregated_non_fatal: mci.pending_aggregated_non_fatal_errors() & of.aggregated_non_fatal,
    }
}

fn acknowledge(mci: &mut Mci, events: Events) {
    mci.acknowledge_errors(events.errors);
    mci.acknowledge_notifications(events.notifications);
    mci.acknowledge_aggregated_fatal_errors(events.aggregated_fatal);
    mci.acknowledge_aggregated_non_fatal_errors(events.aggregated_non_fatal);
}

fn trigger(mci: &mut Mci, events: Events) {
    mci.trigger_errors(events.errors);
    mci.trigger_notifications(events.notifications);
    mci.trigger_aggregated_fatal_errors(events.aggregated_fatal);
    mci.trigger_aggregated_non_fatal_errors(events.aggregated_non_fatal);
}

/// `events`, without the aggregated errors on a platform where they do not
/// work.
fn on_this_platform(events: Events) -> Events {
    if HAS_AGGREGATED_ERRORS {
        events
    } else {
        Events {
            errors: events.errors,
            notifications: events.notifications,
            ..Events::empty()
        }
    }
}

fn accumulate(handled: &AtomicU32, bits: u32) {
    handled.store(handled.load(Ordering::SeqCst) | bits, Ordering::SeqCst);
}

/// The events that the handler has acknowledged since the image started.
fn handled() -> Events {
    Events {
        errors: ErrorEvents::from_bits_retain(HANDLED_ERRORS.load(Ordering::SeqCst)),
        notifications: Notifications::from_bits_retain(
            HANDLED_NOTIFICATIONS.load(Ordering::SeqCst),
        ),
        aggregated_fatal: AggregatedErrors::from_bits_retain(
            HANDLED_AGGREGATED_FATAL.load(Ordering::SeqCst),
        ),
        aggregated_non_fatal: AggregatedErrors::from_bits_retain(
            HANDLED_AGGREGATED_NON_FATAL.load(Ordering::SeqCst),
        ),
    }
}

pub fn mci_interrupt_handler<K: kernel::Kernel>(_kernel: K) {
    // SAFETY: this code runs in the kernel, which can access the MCI
    // registers. The thread has no driver while this interrupt is enabled in
    // the PIC.
    let mut mci = unsafe { Mci::new(MCI_BASE) };
    let events = pending(&mci, ENABLED);
    if events == Events::empty() {
        // No event that the test enables is pending, so the handler has
        // nothing to acknowledge and the interrupt would stay asserted. It
        // disables the interrupt for every event so that the thread runs and
        // reports the failure.
        mci.enable_interrupts(Events::empty());
        NOTHING_TO_HANDLE.store(true, Ordering::SeqCst);
    }
    acknowledge(&mut mci, events);
    accumulate(&HANDLED_ERRORS, events.errors.bits());
    accumulate(&HANDLED_NOTIFICATIONS, events.notifications.bits());
    accumulate(&HANDLED_AGGREGATED_FATAL, events.aggregated_fatal.bits());
    accumulate(
        &HANDLED_AGGREGATED_NON_FATAL,
        events.aggregated_non_fatal.bits(),
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
    pw_log::info!(
        "Recorded errors: hardware fatal {:#x}, non-fatal {:#x}, aggregated fatal {:#x}, non-fatal {:#x}",
        mci.recorded_hw_fatal_errors().bits() as u32,
        mci.recorded_hw_non_fatal_errors().bits() as u32,
        mci.recorded_aggregated_fatal_errors().bits() as u32,
        mci.recorded_aggregated_non_fatal_errors().bits() as u32,
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

/// Makes `events` pending, then checks that the handler has run `runs` times
/// in total, one more than before, and has acknowledged `expected` in total.
fn check_interrupt(
    events: Events,
    runs: u32,
    expected: Events,
    failure: &'static str,
) -> Result<(), &'static str> {
    with_mci(|mci| trigger(mci, events));
    take_interrupts(runs);
    if HANDLER_RUNS.load(Ordering::SeqCst) != runs || handled() != expected {
        return Err(failure);
    }
    Ok(())
}

fn check_interrupts() -> Result<(), &'static str> {
    let enabled = on_this_platform(ENABLED);
    let masked = on_this_platform(MASKED);

    with_mci(|mci| {
        pw_log::info!(
            "Pending at start: errors {:#x}, notifications {:#x}, aggregated fatal {:#x}, non-fatal {:#x}",
            mci.pending_errors().bits() as u32,
            mci.pending_notifications().bits() as u32,
            mci.pending_aggregated_fatal_errors().bits() as u32,
            mci.pending_aggregated_non_fatal_errors().bits() as u32,
        );
        if mci
            .pending_notifications()
            .contains(Notifications::CPTRA_MCU_RESET_REQUEST)
        {
            return Err("the ROM left Caliptra's request for an MCU reset pending");
        }

        acknowledge(mci, ENABLED);
        acknowledge(mci, MASKED);
        mci.enable_interrupts(ENABLED);
        trigger(mci, masked);
        if pending(mci, MASKED) != masked {
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

    let mut expected = Events::empty();
    expected.notifications = enabled.notifications;
    check_interrupt(
        Events {
            notifications: enabled.notifications,
            ..Events::empty()
        },
        1,
        expected,
        "the handler did not run exactly once for the notification alone",
    )?;
    expected.errors = enabled.errors;
    check_interrupt(
        Events {
            errors: enabled.errors,
            ..Events::empty()
        },
        2,
        expected,
        "the handler did not run exactly once for the error event alone",
    )?;
    let mut runs = 2;
    if HAS_AGGREGATED_ERRORS {
        expected.aggregated_fatal = enabled.aggregated_fatal;
        check_interrupt(
            Events {
                aggregated_fatal: enabled.aggregated_fatal,
                ..Events::empty()
            },
            3,
            expected,
            "the handler did not run exactly once for the aggregated fatal error alone",
        )?;
        expected.aggregated_non_fatal = enabled.aggregated_non_fatal;
        check_interrupt(
            Events {
                aggregated_non_fatal: enabled.aggregated_non_fatal,
                ..Events::empty()
            },
            4,
            expected,
            "the handler did not run exactly once for the aggregated non-fatal error alone",
        )?;
        runs = 4;
    }

    // The handler acknowledged the events, so the interrupt is not asserted.
    take_interrupts(u32::MAX);
    if HANDLER_RUNS.load(Ordering::SeqCst) != runs {
        return Err("the interrupt stayed asserted after the handler ran");
    }
    if NOTHING_TO_HANDLE.load(Ordering::SeqCst) {
        return Err("the handler ran with no enabled event pending");
    }

    with_mci(|mci| {
        // Acknowledging one event leaves the others pending.
        if pending(mci, ENABLED) != Events::empty() || pending(mci, MASKED) != masked {
            return Err("the handler's acknowledgement changed other pending events");
        }
        acknowledge(mci, MASKED);
        mci.enable_interrupts(Events::empty());
        if pending(mci, MASKED) != Events::empty() {
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
