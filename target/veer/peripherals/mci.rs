// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for the control and status registers of the Caliptra-SS MCI block.
//!
//! The driver covers the reason for the last MCU reset, the request for an
//! MCU reset, the firmware flow status, the firmware error registers, the
//! security state, the hardware revision and configuration, and the generic
//! wires. The watchdog registers have their own driver, `veer_wdt`. The
//! driver does not use the interrupt registers.
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
use mci::bits::{HwConfig0, HwConfig1, HwRevId, ResetRequest};
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
    /// [`Reg`].
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
    /// caller, and no other code may write the registers in [`Reg`].
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
    /// `mci_reg.rdl`: writes to a read-only register have no effect.
    #[derive(Default)]
    struct FakeMci {
        hw_rev_id: u32,
        hw_config0: u32,
        hw_config1: u32,
        fw_flow_status: u32,
        reset_reason: u32,
        security_state: u32,
        fw_error_fatal: u32,
        fw_error_non_fatal: u32,
        fw_extended_error_info: [u32; EXTENDED_ERROR_INFO_WORDS],
        reset_request: u32,
        generic_input_wires: [u32; 2],
        generic_output_wires: [u32; 2],
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
                Reg::FwErrorFatal => self.fw_error_fatal,
                Reg::FwErrorNonFatal => self.fw_error_non_fatal,
                Reg::FwExtendedErrorInfo(index) => self.fw_extended_error_info[usize::from(index)],
                Reg::ResetRequest => self.reset_request,
                Reg::GenericInputWiresLow => self.generic_input_wires[0],
                Reg::GenericInputWiresHigh => self.generic_input_wires[1],
                Reg::GenericOutputWiresLow => self.generic_output_wires[0],
                Reg::GenericOutputWiresHigh => self.generic_output_wires[1],
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
        assert_eq!(Reg::FwErrorFatal.offset(), 0x60);
        assert_eq!(Reg::FwErrorNonFatal.offset(), 0x64);
        assert_eq!(Reg::FwExtendedErrorInfo(0).offset(), 0x70);
        assert_eq!(Reg::FwExtendedErrorInfo(7).offset(), 0x8c);
        assert_eq!(Reg::ResetRequest.offset(), 0x100);
        assert_eq!(Reg::GenericInputWiresLow.offset(), 0x400);
        assert_eq!(Reg::GenericInputWiresHigh.offset(), 0x404);
        assert_eq!(Reg::GenericOutputWiresLow.offset(), 0x408);
        assert_eq!(Reg::GenericOutputWiresHigh.offset(), 0x40c);
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
}
