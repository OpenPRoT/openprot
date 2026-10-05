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

use flash_backend::{Backend, NoWaitBlocking};
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_api::{FdStatus, RejectReason, ResponseCode, TransferMode};
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
use pldm_server::{dispatch, FdIpcHandler};
use pw_status::Error;
use userspace::syscall::Signals;
use userspace::time::{Clock, Instant, SystemClock};
use userspace::{entry, syscall};

use util_error::ErrorCode;
use util_region::Region;

use app_pldm_fd::handle;
use app_pldm_fd_regions::{take_mmaps, FmcCs0Window, FmcCs1Window, FmcRegs};

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

/// Where the image lands on CS1, standing in for the managed device's boot
/// flash. A scratch region, erased at startup and overwritten without
/// backup: holding the image is what this test is for.
const IMAGE_BASE: u32 = 0x10_0000;

/// Readback chunk used by `verify`, one SPI NOR page.
const READBACK_CHUNK: usize = 256;

/// The FMC backend bound to this process's register mapping.
type Backend_ = Backend<FmcRegs>;

/// How long the device waits for the RoT to command it. The RoT is a local
/// process with nothing to do but answer, so this bounds a deadlock rather
/// than budgeting work.
const DECISION_TIMEOUT_MILLIS: u64 = 5_000;

/// Largest IPC frame either way on the RoT's channel.
const IPC_BUF_SIZE: usize = 128;

/// The component the RoT is asked about. A PLDM component identifier, not
/// an orchestrator `ComponentId`: the RoT maps one to the other, and this
/// side of the channel speaks PLDM.
const ORCH_TARGET: u16 = COMP_IDENTIFIER;

/// Which way the RoT answered the decision the device was waiting on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision {
    Perform,
    Reject,
}

/// Where the device is in the update, as `QueryStatus` reports it.
///
/// The first four are points DSP0267 already lets the firmware device take
/// time at, which is why the device can sit at one serving its channel
/// instead of running the update. `Failed` is not a wait: the device has
/// already told the agent the phase failed, and raises the signal once so
/// the RoT hears the same thing.
///
/// There is no step for "verify succeeded". Asking for apply is that: the
/// device only reaches `Apply` after reading the image back and finding it
/// whole, so `ApplyPending` is the device's verdict and `PhaseFailed` is
/// the other half of it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Awaiting {
    Nothing,
    Offer,
    Verify,
    Apply,
    Activation,
    Failed,
}

impl Awaiting {
    /// What `QueryStatus` answers at this step.
    fn status(self, total: u32) -> FdStatus {
        match self {
            Awaiting::Nothing => FdStatus::Idle { reason: 0 },
            Awaiting::Offer => FdStatus::OfferPending {
                target: ORCH_TARGET,
                total,
                // The device pulls its own chunks from the agent, which
                // is what DSP0267 has a firmware device do.
                mode: TransferMode::InTransport,
                svn_delayed: false,
            },
            Awaiting::Verify => FdStatus::VerifyPending,
            Awaiting::Apply => FdStatus::ApplyPending,
            Awaiting::Activation => FdStatus::ActivationPending,
            // The phase and result code the device already sent the agent.
            Awaiting::Failed => FdStatus::PhaseFailed {
                phase: 0,
                result_code: 1,
            },
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
    flash: RefCell<BlockingFlash<Backend_, NoWaitBlocking>>,
    bytes_received: Cell<usize>,
    corrupt: Cell<bool>,
    verified: Cell<bool>,
    activated: Cell<bool>,
    /// A rendezvous pair, not two independent flags: `awaiting` is the
    /// step `QueryStatus` reports, `decision` is the answer the serve loop
    /// ends on. One outstanding at a time, because the device is not
    /// running the update while it waits.
    awaiting: Cell<Awaiting>,
    decision: Cell<Option<Decision>>,
    /// Set when the RoT asked what this device is doing. The failure path
    /// waits for one of these rather than for a command, because there is
    /// nothing to command once a phase has failed.
    status_asked: Cell<bool>,
    /// Set when the RoT rejected a step. The run fails on this even if
    /// PLDM itself was happy, because an update the RoT refused must not
    /// look like a success.
    refused: Cell<bool>,
    /// Set when the channel itself broke: no signal, no answer, a command
    /// for the wrong step. Kept apart from `refused` so a scenario that
    /// claims the RoT refused cannot pass on an IPC that never happened.
    ipc_failed: Cell<bool>,
}

impl QemuFdOps {
    fn new(flash: BlockingFlash<Backend_, NoWaitBlocking>) -> Self {
        Self {
            flash: RefCell::new(flash),
            bytes_received: Cell::new(0),
            corrupt: Cell::new(false),
            verified: Cell::new(false),
            activated: Cell::new(false),
            awaiting: Cell::new(Awaiting::Nothing),
            decision: Cell::new(None),
            status_asked: Cell::new(false),
            refused: Cell::new(false),
            ipc_failed: Cell::new(false),
        }
    }

    /// Asks the RoT to decide `step` and blocks until it answers.
    ///
    /// The device raises the peer's `USER` signal and then serves its own
    /// channel until a command lands. It is not running the update while it
    /// waits, so serving here costs nothing: the RoT's `QueryStatus` is
    /// answered from the same loop that is waiting for its command.
    fn await_decision(&self, step: Awaiting) -> Decision {
        if !self.serve(step, |ops| ops.decision.get().is_some()) {
            // No answer. Treat it as a refusal so the update stops, but
            // record that the channel broke rather than that the RoT said
            // no, so a scenario asserting a refusal cannot pass on this.
            self.ipc_failed.set(true);
            return Decision::Reject;
        }

        let decision = self.decision.get().unwrap_or(Decision::Reject);
        if decision == Decision::Reject {
            self.refused.set(true);
        }
        decision
    }

    /// Tells the RoT a phase failed, and waits only long enough for it to
    /// ask. Nothing is commanded after a failure, so waiting for a command
    /// would wait forever.
    fn report_failure(&self) {
        if !self.serve(Awaiting::Failed, |ops| ops.status_asked.get()) {
            self.ipc_failed.set(true);
        }
    }

    /// Raises the RoT's signal, then answers its requests until `done` or
    /// the deadline. Returns whether `done` came true.
    ///
    /// The device is not running the update while it sits here, so serving
    /// the channel from this loop costs nothing: the RoT's `QueryStatus`
    /// is answered by the same loop that is waiting for its command.
    fn serve(&self, step: Awaiting, done: impl Fn(&Self) -> bool) -> bool {
        self.awaiting.set(step);
        self.decision.set(None);
        self.status_asked.set(false);

        if syscall::object_set_peer_user_signal(handle::ORCH, true).is_err() {
            pw_log::error!("FD: could not raise the RoT's signal");
            self.awaiting.set(Awaiting::Nothing);
            return false;
        }

        let deadline = Instant::from_ticks(
            SystemClock::now().ticks()
                + DECISION_TIMEOUT_MILLIS * SystemClock::TICKS_PER_SEC / 1000,
        );
        let mut request = [0u8; IPC_BUF_SIZE];
        let mut response = [0u8; IPC_BUF_SIZE];
        let mut answered = false;

        while !done(self) {
            if syscall::object_wait(handle::ORCH, Signals::READABLE, deadline).is_err() {
                pw_log::error!("FD: the RoT never answered");
                break;
            }
            let Ok(len) = syscall::channel_read(handle::ORCH, 0usize, &mut request) else {
                continue;
            };
            let mut gate = OrchGate { ops: self };
            // dispatch only errors when even an error frame will not fit,
            // which IPC_BUF_SIZE rules out. Answering nothing would leave
            // the RoT reading a truncated frame, so say so instead.
            let written = match dispatch(&mut gate, &request[..len], &mut response) {
                Ok(written) => written,
                Err(_) => {
                    pw_log::error!("FD: {} byte IPC buffer is too small", IPC_BUF_SIZE as u32);
                    break;
                }
            };
            let _ = syscall::channel_respond(handle::ORCH, &response[..written]);
            answered = true;
        }

        // Leaving with a request unanswered strands the RoT's transaction:
        // its buffers stay lent to the kernel and its next command fails.
        // The loop above answers whatever it read, so this only has to
        // cover the deadline case, where nothing was read at all.
        let settled = done(self);
        if !settled && answered {
            pw_log::error!("FD: giving up with the RoT mid-exchange");
        }

        let _ = syscall::object_set_peer_user_signal(handle::ORCH, false);
        // Single-threaded: nothing reads these between the two writes.
        self.awaiting.set(Awaiting::Nothing);
        settled
    }

    /// True when the RoT allowed every step it was asked about, and the
    /// channel worked.
    fn orchestrator_consented(&self) -> bool {
        !self.refused.get() && !self.ipc_failed.get()
    }

    fn image_is_good(&self) -> bool {
        self.verified.get() && !self.corrupt.get() && self.bytes_received.get() == IMAGE_SIZE
    }
}

/// The RoT's view of this device: one method per command it can send.
///
/// Every method records a decision and returns. Nothing here runs the
/// update; the `FdOps` call that is waiting picks the decision up and
/// carries on.
struct OrchGate<'a> {
    ops: &'a QemuFdOps,
}

impl OrchGate<'_> {
    /// Records `decision` if the device is waiting on `step`, and refuses
    /// otherwise. A command for a step the device is not at is a bug in
    /// the RoT, not something to act on.
    fn answer(&self, step: Awaiting, decision: Decision) -> Result<(), ResponseCode> {
        if self.ops.awaiting.get() != step {
            pw_log::error!("FD: a command arrived for a step this device is not at");
            self.ops.ipc_failed.set(true);
            return Err(ResponseCode::InvalidOp);
        }
        self.ops.decision.set(Some(decision));
        Ok(())
    }

    /// The device never reaches this step in this scenario, so being
    /// commanded here means the RoT thinks it is somewhere it is not.
    fn not_reached(&self) -> Result<(), ResponseCode> {
        pw_log::error!("FD: commanded a step this scenario never reaches");
        self.ops.ipc_failed.set(true);
        Err(ResponseCode::InvalidOp)
    }
}

impl FdIpcHandler for OrchGate<'_> {
    fn accept_offer(&mut self, _staging_base: u32) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Offer, Decision::Perform)
    }

    fn reject_offer(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Offer, Decision::Reject)
    }

    fn perform_verify(&mut self) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Verify, Decision::Perform)
    }

    fn reject_verify(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Verify, Decision::Reject)
    }

    fn perform_apply(&mut self) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Apply, Decision::Perform)
    }

    fn reject_apply(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Apply, Decision::Reject)
    }

    fn perform_activate(&mut self) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Activation, Decision::Perform)
    }

    fn reject_activate(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
        self.answer(Awaiting::Activation, Decision::Reject)
    }

    fn query_status(&mut self) -> Result<FdStatus, ResponseCode> {
        self.ops.status_asked.set(true);
        Ok(self.ops.awaiting.get().status(IMAGE_SIZE as u32))
    }

    fn ack_cancel(&mut self) -> Result<(), ResponseCode> {
        self.not_reached()
    }

    fn perform_svn_commit(&mut self) -> Result<(), ResponseCode> {
        self.not_reached()
    }

    fn reject_svn_commit(&mut self, _reason: RejectReason) -> Result<(), ResponseCode> {
        self.not_reached()
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
        op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        let code = component.evaluate_update_eligibility(fw_params);
        if code != ComponentResponseCode::CompCanBeUpdated {
            pw_log::error!("FD: component refused, code {}", code as u32);
            return Ok(code);
        }
        // The agent asks twice, once to pass the component table and once
        // to start the component. The RoT is asked on the second: that is
        // the one that commits to a transfer, and asking twice would have
        // it record the job and then refuse its own job as already in
        // flight.
        if op == ComponentOperation::UpdateComponent
            && self.await_decision(Awaiting::Offer) == Decision::Reject
        {
            pw_log::error!("FD: the RoT refused the offer");
            return Ok(ComponentResponseCode::CompNotSupported);
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
        // Written only after the content check, so a wrong byte never
        // reaches flash and the image on the device stays whatever it was.
        if let Err(e) = self
            .flash
            .borrow_mut()
            .program(FlashAddress::new(IMAGE_BASE + offset as u32), data)
        {
            self.corrupt.set(true);
            pw_log::error!(
                "FD: program at {} failed: {:08x}",
                offset as u32,
                e.0.get() as u32
            );
            return Ok(TransferResult::FdAbortedTransfer);
        }
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
            self.report_failure();
            return Ok(VerifyResult::VerifyGenericError);
        }

        if self.await_decision(Awaiting::Verify) == Decision::Reject {
            // No report: the RoT refused this itself and knows the outcome.
            pw_log::error!("FD: the RoT refused the verify");
            return Ok(VerifyResult::VerifyGenericError);
        }

        // Read the image back out of flash rather than trusting the
        // running tally. A write that silently did not land, or landed
        // somewhere else, fails here instead of passing. Signature
        // checking belongs to the crypto service and is stubbed until
        // after the demo, so this is a content check.
        let mut flash = self.flash.borrow_mut();
        let mut chunk = [0u8; READBACK_CHUNK];
        for base in (0..IMAGE_SIZE).step_by(READBACK_CHUNK) {
            if let Err(e) = flash.read(FlashAddress::new(IMAGE_BASE + base as u32), &mut chunk) {
                pw_log::error!(
                    "FD: read at {} failed: {:08x}",
                    base as u32,
                    e.0.get() as u32
                );
                drop(flash);
                self.report_failure();
                return Ok(VerifyResult::VerifyGenericError);
            }
            for (i, byte) in chunk.iter().enumerate() {
                if *byte != expected_byte(base + i) {
                    pw_log::error!("FD: flash byte {} is wrong", (base + i) as u32);
                    drop(flash);
                    self.report_failure();
                    return Ok(VerifyResult::VerifyGenericError);
                }
            }
        }

        self.verified.set(true);
        pw_log::info!("FD: image verified in flash, {} bytes", IMAGE_SIZE as u32);
        Ok(VerifyResult::VerifySuccess)
    }

    fn apply(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        if self.await_decision(Awaiting::Apply) == Decision::Reject {
            pw_log::error!("FD: the RoT refused the apply");
            return Ok(ApplyResult::ApplyGenericError);
        }
        Ok(ApplyResult::ApplySuccess)
    }

    fn activate(
        &self,
        _self_contained_activation: u8,
        estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        // Nothing is deferred, so the agent is told to expect no wait.
        *estimated_time = 0;
        if self.await_decision(Awaiting::Activation) == Decision::Reject {
            pw_log::error!("FD: the RoT refused the activation");
            return Ok(PldmBaseCompletionCode::Error as u8);
        }
        self.activated.set(true);
        pw_log::info!("FD: activated");
        Ok(PldmBaseCompletionCode::Success as u8)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

/// Brings up the FMC and erases the sector the image lands in.
///
/// The kernel applied the FMC pinmux before any process started, so there
/// is no SCU access here.
fn init_flash(
    fmc_regs: Region<FmcRegs>,
    fmc_cs0_window: Region<FmcCs0Window>,
    fmc_cs1_window: Region<FmcCs1Window>,
) -> Result<BlockingFlash<Backend_, NoWaitBlocking>, ErrorCode> {
    let driver = Backend::new(fmc_regs, fmc_cs0_window, fmc_cs1_window)?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let (capacity, sector, _) = flash.geometry()?;
    pw_log::info!(
        "FD: CS1 is {} bytes, {} byte sectors",
        capacity.get() as u32,
        sector.get() as u32
    );
    flash.erase(FlashAddress::new(IMAGE_BASE), sector)?;
    Ok(flash)
}

#[entry]
fn entry() {
    pw_log::info!("FD: app started");

    // SAFETY: mints this process's memory mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let flash = match init_flash(mmaps.fmc_regs, mmaps.fmc_cs0_window, mmaps.fmc_cs1_window) {
        Ok(flash) => flash,
        Err(e) => {
            pw_log::error!("FD: flash init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };
    let fd_ops = QemuFdOps::new(flash);

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
        &mut (),
    ) {
        RunTerminusResult::Completed => {}
        RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) => {
            pw_log::error!("FD: run_terminus stopped, MCTP code {}", e.code as u32);
        }
        RunTerminusResult::StoppedByError(_) => {
            pw_log::error!("FD: run_terminus stopped on a PLDM error");
        }
    }

    let completed =
        fd_ops.image_is_good() && fd_ops.activated.get() && fd_ops.orchestrator_consented();

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
fn verdict(fd_ops: &QemuFdOps, completed: bool) -> bool {
    if completed {
        pw_log::error!("FD: the update went through without consent");
        return false;
    }
    if fd_ops.orchestrator_consented() {
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
