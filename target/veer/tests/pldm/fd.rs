// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM firmware device: answers the PLDM firmware update commands of an
//! update agent as a client of the MCTP server, then replaces the running
//! image with the one that the agent sent.
//!
//! The device runs the `//services/pldm` firmware device over MCTP.
//! `update_ops.rs` has the operations that the service calls: where the
//! components are staged, how they are verified, and how the staged image
//! becomes the running one.

#![no_main]
#![no_std]

mod update_ops;

use kernel_config::{MBOX_BASE, MCI_BASE, SOC_IFC_BASE};
use mctp_i3c_ids::HOST_EID;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult, FD_MAX_MSG};
use openprot_pldm_service::MctpPldmTransport;
use pldm_fd_codegen::handle;
use pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES;
use update_ops::UpdateOps;
use userspace::process_entry;
use userspace::syscall;
use userspace::time::{self, Clock, Duration, Instant, SystemClock};
use veer_mailbox::Mailbox;
use veer_mci::Mci;

/// A listener timeout of 0 blocks until a message arrives.
const NO_TIMEOUT: u32 = 0;

/// How long the device waits for the update agent's response to a request
/// that the device sends.
const REQUEST_TIMEOUT_MS: u32 = 30_000;

/// How many times the device restarts the service after an error before it
/// gives up.
const MAX_SERVICE_ERRORS: u32 = 8;

/// How long the device leaves the update agent to read the response to
/// `ActivateFirmware` before it resets.
const ACTIVATION_DELAY_MS: u64 = 500;

/// Logs `reason` and stops the test with a failure.
fn fail(reason: &'static str) -> ! {
    pw_log::error!("pldm fd failed: {}", reason as &str);
    let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}

#[process_entry("pldm_fd")]
fn entry() {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };
    // SAFETY: the system config maps the MCI registers for this process. The
    // kernel uses only the timer registers of the MCI, which this driver
    // does not write.
    let mut mci = unsafe { Mci::new(MCI_BASE) };

    let ops = UpdateOps::new(&mailbox);
    let mut device = FirmwareDevice::init(
        &ops,
        &PLDM_PROTOCOL_CAPABILITIES,
        MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP)),
        MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP)),
    );
    let mut buffer = [0u8; FD_MAX_MSG];

    pw_log::info!("{}", pldm_update_ids::READY as &str);
    let mut errors = 0u32;
    loop {
        match device.run_until(
            HOST_EID,
            &mut buffer,
            NO_TIMEOUT,
            REQUEST_TIMEOUT_MS,
            &mut (),
            || !ops.activation_requested(),
        ) {
            RunTerminusResult::Completed => break,
            // The service stops on any error. Its state is kept, so the
            // device starts it again.
            RunTerminusResult::StoppedByError(_) => {
                errors += 1;
                pw_log::warn!("pldm fd: service stopped, restart {}", errors as u32);
                if errors >= MAX_SERVICE_ERRORS {
                    fail("service stopped too often");
                }
            }
        }
    }

    pw_log::info!("pldm fd: activating the update");
    let deadline = SystemClock::now()
        .checked_add_duration(Duration::from_millis(ACTIVATION_DELAY_MS))
        .unwrap_or(Instant::MAX);
    let _ = time::sleep_until(deadline);

    match ops.switch_image(&mut mci) {
        update_ops::SwitchError::Manifest => fail("Caliptra rejected the SoC manifest"),
        update_ops::SwitchError::Activation => fail("Caliptra rejected the activation"),
        update_ops::SwitchError::NoResetRequest => fail("no reset request from Caliptra"),
        update_ops::SwitchError::NoReset => fail("the MCU was not reset"),
    }
}
