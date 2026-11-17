// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Reads and writes the MCI registers of the emulator or the FPGA with the
//! MCI driver.
//!
//! The image does not request an MCU reset. After that reset the ROM starts
//! the image in MCU SRAM again without loading it, and the image cannot run
//! a second time: its initialized data keeps the values of the first run.

#![no_std]
#![no_main]

use entry::exit;
use kernel_config::MCI_BASE;
use target_common::{declare_target, TargetInterface};
use veer_mci::{Mci, Milestones, ResetReason};
use {codegen as _, console_backend as _};

/// A value for the generic output wires with bits set in both halves.
const OUTPUT_WIRES: u64 = 0x8000_0001_4000_0002;

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

pub struct Target {}

impl TargetInterface for Target {
    const NAME: &'static str = "Caliptra-MCU MCI";

    fn main() -> ! {
        // SAFETY: this code runs in the kernel, which can access the MCI
        // registers, and no other code in this image writes the registers
        // that the driver uses.
        let mut mci = unsafe { Mci::new(MCI_BASE) };
        log_state(&mci);
        match check_registers(&mut mci) {
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

declare_target!(Target);
