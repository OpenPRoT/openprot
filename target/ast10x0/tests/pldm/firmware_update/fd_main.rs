// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM Firmware Device app (the RoT).
//!
//! Runs the real [`FirmwareDevice`] state machine against the update agent on
//! the mock BMC over MCTP/I2C2, streams the image it is handed into the shared
//! staging flash on SPI1 chip select 0, and reads it back. The demo's claim is that
//! the bytes the update agent sent are the bytes now sitting in flash, and that
//! activating the update resets the BMC so it would boot them.

#![no_main]
#![no_std]

use core::cell::{Cell, RefCell};
use core::time::Duration;

use ast10x0_board::apply_spim_external_mux;
use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::gpio::{bind_gpio, GpioBlock};
use ast10x0_peripherals::scu::{ScuExtMuxSelect, ScuRegisters, SpiMonitorInstance};
use flash_backend::NoWaitBlocking;
use hal_flash::{BlockingFlash, Flash, FlashAddress};
use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::{DelayNs, InputPin, OutputPin, ResetControl};
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_orchestrator_pldm_adapter::{verify_outcome_event, UpdateRequestLatch};
use openprot_orchestrator_sm::{
    Chain, ComponentAttrs, ComponentId, Effect, EffectError, Event, Orchestrator, Platform,
    PowerOnResult, State,
};
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use orchestrator_capabilities::BootStatus;
use orchestrator_hal_adapters::{GpioReadyMonitor, GpioResetControl};
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
use userspace::{entry, syscall};
use util_error::ErrorCode;
use util_region::Region;
use util_types::PowerOf2Usize;

use app_pldm_fd::handle;
use app_pldm_fd_regions::{take_mmaps, Spi1Cs0Window, Spi1Regs};

/// The SPI1 backend bound to this process's register mapping.
type Backend = flash_backend::Ast10x0Spi1FlashDriver<Spi1Regs>;

/// This board's EID, matching the MCTP server app underneath it.
const FD_EID: u8 = 8;
/// The update agent's EID on the mock BMC.
const UA_EID: u8 = 9;

/// This device's UUID, the one descriptor QueryDeviceIdentifiers reports. The
/// update agent holds the same value and refuses to update anything else.
const DEVICE_UUID: [u8; 16] = [
    0x4f, 0x50, 0x52, 0x54, 0x00, 0x01, 0x40, 0x00, 0xa0, 0x00, 0xde, 0xad, 0xbe, 0xef, 0x00, 0x01,
];

/// The single updatable component this device advertises. The update agent
/// learns it from GetFirmwareParameters rather than assuming it.
const COMP_IDENTIFIER: u16 = 0x0001;

/// Version strings the device reports as running today. The update agent must
/// offer something newer for the component to be accepted.
const ACTIVE_COMP_VERSION: &str = "v0.9";
const ACTIVE_IMAGE_SET_VERSION: &str = "openprot-demo-v0.9";

/// Where the image lands on the shared staging flash. Scratch
/// region, sector-aligned, and clobbered without backup — persisting the image
/// is the point of the test.
const IMAGE_BASE: u32 = 0x10_0000;

/// Readback chunk used by `verify`; one SPI NOR page.
const READBACK_CHUNK: usize = 256;

/// How long the FD waits for an update agent command before giving up and
/// reporting the result. This board is flashed first, so it must outlast the
/// mock BMC's UART upload (~50s for a 512 KiB image at 115200 baud), the
/// staging of that image into its boot flash, and the reset that boots it.
const IDLE_TIMEOUT_MILLIS: u32 = 180_000;
/// How long each FD-initiated request waits for the update agent's reply.
const REQUESTER_TIMEOUT_MILLIS: u32 = 5_000;

const FD_BUF_SIZE: usize = 1024;

/// The one component this test's orchestrator supervises, standing in for the
/// device this app itself is running on.
const ORCH_COMPONENT: ComponentId = ComponentId::new(0);

/// Chain capacity and effect-sink cap for the core (`E >= 2*N + 2`).
const ORCH_N: usize = 1;
const ORCH_E: usize = 2 * ORCH_N + 2;
const ORCH_MAX_RETRY: u8 = 1;

type OrchestratorCore = Orchestrator<ORCH_N, ORCH_E>;

/// GPIOJ0, the reset-passthrough line the Pi harness watches. Raising it asks
/// the harness to hold the mock BMC's board in reset once its (pretend) update
/// has gone through. One line, so `GpioResetControl` needs an id type with
/// exactly one variant to name it.
#[derive(Clone, PartialEq)]
enum PassthroughReset {
    MockBmc,
}

/// Busy-waits on the monotonic system clock, so `GpioResetControl` (generic over
/// any blocking delay provider) can drive its own pulse timing. The board's
/// `delay_us` is an uncalibrated spin loop and returns several times too early.
struct BoardDelay;

impl DelayNs for BoardDelay {
    fn delay_ns(&mut self, ns: u32) {
        self.delay_us(ns.div_ceil(1000));
    }

    fn delay_us(&mut self, us: u32) {
        use userspace::time::Duration;
        let until = syscall::debug_clock_now() + Duration::from_micros(u64::from(us));
        while syscall::debug_clock_now() < until {}
    }
}

/// How often [`wait_for_boot_status`] re-reads the booted line while waiting.
const ALIVE_POLL_INTERVAL_MICROS: u32 = 10_000;

/// How long to wait for the mock BMC to go quiet. It has to cover the Pi's
/// mirror noticing GPIOJ0, which is a 50ms sampling interval plus a `pinctrl`
/// subprocess.
const RESET_WINDOW: Duration = Duration::from_secs(1);

/// How long to wait for the mock BMC to come back up. It has to cover the boot
/// ROM copying the whole image out of flash into SRAM, then kernel init.
const BOOT_WINDOW: Duration = Duration::from_secs(30);

/// A [`Platform`] that presses the mock BMC's reset on [`Effect::ActivateUpdate`]
/// — standing in for the reboot a real activation would cause. Every other
/// effect is a no-op: boot verification and update staging aren't what this
/// test proves, only that a real RequestUpdate, arriving over the wire from
/// the update agent, drives the orchestrator through to activation.
///
/// `execute` never manufactures the reboot's outcome: it drives the line and
/// watches, and only reports `Event::BootConfirmed` once the mock BMC's alive
/// line actually says so.
struct TestPlatform<OutP, InP> {
    reset: GpioResetControl<OutP, BoardDelay, PassthroughReset>,
    ready: GpioReadyMonitor<InP>,
}

impl<OutP: OutputPin, InP: InputPin> Platform for TestPlatform<OutP, InP> {
    fn execute(&mut self, effect: Effect) -> Result<Option<Event>, EffectError> {
        match effect {
            Effect::ActivateUpdate => {
                // The fall has to be watched while the line is still high: the
                // Pi's mirror drops the mock BMC's reset as soon as it sees
                // GPIOJ0, so the line falls during the hold. The rise can only
                // be watched after letting go, because the mirror holds the
                // mock BMC down for as long as GPIOJ0 stays asserted.
                //
                // The `expect`s document an unreachable branch, not a swallowed
                // error: the GPIO output's error type is `Infallible`, and
                // `PassthroughReset` has a single variant, so
                // `GpioResetControl`'s own id check always matches.
                self.reset
                    .reset_assert(&PassthroughReset::MockBmc)
                    .expect("GPIOJ0 is Infallible and PassthroughReset has one variant");
                wait_for_boot_status(&mut self.ready, BootStatus::Booting, RESET_WINDOW);
                self.reset
                    .reset_deassert(&PassthroughReset::MockBmc)
                    .expect("GPIOJ0 is Infallible and PassthroughReset has one variant");

                if !wait_for_boot_status(&mut self.ready, BootStatus::Booted, BOOT_WINDOW) {
                    pw_log::error!("FD: mock BMC reset, but never came back up");
                    return Ok(None);
                }

                pw_log::info!("FD: GPIOJ0 reset landed; the mock BMC rebooted");
                Ok(Some(Event::BootConfirmed(ORCH_COMPONENT)))
            }
            _ => Ok(None),
        }
    }
}

/// Watches the mock BMC's booted line (GPIOH4) until it reads `want` or
/// `window` elapses, returning whether it was observed in time. The mock BMC
/// raises this line at its entry point and leaves it there, so `Booting` is the
/// low gap a reset carves out and `Booted` is the board back up afterwards. A
/// read error is treated the same as not yet
/// matching, following `CheckpointWalk`'s silence-tolerance convention.
fn wait_for_boot_status<InP: InputPin>(
    alive: &mut GpioReadyMonitor<InP>,
    want: BootStatus,
    window: Duration,
) -> bool {
    let deadline_us = window.as_micros() as u64;
    let mut waited_us: u64 = 0;
    loop {
        if matches!(alive.boot_status(), Ok(status) if status == want) {
            return true;
        }
        if waited_us >= deadline_us {
            return false;
        }
        BoardDelay.delay_us(ALIVE_POLL_INTERVAL_MICROS);
        waited_us += ALIVE_POLL_INTERVAL_MICROS as u64;
    }
}

/// Both of [`new_ready_core`]'s fallible setup steps are capacity overruns
/// against a `heapless::Vec` sized `ORCH_N == 1`, unreachable with this
/// test's single component; the concrete error carries nothing worth
/// preserving, so every failure maps to the same code.
fn setup_err<E>(_: E) -> ErrorCode {
    ErrorCode::kernel_error(Error::Internal)
}

/// A fresh core supervising just [`ORCH_COMPONENT`], already released to
/// [`State::Ready`]. Boot verification is bypassed here, not exercised by
/// this test.
fn new_ready_core<OutP: OutputPin, InP: InputPin>(
    platform: &mut TestPlatform<OutP, InP>,
) -> Result<OrchestratorCore, ErrorCode> {
    let mut components = heapless::Vec::<(ComponentId, ComponentAttrs), ORCH_N>::new();
    components
        .push((ORCH_COMPONENT, ComponentAttrs::passive_required()))
        .map_err(setup_err)?;
    let chain: Chain<ORCH_N> = components.try_into().map_err(setup_err)?;
    let mut core = Orchestrator::new(chain, ORCH_MAX_RETRY);
    core.dispatch(platform, Event::PowerGood(PowerOnResult::Provisioned));
    core.dispatch(platform, Event::VerificationPassed(ORCH_COMPONENT));
    Ok(core)
}

/// The image's byte count, as the update agent declared it in UpdateComponent.
/// `PassComponentTable` leaves it unset, but every download-phase callback runs
/// after UpdateComponent, so it is populated by then.
fn image_size(component: &FirmwareComponent) -> usize {
    component.comp_image_size.unwrap_or(0) as usize
}

/// FNV-1a seed and multiplier. Order-sensitive, so a readback that is
/// truncated, padded, or out of order fails even when it holds the same bytes.
const CHECKSUM_INIT: u32 = 0x811c_9dc5;

fn checksum(mut acc: u32, data: &[u8]) -> u32 {
    for byte in data {
        acc ^= *byte as u32;
        acc = acc.wrapping_mul(0x0100_0193);
    }
    acc
}

/// Firmware-device operations for the demo: program the image into SPI1 CS0 as it
/// arrives, then read it back.
///
/// `FdOps` takes `&self` throughout, so the flash handle lives behind a
/// `RefCell`.
struct DemoFdOps {
    flash: RefCell<BlockingFlash<Backend, NoWaitBlocking>>,
    sector: PowerOf2Usize,
    capacity: usize,
    erased_through: Cell<usize>,
    bytes_received: Cell<usize>,
    checksum: Cell<u32>,
    corrupt: Cell<bool>,
    verified: Cell<bool>,
    activated: Cell<bool>,
}

impl DemoFdOps {
    fn new(
        flash: BlockingFlash<Backend, NoWaitBlocking>,
        sector: PowerOf2Usize,
        capacity: usize,
    ) -> Self {
        DemoFdOps {
            flash: RefCell::new(flash),
            sector,
            capacity,
            erased_through: Cell::new(0),
            bytes_received: Cell::new(0),
            checksum: Cell::new(CHECKSUM_INIT),
            corrupt: Cell::new(false),
            verified: Cell::new(false),
            activated: Cell::new(false),
        }
    }

    /// True once the whole image was read back out of flash intact.
    fn image_is_good(&self) -> bool {
        self.verified.get()
    }
}

impl FdOps for DemoFdOps {
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

        // Self-contained: the device applies the image itself, so the update
        // agent's ActivateFirmware is all that is needed to finish.
        let mut activation = ComponentActivationMethods(0);
        activation.set_self_contained(true);

        let component = ComponentParameterEntry::new(
            ComponentClassification::Firmware,
            COMP_IDENTIFIER,
            0, // comp_classification_index
            &PldmFirmwareVersion::new(0, &comp_version, None),
            &PldmFirmwareVersion::default(),
            activation,
            FirmwareDeviceCapability(0),
        );

        *firmware_params = FirmwareParameters::new(
            FirmwareDeviceCapability(0),
            1, // comp_count
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
        // Matches the offered component against what this device advertised:
        // an unknown identifier, or a version no newer than the running one,
        // is refused.
        let code = component.evaluate_update_eligibility(fw_params);
        if code != ComponentResponseCode::CompCanBeUpdated {
            pw_log::error!("FD: component refused, code {}", code as u32);
            return Ok(code);
        }
        // The staging region starts partway up the part, so an image that fits
        // the flash can still run off the end of what is left above IMAGE_BASE.
        let size = image_size(component);
        if size > self.capacity.saturating_sub(IMAGE_BASE as usize) {
            pw_log::error!(
                "FD: {} byte image does not fit the {} bytes above the staging base",
                size as u32,
                (self.capacity.saturating_sub(IMAGE_BASE as usize)) as u32
            );
            return Ok(ComponentResponseCode::CompNotSupported);
        }
        Ok(code)
    }

    fn query_download_offset_and_length(
        &self,
        component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        // The state machine keeps no cursor of its own: whatever this returns is
        // the offset of the next RequestFirmwareData, verbatim.
        let done = self.bytes_received.get();
        Ok((done, image_size(component).saturating_sub(done)))
    }

    fn download_fw_data(
        &self,
        offset: usize,
        data: &[u8],
        _component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        let done = self.bytes_received.get();
        if offset != done {
            // A retry re-delivers a window already programmed, and NOR cannot
            // set bits back to 1, so reprogramming it would corrupt the image.
            return Ok(TransferResult::TransferSuccess);
        }
        let mut flash = self.flash.borrow_mut();
        // Erase lazily, a sector ahead of the write, so the image's real size
        // does not have to be known before the first chunk arrives.
        let end = offset + data.len();
        while self.erased_through.get() < end {
            let at = self.erased_through.get();
            if let Err(e) = flash.erase(FlashAddress::new(IMAGE_BASE + at as u32), self.sector) {
                self.corrupt.set(true);
                pw_log::error!(
                    "FD: erase at {} failed: {:08x}",
                    at as u32,
                    e.0.get() as u32
                );
                return Ok(TransferResult::FdAbortedTransfer);
            }
            self.erased_through.set(at + self.sector.get());
        }
        if let Err(e) = flash.program(FlashAddress::new(IMAGE_BASE + offset as u32), data) {
            self.corrupt.set(true);
            pw_log::error!(
                "FD: program at {} failed: {:08x}",
                offset as u32,
                e.0.get() as u32
            );
            return Ok(TransferResult::FdAbortedTransfer);
        }
        self.checksum.set(checksum(self.checksum.get(), data));
        self.bytes_received.set(done + data.len());
        Ok(TransferResult::TransferSuccess)
    }

    fn is_download_complete(&self, component: &FirmwareComponent) -> bool {
        self.bytes_received.get() >= image_size(component)
    }

    fn query_download_progress(
        &self,
        component: &FirmwareComponent,
        progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        let size = image_size(component);
        let pct = if size == 0 {
            0
        } else {
            (self.bytes_received.get() * 100 / size) as u8
        };
        progress_percent
            .set_value(pct.min(100))
            .map_err(|_| FdOpsError::FwDownloadError)?;
        Ok(())
    }

    fn verify(
        &self,
        component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
        let size = image_size(component);
        // A zero size would make the readback loop and the comparison below
        // both vacuous, reporting success over an empty flash.
        if size == 0 || self.corrupt.get() || self.bytes_received.get() < size {
            pw_log::error!(
                "FD: image incomplete, {} bytes",
                self.bytes_received.get() as u32
            );
            return Ok(VerifyResult::VerifyGenericError);
        }

        let mut flash = self.flash.borrow_mut();
        let mut buf = [0u8; READBACK_CHUNK];
        let mut read_back = CHECKSUM_INIT;
        for base in (0..size).step_by(READBACK_CHUNK) {
            let n = (size - base).min(READBACK_CHUNK);
            if let Err(e) = flash.read(FlashAddress::new(IMAGE_BASE + base as u32), &mut buf[..n]) {
                pw_log::error!(
                    "FD: read at {} failed: {:08x}",
                    base as u32,
                    e.0.get() as u32
                );
                return Ok(VerifyResult::VerifyGenericError);
            }
            read_back = checksum(read_back, &buf[..n]);
        }
        if read_back != self.checksum.get() {
            pw_log::error!(
                "FD: flash checksum {:08x}, expected {:08x}",
                read_back as u32,
                self.checksum.get() as u32
            );
            return Ok(VerifyResult::VerifyGenericError);
        }

        self.verified.set(true);
        pw_log::info!("FD: image verified in flash, {} bytes", size as u32);

        // This round's last flash access on this side, so hand the staging flash
        // to the mock BMC: release SPIM0's input from our SPI1 master, then flip
        // the fixture-level select. Passthrough stays on; that is the BMC's path.
        // SAFETY: this process holds the SCU mapping and is its only writer.
        let scu = unsafe { ScuRegisters::new_global_unlocked() };
        scu.clear_spim_internal_master_route();
        apply_spim_external_mux(SpiMonitorInstance::Spim0, ScuExtMuxSelect::Mux0);

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
        // The image is already in flash; nothing is deferred, so the update
        // agent is told to expect no wait.
        *estimated_time = 0;
        self.activated.set(true);
        pw_log::info!("FD: activated");
        Ok(PldmBaseCompletionCode::Success as u8)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Ok(())
    }
}

/// Bring up SPI1 and report its sector size, which the download erases by.
fn init_flash(
    spi1_regs: Region<Spi1Regs>,
    spi1_cs0_window: Region<Spi1Cs0Window>,
) -> Result<(BlockingFlash<Backend, NoWaitBlocking>, PowerOf2Usize, usize), ErrorCode> {
    // The kernel target applied the SPI1 pinmux and the SPIM0 route to the BMC
    // flash before any process started.
    let driver = Backend::new(spi1_regs, spi1_cs0_window)?;
    let mut flash = BlockingFlash {
        driver,
        blocking: NoWaitBlocking,
    };
    let (capacity, sector, _) = flash.geometry()?;
    pw_log::info!(
        "FD: SPI1 CS0 is {} bytes, {} byte sectors",
        capacity.get() as u32,
        sector.get() as u32
    );
    Ok((flash, sector, capacity.get()))
}

#[entry]
fn entry() {
    // SAFETY: mints this process's memory mappings once, at its entry point.
    let mmaps = unsafe { take_mmaps() };
    let (flash, sector, capacity) = match init_flash(mmaps.spi1_regs, mmaps.spi1_cs0_window) {
        Ok(flash) => flash,
        Err(e) => {
            pw_log::error!("FD: flash init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };
    let fd_ops = DemoFdOps::new(flash, sector, capacity);

    // SAFETY: sole pin creation site in this binary; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let gpio = GpioBlock::new(mmaps.gpio_regs);
    let mut reset = match GpioResetControl::new(
        bind_gpio(pins.scu418_8, &gpio).into_output(),
        BoardDelay,
        PassthroughReset::MockBmc,
        ActivePolarity::ActiveHigh,
    ) {
        Ok(reset) => reset,
        Err(_) => {
            pw_log::error!("FD: GPIOJ0 bind failed");
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };
    // `GpioResetControl::new` asserts unconditionally at bind time; force the
    // line back to the level the kernel already left it at (low/deasserted),
    // so binding it here doesn't itself request a reset.
    let _ = reset.reset_deassert(&PassthroughReset::MockBmc);
    let ready = GpioReadyMonitor::new(
        bind_gpio(pins.scu414_28, &gpio).into_input(),
        ActivePolarity::ActiveHigh,
    );

    let mut platform = TestPlatform { reset, ready };
    let mut orchestrator = match new_ready_core(&mut platform) {
        Ok(core) => core,
        Err(e) => {
            pw_log::error!("FD: orchestrator init failed: {:08x}", e.0.get() as u32);
            let _ = syscall::debug_shutdown(Err(Error::Internal));
            loop {}
        }
    };

    // Both transports talk to the same MCTP server over the same IPC channel.
    // Nothing is ever in flight on both at once: run_terminus alternates its
    // initiator and responder phases from this single thread.
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
    let mut update_latch = UpdateRequestLatch::new(ORCH_COMPONENT);
    // Returns as soon as ActivateFirmware puts the device back in Idle; an
    // idle timeout arrives as an error instead.
    match fd.run_terminus(
        UA_EID,
        &mut buf,
        IDLE_TIMEOUT_MILLIS,
        REQUESTER_TIMEOUT_MILLIS,
        &mut update_latch,
    ) {
        RunTerminusResult::Completed => {}
        // A plain idle timeout arrives here too, so the code distinguishes
        // "the update agent never spoke" from a real transport fault.
        RunTerminusResult::StoppedByError(PldmServiceError::Mctp(e)) => {
            pw_log::error!("FD: run_terminus stopped, MCTP code {}", e.code as u32);
        }
        RunTerminusResult::StoppedByError(_) => {
            pw_log::error!("FD: run_terminus stopped on a PLDM error");
        }
    }

    if let Some(event) = update_latch.take() {
        orchestrator.dispatch(&mut platform, event);
    }
    let orchestrator_reached_updating = orchestrator.state() == State::Updating(ORCH_COMPONENT);
    if !orchestrator_reached_updating {
        pw_log::error!("FD: orchestrator never reached State::Updating");
    }

    // The real verify() outcome, already settled by the time run_terminus
    // returns, becomes the event that lets the orchestrator leave Updating.
    if orchestrator_reached_updating {
        // ActivateUpdate drives GPIOJ0 and watches the alive line itself, so
        // the BootConfirmed it reports comes back through this one dispatch.
        // The rise is the proof: it says the reset request travelled through the
        // Pi, landed on the mock BMC, and the board came back up.
        orchestrator.dispatch(&mut platform, verify_outcome_event(fd_ops.image_is_good()));
        // Activation only proposes the image, so the orchestrator re-walks its
        // verification sequence. Boot verification is bypassed here, as it is in
        // new_ready_core, so the walk is released the same way.
        orchestrator.dispatch(&mut platform, Event::VerificationPassed(ORCH_COMPONENT));
    }
    let orchestrator_closed_the_loop =
        orchestrator_reached_updating && orchestrator.state() == State::Ready;
    if !orchestrator_closed_the_loop {
        pw_log::error!("FD: orchestrator never returned to State::Ready after verify");
    }

    if fd_ops.image_is_good() && fd_ops.activated.get() && orchestrator_closed_the_loop {
        pw_log::info!("FD: update flow complete");
        let _ = syscall::debug_shutdown(Ok(()));
    } else {
        pw_log::error!(
            "FD: update flow did not complete, {} bytes received",
            fd_ops.bytes_received.get() as u32
        );
        let _ = syscall::debug_shutdown(Err(Error::Internal));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FD: panic");
    let _ = syscall::debug_shutdown(Err(Error::Internal));
    loop {}
}
