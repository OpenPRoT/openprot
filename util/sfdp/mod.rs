// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(test), no_std)]

const _: () = assert!(cfg!(target_endian = "little"));

use bitfield_struct::bitfield;
use core::mem::offset_of;
use util_error as error;
use util_error::ErrorCode;
use util_io::RandomRead;
use util_types::{MultiplyDuration, Nanoseconds};
use zerocopy::FromBytes;
use zerocopy::FromZeros;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
#[repr(transparent)]
pub struct SfdpSignature(pub u32);
impl SfdpSignature {
    pub const EXPECTED_VALUE: Self = Self(0x50444653);

    pub const fn new() -> Self {
        Self::EXPECTED_VALUE
    }
    pub fn is_valid(self) -> bool {
        self == Self::EXPECTED_VALUE
    }
}
impl Default for SfdpSignature {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
#[repr(transparent)]
pub struct AccessProtocol(pub u8);
impl AccessProtocol {
    pub const XSPI_NAND_CLASS2: Self = Self(241);
    pub const SPI_NAND_CLASS_1: Self = Self(244);
    pub const SPI_NAND_CLASS_2: Self = Self(245);
    pub const XSPI_NOR_PROFILE_2_5B: Self = Self(250);
    pub const XSPI_NOR_PROFILE_1_OCTAL_3B_8W: Self = Self(252);
    pub const XSPI_NOR_PROFILE_1_OCTAL_4B_20W: Self = Self(253);
    pub const XSPI_NOR_PROFILE_1_OCTAL_4B_8W: Self = Self(254);
    pub const LEGACY: Self = Self(255);
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
#[repr(C)]
pub struct SfdpHeader {
    pub sig: SfdpSignature,
    pub minor_rev: u8,
    pub major_rev: u8,
    pub num_parameter_header: u8,
    pub access_protocol: AccessProtocol,
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct U24([u8; 3]);
impl U24 {
    pub const fn new(val: u32) -> Self {
        let [b0, b1, b2, _] = val.to_le_bytes();
        Self([b0, b1, b2])
    }
    pub const fn as_usize(&self) -> usize {
        const { assert!(size_of::<usize>() >= size_of::<u32>()) }
        self.as_u32() as usize
    }
    pub const fn as_u32(&self) -> u32 {
        let [b0, b1, b2] = self.0;
        u32::from_le_bytes([b0, b1, b2, 0])
    }
}
impl core::fmt::Debug for U24 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_u32(), f)
    }
}
impl core::fmt::Display for U24 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.as_u32(), f)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
#[repr(C)]
pub struct ParameterHeader {
    pub parameter_id_lsb: u8,
    pub minor_rev: u8,
    pub major_rev: u8,
    pub len_in_dwords: u8,
    pub ptr: U24,
    pub parameter_id_msb: u8,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct FunctionSpecificParameterTableIdAssignments(pub u16);
impl FunctionSpecificParameterTableIdAssignments {
    pub const BASIC_FLASH: Self = Self(0xFF00);
    pub const _4B_ADDRESS_INSTRUCTION_TABLE: Self = Self(0xFF84);
}

impl ParameterHeader {
    pub fn parameter_id(&self) -> FunctionSpecificParameterTableIdAssignments {
        let val = ((self.parameter_id_msb as u16) << 8) | self.parameter_id_lsb as u16;
        FunctionSpecificParameterTableIdAssignments(val)
    }
}

#[derive(Debug)]
pub enum LegacyEraseSizes {
    Reserved0 = 0,
    Erase4k = 1,
    Reserved2 = 2,
    Erase4kNotAvailable = 3,
}
impl LegacyEraseSizes {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Reserved0,
            0b01 => Self::Erase4k,
            0b10 => Self::Reserved2,
            _ => Self::Erase4kNotAvailable,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug)]
pub enum LegacyWriteGranularity {
    SingleByte = 0,
    Buffer64 = 1,
}
impl LegacyWriteGranularity {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0x1 {
            0 => Self::SingleByte,
            _ => Self::Buffer64,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug)]
pub enum AddressBytes {
    _3ByteOnly = 0,
    _3Or4Byte = 1,
    _4ByteOnly = 2,
    Reserved3 = 3,
}
impl AddressBytes {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::_3ByteOnly,
            0b01 => Self::_3Or4Byte,
            0b10 => Self::_4ByteOnly,
            _ => Self::Reserved3,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord1 {
    #[bits(2)]
    pub legacy_erase_sizes: LegacyEraseSizes,

    #[bits(1)]
    pub legacy_write_granularity: LegacyWriteGranularity,

    #[bits(1)]
    pub block_protect_is_volatile: bool,

    #[bits(1)]
    pub status_write_requires_write_enable: bool,

    #[bits(3)]
    __: u8,

    pub erase4k_instr: u8,

    pub supports_1s_1s_2s_read: bool,

    #[bits(2)]
    pub addr_bytes: AddressBytes,

    pub dtr_clocking_supported: bool,

    pub supports_1s_2s_2s_read: bool,

    pub supports_1s_4s_4s_read: bool,

    pub supports_1s_1s_4s_read: bool,

    #[bits(9)]
    __: u16,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord3 {
    #[bits(5)]
    pub fast_read_1s_4s_4s_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_4s_4s_mode_clocks: u8,

    pub fast_read_1s_4s_4s_instr: u8,

    #[bits(5)]
    pub fast_read_1s_1s_4s_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_1s_4s_mode_clocks: u8,

    pub fast_read_1s_1s_4s_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord4 {
    #[bits(5)]
    pub fast_read_1s_1s_2s_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_1s_2s_mode_clocks: u8,

    pub fast_read_1s_1s_2s_instr: u8,

    #[bits(5)]
    pub fast_read_1s_2s_2s_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_2s_2s_mode_clocks: u8,

    pub fast_read_1s_2s_2s_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord5 {
    pub fast_read_2x_2s_2s_supported: bool,

    #[bits(3)]
    __: u8,

    pub fast_read_4x_4s_4s_supported: bool,

    #[bits(27)]
    __: u32,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord6 {
    #[bits(16)]
    __: u16,

    #[bits(5)]
    pub fast_read_2s_2s_2s_wait_states: u8,

    #[bits(3)]
    pub fast_read_2s_2s_2s_mode_clocks: u8,

    pub fast_read_2s_2s_2s_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord7 {
    #[bits(16)]
    __: u16,

    #[bits(5)]
    pub fast_read_4s_4s_4s_wait_states: u8,

    #[bits(3)]
    pub fast_read_4s_4s_4s_mode_clocks: u8,

    pub fast_read_4s_4s_4s_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord8 {
    #[bits(8)]
    pub erase_type_1_size: PowerOf2,

    pub erase_type_1_instr: u8,

    #[bits(8)]
    pub erase_type_2_size: PowerOf2,

    pub erase_type_2_instr: u8,
}

#[bitfield(u8)]
#[derive(Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct PowerOf2 {
    pub n: u8,
}
impl PowerOf2 {
    pub fn value(&self) -> Option<usize> {
        let n = self.n();
        if n < (usize::BITS as u8) {
            Some(1 << n)
        } else {
            None
        }
    }
}

#[bitfield(u8)]
#[derive(Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct SmallPowerOf2 {
    #[bits(4)]
    pub n: u8,
    #[bits(4)]
    __: u8,
}
impl SmallPowerOf2 {
    pub fn value(&self) -> usize {
        1 << self.n()
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord9 {
    #[bits(8)]
    pub erase_type_3_size: PowerOf2,

    pub erase_type_3_instr: u8,

    #[bits(8)]
    pub erase_type_4_size: PowerOf2,

    pub erase_type_4_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord10 {
    #[bits(4)]
    pub multiplier_max_erase_time: u8,

    #[bits(7)]
    pub erase_type_1_time: EraseTime,
    #[bits(7)]
    pub erase_type_2_time: EraseTime,
    #[bits(7)]
    pub erase_type_3_time: EraseTime,
    #[bits(7)]
    pub erase_type_4_time: EraseTime,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct EraseTime {
    #[bits(5)]
    pub count: u8,

    #[bits(2)]
    pub units: EraseTimeUnit,

    __: bool,
}

impl From<EraseTime> for Nanoseconds {
    fn from(value: EraseTime) -> Self {
        Nanoseconds::from(value.units()).mul(value.count() as i64 + 1)
    }
}

#[cfg(test)]
mod duration_tests {
    use super::*;

    #[test]
    fn test_duration() {
        let et = EraseTime::from_bits(0b1000010);
        assert_eq!(Nanoseconds::from(et), Nanoseconds::from_millis(384));
    }
}

#[derive(Debug)]
pub enum EraseTimeUnit {
    _1ms = 0b00,
    _16ms = 0b01,
    _128ms = 0b10,
    _1s = 0b11,
}

impl EraseTimeUnit {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::_1ms,
            0b01 => Self::_16ms,
            0b10 => Self::_128ms,
            _ => Self::_1s,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

impl From<EraseTimeUnit> for Nanoseconds {
    fn from(value: EraseTimeUnit) -> Self {
        match value {
            EraseTimeUnit::_1ms => Nanoseconds::from_millis(1),
            EraseTimeUnit::_16ms => Nanoseconds::from_millis(16),
            EraseTimeUnit::_128ms => Nanoseconds::from_millis(128),
            EraseTimeUnit::_1s => Nanoseconds::from_secs(1),
        }
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord11 {
    #[bits(4)]
    pub multiplier_max_page_program: u8,

    #[bits(4)]
    pub page_size: SmallPowerOf2,

    #[bits(6)]
    pub page_program_time: PageProgramTime,

    #[bits(5)]
    pub byte_program_first_time: ByteProgramTime,

    #[bits(5)]
    pub byte_program_additional_time: ByteProgramTime,

    #[bits(7)]
    pub chip_erase_time: ChipEraseTime,

    __: bool,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct PageProgramTime {
    #[bits(5)]
    pub count: u8,

    #[bits(1)]
    pub units: PageProgramTimeUnit,

    #[bits(2)]
    __: u8,
}

impl From<PageProgramTime> for Nanoseconds {
    fn from(value: PageProgramTime) -> Self {
        Nanoseconds::from(value.units()).mul(value.count() as i64 + 1)
    }
}

#[derive(Debug)]
pub enum PageProgramTimeUnit {
    _8us = 0b0,
    _64us = 0b1,
}

impl PageProgramTimeUnit {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b1 {
            0b0 => Self::_8us,
            _ => Self::_64us,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

impl From<PageProgramTimeUnit> for Nanoseconds {
    fn from(value: PageProgramTimeUnit) -> Self {
        match value {
            PageProgramTimeUnit::_8us => Nanoseconds::from_micros(8),
            PageProgramTimeUnit::_64us => Nanoseconds::from_micros(64),
        }
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ByteProgramTime {
    #[bits(4)]
    pub count: u8,
    #[bits(1)]
    pub unit: ByteProgramTimeUnit,
    #[bits(3)]
    __: u8,
}

impl From<ByteProgramTime> for Nanoseconds {
    fn from(value: ByteProgramTime) -> Self {
        Nanoseconds::from(value.unit()).mul(value.count() as i64 + 1)
    }
}

#[derive(Debug)]
pub enum ByteProgramTimeUnit {
    _1us = 0b0,
    _8us = 0b1,
}

impl ByteProgramTimeUnit {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b1 {
            0b0 => Self::_1us,
            _ => Self::_8us,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

impl From<ByteProgramTimeUnit> for Nanoseconds {
    fn from(value: ByteProgramTimeUnit) -> Self {
        match value {
            ByteProgramTimeUnit::_1us => Nanoseconds::from_micros(1),
            ByteProgramTimeUnit::_8us => Nanoseconds::from_micros(8),
        }
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ChipEraseTime {
    #[bits(4)]
    pub count: u8,
    #[bits(2)]
    pub units: ChipEraseTimeUnit,
    #[bits(2)]
    __: u8,
}

impl From<ChipEraseTime> for Nanoseconds {
    fn from(value: ChipEraseTime) -> Self {
        Nanoseconds::from(value.units()).mul(value.count() as i64 + 1)
    }
}

#[derive(Debug)]
pub enum ChipEraseTimeUnit {
    _16ms = 0b00,
    _256ms = 0b01,
    _4s = 0b10,
    _64s = 0b11,
}

impl ChipEraseTimeUnit {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::_16ms,
            0b01 => Self::_256ms,
            0b10 => Self::_4s,
            _ => Self::_64s,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

impl From<ChipEraseTimeUnit> for Nanoseconds {
    fn from(value: ChipEraseTimeUnit) -> Self {
        match value {
            ChipEraseTimeUnit::_16ms => Nanoseconds::from_millis(16),
            ChipEraseTimeUnit::_256ms => Nanoseconds::from_millis(256),
            ChipEraseTimeUnit::_4s => Nanoseconds::from_secs(4),
            ChipEraseTimeUnit::_64s => Nanoseconds::from_secs(64),
        }
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord12 {
    #[bits(4)]
    pub prohibited_ops_during_program_suspend: ProhibitedOpsDuringProgramSuspend,

    #[bits(4)]
    pub prohibited_ops_during_erase_suspend: ProhibitedOpsDuringEraseSuspend,

    __: bool,

    #[bits(4)]
    pub program_resume_suspend_interval: Interval64us,

    #[bits(7)]
    pub suspend_in_progress_program_max_latency: DurationEnumA,

    #[bits(4)]
    pub erase_resume_suspend_interval: Interval64us,

    #[bits(7)]
    pub suspend_in_progress_erase_max_latency: DurationEnumA,

    pub suspend_resume_supported: bool,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ProhibitedOpsDuringProgramSuspend {
    pub erase_nesting_permitted: bool,
    pub program_nesting_permitted: bool,
    pub no_read: bool,
    pub erase_and_program_restrictions_are_sufficient: bool,
    #[bits(4)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ProhibitedOpsDuringEraseSuspend {
    pub erase_nesting_permitted: bool,
    pub program_nesting_permitted: bool,
    pub no_read: bool,
    pub erase_and_program_restrictions_are_sufficient: bool,
    #[bits(4)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Interval64us {
    #[bits(4)]
    pub counts_64ms: u8,
    #[bits(4)]
    __: u8,
}

impl From<Interval64us> for Nanoseconds {
    fn from(value: Interval64us) -> Self {
        Nanoseconds::from_millis(64).mul(value.counts_64ms() as i64)
    }
}

/// This duration type is used in several places
#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct DurationEnumA {
    #[bits(5)]
    pub count: u8,

    #[bits(2)]
    pub units: DurationEnumAUnit,

    __: bool,
}

#[derive(Debug)]
pub enum DurationEnumAUnit {
    _128ns = 0b00,
    _1us = 0b01,
    _8us = 0b10,
    _64us = 0b11,
}

impl DurationEnumAUnit {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::_128ns,
            0b01 => Self::_1us,
            0b10 => Self::_8us,
            _ => Self::_64us,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

impl From<DurationEnumAUnit> for Nanoseconds {
    fn from(value: DurationEnumAUnit) -> Self {
        match value {
            DurationEnumAUnit::_128ns => Nanoseconds::from_nanos(128),
            DurationEnumAUnit::_1us => Nanoseconds::from_micros(1),
            DurationEnumAUnit::_8us => Nanoseconds::from_micros(8),
            DurationEnumAUnit::_64us => Nanoseconds::from_micros(64),
        }
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord13 {
    pub program_resume_instr: u8,

    pub program_suspend_instr: u8,

    pub resume_instr: u8,

    pub suspend_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord14 {
    #[bits(2)]
    __: u8,

    #[bits(6)]
    pub status_register_polling_device_busy: StatusRegisterPollingDeviceBusy,

    #[bits(7)]
    pub exit_deep_powerdown_next_op_delay: DurationEnumA,

    pub exit_deep_powerdown_instr: u8,

    pub enter_deep_powerdown_instr: u8,

    pub deep_powerdown_supported: bool,
}

#[bitfield(u8)]
pub struct StatusRegisterPollingDeviceBusy {
    pub legacy_polling_supported: bool,

    pub bit_7_polled_any_time: bool,

    #[bits(6)]
    __: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord15 {
    #[bits(4)]
    pub mode_disable_sequence_4s_4s_4s: ModeDisableSequence4S4S4S,

    #[bits(5)]
    pub mode_enable_sequence_4s_4s_4s: ModeEnableSequence4S4S4S,

    pub mode_0_4_4_supported: bool,

    #[bits(6)]
    pub mode_exit_method_0_4_4: ModeExitMethod044,

    #[bits(4)]
    pub mode_entry_method_0_4_4: ModeEntryMethod044,

    #[bits(3)]
    pub quad_enable_requirements: QuadEnableRequirements,

    pub hold_or_reset_disable: bool,

    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ModeDisableSequence4S4S4S {
    pub issue_ff_instr: bool,
    pub issue_f5_instr: bool,
    pub read_mod_write_seq: bool,
    pub issue_soft_reset: bool,
    #[bits(4)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ModeEnableSequence4S4S4S {
    pub set_qe: bool,
    pub issue_38_instr: bool,
    pub issue_35_instr: bool,
    pub read_mod_write_seq1: bool,
    pub read_mod_write_seq2: bool,
    #[bits(3)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ModeExitMethod044 {
    pub mod_bits_00: bool,
    pub input_io1: bool,
    __: bool,
    pub input_io2: bool,
    pub mod_bits_not_ax: bool,
    #[bits(3)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct ModeEntryMethod044 {
    pub mod_bits_a5: bool,
    pub instr_85: bool,
    pub mod_bits_ax: bool,
    #[bits(5)]
    __: u8,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum QuadEnableRequirements {
    NoQeBit = 0b000,
    QeBit1SR2A = 0b001,
    QeBit6SR1 = 0b010,
    QeBit7SR2B = 0b011,
    QeBit1SR2C = 0b100,
    QeBit1SR2D = 0b101,
    QeBit1SR2E = 0b110,
    Reserved,
}

impl QuadEnableRequirements {
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b111 {
            0b000 => Self::NoQeBit,
            0b001 => Self::QeBit1SR2A,
            0b010 => Self::QeBit6SR1,
            0b011 => Self::QeBit7SR2B,
            0b100 => Self::QeBit1SR2C,
            0b101 => Self::QeBit1SR2D,
            0b110 => Self::QeBit1SR2E,
            _ => Self::Reserved,
        }
    }
    pub const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord16 {
    #[bits(7)]
    pub volatile_non_register_write_enable_instr: VolatileNonRegisterWriteEnableInstr,

    __: bool,

    #[bits(6)]
    pub soft_reset_rescue_seq_support: SoftResetRescureSeqSupport,

    #[bits(10)]
    pub exit_4_b_addressing: Exit4BAddressing,

    #[bits(8)]
    pub enter_4_b_addressing: Enter4BAddressing,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct VolatileNonRegisterWriteEnableInstr {
    pub non_volatile_reg1_last_written_we06: bool,
    pub volatile_reg1_last_written_we06: bool,
    pub volatile_reg1_last_written_we50: bool,
    pub volatile_non_reg1_last_written_we0650: bool,
    pub mix_volatile_we06: bool,
    #[bits(3)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct SoftResetRescureSeqSupport {
    pub drive0_f8_clk: bool,
    pub drive0_f10_clk: bool,
    pub drive0_f16_clk: bool,
    pub instr_f0: bool,
    pub instr66_then99: bool,
    pub exit044_first: bool,
    #[bits(2)]
    __: u8,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Exit4BAddressing {
    pub instr_e9: bool,
    pub instr_06_then_e9: bool,
    pub ear: bool,
    pub bank_reg: bool,
    pub conf_reg: bool,
    pub hardware_reset: bool,
    pub software_reset: bool,
    pub power_cycle: bool,
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Enter4BAddressing {
    pub intr_b7: bool,
    pub instr_06_then_b7: bool,
    pub ear: bool,
    pub bank_reg: bool,
    pub conf_reg: bool,
    pub dedicated_add: bool,
    pub always_4b: bool,
    __: bool,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord17 {
    #[bits(5)]
    pub fast_read_wait_states_1s_8s_8s: u8,

    #[bits(3)]
    pub fast_read_mode_clocks_1s_8s_8s: u8,

    pub fast_read_instr_1s_8s_8s: u8,

    #[bits(5)]
    pub fast_read_wait_states_1s_1s_8s: u8,

    #[bits(3)]
    pub fast_read_mode_clocks_1s_1s_8s: u8,

    pub fast_read_instr_1s_1s_8s: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord18 {
    #[bits(18)]
    __: u32,

    #[bits(5)]
    pub output_driver_str: OuputDriverStr,

    pub jedec_spi_protocol_reset_implemented: bool,

    #[bits(2)]
    pub data_strobe_waveform_str: DataStrobeWaveformStr,

    pub data_strobe_support_qpi_str_4s_4s_4s: bool,

    pub data_strobe_support_qpi_dtr_4s_4d_4d: bool,

    __: bool,

    #[bits(2)]
    pub octal_dtr_8d_8d_8d_cmd: OctalDtrCmd,

    #[bits(1)]
    pub octal_dtr_8d_8d_8d_mode: ByteOrder8D8D8D,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct OuputDriverStr {
    pub type0_supported: bool,
    pub type1_supported: bool,
    pub type2_supported: bool,
    pub type3_supported: bool,
    pub type4_supported: bool,
    #[bits(3)]
    __: u8,
}

#[derive(Debug)]
pub enum DataStrobeWaveformStr {
    Reserved = 0b00,
    DataStartRisingDS = 0b01,
    RisingDSMiddleData = 0b10,
    RisingDsBeforeData = 0b11,
}

impl DataStrobeWaveformStr {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Reserved,
            0b01 => Self::DataStartRisingDS,
            0b10 => Self::RisingDSMiddleData,
            _ => Self::RisingDsBeforeData,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug)]
pub enum OctalDtrCmd {
    CmdExtSameAsCmd = 0b00,
    CmdExtInverseCmd = 0b01,
    Reserved = 0b10,
    CmdAndCmdExtMake16b = 0b11,
}

impl OctalDtrCmd {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::CmdExtSameAsCmd,
            0b01 => Self::CmdExtInverseCmd,
            0b10 => Self::Reserved,
            _ => Self::CmdAndCmdExtMake16b,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug)]
pub enum ByteOrder8D8D8D {
    SameAs1S1S1S = 0b0,
    Swapped = 0b1,
}

impl ByteOrder8D8D8D {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b1 {
            0b0 => Self::SameAs1S1S1S,
            _ => Self::Swapped,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord19 {
    #[bits(4)]
    pub mode_8s_8s_8s_disable_sequences: Mode8s8s8sDisableSequences,

    #[bits(5)]
    pub mode_8s_8s_8s_enable_sequences: Mode8s8s8sEnableSequences,

    pub mode_0_8_8_supported: bool,

    #[bits(6)]
    pub mode_0_8_8_exit_method: Mode088ExitMethod,

    #[bits(4)]
    pub mode_0_8_8_entry_method: Mode088EntryMethod,

    #[bits(3)]
    pub octal_enable_requirements: OctalEnableReqs,

    #[bits(9)]
    __: u16,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Mode8s8s8sDisableSequences {
    pub instr_06_then_ff: bool,
    #[bits(2)]
    __: u8,
    pub soft_reset_66_99: bool,
    #[bits(4)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Mode8s8s8sEnableSequences {
    __: bool,
    pub instr_06_then_e8: bool,
    pub instr_06_then_72: bool,
    #[bits(5)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Mode088ExitMethod {
    pub mode_bits: bool,
    pub ff_on_dq: bool,
    #[bits(6)]
    __: u8,
}

#[bitfield(u8)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Mode088EntryMethod {
    pub read_conf_reg: bool,
    #[bits(7)]
    __: u8,
}

#[derive(Debug)]
pub enum OctalEnableReqs {
    NoOctalEnableBit = 0b000,
    OctalEnableIsBit3 = 0b001,
    Reserved0 = 0b010,
    Reserved1 = 0b100,
    Reserved2 = 0b101,
    Reserved3 = 0b110,
    Reserved4 = 0b111,
}

impl OctalEnableReqs {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b111 {
            0b000 => Self::NoOctalEnableBit,
            0b001 => Self::OctalEnableIsBit3,
            0b010 => Self::Reserved0,
            0b100 => Self::Reserved1,
            0b101 => Self::Reserved2,
            0b110 => Self::Reserved3,
            _ => Self::Reserved4,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord20 {
    #[bits(4)]
    pub max_speed_4s_4s_4s_no_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_4s_4s_4s_with_data_strobe: MaxSpeed4s4s4sWithDataStrobe,

    #[bits(4)]
    pub max_speed_4s_4d_4d_no_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_4s_4d_4d_with_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_8s_8s_8s_no_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_8s_8s_8s_with_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_8d_8d_8d_no_data_strobe: MaxSpeed,

    #[bits(4)]
    pub max_speed_8d_8d_8d_with_data_strobe: MaxSpeed,
}

#[derive(Debug)]
pub enum MaxSpeed {
    Reserved0 = 0b0000,
    _33Mhz = 0b0001,
    _50Mhz = 0b0010,
    _66Mhz = 0b0011,
    _80Mhz = 0b0100,
    _100Mhz = 0b0101,
    _133Mhz = 0b0110,
    _166Mhz = 0b0111,
    _200Mhz = 0b1000,
    _250Mhz = 0b1001,
    _266Mhz = 0b1010,
    _333Mhz = 0b1011,
    _400Mhz = 0b1100,
    Reserved1 = 0b1101,
    NotCharacterized = 0b1110,
    NotSupported = 0b1111,
}

impl MaxSpeed {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b1111 {
            0b0000 => Self::Reserved0,
            0b0001 => Self::_33Mhz,
            0b0010 => Self::_50Mhz,
            0b0011 => Self::_66Mhz,
            0b0100 => Self::_80Mhz,
            0b0101 => Self::_100Mhz,
            0b0110 => Self::_133Mhz,
            0b0111 => Self::_166Mhz,
            0b1000 => Self::_200Mhz,
            0b1001 => Self::_250Mhz,
            0b1010 => Self::_266Mhz,
            0b1011 => Self::_333Mhz,
            0b1100 => Self::_400Mhz,
            0b1101 => Self::Reserved1,
            0b1110 => Self::NotCharacterized,
            _ => Self::NotSupported,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Debug)]
pub enum MaxSpeed4s4s4sWithDataStrobe {
    Reserved0 = 0b0000,
    _33Mhz = 0b0001,
    _50Mhz = 0b0010,
    _66Mhz = 0b0011,
    _80Mhz = 0b0100,
    _100Mhz = 0b0101,
    _133Mhz = 0b0110,
    _166Mhz = 0b0111,
    Reserved1 = 0b1000,
    Reserved2 = 0b1001,
    Reserved3 = 0b1010,
    Reserved4 = 0b1011,
    Reserved5 = 0b1100,
    Reserved6 = 0b1101,
    NotCharacterized = 0b1110,
    NotSupported = 0b1111,
}

impl MaxSpeed4s4s4sWithDataStrobe {
    const fn from_bits(bits: u8) -> Self {
        match bits & 0b1111 {
            0b0000 => Self::Reserved0,
            0b0001 => Self::_33Mhz,
            0b0010 => Self::_50Mhz,
            0b0011 => Self::_66Mhz,
            0b0100 => Self::_80Mhz,
            0b0101 => Self::_100Mhz,
            0b0110 => Self::_133Mhz,
            0b0111 => Self::_166Mhz,
            0b1000 => Self::Reserved1,
            0b1001 => Self::Reserved2,
            0b1010 => Self::Reserved3,
            0b1011 => Self::Reserved4,
            0b1100 => Self::Reserved5,
            0b1101 => Self::Reserved6,
            0b1110 => Self::NotCharacterized,
            _ => Self::NotSupported,
        }
    }
    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord21 {
    pub support_1s_1d_1d_fast_read: bool,

    pub support_1s_2d_2d_fast_read: bool,

    pub support_1s_4d_4d_fast_read: bool,

    pub support_4s_4d_4d_fast_read: bool,

    #[bits(28)]
    __: u32,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord22 {
    #[bits(5)]
    pub fast_read_1s_1d_1d_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_1d_1d_mode_clocks: u8,

    pub fast_read_1s_1d_1d_instr: u8,

    #[bits(5)]
    pub fast_read_1s_2d_2d_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_2d_2d_mode_clocks: u8,

    pub fast_read_1s_2d_2d_instr: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct BasicFlashWord23 {
    #[bits(5)]
    pub fast_read_1s_4d_4d_wait_states: u8,

    #[bits(3)]
    pub fast_read_1s_4d_4d_mode_clocks: u8,

    pub fast_read_1s_4d_4d_instr: u8,

    #[bits(5)]
    pub fast_read_4s_4d_4d_wait_states: u8,

    #[bits(3)]
    pub fast_read_4s_4d_4d_mode_clocks: u8,

    pub fast_read_4s_4d_4d_instr: u8,
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, KnownLayout, Debug)]
pub struct MemoryDensity(u32);
impl MemoryDensity {
    pub const fn from_byte_len(byte_len: u32) -> Result<Self, ErrorCode> {
        if byte_len == 0 {
            return Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY);
        }
        match byte_len.checked_mul(8) {
            Some(bit_len) if bit_len <= 0x8000_0000 => Ok(Self(bit_len - 1)),
            _ => {
                if !byte_len.is_power_of_two() {
                    return Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY);
                }
                let shift = byte_len.trailing_zeros();
                Ok(Self(0x8000_0000 | (shift + 3)))
            }
        }
    }
    pub const fn byte_len(&self) -> Result<u32, ErrorCode> {
        if (self.0 & 0x8000_0000) == 0 {
            return Ok((self.0 + 1) / 8);
        }
        // -3 to convert bits to bytes
        let bits_power = self.0 & !0x8000_0000;
        if bits_power < 3 {
            return Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY);
        }
        let power = bits_power - 3;
        if power > 31 {
            return Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY);
        }
        Ok(1 << power)
    }
}

#[cfg(test)]
mod density_tests {
    use zerocopy::FromBytes;

    use super::*;

    #[test]
    fn small() {
        let density: MemoryDensity =
            MemoryDensity::read_from_bytes(&[0xFF, 0xFF, 0xFF, 0x00]).unwrap();
        assert_eq!(density.byte_len().unwrap(), 16 * 1024 * 1024 / 8);
        assert_eq!(
            MemoryDensity::from_byte_len(2 * 1024 * 1024)
                .unwrap()
                .as_bytes(),
            &[0xFF, 0xFF, 0xFF, 0x00]
        );
    }
    #[test]
    fn big() {
        let density: MemoryDensity =
            MemoryDensity::read_from_bytes(&[0x21, 0x00, 0x00, 0x80]).unwrap();
        assert_eq!(density.byte_len().unwrap(), 1024 * 1024 * 1024);
        assert_eq!(
            MemoryDensity::from_byte_len(1024 * 1024 * 1024)
                .unwrap()
                .as_bytes(),
            &[0x21, 0x00, 0x00, 0x80]
        );
    }
    #[test]
    fn too_big() {
        let density: MemoryDensity =
            MemoryDensity::read_from_bytes(&[0x24, 0x00, 0x00, 0x80]).unwrap();
        assert_eq!(
            density.byte_len(),
            Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY)
        );
    }

    #[test]
    fn from_byte_len() {
        assert_eq!(
            MemoryDensity::from_byte_len(0),
            Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY)
        );
        assert_eq!(MemoryDensity::from_byte_len(1), Ok(MemoryDensity(0x7)));
        assert_eq!(MemoryDensity::from_byte_len(2), Ok(MemoryDensity(0xf)));
        assert_eq!(MemoryDensity::from_byte_len(3), Ok(MemoryDensity(0x17)));
        assert_eq!(MemoryDensity::from_byte_len(4), Ok(MemoryDensity(0x1f)));
        assert_eq!(
            MemoryDensity::from_byte_len(1024),
            Ok(MemoryDensity(0x1fff))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(1025),
            Ok(MemoryDensity(0x2007))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(1024 * 1024),
            Ok(MemoryDensity(0x7f_ffff))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(256 * 1024 * 1024),
            Ok(MemoryDensity(0x7fff_ffff))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(256 * 1024 * 1024 + 1),
            Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY)
        );
        assert_eq!(
            MemoryDensity::from_byte_len(512 * 1024 * 1024),
            Ok(MemoryDensity(0x8000_0020))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(1024 * 1024 * 1024),
            Ok(MemoryDensity(0x8000_0021))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(2048 * 1024 * 1024),
            Ok(MemoryDensity(0x8000_0022))
        );
        assert_eq!(
            MemoryDensity::from_byte_len(u32::MAX),
            Err(error::FLASH_GENERIC_SFDP_INVALID_MEMORY_DENSITY)
        );
    }
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct BasicFlashParameterTable {
    pub table_jesd216: BasicFlashParameterTableJESD216,
    pub table_jesd216a: BasicFlashParameterTableJESD216A,
    pub table_jesd216c: BasicFlashParameterTableJESD216C,
    pub table_jesd216f: BasicFlashParameterTableJESD216F,
}

trait ParameterTable {
    const FUNCTION: FunctionSpecificParameterTableIdAssignments;
    const MIN_SIZE: usize;
}

impl ParameterTable for BasicFlashParameterTable {
    const FUNCTION: FunctionSpecificParameterTableIdAssignments =
        FunctionSpecificParameterTableIdAssignments::BASIC_FLASH;
    const MIN_SIZE: usize = size_of::<BasicFlashParameterTableJESD216>();
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct BasicFlashParameterTableJESD216 {
    pub word1: BasicFlashWord1,
    pub memory_density: MemoryDensity,
    pub word3: BasicFlashWord3,
    pub word4: BasicFlashWord4,
    pub word5: BasicFlashWord5,
    pub word6: BasicFlashWord6,
    pub word7: BasicFlashWord7,
    pub word8: BasicFlashWord8,
    pub word9: BasicFlashWord9,
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct BasicFlashParameterTableJESD216A {
    pub word10: BasicFlashWord10,
    pub word11: BasicFlashWord11,
    pub word12: BasicFlashWord12,
    pub word13: BasicFlashWord13,
    pub word14: BasicFlashWord14,
    pub word15: BasicFlashWord15,
    pub word16: BasicFlashWord16,
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct BasicFlashParameterTableJESD216C {
    pub word17: BasicFlashWord17,
    pub word18: BasicFlashWord18,
    pub word19: BasicFlashWord19,
    pub word20: BasicFlashWord20,
}

#[derive(Clone, Copy, Eq, PartialEq, FromBytes, IntoBytes, Immutable, Debug)]
#[repr(C)]
pub struct BasicFlashParameterTableJESD216F {
    pub word21: BasicFlashWord21,
    pub word22: BasicFlashWord22,
    pub word23: BasicFlashWord23,
}

macro_rules! field_opt_getter {
    ($param1:ident, $param2:ty) => {
        pub fn $param1(&self) -> Option<&$param2> {
            if self.header.len_in_dwords as usize * size_of::<u32>()
                >= offset_of!(BasicFlashParameterTable, $param1) + size_of::<$param2>()
            {
                Some(&self.table.$param1)
            } else {
                None
            }
        }
    };
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, KnownLayout)]
pub struct _4BInstructionTableWord1 {
    pub _1s_1s_1s_read_support: bool,
    pub _1s_1s_1s_fast_read_support: bool,
    pub _1s_1s_2s_fast_read_support: bool,
    pub _1s_2s_2s_fast_read_support: bool,
    pub _1s_1s_4s_fast_read_support: bool,
    pub _1s_4s_4s_fast_read_support: bool,
    pub _1s_1s_1s_page_program_support: bool,
    pub _1s_1s_4s_page_program_support: bool,
    pub _1s_4s_4s_page_program_support: bool,
    pub erase_type_1_support: bool,
    pub erase_type_2_support: bool,
    pub erase_type_3_support: bool,
    pub erase_type_4_support: bool,
    pub _1s_1d_1d_dtr_read_support: bool,
    pub _1s_2d_2d_dtr_read_support: bool,
    pub _1s_4d_4d_dtr_read_support: bool,
    pub volatile_sector_lock_read_support: bool,
    pub volatile_sector_lock_write_support: bool,
    pub non_volatile_sector_lock_read_support: bool,
    pub non_volatile_sector_lock_write_support: bool,
    pub _1s_1s_8s_fast_read_support: bool,
    pub _1s_8s_8s_fast_read_support: bool,
    pub _1s_8d_8d_dtr_read_support: bool,
    pub _1s_1s_8s_page_program_support: bool,
    pub _1s_8s_8s_page_program_support: bool,
    #[bits(7)]
    __: u8,
}

#[bitfield(u32)]
#[derive(PartialEq, Eq, FromBytes, IntoBytes, KnownLayout)]
pub struct _4BInstructionTableWord2 {
    pub erase_type_1_instr: u8,
    pub erase_type_2_instr: u8,
    pub erase_type_3_instr: u8,
    pub erase_type_4_instr: u8,
}

#[derive(Debug, PartialEq, Eq, FromBytes, IntoBytes, KnownLayout)]
#[repr(C)]
pub struct _4BInstructionTable {
    pub word1: _4BInstructionTableWord1,
    pub word2: _4BInstructionTableWord2,
}

impl ParameterTable for _4BInstructionTable {
    const FUNCTION: FunctionSpecificParameterTableIdAssignments =
        FunctionSpecificParameterTableIdAssignments::_4B_ADDRESS_INSTRUCTION_TABLE;
    const MIN_SIZE: usize = size_of::<_4BInstructionTable>();
}

#[derive(Debug, PartialEq, Eq)]
pub struct HeaderAndTable<T> {
    pub header: ParameterHeader,
    table: T,
}

impl HeaderAndTable<BasicFlashParameterTable> {
    pub fn table_jesd216(&self) -> BasicFlashParameterTableJESD216 {
        self.table.table_jesd216
    }
    field_opt_getter! {table_jesd216a, BasicFlashParameterTableJESD216A}
    field_opt_getter! {table_jesd216c, BasicFlashParameterTableJESD216C}
    field_opt_getter! {table_jesd216f, BasicFlashParameterTableJESD216F}
}

pub struct SfdpReader<R: RandomRead> {
    reader: R,
    basic_flash_header: Option<ParameterHeader>,
    _4b_instructions_header: Option<ParameterHeader>,
}

impl<R: RandomRead<Error: From<ErrorCode>>> SfdpReader<R> {
    pub fn new(reader: R) -> Result<Self, R::Error> {
        let mut sfdp_reader = Self {
            reader,
            basic_flash_header: None,
            _4b_instructions_header: None,
        };
        sfdp_reader.parse_headers()?;
        Ok(sfdp_reader)
    }

    pub fn header(&mut self) -> Result<SfdpHeader, R::Error> {
        let mut header = SfdpHeader::new_zeroed();
        self.reader.read(0, header.as_mut_bytes())?;
        Ok(header)
    }

    pub fn parameter_header(&mut self, index: u8) -> Result<ParameterHeader, R::Error> {
        let mut parameter_header = ParameterHeader::new_zeroed();
        self.reader.read(
            size_of::<SfdpHeader>() * (index as usize + 1),
            parameter_header.as_mut_bytes(),
        )?;
        Ok(parameter_header)
    }

    fn parse_headers(&mut self) -> Result<(), R::Error> {
        let sfdp_header = self.header()?;
        if !sfdp_header.sig.is_valid() {
            return Err(error::FLASH_GENERIC_SFDP_INVALID_SIGNATURE.into());
        }
        if sfdp_header.major_rev != 1 {
            return Err(error::FLASH_GENERIC_SFDP_UNSUPPORTED_HEADER_MAJOR_REV.into());
        }

        // TODO: b/403297476 - Use the most up to date version we support
        // Right now it will just overwrite with the latest header that matches the function
        for index in 0..=sfdp_header.num_parameter_header {
            let Ok(parameter_header) = self.parameter_header(index) else {
                // Failed to parse this header.
                continue;
            };
            if parameter_header.major_rev != 1 {
                // unsupported header, but maybe we can find another supported one
                continue;
            }
            let h = match parameter_header.parameter_id() {
                FunctionSpecificParameterTableIdAssignments::BASIC_FLASH => {
                    &mut self.basic_flash_header
                }
                FunctionSpecificParameterTableIdAssignments::_4B_ADDRESS_INSTRUCTION_TABLE => {
                    &mut self._4b_instructions_header
                }
                _ => {
                    // Skip header
                    continue;
                }
            };
            *h = Some(parameter_header);
        }
        Ok(())
    }

    fn read_table<T: Sized + FromZeros + IntoBytes + FromBytes + ParameterTable>(
        &mut self,
    ) -> Result<HeaderAndTable<T>, R::Error> {
        let Some(parameter_header) = (match T::FUNCTION {
            FunctionSpecificParameterTableIdAssignments::BASIC_FLASH => self.basic_flash_header,
            FunctionSpecificParameterTableIdAssignments::_4B_ADDRESS_INSTRUCTION_TABLE => {
                self._4b_instructions_header
            }
            _ => return Err(error::FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND.into()),
        }) else {
            return Err(error::FLASH_GENERIC_SFDP_NO_VALID_PARAMETER_HEADER_FOUND.into());
        };
        let mut table_inner = T::new_zeroed();

        self.read_table_common(parameter_header, T::MIN_SIZE, table_inner.as_mut_bytes())?;

        Ok(HeaderAndTable {
            header: parameter_header,
            table: table_inner,
        })
    }

    fn read_table_common(
        &mut self,
        parameter_header: ParameterHeader,
        min_bytes: usize,
        target_slice: &mut [u8],
    ) -> Result<(), R::Error> {
        let bytes_len = (parameter_header.len_in_dwords as usize)
            .checked_mul(size_of::<u32>())
            .ok_or(error::FLASH_GENERIC_SFDP_PARAMETERS_TOO_LONG)?
            .min(target_slice.len());
        if bytes_len < min_bytes {
            return Err(error::FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT.into());
        }

        let target = &mut target_slice[..bytes_len];
        self.reader.read(parameter_header.ptr.as_usize(), target)?;
        Ok(())
    }

    pub fn basic_flash_parameters(
        &mut self,
    ) -> Result<HeaderAndTable<BasicFlashParameterTable>, R::Error> {
        self.read_table::<BasicFlashParameterTable>()
    }

    pub fn _4b_instructions_parameters(
        &mut self,
    ) -> Result<HeaderAndTable<_4BInstructionTable>, R::Error> {
        self.read_table::<_4BInstructionTable>()
    }
}

#[cfg(test)]
mod test {
    use error::FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT;

    use super::*;

    #[test]
    fn too_long() {
        let sfdp_bytes_params_too_long: &[u8] = &[
            0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x00, 0xff, 0x00, 0x05, 0x01,
            /*param len =*/ 0x20, 0x80, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xe5, 0x20, 0xf9, 0xff, 0xff, 0xff,
            0xff, 0x07, 0x44, 0xeb, 0x08, 0x6b, 0x08, 0x3b, 0x42, 0xbb, 0xfe, 0xff, 0xff, 0xff,
            0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0x40, 0xeb, 0x0c, 0x20, 0x0f, 0x52, 0x10, 0xd8,
            0x00, 0x00, 0x36, 0x02, 0xa6, 0x00, 0x82, 0xea, 0x14, 0xc9, 0xe9, 0x63, 0x76, 0x33,
            0x7a, 0x75, 0x7a, 0x75, 0xf7, 0xa2, 0xd5, 0x5c, 0x19, 0xf7, 0x4d, 0xff, 0xe9, 0x30,
            0xf8, 0x80, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];
        let mut sfdp_reader: SfdpReader<&[u8]> =
            SfdpReader::new(sfdp_bytes_params_too_long).unwrap();
        let params = sfdp_reader.basic_flash_parameters().unwrap();
        assert!(params.table_jesd216f().is_some());
    }

    #[test]
    fn too_short() {
        let sfdp_bytes_params_too_short: &[u8] = &[
            0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x00, 0xff, 0x00, 0x05, 0x01,
            /*param len =*/ 0x02, 0x80, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xe5, 0x20, 0xf9, 0xff, 0xff, 0xff,
            0xff, 0x07, 0x44, 0xeb, 0x08, 0x6b, 0x08, 0x3b, 0x42, 0xbb, 0xfe, 0xff, 0xff, 0xff,
            0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0x40, 0xeb, 0x0c, 0x20, 0x0f, 0x52, 0x10, 0xd8,
            0x00, 0x00, 0x36, 0x02, 0xa6, 0x00, 0x82, 0xea, 0x14, 0xc9, 0xe9, 0x63, 0x76, 0x33,
            0x7a, 0x75, 0x7a, 0x75, 0xf7, 0xa2, 0xd5, 0x5c, 0x19, 0xf7, 0x4d, 0xff, 0xe9, 0x30,
            0xf8, 0x80, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];
        let mut sfdp_reader: SfdpReader<&[u8]> =
            SfdpReader::new(sfdp_bytes_params_too_short).unwrap();
        assert_eq!(
            sfdp_reader.basic_flash_parameters(),
            Err(FLASH_GENERIC_SFDP_PARAMETERS_TOO_SHORT)
        );
    }

    #[test]
    fn print_params() {
        let sfdp_bytes: &[u8] = &[
            0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x00, 0xff, 0x00, 0x05, 0x01, 0x10, 0x80, 0x00,
            0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xe5, 0x20, 0xf9, 0xff, 0xff, 0xff, 0xff, 0x07, 0x44, 0xeb, 0x08, 0x6b,
            0x08, 0x3b, 0x42, 0xbb, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0xff, 0xff,
            0x40, 0xeb, 0x0c, 0x20, 0x0f, 0x52, 0x10, 0xd8, 0x00, 0x00, 0x36, 0x02, 0xa6, 0x00,
            0x82, 0xea, 0x14, 0xc9, 0xe9, 0x63, 0x76, 0x33, 0x7a, 0x75, 0x7a, 0x75, 0xf7, 0xa2,
            0xd5, 0x5c, 0x19, 0xf7, 0x4d, 0xff, 0xe9, 0x30, 0xf8, 0x80, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff,
        ];

        let mut sfdp_reader: SfdpReader<&[u8]> = SfdpReader::new(sfdp_bytes).unwrap();

        let sfdp_header = sfdp_reader.header().unwrap();
        assert!(sfdp_header.sig.is_valid());
        assert_eq!(sfdp_header.access_protocol, AccessProtocol::LEGACY);
        assert_eq!(sfdp_header.major_rev, 1);
        assert_eq!(sfdp_header.minor_rev, 5);
        assert_eq!(sfdp_header.num_parameter_header, 0);

        let basic_flash = sfdp_reader.basic_flash_parameters().unwrap();

        assert_eq!(
            basic_flash.header.parameter_id(),
            FunctionSpecificParameterTableIdAssignments::BASIC_FLASH
        );
        assert_eq!(basic_flash.header.major_rev, 1);
        assert_eq!(basic_flash.header.minor_rev, 5);
        assert_eq!(basic_flash.header.len_in_dwords, 16);
        assert_eq!(basic_flash.header.ptr.as_u32(), 128);

        assert_eq!(
            format!("{:#?}", basic_flash.table_jesd216()),
            r#"BasicFlashParameterTableJESD216 {
    word1: BasicFlashWord1 {
        legacy_erase_sizes: Erase4k,
        legacy_write_granularity: Buffer64,
        block_protect_is_volatile: false,
        status_write_requires_write_enable: false,
        erase4k_instr: 32,
        supports_1s_1s_2s_read: true,
        addr_bytes: _3ByteOnly,
        dtr_clocking_supported: true,
        supports_1s_2s_2s_read: true,
        supports_1s_4s_4s_read: true,
        supports_1s_1s_4s_read: true,
    },
    memory_density: MemoryDensity(
        134217727,
    ),
    word3: BasicFlashWord3 {
        fast_read_1s_4s_4s_wait_states: 4,
        fast_read_1s_4s_4s_mode_clocks: 2,
        fast_read_1s_4s_4s_instr: 235,
        fast_read_1s_1s_4s_wait_states: 8,
        fast_read_1s_1s_4s_mode_clocks: 0,
        fast_read_1s_1s_4s_instr: 107,
    },
    word4: BasicFlashWord4 {
        fast_read_1s_1s_2s_wait_states: 8,
        fast_read_1s_1s_2s_mode_clocks: 0,
        fast_read_1s_1s_2s_instr: 59,
        fast_read_1s_2s_2s_wait_states: 2,
        fast_read_1s_2s_2s_mode_clocks: 2,
        fast_read_1s_2s_2s_instr: 187,
    },
    word5: BasicFlashWord5 {
        fast_read_2x_2s_2s_supported: false,
        fast_read_4x_4s_4s_supported: true,
    },
    word6: BasicFlashWord6 {
        fast_read_2s_2s_2s_wait_states: 0,
        fast_read_2s_2s_2s_mode_clocks: 0,
        fast_read_2s_2s_2s_instr: 0,
    },
    word7: BasicFlashWord7 {
        fast_read_4s_4s_4s_wait_states: 0,
        fast_read_4s_4s_4s_mode_clocks: 2,
        fast_read_4s_4s_4s_instr: 235,
    },
    word8: BasicFlashWord8 {
        erase_type_1_size: PowerOf2 {
            n: 12,
        },
        erase_type_1_instr: 32,
        erase_type_2_size: PowerOf2 {
            n: 15,
        },
        erase_type_2_instr: 82,
    },
    word9: BasicFlashWord9 {
        erase_type_3_size: PowerOf2 {
            n: 16,
        },
        erase_type_3_instr: 216,
        erase_type_4_size: PowerOf2 {
            n: 0,
        },
        erase_type_4_instr: 0,
    },
}"#
        );
        assert_eq!(
            format!("{:#?}", basic_flash.table_jesd216a().unwrap()),
            r#"BasicFlashParameterTableJESD216A {
    word10: BasicFlashWord10 {
        multiplier_max_erase_time: 6,
        erase_type_1_time: EraseTime {
            count: 3,
            units: _16ms,
        },
        erase_type_2_time: EraseTime {
            count: 0,
            units: _128ms,
        },
        erase_type_3_time: EraseTime {
            count: 9,
            units: _16ms,
        },
        erase_type_4_time: EraseTime {
            count: 0,
            units: _1ms,
        },
    },
    word11: BasicFlashWord11 {
        multiplier_max_page_program: 2,
        page_size: SmallPowerOf2 {
            n: 8,
        },
        page_program_time: PageProgramTime {
            count: 10,
            units: _64us,
        },
        byte_program_first_time: ByteProgramTime {
            count: 3,
            unit: _8us,
        },
        byte_program_additional_time: ByteProgramTime {
            count: 2,
            unit: _1us,
        },
        chip_erase_time: ChipEraseTime {
            count: 9,
            units: _16ms,
        },
    },
    word12: BasicFlashWord12 {
        prohibited_ops_during_program_suspend: ProhibitedOpsDuringProgramSuspend {
            erase_nesting_permitted: true,
            program_nesting_permitted: false,
            no_read: false,
            erase_and_program_restrictions_are_sufficient: true,
        },
        prohibited_ops_during_erase_suspend: ProhibitedOpsDuringEraseSuspend {
            erase_nesting_permitted: false,
            program_nesting_permitted: true,
            no_read: true,
            erase_and_program_restrictions_are_sufficient: true,
        },
        program_resume_suspend_interval: Interval64us {
            counts_64ms: 1,
        },
        suspend_in_progress_program_max_latency: DurationEnumA {
            count: 19,
            units: _1us,
        },
        erase_resume_suspend_interval: Interval64us {
            counts_64ms: 7,
        },
        suspend_in_progress_erase_max_latency: DurationEnumA {
            count: 19,
            units: _1us,
        },
        suspend_resume_supported: false,
    },
    word13: BasicFlashWord13 {
        program_resume_instr: 122,
        program_suspend_instr: 117,
        resume_instr: 122,
        suspend_instr: 117,
    },
    word14: BasicFlashWord14 {
        status_register_polling_device_busy: StatusRegisterPollingDeviceBusy {
            legacy_polling_supported: true,
            bit_7_polled_any_time: false,
        },
        exit_deep_powerdown_next_op_delay: DurationEnumA {
            count: 2,
            units: _1us,
        },
        exit_deep_powerdown_instr: 171,
        enter_deep_powerdown_instr: 185,
        deep_powerdown_supported: false,
    },
    word15: BasicFlashWord15 {
        mode_disable_sequence_4s_4s_4s: ModeDisableSequence4S4S4S {
            issue_ff_instr: true,
            issue_f5_instr: false,
            read_mod_write_seq: false,
            issue_soft_reset: true,
        },
        mode_enable_sequence_4s_4s_4s: ModeEnableSequence4S4S4S {
            set_qe: true,
            issue_38_instr: false,
            issue_35_instr: false,
            read_mod_write_seq1: false,
            read_mod_write_seq2: true,
        },
        mode_0_4_4_supported: true,
        mode_exit_method_0_4_4: ModeExitMethod044 {
            mod_bits_00: true,
            input_io1: false,
            input_io2: true,
            mod_bits_not_ax: true,
        },
        mode_entry_method_0_4_4: ModeEntryMethod044 {
            mod_bits_a5: true,
            instr_85: false,
            mod_bits_ax: true,
        },
        quad_enable_requirements: QeBit1SR2C,
        hold_or_reset_disable: false,
    },
    word16: BasicFlashWord16 {
        volatile_non_register_write_enable_instr: VolatileNonRegisterWriteEnableInstr {
            non_volatile_reg1_last_written_we06: true,
            volatile_reg1_last_written_we06: false,
            volatile_reg1_last_written_we50: false,
            volatile_non_reg1_last_written_we0650: true,
            mix_volatile_we06: false,
        },
        soft_reset_rescue_seq_support: SoftResetRescureSeqSupport {
            drive0_f8_clk: false,
            drive0_f10_clk: false,
            drive0_f16_clk: false,
            instr_f0: false,
            instr66_then99: true,
            exit044_first: true,
        },
        exit_4_b_addressing: Exit4BAddressing {
            instr_e9: false,
            instr_06_then_e9: false,
            ear: false,
            bank_reg: false,
            conf_reg: false,
            hardware_reset: true,
            software_reset: true,
            power_cycle: true,
        },
        enter_4_b_addressing: Enter4BAddressing {
            intr_b7: false,
            instr_06_then_b7: false,
            ear: false,
            bank_reg: false,
            conf_reg: false,
            dedicated_add: false,
            always_4b: false,
        },
    },
}"#
        );

        assert!(basic_flash.table_jesd216c().is_none());
        assert!(basic_flash.table_jesd216f().is_none());
    }

    struct TestSample<'a> {
        name: &'a str,
        sfdp_bytes: &'a [u8],
        expected_mem_density: u32,
        expected_page_size_from_jesd216a: Option<usize>,
        expected_4b_table: bool,
    }

    const TEST_SAMPLES: &[TestSample] = &[
        TestSample {
            name: "test_1",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x01, 0xFF, 0x00, 0x05, 0x01, 0x10, 0x18, 0x00,
                0x00, 0xFF, 0x26, 0x00, 0x01, 0x04, 0x58, 0x00, 0x00, 0xFF, 0xF5, 0x20, 0x85, 0xFF,
                0xFF, 0xFF, 0xFF, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x08, 0x3B, 0x00, 0x00, 0xEE, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x0C, 0x20, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x81, 0xEF, 0xFF, 0xE1, 0x00, 0x01,
                0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0xFF,
                0x82, 0x00, 0x00, 0x40, 0x47, 0x4F, 0x4F, 0x47, 0x00, 0x00, 0x70, 0x00, 0x00, 0x04,
                0x00, 0x00, 0xED, 0x2B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            expected_mem_density: 64 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: false,
        },
        TestSample {
            name: "test_2",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x00, 0x01, 0x01, 0xFF, 0x00, 0x00, 0x01, 0x09, 0x30, 0x00,
                0x00, 0xFF, 0xC2, 0x00, 0x01, 0x04, 0x60, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xE5, 0x20, 0xF3, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F,
                0x44, 0xEB, 0x08, 0x6B, 0x08, 0x3B, 0x04, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0x00, 0xFF, 0xFF, 0xFF, 0x44, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x36,
                0x00, 0x27, 0x9D, 0xF9, 0xC0, 0x64, 0x85, 0xCB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ],
            expected_mem_density: 32 * 1024 * 1024,
            expected_page_size_from_jesd216a: None, // doesn't support JESD216A
            expected_4b_table: false,
        },
        TestSample {
            name: "test_3",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x01, 0xFF, 0x00, 0x05, 0x01, 0x10, 0x18, 0x00,
                0x00, 0xFF, 0x26, 0x00, 0x01, 0x04, 0x58, 0x00, 0x00, 0xFF, 0xF5, 0x20, 0x85, 0xFF,
                0xFF, 0xFF, 0xFF, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x08, 0x3B, 0x00, 0x00, 0xEE, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x0C, 0x20, 0x10, 0xD8,
                0x00, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x81, 0xEF, 0xFF, 0xE1, 0x00, 0x01,
                0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0xFF,
                0x82, 0x00, 0x00, 0x40, 0x47, 0x4F, 0x4F, 0x47, 0x00, 0x00, 0x90, 0x00, 0x00, 0x04,
                0x00, 0x00, 0xEF, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            expected_mem_density: 32 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: false,
        },
        TestSample {
            name: "test_4",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x00, 0x01, 0x01, 0xFF, 0x00, 0x00, 0x01, 0x09, 0x30, 0x00,
                0x00, 0xFF, 0xC2, 0x00, 0x01, 0x04, 0x60, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xE5, 0x20, 0xF3, 0xFF, 0xFF, 0xFF, 0xFF, 0x1F,
                0x44, 0xEB, 0x08, 0x6B, 0x08, 0x3B, 0x04, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0x00, 0xFF, 0xFF, 0xFF, 0x44, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x36,
                0x00, 0x27, 0x9D, 0xF9, 0xC0, 0x64, 0x85, 0xCB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ],
            expected_mem_density: 64 * 1024 * 1024,
            expected_page_size_from_jesd216a: None, // doesn't support JESD216A
            expected_4b_table: false,
        },
        TestSample {
            name: "test_5",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x06, 0x01, 0x02, 0xFF, //
                // v=1.6, l=0x10, ptr=0x30, f=0xff00
                0x00, 0x06, 0x01, 0x10, 0x30, 0x00, 0x00, 0xFF, //
                // v=1.0, l=0x04, ptr=0x10, f=0xffc2
                0xC2, 0x00, 0x01, 0x04, 0x10, 0x01, 0x00, 0xFF, //
                // v=1.0, l=0x02, ptr=0xC0, f=0xff84, the data is outside of this sample data
                0x84, 0x00, 0x01, 0x02, 0xC0, 0x00, 0x00, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xE5, 0x20, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0x1F, //
                0x44, 0xEB, 0x08, 0x6B, 0x08, 0x3B, 0x04, 0xBB, //
                0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, //
                0xFF, 0xFF, 0x44, 0xEB, 0x0C, 0x20, 0x0F, 0x52, //
                0x10, 0xD8, 0x00, 0xFF, 0xD6, 0x49, 0xC5, 0x00, //
                0x81, 0xDF, 0x04, 0xE3, 0x44, 0x03, 0x67, 0x38, //
                0x30, 0xB0, 0x30, 0xB0, 0xF7, 0xBD, 0xD5, 0x5C, //
                0x4A, 0x9E, 0x29, 0xFF, 0xF0, 0x50, 0xF9, 0x85, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
            ],
            expected_mem_density: 64 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: true,
        },
        TestSample {
            name: "test_6",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x06, 0x01, 0x02, 0xFF, 0x00, 0x06, 0x01, 0x10, 0x30, 0x00,
                0x00, 0xFF, 0xC2, 0x00, 0x01, 0x04, 0x10, 0x01, 0x00, 0xFF, 0x84, 0x00, 0x01, 0x02,
                0xC0, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xE5, 0x20, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0x3F,
                0x44, 0xEB, 0x08, 0x6B, 0x08, 0x3B, 0x04, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0x00, 0xFF, 0xFF, 0xFF, 0x44, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0xFF,
                0xD6, 0x49, 0xC5, 0x00, 0x85, 0xDF, 0x04, 0xE3, 0x44, 0x03, 0x67, 0x38, 0x30, 0xB0,
                0x30, 0xB0, 0xF7, 0xBD, 0xD5, 0x5C, 0x4A, 0x9E, 0x29, 0xFF, 0xF0, 0x50, 0xF9, 0x85,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ],
            expected_mem_density: 128 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: true,
        },
        TestSample {
            name: "test_7",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x06, 0x01, 0x02, 0xFF, 0x00, 0x06, 0x01, 0x10, 0x30, 0x00,
                0x00, 0xFF, 0xC2, 0x00, 0x01, 0x04, 0x10, 0x01, 0x00, 0xFF, 0x84, 0x00, 0x01, 0x02,
                0xC0, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xE5, 0x20, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0x3F,
                0x44, 0xEB, 0x08, 0x6B, 0x08, 0x3B, 0x04, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0x00, 0xFF, 0xFF, 0xFF, 0x44, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0xFF,
                0xD6, 0x49, 0xC5, 0x00, 0x85, 0xDF, 0x04, 0xE3, 0x44, 0x03, 0x67, 0x38, 0x30, 0xB0,
                0x30, 0xB0, 0xF7, 0xBD, 0xD5, 0x5C, 0x4A, 0x9E, 0x29, 0xFF, 0xF0, 0x50, 0xF9, 0x85,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ],
            expected_mem_density: 128 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: true,
        },
        TestSample {
            name: "test_8",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x05, 0x01, 0x00, 0xFF, 0x00, 0x05, 0x01, 0x10, 0x80, 0x00,
                0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xE5, 0x20, 0xF9, 0xFF, 0xFF, 0xFF, 0xFF, 0x07, 0x44, 0xEB, 0x08, 0x6B,
                0x08, 0x3B, 0x42, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF,
                0x40, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0x00, 0x36, 0x02, 0xA6, 0x00,
                0x82, 0xEA, 0x14, 0xC9, 0xE9, 0x63, 0x76, 0x33, 0x7A, 0x75, 0x7A, 0x75, 0xF7, 0xA2,
                0xD5, 0x5C, 0x19, 0xF7, 0x4D, 0xFF, 0xE9, 0x30, 0xF8, 0x80,
            ],
            expected_mem_density: 16 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: false,
        },
        TestSample {
            name: "test_9",
            sfdp_bytes: &[
                0x53, 0x46, 0x44, 0x50, 0x06, 0x01, 0x01, 0xFF, 0x00, 0x06, 0x01, 0x10, 0x80, 0x00,
                0x00, 0xFF, 0x84, 0x00, 0x01, 0x02, 0xD0, 0x00, 0x00, 0xFF, 0x03, 0x00, 0x01, 0x02,
                0xF0, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xE5, 0x20, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0x3F, 0x44, 0xEB, 0x08, 0x6B,
                0x08, 0x3B, 0x42, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF,
                0x40, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0x00, 0x36, 0x02, 0xA6, 0x00,
                0x82, 0xEA, 0x14, 0xE2, 0xE9, 0x63, 0x76, 0x33, 0x7A, 0x75, 0x7A, 0x75, 0xF7, 0xA2,
                0xD5, 0x5C, 0x19, 0xF7, 0x4D, 0xFF, 0xE9, 0x70, 0xF9, 0xA5, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A,
                0xF0, 0xFF, 0x21, 0xFF, 0xDC, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF, 0xFF, 0xFF,
            ],
            expected_mem_density: 128 * 1024 * 1024,
            expected_page_size_from_jesd216a: Some(256),
            expected_4b_table: true,
        },
    ];

    #[test]
    fn test_all_samples() {
        for td in TEST_SAMPLES {
            println!("Sample name: {}", td.name);
            let mut sfdp_reader: SfdpReader<&[u8]> = SfdpReader::new(td.sfdp_bytes).unwrap();
            let basic_flash = sfdp_reader.basic_flash_parameters().unwrap();
            assert_eq!(
                basic_flash
                    .table_jesd216()
                    .memory_density
                    .byte_len()
                    .unwrap(),
                td.expected_mem_density
            );
            assert_eq!(
                basic_flash
                    .table_jesd216a()
                    .map(|t| t.word11.page_size().value()),
                td.expected_page_size_from_jesd216a
            );
            if td.expected_4b_table {
                assert!(sfdp_reader._4b_instructions_header.is_some());
            } else {
                assert_eq!(sfdp_reader._4b_instructions_header, None);
            }
        }
    }

    #[test]
    fn get_4b_table() {
        let sfdp_bytes: &[u8] = &[
            0x53, 0x46, 0x44, 0x50, 0x06, 0x01, 0x01, 0xFF, 0x00, 0x06, 0x01, 0x10, 0x80, 0x00,
            0x00, 0xFF, 0x84, 0x00, 0x01, 0x02, 0xD0, 0x00, 0x00, 0xFF, 0x03, 0x00, 0x01, 0x02,
            0xF0, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xE5, 0x20, 0xFB, 0xFF, 0xFF, 0xFF, 0xFF, 0x3F, 0x44, 0xEB, 0x08, 0x6B,
            0x08, 0x3B, 0x42, 0xBB, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF,
            0x40, 0xEB, 0x0C, 0x20, 0x0F, 0x52, 0x10, 0xD8, 0x00, 0x00, 0x36, 0x02, 0xA6, 0x00,
            0x82, 0xEA, 0x14, 0xE2, 0xE9, 0x63, 0x76, 0x33, 0x7A, 0x75, 0x7A, 0x75, 0xF7, 0xA2,
            0xD5, 0x5C, 0x19, 0xF7, 0x4D, 0xFF, 0xE9, 0x70, 0xF9, 0xA5, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0A,
            0xF0, 0xFF, 0x21, 0xFF, 0xDC, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let mut sfdp_reader: SfdpReader<&[u8]> = SfdpReader::new(sfdp_bytes).unwrap();
        let table = sfdp_reader.read_table::<_4BInstructionTable>().unwrap();
        assert!(table.table.word1.erase_type_1_support());
        assert!(table.table.word1.erase_type_3_support());
        assert_eq!(table.table.word2.erase_type_1_instr(), 0x21);
        assert_eq!(table.table.word2.erase_type_3_instr(), 0xdc);
    }
}
