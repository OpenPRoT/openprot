// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for the control and status registers of the Caliptra-SS MCI block.
//!
//! The driver covers the reason for the last MCU reset, the request for an
//! MCU reset, the firmware flow status, the firmware error registers, the
//! errors that the hardware has recorded, the security state, the hardware
//! revision and configuration, the generic wires, and the interrupt for the
//! events of the MCI. The watchdog registers have their own driver,
//! `veer_wdt`.
//!
//! The MCI has one interrupt to the MCU. An event sets its bit in a status
//! register, and the interrupt is asserted while a pending event has its
//! interrupt enable set. The MCI has four groups of events, which [`Events`]
//! holds: [`ErrorEvents`], [`Notifications`], and the fatal and the non-fatal
//! [`AggregatedErrors`] from the SoC.
//!
//! Register offsets and bit fields come from the register definitions that
//! caliptra-mcu-sw generates (`caliptra_ss_registers::mci`).
//!
//! The firmware flow status register has no fields in hardware. This driver
//! follows caliptra-mcu-sw (`romtime/src/mci.rs` and
//! `romtime/src/boot_status.rs`): the low half is the last checkpoint that
//! the ROM reached, and the high half is a set of [`Milestones`].

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::mci;
use mci::bits::{
    Error0IntrT, GlobalIntrEnT, HwConfig0, HwConfig1, HwErrorFatal, HwErrorNonFatal, HwRevId,
    Notif0IntrT, ResetRequest,
};
use tock_registers::LocalRegisterCopy;

/// Number of words in `FW_EXTENDED_ERROR_INFO`.
pub const EXTENDED_ERROR_INFO_WORDS: usize = 8;

/// The MCI registers this driver uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reg {
    /// `HW_REV_ID`.
    HwRevId,
    /// `HW_CONFIG0`: sizes of the MCU mailbox SRAMs.
    HwConfig0,
    /// `HW_CONFIG1`: size of the MCU SRAM.
    HwConfig1,
    /// `FW_FLOW_STATUS`.
    FwFlowStatus,
    /// `RESET_REASON`.
    ResetReason,
    /// `SECURITY_STATE`.
    SecurityState,
    /// `HW_ERROR_FATAL`: write-one-to-clear record of [`HwFatalErrors`].
    HwErrorFatal,
    /// `AGG_ERROR_FATAL`: write-one-to-clear record of the aggregated fatal
    /// errors.
    AggErrorFatal,
    /// `HW_ERROR_NON_FATAL`: write-one-to-clear record of
    /// [`HwNonFatalErrors`].
    HwErrorNonFatal,
    /// `AGG_ERROR_NON_FATAL`: write-one-to-clear record of the aggregated
    /// non-fatal errors.
    AggErrorNonFatal,
    /// `FW_ERROR_FATAL`.
    FwErrorFatal,
    /// `FW_ERROR_NON_FATAL`.
    FwErrorNonFatal,
    /// Word `index % 8` of `FW_EXTENDED_ERROR_INFO`.
    FwExtendedErrorInfo(u8),
    /// `RESET_REQUEST`.
    ResetRequest,
    /// `GENERIC_INPUT_WIRES[0]`: wires 0 to 31.
    GenericInputWiresLow,
    /// `GENERIC_INPUT_WIRES[1]`: wires 32 to 63.
    GenericInputWiresHigh,
    /// `GENERIC_OUTPUT_WIRES[0]`: wires 0 to 31.
    GenericOutputWiresLow,
    /// `GENERIC_OUTPUT_WIRES[1]`: wires 32 to 63.
    GenericOutputWiresHigh,
    /// `global_intr_en_r`: one interrupt enable for all error events and one
    /// for all notifications.
    GlobalInterruptEnable,
    /// `error0_intr_en_r`: interrupt enables of [`ErrorEvents`].
    ErrorEnable,
    /// `error1_intr_en_r`: interrupt enables of the aggregated fatal errors.
    AggregateFatalEnable,
    /// `notif0_intr_en_r`: interrupt enables of [`Notifications`].
    NotificationEnable,
    /// `notif1_intr_en_r`: interrupt enables of the aggregated non-fatal
    /// errors.
    AggregateNonFatalEnable,
    /// `error0_internal_intr_r`: write-one-to-clear status of
    /// [`ErrorEvents`].
    ErrorStatus,
    /// `notif0_internal_intr_r`: write-one-to-clear status of
    /// [`Notifications`].
    NotificationStatus,
    /// `error1_internal_intr_r`: write-one-to-clear status of the aggregated
    /// fatal errors.
    AggregateFatalStatus,
    /// `notif1_internal_intr_r`: write-one-to-clear status of the aggregated
    /// non-fatal errors.
    AggregateNonFatalStatus,
    /// `error0_intr_trig_r`: writing 1 to a bit sets the bit in the error
    /// status.
    ErrorTrigger,
    /// `notif0_intr_trig_r`: writing 1 to a bit sets the bit in the
    /// notification status.
    NotificationTrigger,
    /// `error1_intr_trig_r`: writing 1 to a bit sets the bit in the status of
    /// the aggregated fatal errors.
    AggregateFatalTrigger,
    /// `notif1_intr_trig_r`: writing 1 to a bit sets the bit in the status of
    /// the aggregated non-fatal errors.
    AggregateNonFatalTrigger,
}

impl Reg {
    /// Byte offset of the register from the MCI block base.
    pub const fn offset(self) -> usize {
        match self {
            Reg::HwRevId => offset_of!(mci::regs::Mci, mci_reg_hw_rev_id),
            Reg::HwConfig0 => offset_of!(mci::regs::Mci, mci_reg_hw_config0),
            Reg::HwConfig1 => offset_of!(mci::regs::Mci, mci_reg_hw_config1),
            Reg::FwFlowStatus => offset_of!(mci::regs::Mci, mci_reg_fw_flow_status),
            Reg::ResetReason => offset_of!(mci::regs::Mci, mci_reg_reset_reason),
            Reg::SecurityState => offset_of!(mci::regs::Mci, mci_reg_security_state),
            Reg::HwErrorFatal => offset_of!(mci::regs::Mci, mci_reg_hw_error_fatal),
            Reg::AggErrorFatal => offset_of!(mci::regs::Mci, mci_reg_agg_error_fatal),
            Reg::HwErrorNonFatal => offset_of!(mci::regs::Mci, mci_reg_hw_error_non_fatal),
            Reg::AggErrorNonFatal => offset_of!(mci::regs::Mci, mci_reg_agg_error_non_fatal),
            Reg::FwErrorFatal => offset_of!(mci::regs::Mci, mci_reg_fw_error_fatal),
            Reg::FwErrorNonFatal => offset_of!(mci::regs::Mci, mci_reg_fw_error_non_fatal),
            Reg::FwExtendedErrorInfo(index) => {
                offset_of!(mci::regs::Mci, mci_reg_fw_extended_error_info)
                    + 4 * (index as usize % EXTENDED_ERROR_INFO_WORDS)
            }
            Reg::ResetRequest => offset_of!(mci::regs::Mci, mci_reg_reset_request),
            Reg::GenericInputWiresLow => offset_of!(mci::regs::Mci, mci_reg_generic_input_wires),
            Reg::GenericInputWiresHigh => {
                offset_of!(mci::regs::Mci, mci_reg_generic_input_wires) + 4
            }
            Reg::GenericOutputWiresLow => {
                offset_of!(mci::regs::Mci, mci_reg_generic_output_wires)
            }
            Reg::GenericOutputWiresHigh => {
                offset_of!(mci::regs::Mci, mci_reg_generic_output_wires) + 4
            }
            Reg::GlobalInterruptEnable => {
                offset_of!(mci::regs::Mci, intr_block_rf_global_intr_en_r)
            }
            Reg::ErrorEnable => offset_of!(mci::regs::Mci, intr_block_rf_error0_intr_en_r),
            Reg::AggregateFatalEnable => {
                offset_of!(mci::regs::Mci, intr_block_rf_error1_intr_en_r)
            }
            Reg::NotificationEnable => offset_of!(mci::regs::Mci, intr_block_rf_notif0_intr_en_r),
            Reg::AggregateNonFatalEnable => {
                offset_of!(mci::regs::Mci, intr_block_rf_notif1_intr_en_r)
            }
            Reg::ErrorStatus => offset_of!(mci::regs::Mci, intr_block_rf_error0_internal_intr_r),
            Reg::NotificationStatus => {
                offset_of!(mci::regs::Mci, intr_block_rf_notif0_internal_intr_r)
            }
            Reg::AggregateFatalStatus => {
                offset_of!(mci::regs::Mci, intr_block_rf_error1_internal_intr_r)
            }
            Reg::AggregateNonFatalStatus => {
                offset_of!(mci::regs::Mci, intr_block_rf_notif1_internal_intr_r)
            }
            Reg::ErrorTrigger => offset_of!(mci::regs::Mci, intr_block_rf_error0_intr_trig_r),
            Reg::NotificationTrigger => {
                offset_of!(mci::regs::Mci, intr_block_rf_notif0_intr_trig_r)
            }
            Reg::AggregateFatalTrigger => {
                offset_of!(mci::regs::Mci, intr_block_rf_error1_intr_trig_r)
            }
            Reg::AggregateNonFatalTrigger => {
                offset_of!(mci::regs::Mci, intr_block_rf_notif1_intr_trig_r)
            }
        }
    }
}

/// Access to the MCI registers. The driver is generic over this trait so that
/// host tests can use a model in place of the hardware.
pub trait Mmio {
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// The MCI's memory-mapped registers.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of the MCI register block. The block must be
    /// mapped for the caller, and no other code may write the registers in
    /// [`Reg`], except that `veer_wdt` may clear the watchdog bits of the error
    /// status. A write to a status register changes only the bits that are
    /// written as 1.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }
}

impl Mmio for MmioBlock {
    fn read(&self, reg: Reg) -> u32 {
        let addr = self.base.wrapping_add(reg.offset());
        // SAFETY: every `Reg` is the offset of a register inside the block that
        // the caller of `new` guaranteed is mapped.
        unsafe { with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        let addr = self.base.wrapping_add(reg.offset());
        // SAFETY: as for `read`.
        unsafe { with_exposed_provenance_mut::<u32>(addr).write_volatile(value) }
    }
}

/// Why the MCU was last reset, from `RESET_REASON`.
///
/// The caliptra-mcu-sw ROM starts the runtime image only when the reason is
/// [`FirmwareBoot`](Self::FirmwareBoot) or
/// [`FirmwareHitlessUpdate`](Self::FirmwareHitlessUpdate). For the other
/// reasons it sets up the hardware and then requests an MCU reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetReason {
    /// No bit is set: the subsystem was powered on.
    PowerOn,
    /// The subsystem was reset while it stayed powered.
    WarmReset,
    /// The MCU was reset to start the first firmware image that was loaded
    /// since the MCI was reset.
    FirmwareBoot,
    /// The MCU was reset to start an image that replaced a running one.
    FirmwareHitlessUpdate,
    /// More than one bit is set. The value is the register.
    Invalid(u32),
}

impl ResetReason {
    const HITLESS_UPDATE: u32 = mci::bits::ResetReason::FwHitlessUpdReset::SET.value;
    const BOOT_UPDATE: u32 = mci::bits::ResetReason::FwBootUpdReset::SET.value;
    const WARM_RESET: u32 = mci::bits::ResetReason::WarmReset::SET.value;

    const fn from_register(value: u32) -> Self {
        match value {
            0 => Self::PowerOn,
            Self::WARM_RESET => Self::WarmReset,
            Self::BOOT_UPDATE => Self::FirmwareBoot,
            Self::HITLESS_UPDATE => Self::FirmwareHitlessUpdate,
            _ => Self::Invalid(value),
        }
    }

    /// A name for logs.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PowerOn => "power on",
            Self::WarmReset => "warm reset",
            Self::FirmwareBoot => "firmware boot",
            Self::FirmwareHitlessUpdate => "firmware hitless update",
            Self::Invalid(_) => "invalid",
        }
    }
}

bitflags::bitflags! {
    /// The milestones of the boot that the firmware flow status records. The
    /// ROM sets the first eight and the runtime image sets the others. The
    /// values are those of `McuBootMilestones` in caliptra-mcu-sw.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Milestones: u16 {
        /// The ROM started.
        const ROM_STARTED = 1 << 0;
        /// The ROM let Caliptra boot.
        const CPTRA_BOOT_GO_ASSERTED = 1 << 1;
        /// The ROM wrote Caliptra's fuse registers.
        const CPTRA_FUSES_WRITTEN = 1 << 2;
        /// Caliptra accepted the command to load firmware through the
        /// recovery interface.
        const RI_DOWNLOAD_COMPLETED = 1 << 3;
        /// The ROM loaded the firmware from flash.
        const FLASH_RECOVERY_FLOW_COMPLETED = 1 << 4;
        /// The ROM finished its cold boot flow.
        const COLD_BOOT_FLOW_COMPLETE = 1 << 5;
        /// The ROM finished its warm reset flow.
        const WARM_RESET_FLOW_COMPLETE = 1 << 6;
        /// The ROM is about to start the runtime image.
        const FIRMWARE_BOOT_FLOW_COMPLETE = 1 << 7;
        /// The runtime image's kernel has started.
        const FIRMWARE_OS_INITIALIZED = 1 << 8;
        /// The runtime image serves the MCU mailbox.
        const FIRMWARE_MAILBOX_READY = 1 << 9;
        /// The SPDM responder over MCTP is ready.
        const FIRMWARE_SPDM_MCTP_READY = 1 << 10;
        /// The SPDM responder over DOE is ready.
        const FIRMWARE_SPDM_DOE_READY = 1 << 11;
        /// The PLDM responder is ready.
        const FIRMWARE_PLDM_READY = 1 << 12;
    }
}

bitflags::bitflags! {
    /// The error events of the MCI, as bits of `error0_internal_intr_r`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ErrorEvents: u32 {
        /// An AXI access and a debug access to MCU SRAM collided.
        const MCU_SRAM_DMI_AXI_COLLISION = Error0IntrT::ErrorMcuSramDmiAxiCollisionSts::SET.value;
        /// An internal error of the MCI.
        const INTERNAL = Error0IntrT::ErrorInternalSts::SET.value;
        /// The SRAM of MCU mailbox 0 had an ECC error that was not corrected.
        const MBOX0_ECC_UNCORRECTED = Error0IntrT::ErrorMbox0EccUncSts::SET.value;
        /// The SRAM of MCU mailbox 1 had an ECC error that was not corrected.
        const MBOX1_ECC_UNCORRECTED = Error0IntrT::ErrorMbox1EccUncSts::SET.value;
        /// Watchdog timer 1 timed out. Clearing the event restarts the
        /// watchdog; see `veer_wdt`.
        const WDT_TIMER1_TIMEOUT = Error0IntrT::ErrorWdtTimer1TimeoutSts::SET.value;
        /// Watchdog timer 2 timed out.
        const WDT_TIMER2_TIMEOUT = Error0IntrT::ErrorWdtTimer2TimeoutSts::SET.value;
    }
}

bitflags::bitflags! {
    /// The notifications of the MCI, as bits of `notif0_internal_intr_r`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Notifications: u32 {
        /// MCU SRAM had an ECC error that was corrected.
        const MCU_SRAM_ECC_CORRECTED = Notif0IntrT::NotifMcuSramEccCorSts::SET.value;
        /// Caliptra requests an MCU reset. The MCU then asks the MCI for the
        /// reset with [`Mci::request_mcu_reset`].
        const CPTRA_MCU_RESET_REQUEST = Notif0IntrT::NotifCptraMcuResetReqSts::SET.value;
        /// A generic input wire changed.
        const GENERIC_INPUT_TOGGLE = Notif0IntrT::NotifGenInToggleSts::SET.value;
        /// The target of MCU mailbox 0 is done and its status is available.
        const MBOX0_TARGET_DONE = Notif0IntrT::NotifMbox0TargetDoneSts::SET.value;
        /// The target of MCU mailbox 1 is done and its status is available.
        const MBOX1_TARGET_DONE = Notif0IntrT::NotifMbox1TargetDoneSts::SET.value;
        /// MCU mailbox 0 holds a command for the MCU.
        const MBOX0_COMMAND_AVAILABLE = Notif0IntrT::NotifMbox0CmdAvailSts::SET.value;
        /// MCU mailbox 1 holds a command for the MCU.
        const MBOX1_COMMAND_AVAILABLE = Notif0IntrT::NotifMbox1CmdAvailSts::SET.value;
        /// The Caliptra mailbox holds a command for the MCU.
        const CPTRA_MBOX_COMMAND_AVAILABLE = Notif0IntrT::NotifCptraMboxCmdAvailSts::SET.value;
        /// The SRAM of MCU mailbox 0 had an ECC error that was corrected.
        const MBOX0_ECC_CORRECTED = Notif0IntrT::NotifMbox0EccCorSts::SET.value;
        /// The SRAM of MCU mailbox 1 had an ECC error that was corrected.
        const MBOX1_ECC_CORRECTED = Notif0IntrT::NotifMbox1EccCorSts::SET.value;
        /// The debug-locked bit of the security state changed.
        const DEBUG_LOCKED = Notif0IntrT::NotifDebugLockedSts::SET.value;
        /// The subsystem entered scan mode.
        const SCAN_MODE = Notif0IntrT::NotifScanModeSts::SET.value;
        /// The SoC asked for MCU mailbox 0 while the MCU held its lock.
        const MBOX0_SOC_REQUESTED_LOCK = Notif0IntrT::NotifMbox0SocReqLockSts::SET.value;
        /// The SoC asked for MCU mailbox 1 while the MCU held its lock.
        const MBOX1_SOC_REQUESTED_LOCK = Notif0IntrT::NotifMbox1SocReqLockSts::SET.value;
        /// An operation of the OTP controller is done.
        const OTP_OPERATION_DONE = Notif0IntrT::NotifOtpOperationDoneSts::SET.value;
    }
}

bitflags::bitflags! {
    /// The aggregated error inputs of the MCI, which come from the other
    /// blocks of the SoC. Bit `n` is input `n`. The MCI has one set of inputs
    /// for fatal errors and one for non-fatal errors.
    ///
    /// Each named set holds the inputs that `caliptra_ss_top.sv` connects to
    /// one source. The fatal and the non-fatal inputs are connected the same
    /// way.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct AggregatedErrors: u32 {
        /// Errors of the Caliptra core.
        const CALIPTRA = 0x0000_003f;
        /// ECC errors in the MCU's DCCM: uncorrected ones are fatal and
        /// corrected ones are non-fatal.
        const MCU = 0x0000_0fc0;
        /// Alerts of the lifecycle controller.
        const LIFECYCLE_CONTROLLER = 0x0003_f000;
        /// Alerts of the fuse controller.
        const FUSE_CONTROLLER = 0x00fc_0000;
        /// The peripheral reset and the escalated reset of the I3C core.
        const I3C = 0x3f00_0000;
        /// Inputs that the subsystem ties to 0.
        const SPARE = 0xc000_0000;
    }
}

bitflags::bitflags! {
    /// The fatal errors that the MCI's own hardware records in
    /// `HW_ERROR_FATAL`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct HwFatalErrors: u32 {
        /// MCU SRAM had an ECC error that was not corrected.
        const MCU_SRAM_ECC_UNCORRECTED = HwErrorFatal::McuSramEccUnc::SET.value;
        /// The watchdog raised the MCU's non-maskable interrupt.
        const NMI = HwErrorFatal::NmiPin::SET.value;
        /// An AXI access and a debug access to MCU SRAM collided.
        const MCU_SRAM_DMI_AXI_COLLISION = HwErrorFatal::McuSramDmiAxiCollision::SET.value;
    }
}

bitflags::bitflags! {
    /// The non-fatal errors that the MCI's own hardware records in
    /// `HW_ERROR_NON_FATAL`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct HwNonFatalErrors: u32 {
        /// The SRAM of MCU mailbox 0 had an ECC error that was not corrected.
        const MBOX0_ECC_UNCORRECTED = HwErrorNonFatal::Mbox0EccUnc::SET.value;
        /// The SRAM of MCU mailbox 1 had an ECC error that was not corrected.
        const MBOX1_ECC_UNCORRECTED = HwErrorNonFatal::Mbox1EccUnc::SET.value;
    }
}

/// A set of events from each of the four groups of the MCI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Events {
    pub errors: ErrorEvents,
    pub notifications: Notifications,
    pub aggregated_fatal: AggregatedErrors,
    pub aggregated_non_fatal: AggregatedErrors,
}

impl Events {
    /// No event of any group.
    pub const fn empty() -> Self {
        Self {
            errors: ErrorEvents::empty(),
            notifications: Notifications::empty(),
            aggregated_fatal: AggregatedErrors::empty(),
            aggregated_non_fatal: AggregatedErrors::empty(),
        }
    }
}

/// The lifecycle state of the device.
pub type Lifecycle = mci::bits::SecurityState::DeviceLifecycle::Value;

/// A copy of `SECURITY_STATE`.
#[derive(Clone, Copy)]
pub struct SecurityState(LocalRegisterCopy<u32, mci::bits::SecurityState::Register>);

impl SecurityState {
    /// The lifecycle state, or `None` if the field holds the one value that
    /// is not a state.
    pub fn lifecycle(&self) -> Option<Lifecycle> {
        self.0
            .read_as_enum(mci::bits::SecurityState::DeviceLifecycle)
    }

    /// Whether debug access to the subsystem is locked.
    pub fn debug_locked(&self) -> bool {
        self.0.is_set(mci::bits::SecurityState::DebugLocked)
    }

    /// Whether the subsystem is in scan mode.
    pub fn scan_mode(&self) -> bool {
        self.0.is_set(mci::bits::SecurityState::ScanMode)
    }
}

/// The Caliptra Subsystem release that the MCI and MCU hardware match, from
/// `HW_REV_ID`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HwRevision {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

/// The MCI control and status registers.
pub struct Mci<M: Mmio = MmioBlock> {
    mmio: M,
    // !Send + !Sync: exclusive ownership of one physical peripheral
    _not_send_sync: PhantomData<*mut ()>,
}

impl Mci<MmioBlock> {
    /// `mci_base` is the address of the MCI register block (`mci_offset` in
    /// the caliptra-mcu-sw memory map).
    ///
    /// # Safety
    ///
    /// The MCI register block must be at `mci_base` and stay mapped for the
    /// caller, and no other code may write the registers in [`Reg`], except
    /// that `veer_wdt` may clear the watchdog bits of the error status.
    pub const unsafe fn new(mci_base: usize) -> Self {
        // SAFETY: the caller guarantees that the block is mapped and that
        // nothing else writes these registers.
        Self::with_mmio(unsafe { MmioBlock::new(mci_base) })
    }
}

impl<M: Mmio> Mci<M> {
    /// Constructs the driver over `mmio`.
    pub const fn with_mmio(mmio: M) -> Self {
        Self {
            mmio,
            _not_send_sync: PhantomData,
        }
    }

    /// Why the MCU was last reset.
    pub fn reset_reason(&self) -> ResetReason {
        ResetReason::from_register(self.mmio.read(Reg::ResetReason))
    }

    /// Asks the MCI to reset the MCU. The MCI halts the MCU and then resets
    /// it, so the caller keeps running for a short time after this returns.
    ///
    /// The MCI does not change `RESET_REASON` for this reset, and nothing
    /// loads the image in MCU SRAM again.
    pub fn request_mcu_reset(&mut self) {
        self.mmio
            .write(Reg::ResetRequest, ResetRequest::McuReq::SET.value);
    }

    /// The last checkpoint that the ROM recorded (`McuRomBootStatus` in
    /// caliptra-mcu-sw).
    pub fn boot_checkpoint(&self) -> u16 {
        self.mmio.read(Reg::FwFlowStatus) as u16
    }

    /// The milestones that the ROM and the runtime image have recorded.
    pub fn milestones(&self) -> Milestones {
        Milestones::from_bits_retain((self.mmio.read(Reg::FwFlowStatus) >> 16) as u16)
    }

    /// Records `milestones` in addition to those already recorded.
    pub fn set_milestones(&mut self, milestones: Milestones) {
        let status = self.mmio.read(Reg::FwFlowStatus);
        self.mmio.write(
            Reg::FwFlowStatus,
            status | (u32::from(milestones.bits()) << 16),
        );
    }

    /// The fatal firmware error code, or 0 if there is none.
    pub fn fw_error_fatal(&self) -> u32 {
        self.mmio.read(Reg::FwErrorFatal)
    }

    /// Reports a fatal firmware error. A code other than 0 asserts the MCI's
    /// fatal error output to the SoC, unless the ROM masked the bits that are
    /// set. The output stays asserted until the MCI is reset.
    pub fn set_fw_error_fatal(&mut self, code: u32) {
        self.mmio.write(Reg::FwErrorFatal, code);
    }

    /// The non-fatal firmware error code, or 0 if there is none.
    pub fn fw_error_non_fatal(&self) -> u32 {
        self.mmio.read(Reg::FwErrorNonFatal)
    }

    /// Reports a non-fatal firmware error. A code other than 0 asserts the
    /// MCI's non-fatal error output to the SoC, unless the ROM masked the
    /// bits that are set. Writing 0 clears the error.
    pub fn set_fw_error_non_fatal(&mut self, code: u32) {
        self.mmio.write(Reg::FwErrorNonFatal, code);
    }

    /// The fatal errors of the MCI's own hardware that are in its record. The
    /// record stays through a warm reset.
    pub fn recorded_hw_fatal_errors(&self) -> HwFatalErrors {
        HwFatalErrors::from_bits_retain(self.mmio.read(Reg::HwErrorFatal))
    }

    /// The non-fatal errors of the MCI's own hardware that are in its record.
    /// The record stays through a warm reset.
    pub fn recorded_hw_non_fatal_errors(&self) -> HwNonFatalErrors {
        HwNonFatalErrors::from_bits_retain(self.mmio.read(Reg::HwErrorNonFatal))
    }

    /// The aggregated fatal errors that are in the MCI's record. The record
    /// stays through a warm reset.
    pub fn recorded_aggregated_fatal_errors(&self) -> AggregatedErrors {
        AggregatedErrors::from_bits_retain(self.mmio.read(Reg::AggErrorFatal))
    }

    /// The aggregated non-fatal errors that are in the MCI's record. The
    /// record stays through a warm reset.
    pub fn recorded_aggregated_non_fatal_errors(&self) -> AggregatedErrors {
        AggregatedErrors::from_bits_retain(self.mmio.read(Reg::AggErrorNonFatal))
    }

    /// The details that firmware stored about an error.
    pub fn fw_extended_error_info(&self) -> [u32; EXTENDED_ERROR_INFO_WORDS] {
        let mut info = [0; EXTENDED_ERROR_INFO_WORDS];
        for (index, word) in (0u8..).zip(info.iter_mut()) {
            *word = self.mmio.read(Reg::FwExtendedErrorInfo(index));
        }
        info
    }

    /// Stores details about an error for the SoC to read. The hardware gives
    /// the words no meaning.
    pub fn set_fw_extended_error_info(&mut self, info: &[u32; EXTENDED_ERROR_INFO_WORDS]) {
        for (index, word) in (0u8..).zip(info) {
            self.mmio.write(Reg::FwExtendedErrorInfo(index), *word);
        }
    }

    /// The hardware security state.
    pub fn security_state(&self) -> SecurityState {
        SecurityState(LocalRegisterCopy::new(self.mmio.read(Reg::SecurityState)))
    }

    /// The hardware revision.
    pub fn hw_revision(&self) -> HwRevision {
        let generation =
            LocalRegisterCopy::<u32, HwRevId::Register>::new(self.mmio.read(Reg::HwRevId))
                .read(HwRevId::McGeneration);
        HwRevision {
            major: ((generation >> 12) & 0xf) as u8,
            minor: ((generation >> 8) & 0xf) as u8,
            patch: generation as u8,
        }
    }

    /// Size of the MCU SRAM in KiB.
    pub fn mcu_sram_size_kib(&self) -> u32 {
        LocalRegisterCopy::<u32, HwConfig1::Register>::new(self.mmio.read(Reg::HwConfig1))
            .read(HwConfig1::McuSramSize)
    }

    /// Size of the SRAM of MCU mailbox 0 in KiB. It is 0 if the hardware has
    /// no mailbox 0.
    pub fn mbox0_sram_size_kib(&self) -> u32 {
        self.hw_config0().read(HwConfig0::McuMbox0SramSize)
    }

    /// Size of the SRAM of MCU mailbox 1 in KiB. It is 0 if the hardware has
    /// no mailbox 1.
    pub fn mbox1_sram_size_kib(&self) -> u32 {
        self.hw_config0().read(HwConfig0::McuMbox1SramSize)
    }

    /// The generic input wires from the SoC. Bit `n` is wire `n`.
    pub fn generic_input_wires(&self) -> u64 {
        self.read_wires(Reg::GenericInputWiresLow, Reg::GenericInputWiresHigh)
    }

    /// The generic output wires to the SoC. Bit `n` is wire `n`.
    pub fn generic_output_wires(&self) -> u64 {
        self.read_wires(Reg::GenericOutputWiresLow, Reg::GenericOutputWiresHigh)
    }

    /// Drives the generic output wires to the SoC. Bit `n` is wire `n`. The
    /// two halves change in separate writes, the low half first.
    pub fn set_generic_output_wires(&mut self, wires: u64) {
        self.mmio.write(Reg::GenericOutputWiresLow, wires as u32);
        self.mmio
            .write(Reg::GenericOutputWiresHigh, (wires >> 32) as u32);
    }

    /// Enables the MCI interrupt for `events` and disables it for every other
    /// event of the MCI.
    ///
    /// An event that is already pending asserts the interrupt as soon as its
    /// interrupt is enabled.
    pub fn enable_interrupts(&mut self, events: Events) {
        // The interrupt is off while the enables change.
        self.mmio.write(Reg::GlobalInterruptEnable, 0);
        self.mmio.write(Reg::ErrorEnable, events.errors.bits());
        self.mmio
            .write(Reg::AggregateFatalEnable, events.aggregated_fatal.bits());
        self.mmio
            .write(Reg::NotificationEnable, events.notifications.bits());
        self.mmio.write(
            Reg::AggregateNonFatalEnable,
            events.aggregated_non_fatal.bits(),
        );
        self.mmio.write(
            Reg::GlobalInterruptEnable,
            GlobalIntrEnT::ErrorEn::SET.value | GlobalIntrEnT::NotifEn::SET.value,
        );
    }

    /// The error events that have happened and are not acknowledged. The set
    /// includes events whose interrupt is not enabled.
    pub fn pending_errors(&self) -> ErrorEvents {
        ErrorEvents::from_bits_retain(self.mmio.read(Reg::ErrorStatus))
    }

    /// The notifications that have happened and are not acknowledged. The
    /// set includes notifications whose interrupt is not enabled.
    pub fn pending_notifications(&self) -> Notifications {
        Notifications::from_bits_retain(self.mmio.read(Reg::NotificationStatus))
    }

    /// The aggregated fatal errors that have happened and are not
    /// acknowledged. The set includes errors whose interrupt is not enabled.
    pub fn pending_aggregated_fatal_errors(&self) -> AggregatedErrors {
        AggregatedErrors::from_bits_retain(self.mmio.read(Reg::AggregateFatalStatus))
    }

    /// The aggregated non-fatal errors that have happened and are not
    /// acknowledged. The set includes errors whose interrupt is not enabled.
    pub fn pending_aggregated_non_fatal_errors(&self) -> AggregatedErrors {
        AggregatedErrors::from_bits_retain(self.mmio.read(Reg::AggregateNonFatalStatus))
    }

    /// Clears `errors` from the pending error events. Other pending events
    /// stay pending.
    pub fn acknowledge_errors(&mut self, errors: ErrorEvents) {
        self.mmio.write(Reg::ErrorStatus, errors.bits());
    }

    /// Clears `notifications` from the pending notifications. Other pending
    /// notifications stay pending.
    pub fn acknowledge_notifications(&mut self, notifications: Notifications) {
        self.mmio
            .write(Reg::NotificationStatus, notifications.bits());
    }

    /// Clears `errors` from the pending aggregated fatal errors. Other
    /// pending errors stay pending. An error whose input is still asserted
    /// becomes pending again.
    pub fn acknowledge_aggregated_fatal_errors(&mut self, errors: AggregatedErrors) {
        self.mmio.write(Reg::AggregateFatalStatus, errors.bits());
    }

    /// Clears `errors` from the pending aggregated non-fatal errors. Other
    /// pending errors stay pending. An error whose input is still asserted
    /// becomes pending again.
    pub fn acknowledge_aggregated_non_fatal_errors(&mut self, errors: AggregatedErrors) {
        self.mmio.write(Reg::AggregateNonFatalStatus, errors.bits());
    }

    /// Makes `errors` pending, as if the events had happened. For tests of
    /// interrupt handling.
    pub fn trigger_errors(&mut self, errors: ErrorEvents) {
        self.mmio.write(Reg::ErrorTrigger, errors.bits());
    }

    /// Makes `notifications` pending, as if they had happened. For tests of
    /// interrupt handling.
    pub fn trigger_notifications(&mut self, notifications: Notifications) {
        self.mmio
            .write(Reg::NotificationTrigger, notifications.bits());
    }

    /// Makes `errors` pending as aggregated fatal errors, as if their inputs
    /// had been asserted. For tests of interrupt handling.
    pub fn trigger_aggregated_fatal_errors(&mut self, errors: AggregatedErrors) {
        self.mmio.write(Reg::AggregateFatalTrigger, errors.bits());
    }

    /// Makes `errors` pending as aggregated non-fatal errors, as if their
    /// inputs had been asserted. For tests of interrupt handling.
    pub fn trigger_aggregated_non_fatal_errors(&mut self, errors: AggregatedErrors) {
        self.mmio
            .write(Reg::AggregateNonFatalTrigger, errors.bits());
    }

    fn hw_config0(&self) -> LocalRegisterCopy<u32, HwConfig0::Register> {
        LocalRegisterCopy::new(self.mmio.read(Reg::HwConfig0))
    }

    fn read_wires(&self, low: Reg, high: Reg) -> u64 {
        u64::from(self.mmio.read(low)) | (u64::from(self.mmio.read(high)) << 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The MCI registers in [`Reg`] as memory, with the access rules of
    /// `mci_reg.rdl`: writes to a read-only register have no effect, a write
    /// to a status register clears the bits that are written as 1, and a
    /// write to a trigger register sets those bits in the status register.
    #[derive(Default)]
    struct FakeMci {
        hw_rev_id: u32,
        hw_config0: u32,
        hw_config1: u32,
        fw_flow_status: u32,
        reset_reason: u32,
        security_state: u32,
        hw_error_fatal: u32,
        agg_error_fatal: u32,
        hw_error_non_fatal: u32,
        agg_error_non_fatal: u32,
        fw_error_fatal: u32,
        fw_error_non_fatal: u32,
        fw_extended_error_info: [u32; EXTENDED_ERROR_INFO_WORDS],
        reset_request: u32,
        generic_input_wires: [u32; 2],
        generic_output_wires: [u32; 2],
        global_interrupt_enable: u32,
        error_enable: u32,
        aggregate_fatal_enable: u32,
        notification_enable: u32,
        aggregate_non_fatal_enable: u32,
        error_status: u32,
        notification_status: u32,
        aggregate_fatal_status: u32,
        aggregate_non_fatal_status: u32,
    }

    impl FakeMci {
        /// Whether the MCI asserts its interrupt to the MCU.
        fn interrupt(&self) -> bool {
            let errors = self.global_interrupt_enable & 0b01 != 0
                && (self.error_status & self.error_enable != 0
                    || self.aggregate_fatal_status & self.aggregate_fatal_enable != 0);
            let notifications = self.global_interrupt_enable & 0b10 != 0
                && (self.notification_status & self.notification_enable != 0
                    || self.aggregate_non_fatal_status & self.aggregate_non_fatal_enable != 0);
            errors || notifications
        }
    }

    impl Mmio for FakeMci {
        fn read(&self, reg: Reg) -> u32 {
            match reg {
                Reg::HwRevId => self.hw_rev_id,
                Reg::HwConfig0 => self.hw_config0,
                Reg::HwConfig1 => self.hw_config1,
                Reg::FwFlowStatus => self.fw_flow_status,
                Reg::ResetReason => self.reset_reason,
                Reg::SecurityState => self.security_state,
                Reg::HwErrorFatal => self.hw_error_fatal,
                Reg::AggErrorFatal => self.agg_error_fatal,
                Reg::HwErrorNonFatal => self.hw_error_non_fatal,
                Reg::AggErrorNonFatal => self.agg_error_non_fatal,
                Reg::FwErrorFatal => self.fw_error_fatal,
                Reg::FwErrorNonFatal => self.fw_error_non_fatal,
                Reg::FwExtendedErrorInfo(index) => self.fw_extended_error_info[usize::from(index)],
                Reg::ResetRequest => self.reset_request,
                Reg::GenericInputWiresLow => self.generic_input_wires[0],
                Reg::GenericInputWiresHigh => self.generic_input_wires[1],
                Reg::GenericOutputWiresLow => self.generic_output_wires[0],
                Reg::GenericOutputWiresHigh => self.generic_output_wires[1],
                Reg::GlobalInterruptEnable => self.global_interrupt_enable,
                Reg::ErrorEnable => self.error_enable,
                Reg::AggregateFatalEnable => self.aggregate_fatal_enable,
                Reg::NotificationEnable => self.notification_enable,
                Reg::AggregateNonFatalEnable => self.aggregate_non_fatal_enable,
                Reg::ErrorStatus => self.error_status,
                Reg::NotificationStatus => self.notification_status,
                Reg::AggregateFatalStatus => self.aggregate_fatal_status,
                Reg::AggregateNonFatalStatus => self.aggregate_non_fatal_status,
                Reg::ErrorTrigger
                | Reg::NotificationTrigger
                | Reg::AggregateFatalTrigger
                | Reg::AggregateNonFatalTrigger => 0,
            }
        }

        fn write(&mut self, reg: Reg, value: u32) {
            match reg {
                Reg::HwRevId
                | Reg::HwConfig0
                | Reg::HwConfig1
                | Reg::SecurityState
                | Reg::GenericInputWiresLow
                | Reg::GenericInputWiresHigh => {}
                Reg::HwErrorFatal => self.hw_error_fatal &= !value,
                Reg::AggErrorFatal => self.agg_error_fatal &= !value,
                Reg::HwErrorNonFatal => self.hw_error_non_fatal &= !value,
                Reg::AggErrorNonFatal => self.agg_error_non_fatal &= !value,
                Reg::FwFlowStatus => self.fw_flow_status = value,
                Reg::ResetReason => self.reset_reason = value,
                Reg::FwErrorFatal => self.fw_error_fatal = value,
                Reg::FwErrorNonFatal => self.fw_error_non_fatal = value,
                Reg::FwExtendedErrorInfo(index) => {
                    self.fw_extended_error_info[usize::from(index)] = value
                }
                Reg::ResetRequest => self.reset_request = value,
                Reg::GenericOutputWiresLow => self.generic_output_wires[0] = value,
                Reg::GenericOutputWiresHigh => self.generic_output_wires[1] = value,
                Reg::GlobalInterruptEnable => self.global_interrupt_enable = value,
                Reg::ErrorEnable => self.error_enable = value,
                Reg::AggregateFatalEnable => self.aggregate_fatal_enable = value,
                Reg::NotificationEnable => self.notification_enable = value,
                Reg::AggregateNonFatalEnable => self.aggregate_non_fatal_enable = value,
                Reg::ErrorStatus => self.error_status &= !value,
                Reg::NotificationStatus => self.notification_status &= !value,
                Reg::AggregateFatalStatus => self.aggregate_fatal_status &= !value,
                Reg::AggregateNonFatalStatus => self.aggregate_non_fatal_status &= !value,
                Reg::ErrorTrigger => self.error_status |= value,
                Reg::NotificationTrigger => self.notification_status |= value,
                Reg::AggregateFatalTrigger => self.aggregate_fatal_status |= value,
                Reg::AggregateNonFatalTrigger => self.aggregate_non_fatal_status |= value,
            }
        }
    }

    fn mci(fake: FakeMci) -> Mci<FakeMci> {
        Mci::with_mmio(fake)
    }

    #[test]
    fn register_offsets_match_the_hardware_map() {
        assert_eq!(Reg::HwRevId.offset(), 0x0c);
        assert_eq!(Reg::HwConfig0.offset(), 0x18);
        assert_eq!(Reg::HwConfig1.offset(), 0x1c);
        assert_eq!(Reg::FwFlowStatus.offset(), 0x30);
        assert_eq!(Reg::ResetReason.offset(), 0x38);
        assert_eq!(Reg::SecurityState.offset(), 0x40);
        assert_eq!(Reg::HwErrorFatal.offset(), 0x50);
        assert_eq!(Reg::AggErrorFatal.offset(), 0x54);
        assert_eq!(Reg::HwErrorNonFatal.offset(), 0x58);
        assert_eq!(Reg::AggErrorNonFatal.offset(), 0x5c);
        assert_eq!(Reg::FwErrorFatal.offset(), 0x60);
        assert_eq!(Reg::FwErrorNonFatal.offset(), 0x64);
        assert_eq!(Reg::FwExtendedErrorInfo(0).offset(), 0x70);
        assert_eq!(Reg::FwExtendedErrorInfo(7).offset(), 0x8c);
        assert_eq!(Reg::ResetRequest.offset(), 0x100);
        assert_eq!(Reg::GenericInputWiresLow.offset(), 0x400);
        assert_eq!(Reg::GenericInputWiresHigh.offset(), 0x404);
        assert_eq!(Reg::GenericOutputWiresLow.offset(), 0x408);
        assert_eq!(Reg::GenericOutputWiresHigh.offset(), 0x40c);
        assert_eq!(Reg::GlobalInterruptEnable.offset(), 0x1000);
        assert_eq!(Reg::ErrorEnable.offset(), 0x1004);
        assert_eq!(Reg::AggregateFatalEnable.offset(), 0x1008);
        assert_eq!(Reg::NotificationEnable.offset(), 0x100c);
        assert_eq!(Reg::AggregateNonFatalEnable.offset(), 0x1010);
        assert_eq!(Reg::ErrorStatus.offset(), 0x101c);
        assert_eq!(Reg::AggregateFatalStatus.offset(), 0x1020);
        assert_eq!(Reg::NotificationStatus.offset(), 0x1024);
        assert_eq!(Reg::AggregateNonFatalStatus.offset(), 0x1028);
        assert_eq!(Reg::ErrorTrigger.offset(), 0x102c);
        assert_eq!(Reg::AggregateFatalTrigger.offset(), 0x1030);
        assert_eq!(Reg::NotificationTrigger.offset(), 0x1034);
        assert_eq!(Reg::AggregateNonFatalTrigger.offset(), 0x1038);
    }

    #[test]
    fn extended_error_info_offset_stays_inside_the_register() {
        assert_eq!(Reg::FwExtendedErrorInfo(8).offset(), 0x70);
        assert_eq!(Reg::FwExtendedErrorInfo(u8::MAX).offset(), 0x8c);
    }

    #[test]
    fn decodes_each_reset_reason() {
        let reason = |reset_reason| {
            mci(FakeMci {
                reset_reason,
                ..Default::default()
            })
            .reset_reason()
        };
        assert_eq!(reason(0), ResetReason::PowerOn);
        assert_eq!(reason(0b001), ResetReason::FirmwareHitlessUpdate);
        assert_eq!(reason(0b010), ResetReason::FirmwareBoot);
        assert_eq!(reason(0b100), ResetReason::WarmReset);
        assert_eq!(reason(0b011), ResetReason::Invalid(0b011));
        assert_eq!(reason(0b110), ResetReason::Invalid(0b110));
        assert_eq!(reason(0b1000), ResetReason::Invalid(0b1000));
    }

    #[test]
    fn requests_an_mcu_reset() {
        let mut mci = mci(FakeMci::default());
        mci.request_mcu_reset();
        assert_eq!(mci.mmio.reset_request, 1);
    }

    #[test]
    fn splits_the_flow_status_into_checkpoint_and_milestones() {
        let mci = mci(FakeMci {
            fw_flow_status: 0x00af_01c5,
            ..Default::default()
        });
        assert_eq!(mci.boot_checkpoint(), 0x01c5);
        assert_eq!(
            mci.milestones(),
            Milestones::ROM_STARTED
                | Milestones::CPTRA_BOOT_GO_ASSERTED
                | Milestones::CPTRA_FUSES_WRITTEN
                | Milestones::RI_DOWNLOAD_COMPLETED
                | Milestones::COLD_BOOT_FLOW_COMPLETE
                | Milestones::FIRMWARE_BOOT_FLOW_COMPLETE
        );
    }

    #[test]
    fn set_milestones_keeps_the_checkpoint_and_earlier_milestones() {
        let mut mci = mci(FakeMci {
            fw_flow_status: 0x00af_01c5,
            ..Default::default()
        });
        mci.set_milestones(Milestones::FIRMWARE_OS_INITIALIZED);
        assert_eq!(mci.mmio.fw_flow_status, 0x01af_01c5);
        mci.set_milestones(Milestones::FIRMWARE_MAILBOX_READY | Milestones::FIRMWARE_PLDM_READY);
        assert_eq!(mci.mmio.fw_flow_status, 0x13af_01c5);
    }

    #[test]
    fn milestones_keeps_bits_it_has_no_name_for() {
        let mci = mci(FakeMci {
            fw_flow_status: 0x8000_0000,
            ..Default::default()
        });
        assert_eq!(mci.milestones().bits(), 0x8000);
    }

    #[test]
    fn reports_firmware_errors() {
        let mut mci = mci(FakeMci::default());
        assert_eq!(mci.fw_error_fatal(), 0);
        assert_eq!(mci.fw_error_non_fatal(), 0);

        mci.set_fw_error_non_fatal(0x1234);
        mci.set_fw_error_fatal(0xdead_0001);
        assert_eq!(mci.fw_error_non_fatal(), 0x1234);
        assert_eq!(mci.fw_error_fatal(), 0xdead_0001);
        assert_eq!(mci.mmio.fw_error_non_fatal, 0x1234);
        assert_eq!(mci.mmio.fw_error_fatal, 0xdead_0001);

        mci.set_fw_error_non_fatal(0);
        assert_eq!(mci.fw_error_non_fatal(), 0);
    }

    #[test]
    fn stores_extended_error_info_in_word_order() {
        let mut mci = mci(FakeMci::default());
        let info = [1, 2, 3, 4, 5, 6, 7, 8];
        mci.set_fw_extended_error_info(&info);
        assert_eq!(mci.mmio.fw_extended_error_info, info);
        assert_eq!(mci.fw_extended_error_info(), info);
    }

    #[test]
    fn decodes_the_security_state() {
        let state = |security_state| {
            mci(FakeMci {
                security_state,
                ..Default::default()
            })
            .security_state()
        };
        assert!(matches!(
            state(0b0000).lifecycle(),
            Some(Lifecycle::DeviceUnprovisioned)
        ));
        assert!(matches!(
            state(0b0001).lifecycle(),
            Some(Lifecycle::DeviceManufacturing)
        ));
        assert!(matches!(
            state(0b0011).lifecycle(),
            Some(Lifecycle::DeviceProduction)
        ));
        assert!(state(0b0010).lifecycle().is_none());

        assert!(!state(0b0011).debug_locked());
        assert!(state(0b0111).debug_locked());
        assert!(!state(0b0111).scan_mode());
        assert!(state(0b1000).scan_mode());
    }

    #[test]
    fn decodes_the_hardware_revision() {
        let mci = mci(FakeMci {
            hw_rev_id: 0x2102,
            ..Default::default()
        });
        assert_eq!(
            mci.hw_revision(),
            HwRevision {
                major: 2,
                minor: 1,
                patch: 2
            }
        );
    }

    #[test]
    fn reads_the_memory_sizes() {
        // 128 KiB for mailbox 0, 4 KiB for mailbox 1 and 512 KiB of MCU
        // SRAM, next to a reset counter width of 31.
        let mci = mci(FakeMci {
            hw_config0: (128 << 12) | 4,
            hw_config1: (512 << 5) | 31,
            ..Default::default()
        });
        assert_eq!(mci.mbox0_sram_size_kib(), 128);
        assert_eq!(mci.mbox1_sram_size_kib(), 4);
        assert_eq!(mci.mcu_sram_size_kib(), 512);
    }

    #[test]
    fn reads_and_drives_the_generic_wires() {
        let mut mci = mci(FakeMci {
            generic_input_wires: [0x0000_0001, 0x2000_0000],
            ..Default::default()
        });
        assert_eq!(mci.generic_input_wires(), 0x2000_0000_0000_0001);

        mci.set_generic_output_wires(0x8000_0000_0000_0002);
        assert_eq!(mci.mmio.generic_output_wires, [0x0000_0002, 0x8000_0000]);
        assert_eq!(mci.generic_output_wires(), 0x8000_0000_0000_0002);
    }

    #[test]
    fn events_have_the_same_bits_in_the_enable_and_trigger_registers() {
        use mci::bits::{Error0IntrEnT, Error0IntrTrigT, Notif0IntrEnT, Notif0IntrTrigT};

        let errors = [
            (
                ErrorEvents::MCU_SRAM_DMI_AXI_COLLISION,
                Error0IntrEnT::ErrorMcuSramDmiAxiCollisionEn::SET.value,
                Error0IntrTrigT::ErrorMcuSramDmiAxiCollisionTrig::SET.value,
            ),
            (
                ErrorEvents::INTERNAL,
                Error0IntrEnT::ErrorInternalEn::SET.value,
                Error0IntrTrigT::ErrorInternalTrig::SET.value,
            ),
            (
                ErrorEvents::MBOX0_ECC_UNCORRECTED,
                Error0IntrEnT::ErrorMbox0EccUncEn::SET.value,
                Error0IntrTrigT::ErrorMbox0EccUncTrig::SET.value,
            ),
            (
                ErrorEvents::MBOX1_ECC_UNCORRECTED,
                Error0IntrEnT::ErrorMbox1EccUncEn::SET.value,
                Error0IntrTrigT::ErrorMbox1EccUncTrig::SET.value,
            ),
            (
                ErrorEvents::WDT_TIMER1_TIMEOUT,
                Error0IntrEnT::ErrorWdtTimer1TimeoutEn::SET.value,
                Error0IntrTrigT::ErrorWdtTimer1TimeoutTrig::SET.value,
            ),
            (
                ErrorEvents::WDT_TIMER2_TIMEOUT,
                Error0IntrEnT::ErrorWdtTimer2TimeoutEn::SET.value,
                Error0IntrTrigT::ErrorWdtTimer2TimeoutTrig::SET.value,
            ),
        ];
        for (event, enable, trigger) in errors {
            assert_eq!(event.bits(), enable, "{event:?}");
            assert_eq!(event.bits(), trigger, "{event:?}");
        }
        assert_eq!(ErrorEvents::all().bits(), 0x3f);

        let notifications = [
            (
                Notifications::MCU_SRAM_ECC_CORRECTED,
                Notif0IntrEnT::NotifMcuSramEccCorEn::SET.value,
                Notif0IntrTrigT::NotifMcuSramEccCorTrig::SET.value,
            ),
            (
                Notifications::CPTRA_MCU_RESET_REQUEST,
                Notif0IntrEnT::NotifCptraMcuResetReqEn::SET.value,
                Notif0IntrTrigT::NotifCptraMcuResetReqTrig::SET.value,
            ),
            (
                Notifications::GENERIC_INPUT_TOGGLE,
                Notif0IntrEnT::NotifGenInToggleEn::SET.value,
                Notif0IntrTrigT::NotifGenInToggleTrig::SET.value,
            ),
            (
                Notifications::MBOX0_TARGET_DONE,
                Notif0IntrEnT::NotifMbox0TargetDoneEn::SET.value,
                Notif0IntrTrigT::NotifMbox0TargetDoneTrig::SET.value,
            ),
            (
                Notifications::MBOX1_TARGET_DONE,
                Notif0IntrEnT::NotifMbox1TargetDoneEn::SET.value,
                Notif0IntrTrigT::NotifMbox1TargetDoneTrig::SET.value,
            ),
            (
                Notifications::MBOX0_COMMAND_AVAILABLE,
                Notif0IntrEnT::NotifMbox0CmdAvailEn::SET.value,
                Notif0IntrTrigT::NotifMbox0CmdAvailTrig::SET.value,
            ),
            (
                Notifications::MBOX1_COMMAND_AVAILABLE,
                Notif0IntrEnT::NotifMbox1CmdAvailEn::SET.value,
                Notif0IntrTrigT::NotifMbox1CmdAvailTrig::SET.value,
            ),
            (
                Notifications::CPTRA_MBOX_COMMAND_AVAILABLE,
                Notif0IntrEnT::NotifCptraMboxCmdAvailEn::SET.value,
                Notif0IntrTrigT::NotifCptraMboxCmdAvailTrig::SET.value,
            ),
            (
                Notifications::MBOX0_ECC_CORRECTED,
                Notif0IntrEnT::NotifMbox0EccCorEn::SET.value,
                Notif0IntrTrigT::NotifMbox0EccCorTrig::SET.value,
            ),
            (
                Notifications::MBOX1_ECC_CORRECTED,
                Notif0IntrEnT::NotifMbox1EccCorEn::SET.value,
                Notif0IntrTrigT::NotifMbox1EccCorTrig::SET.value,
            ),
            (
                Notifications::DEBUG_LOCKED,
                Notif0IntrEnT::NotifDebugLockedEn::SET.value,
                Notif0IntrTrigT::NotifDebugLockedTrig::SET.value,
            ),
            (
                Notifications::SCAN_MODE,
                Notif0IntrEnT::NotifScanModeEn::SET.value,
                Notif0IntrTrigT::NotifScanModeTrig::SET.value,
            ),
            (
                Notifications::MBOX0_SOC_REQUESTED_LOCK,
                Notif0IntrEnT::NotifMbox0SocReqLockEn::SET.value,
                Notif0IntrTrigT::NotifMbox0SocReqLockTrig::SET.value,
            ),
            (
                Notifications::MBOX1_SOC_REQUESTED_LOCK,
                Notif0IntrEnT::NotifMbox1SocReqLockEn::SET.value,
                Notif0IntrTrigT::NotifMbox1SocReqLockTrig::SET.value,
            ),
            (
                Notifications::OTP_OPERATION_DONE,
                Notif0IntrEnT::NotifOtpOperationDoneEn::SET.value,
                Notif0IntrTrigT::NotifOtpOperationDoneTrig::SET.value,
            ),
        ];
        for (notification, enable, trigger) in notifications {
            assert_eq!(notification.bits(), enable, "{notification:?}");
            assert_eq!(notification.bits(), trigger, "{notification:?}");
        }
        assert_eq!(Notifications::all().bits(), 0x7fff);
    }

    #[test]
    fn enable_interrupts_enables_only_the_given_events() {
        // Interrupt enables that an earlier stage left set.
        let mut mci = mci(FakeMci {
            global_interrupt_enable: 0b10,
            error_enable: 0x3f,
            aggregate_fatal_enable: 0xffff_ffff,
            notification_enable: 0x7fff,
            aggregate_non_fatal_enable: 0xffff_ffff,
            ..Default::default()
        });
        mci.enable_interrupts(Events {
            errors: ErrorEvents::MBOX0_ECC_UNCORRECTED,
            notifications: Notifications::CPTRA_MCU_RESET_REQUEST
                | Notifications::MBOX0_COMMAND_AVAILABLE,
            ..Events::empty()
        });
        assert_eq!(mci.mmio.global_interrupt_enable, 0b11);
        assert_eq!(mci.mmio.error_enable, 1 << 2);
        assert_eq!(mci.mmio.aggregate_fatal_enable, 0);
        assert_eq!(mci.mmio.notification_enable, (1 << 1) | (1 << 5));
        assert_eq!(mci.mmio.aggregate_non_fatal_enable, 0);

        mci.enable_interrupts(Events {
            aggregated_fatal: AggregatedErrors::CALIPTRA | AggregatedErrors::MCU,
            aggregated_non_fatal: AggregatedErrors::I3C,
            ..Events::empty()
        });
        assert_eq!(mci.mmio.error_enable, 0);
        assert_eq!(mci.mmio.aggregate_fatal_enable, 0xfff);
        assert_eq!(mci.mmio.notification_enable, 0);
        assert_eq!(mci.mmio.aggregate_non_fatal_enable, 0x3f00_0000);

        mci.enable_interrupts(Events::empty());
        assert_eq!(mci.mmio.aggregate_fatal_enable, 0);
        assert_eq!(mci.mmio.aggregate_non_fatal_enable, 0);
    }

    #[test]
    fn the_sources_of_aggregated_errors_cover_every_input_once() {
        let sources = [
            AggregatedErrors::CALIPTRA,
            AggregatedErrors::MCU,
            AggregatedErrors::LIFECYCLE_CONTROLLER,
            AggregatedErrors::FUSE_CONTROLLER,
            AggregatedErrors::I3C,
            AggregatedErrors::SPARE,
        ];
        let mut seen = AggregatedErrors::empty();
        for source in sources {
            assert!(!seen.intersects(source), "{source:?}");
            seen |= source;
        }
        assert_eq!(seen.bits(), u32::MAX);
        // The first input of each source in `caliptra_ss_top.sv`.
        assert!(AggregatedErrors::CALIPTRA.bits() & (1 << 0) != 0);
        assert!(AggregatedErrors::MCU.bits() & (1 << 6) != 0);
        assert!(AggregatedErrors::LIFECYCLE_CONTROLLER.bits() & (1 << 12) != 0);
        assert!(AggregatedErrors::FUSE_CONTROLLER.bits() & (1 << 18) != 0);
        assert!(AggregatedErrors::I3C.bits() & (1 << 24) != 0);
    }

    #[test]
    fn aggregated_errors_assert_the_interrupt_with_their_group_enable() {
        let spare = AggregatedErrors::from_bits_retain(1 << 30);
        let mut mci = mci(FakeMci::default());
        mci.enable_interrupts(Events {
            aggregated_fatal: spare,
            aggregated_non_fatal: spare,
            ..Events::empty()
        });
        mci.trigger_aggregated_fatal_errors(AggregatedErrors::CALIPTRA);
        mci.trigger_aggregated_non_fatal_errors(AggregatedErrors::MCU);
        assert!(!mci.mmio.interrupt());
        assert_eq!(
            mci.pending_aggregated_fatal_errors(),
            AggregatedErrors::CALIPTRA
        );
        assert_eq!(
            mci.pending_aggregated_non_fatal_errors(),
            AggregatedErrors::MCU
        );

        mci.trigger_aggregated_fatal_errors(spare);
        assert!(mci.mmio.interrupt());
        mci.acknowledge_aggregated_fatal_errors(spare);
        assert!(!mci.mmio.interrupt());
        assert_eq!(
            mci.pending_aggregated_fatal_errors(),
            AggregatedErrors::CALIPTRA
        );

        mci.trigger_aggregated_non_fatal_errors(spare);
        assert!(mci.mmio.interrupt());
        mci.acknowledge_aggregated_non_fatal_errors(spare);
        assert!(!mci.mmio.interrupt());
        assert_eq!(
            mci.pending_aggregated_non_fatal_errors(),
            AggregatedErrors::MCU
        );
    }

    #[test]
    fn reads_the_recorded_errors() {
        let mci = mci(FakeMci {
            hw_error_fatal: 0b110,
            agg_error_fatal: 1 << 12,
            hw_error_non_fatal: 0b01,
            agg_error_non_fatal: 1 << 24,
            ..Default::default()
        });
        assert_eq!(
            mci.recorded_hw_fatal_errors(),
            HwFatalErrors::NMI | HwFatalErrors::MCU_SRAM_DMI_AXI_COLLISION
        );
        assert_eq!(
            mci.recorded_hw_non_fatal_errors(),
            HwNonFatalErrors::MBOX0_ECC_UNCORRECTED
        );
        assert_eq!(mci.recorded_aggregated_fatal_errors().bits(), 1 << 12);
        assert_eq!(mci.recorded_aggregated_non_fatal_errors().bits(), 1 << 24);
    }

    #[test]
    fn only_enabled_pending_events_assert_the_interrupt() {
        let mut mci = mci(FakeMci::default());
        mci.enable_interrupts(Events {
            errors: ErrorEvents::INTERNAL,
            notifications: Notifications::GENERIC_INPUT_TOGGLE,
            ..Events::empty()
        });
        assert!(!mci.mmio.interrupt());

        mci.trigger_errors(ErrorEvents::WDT_TIMER2_TIMEOUT);
        mci.trigger_notifications(Notifications::SCAN_MODE);
        assert!(!mci.mmio.interrupt());
        assert_eq!(mci.pending_errors(), ErrorEvents::WDT_TIMER2_TIMEOUT);
        assert_eq!(mci.pending_notifications(), Notifications::SCAN_MODE);

        mci.trigger_notifications(Notifications::GENERIC_INPUT_TOGGLE);
        assert!(mci.mmio.interrupt());
        mci.acknowledge_notifications(Notifications::GENERIC_INPUT_TOGGLE);
        assert!(!mci.mmio.interrupt());

        mci.trigger_errors(ErrorEvents::INTERNAL);
        assert!(mci.mmio.interrupt());
        mci.acknowledge_errors(ErrorEvents::INTERNAL);
        assert!(!mci.mmio.interrupt());
    }

    #[test]
    fn a_pending_event_asserts_the_interrupt_when_it_is_enabled() {
        let mut mci = mci(FakeMci::default());
        mci.trigger_notifications(Notifications::CPTRA_MCU_RESET_REQUEST);
        assert!(!mci.mmio.interrupt());
        mci.enable_interrupts(Events {
            notifications: Notifications::CPTRA_MCU_RESET_REQUEST,
            ..Events::empty()
        });
        assert!(mci.mmio.interrupt());
    }

    #[test]
    fn acknowledge_clears_only_the_given_events() {
        let mut mci = mci(FakeMci {
            error_status: 0b11_0100,
            notification_status: 0b0100_0010_0010,
            ..Default::default()
        });
        assert_eq!(
            mci.pending_errors(),
            ErrorEvents::MBOX0_ECC_UNCORRECTED
                | ErrorEvents::WDT_TIMER1_TIMEOUT
                | ErrorEvents::WDT_TIMER2_TIMEOUT
        );
        assert_eq!(
            mci.pending_notifications(),
            Notifications::CPTRA_MCU_RESET_REQUEST
                | Notifications::MBOX0_COMMAND_AVAILABLE
                | Notifications::DEBUG_LOCKED
        );

        mci.acknowledge_errors(ErrorEvents::MBOX0_ECC_UNCORRECTED);
        assert_eq!(mci.mmio.error_status, 0b11_0000);
        mci.acknowledge_notifications(Notifications::CPTRA_MCU_RESET_REQUEST);
        assert_eq!(mci.mmio.notification_status, 0b0100_0010_0000);

        // An event that is not pending.
        mci.acknowledge_notifications(Notifications::SCAN_MODE);
        assert_eq!(mci.mmio.notification_status, 0b0100_0010_0000);
    }

    #[test]
    fn pending_keeps_bits_it_has_no_name_for() {
        let mci = mci(FakeMci {
            error_status: 1 << 31,
            notification_status: 1 << 31,
            ..Default::default()
        });
        assert_eq!(mci.pending_errors().bits(), 1 << 31);
        assert_eq!(mci.pending_notifications().bits(), 1 << 31);
    }
}
