// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The PLDM firmware device, running the real DSP0267 state machine.
//!
//! The same `FirmwareDevice` the demo ships, reached over the same
//! `IpcMctpClient` call path. What differs from the hardware card is one
//! layer at the bottom and one at the side: the wire is the loopback bus
//! rather than I2C, and the image is staged into RAM rather than SPI NOR.
//!
//! Staging into RAM keeps this app independent of the flash service, so
//! the first scenario proves the PLDM exchange on its own. A later
//! scenario that cares where the bytes land writes them through the
//! device server instead, which is also what makes "verify what is on
//! flash" mean anything.

#![no_main]
#![no_std]

use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicBool, Ordering};

use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_pldm_service::firmware_device::{
    FdEvent, FdEventSink, FirmwareDevice, RunTerminusResult,
};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::FirmwareParameters;
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::message::firmware_update::request_fw_data::MAX_TRANSFER_SIZE;
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::PldmBaseCompletionCode;
use pldm_common::protocol::firmware_update::{
    ComponentActivationMethods, ComponentClassification, ComponentParameterEntry,
    ComponentResponseCode, Descriptor, DescriptorType, FirmwareDeviceCapability,
    PldmFirmwareString, PldmFirmwareVersion,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};
use pw_status::Error;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use app_pldm_fd::handle;

/// This device's endpoint id, matching the bus's firmware-device side.
const FD_EID: u8 = 8;

/// The update agent's endpoint id, matching the bus's other side.
const UA_EID: u8 = 42;

/// Bytes in the image the update agent offers. Small enough to stage in
/// this app's RAM and still take several transfers.
const IMAGE_SIZE: usize = 1024;

/// Identifies this device to the update agent. Arbitrary, but it has to
/// match what the agent looks for.
const DEVICE_UUID: [u8; 16] = [
    0x4f, 0x50, 0x52, 0x4f, 0x54, 0x2d, 0x51, 0x45, 0x4d, 0x55, 0x2d, 0x4d, 0x4f, 0x43, 0x4b, 0x01,
];

/// The one component this device advertises.
const COMP_IDENTIFIER: u16 = 0x0001;
const ACTIVE_COMP_VERSION: &str = "v0.9";
const ACTIVE_IMAGE_SET_VERSION: &str = "openprot-qemu-v0.9";

/// How long the device waits for the agent to say anything before giving
/// up. Generous: a stuck test should fail on the harness timeout with a
/// log, not here with a bare error.
const IDLE_TIMEOUT_MILLIS: u32 = 30_000;

/// How long a request the device itself makes may take to be answered.
const REQUESTER_TIMEOUT_MILLIS: u32 = 5_000;

/// Working buffer for one PLDM message.
const FD_BUF_SIZE: usize = 1024;

/// Message opcodes to the orchestrator, and the answers it gives. The
/// orchestrator decides; this device reports and obeys.
const MSG_UPDATE_REQUESTED: u8 = 1;
const MSG_VERIFY_OUTCOME: u8 = 2;
const MSG_ACTIVATED: u8 = 3;
const REPLY_ACCEPTED: u8 = 1;

/// How long an orchestrator answer may take. It is a local process with no
/// work to do but answer, so this is a deadlock bound, not a budget.
const ORCH_TIMEOUT_MILLIS: u64 = 2_000;

/// What the orchestrator said to each report. Read after the flow ends: a
/// refusal anywhere means the update ran without the RoT's consent, which
/// fails the run even if PLDM itself was happy.
static ORCH_ALLOWED_UPDATE: AtomicBool = AtomicBool::new(false);
static ORCH_TOOK_VERDICT: AtomicBool = AtomicBool::new(false);
static ORCH_COMMITTED: AtomicBool = AtomicBool::new(false);

/// Reports one update-lifecycle step to the orchestrator and returns whether
/// it was accepted. Blocks until the answer arrives: the next PLDM step must
/// not run ahead of the RoT's decision.
fn tell_orchestrator(message: &[u8]) -> bool {
    let deadline = Instant::from_ticks(
        SystemClock::now().ticks() + ORCH_TIMEOUT_MILLIS * SystemClock::TICKS_PER_SEC / 1000,
    );
    let mut reply = [0u8; 1];
    match syscall::channel_transact(handle::ORCH, message, &mut reply, deadline) {
        Ok(len) => len == 1 && reply[0] == REPLY_ACCEPTED,
        Err(_) => {
            pw_log::error!("FD: the orchestrator did not answer");
            false
        }
    }
}

/// Forwards the PLDM state machine's lifecycle events to the orchestrator as
/// they happen, rather than after the flow ends. An accepted `RequestUpdate`
/// is the orchestrator's cue to record the job and enter `Updating`, and it
/// has to be in that state before the verdict arrives.
struct OrchestratorLink;

impl FdEventSink for OrchestratorLink {
    fn notify(&mut self, event: FdEvent) {
        match event {
            FdEvent::UpdateRequested => {
                let len = (IMAGE_SIZE as u32).to_le_bytes();
                let message = [MSG_UPDATE_REQUESTED, len[0], len[1], len[2], len[3]];
                let allowed = tell_orchestrator(&message);
                ORCH_ALLOWED_UPDATE.store(allowed, Ordering::Relaxed);
                if !allowed {
                    pw_log::error!("FD: the orchestrator refused the update");
                }
            }
            // The other lifecycle events have no orchestrator mapping yet.
            // Verify and activation are reported from the FdOps calls that
            // settle them, because only those know the outcome.
            _ => {}
        }
    }
}

/// The byte the agent is expected to send at `offset`. Checking the
/// content, not just the length, is what makes a silent truncation or a
/// misordered window fail rather than pass.
fn expected_byte(offset: usize) -> u8 {
    (offset % 251) as u8
}

/// Staging, plus what the run loop needs to judge the outcome afterwards.
struct QemuFdOps {
    image: RefCell<[u8; IMAGE_SIZE]>,
    bytes_received: Cell<usize>,
    corrupt: Cell<bool>,
    verified: Cell<bool>,
    activated: Cell<bool>,
}

impl QemuFdOps {
    fn new() -> Self {
        Self {
            image: RefCell::new([0u8; IMAGE_SIZE]),
            bytes_received: Cell::new(0),
            corrupt: Cell::new(false),
            verified: Cell::new(false),
            activated: Cell::new(false),
        }
    }

    fn image_is_good(&self) -> bool {
        self.verified.get() && !self.corrupt.get() && self.bytes_received.get() == IMAGE_SIZE
    }
}

impl FdOps for QemuFdOps {
    fn get_device_identifiers(
        &self,
        device_identifiers: &mut [Descriptor],
    ) -> Result<usize, FdOpsError> {
        let uuid = Descriptor::new(DescriptorType::Uuid, &DEVICE_UUID)
            .map_err(|_| FdOpsError::DeviceIdentifiersError)?;
        *device_identifiers
            .first_mut()
            .ok_or(FdOpsError::DeviceIdentifiersError)? = uuid;
        Ok(1)
    }

    fn get_firmware_parms(
        &self,
        firmware_params: &mut FirmwareParameters,
    ) -> Result<(), FdOpsError> {
        let comp_version = PldmFirmwareString::new("ASCII", ACTIVE_COMP_VERSION)
            .map_err(|_| FdOpsError::FirmwareParametersError)?;
        let image_set_version = PldmFirmwareString::new("ASCII", ACTIVE_IMAGE_SET_VERSION)
            .map_err(|_| FdOpsError::FirmwareParametersError)?;

        // Self-contained: the device applies the image itself, so the
        // agent's ActivateFirmware is all that is needed to finish.
        let mut activation = ComponentActivationMethods(0);
        activation.set_self_contained(true);

        let component = ComponentParameterEntry::new(
            ComponentClassification::Firmware,
            COMP_IDENTIFIER,
            0,
            &PldmFirmwareVersion::new(0, &comp_version, None),
            &PldmFirmwareVersion::default(),
            activation,
            FirmwareDeviceCapability(0),
        );

        *firmware_params = FirmwareParameters::new(
            FirmwareDeviceCapability(0),
            1,
            &image_set_version,
            &PldmFirmwareString::default(),
            &[component],
        );
        Ok(())
    }

    fn get_xfer_size(&self, ua_transfer_size: usize) -> Result<usize, FdOpsError> {
        Ok(ua_transfer_size.min(MAX_TRANSFER_SIZE))
    }

    fn handle_component(
        &self,
        component: &FirmwareComponent,
        fw_params: &FirmwareParameters,
        _op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        let code = component.evaluate_update_eligibility(fw_params);
        if code != ComponentResponseCode::CompCanBeUpdated {
            pw_log::error!("FD: component refused, code {}", code as u32);
        }
        Ok(code)
    }

    fn query_download_offset_and_length(
        &self,
        _component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        // The state machine keeps no cursor of its own: whatever this
        // returns is the offset of the next RequestFirmwareData.
        let done = self.bytes_received.get();
        Ok((done, IMAGE_SIZE - done))
    }

    fn download_fw_data(
        &self,
        offset: usize,
        data: &[u8],
        _component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        let done = self.bytes_received.get();
        if offset != done {
            // A retry re-delivers a window already staged. Accepting it
            // without rewriting keeps the cursor honest.
            return Ok(TransferResult::TransferSuccess);
        }
        if offset + data.len() > IMAGE_SIZE {
            self.corrupt.set(true);
            pw_log::error!("FD: window past the end of the image");
            return Ok(TransferResult::FdAbortedTransfer);
        }
        for (i, byte) in data.iter().enumerate() {
            if *byte != expected_byte(offset + i) {
                self.corrupt.set(true);
                pw_log::error!("FD: byte {} is wrong", (offset + i) as u32);
                return Ok(TransferResult::FdAbortedTransfer);
            }
        }
        self.image.borrow_mut()[offset..offset + data.len()].copy_from_slice(data);
        self.bytes_received.set(done + data.len());
        Ok(TransferResult::TransferSuccess)
    }

    fn is_download_complete(&self, _component: &FirmwareComponent) -> bool {
        self.bytes_received.get() >= IMAGE_SIZE
    }

    fn query_download_progress(
        &self,
        _component: &FirmwareComponent,
        progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        let pct = (self.bytes_received.get() * 100 / IMAGE_SIZE) as u8;
        progress_percent
            .set_value(pct.min(100))
            .map_err(|_| FdOpsError::FwDownloadError)?;
        Ok(())
    }

    fn verify(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
        if self.corrupt.get() || self.bytes_received.get() < IMAGE_SIZE {
            pw_log::error!(
                "FD: image incomplete, {} bytes",
                self.bytes_received.get() as u32
            );
            ORCH_TOOK_VERDICT.store(
                tell_orchestrator(&[MSG_VERIFY_OUTCOME, 0]),
                Ordering::Relaxed,
            );
            return Ok(VerifyResult::VerifyGenericError);
        }

        // Read the staged image back rather than trusting the running
        // tally. Signature checking belongs to the crypto service and is
        // stubbed until after the demo, so this is a content check.
        let image = self.image.borrow();
        for (offset, byte) in image.iter().enumerate() {
            if *byte != expected_byte(offset) {
                pw_log::error!("FD: staged byte {} is wrong", offset as u32);
                ORCH_TOOK_VERDICT.store(
                    tell_orchestrator(&[MSG_VERIFY_OUTCOME, 0]),
                    Ordering::Relaxed,
                );
                return Ok(VerifyResult::VerifyGenericError);
            }
        }

        self.verified.set(true);
        pw_log::info!("FD: image verified, {} bytes", IMAGE_SIZE as u32);
        ORCH_TOOK_VERDICT.store(
            tell_orchestrator(&[MSG_VERIFY_OUTCOME, 1]),
            Ordering::Relaxed,
        );
        Ok(VerifyResult::VerifySuccess)
    }

    fn apply(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        Ok(ApplyResult::ApplySuccess)
    }

    fn activate(
        &self,
        _self_contained_activation: u8,
        estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        // Nothing is deferred, so the agent is told to expect no wait.
        *estimated_time = 0;
        self.activated.set(true);
        pw_log::info!("FD: activated");
        // Stands in for the updated device coming back up. Nothing in this
        // image reboots, so the report is what the orchestrator treats as a
        // healthy first boot.
        ORCH_COMMITTED.store(tell_orchestrator(&[MSG_ACTIVATED]), Ordering::Relaxed);
        Ok(PldmBaseCompletionCode::Success as u8)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

#[entry]
fn entry() {
    pw_log::info!("FD: app started");
    let fd_ops = QemuFdOps::new();

    // Both transports reach the same MCTP server over the same channel.
    // Nothing is in flight on both at once: run_terminus alternates its
    // initiator and responder phases from this one thread.
    let responder_transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));
    let requester_transport = MctpPldmTransport::new(IpcMctpClient::new(handle::MCTP));

    if responder_transport.stack().set_eid(FD_EID).is_err() {
        pw_log::error!("FD: set_eid failed");
        let _ = syscall::debug_shutdown(Err(Error::Internal));
        loop {}
    }

    let mut fd = FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        responder_transport,
        requester_transport,
    );

    pw_log::info!("FD: waiting for the update agent at EID {}", UA_EID as u32);

    let mut buf = [0u8; FD_BUF_SIZE];
    match fd.run_terminus(
        UA_EID,
        &mut buf,
        IDLE_TIMEOUT_MILLIS,
        REQUESTER_TIMEOUT_MILLIS,
        &mut OrchestratorLink,
    ) {
        RunTerminusResult::Completed => {}
        RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) => {
            pw_log::error!("FD: run_terminus stopped, MCTP code {}", e.code as u32);
        }
        RunTerminusResult::StoppedByError(_) => {
            pw_log::error!("FD: run_terminus stopped on a PLDM error");
        }
    }

    let orchestrator_consented = ORCH_ALLOWED_UPDATE.load(Ordering::Relaxed)
        && ORCH_TOOK_VERDICT.load(Ordering::Relaxed)
        && ORCH_COMMITTED.load(Ordering::Relaxed);
    let completed = fd_ops.image_is_good() && fd_ops.activated.get() && orchestrator_consented;

    if verdict(&fd_ops, completed) {
        let _ = syscall::debug_shutdown(Ok(()));
    } else {
        let _ = syscall::debug_shutdown(Err(Error::Internal));
    }
    loop {}
}

/// Whether the run did what this scenario asked of it.
///
/// A negative scenario does not just expect the update to fail. It names
/// the failure it arranged, because a run that died of something else
/// would otherwise look like the thing being proven.
#[cfg(not(any(corrupt_image, refused_update)))]
fn verdict(fd_ops: &QemuFdOps, completed: bool) -> bool {
    if completed {
        pw_log::info!("FD: update flow complete");
        return true;
    }
    pw_log::error!(
        "FD: update flow did not complete, {} bytes received",
        fd_ops.bytes_received.get() as u32
    );
    false
}

/// One byte of the agent's image was flipped, so the device must have
/// caught it and the update must not have gone through.
#[cfg(corrupt_image)]
fn verdict(fd_ops: &QemuFdOps, completed: bool) -> bool {
    if completed {
        pw_log::error!("FD: a corrupt image went through");
        return false;
    }
    if !fd_ops.corrupt.get() {
        pw_log::error!("FD: the run failed, but not because of the corrupt byte");
        return false;
    }
    pw_log::info!("FD: the corrupt image was caught and the update refused");
    true
}

/// The orchestrator refused, so nothing it was asked may have been
/// accepted, and the update must not have gone through.
#[cfg(refused_update)]
fn verdict(_fd_ops: &QemuFdOps, completed: bool) -> bool {
    if completed {
        pw_log::error!("FD: the update went through without consent");
        return false;
    }
    if ORCH_ALLOWED_UPDATE.load(Ordering::Relaxed)
        || ORCH_TOOK_VERDICT.load(Ordering::Relaxed)
        || ORCH_COMMITTED.load(Ordering::Relaxed)
    {
        pw_log::error!("FD: the orchestrator refused the request but agreed to something else");
        return false;
    }
    pw_log::info!("FD: the orchestrator withheld consent and the update did not happen");
    true
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
