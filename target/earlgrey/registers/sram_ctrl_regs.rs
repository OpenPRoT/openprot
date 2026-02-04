#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SramCtrlRetAon {
    _priv: (),
}
impl SramCtrlRetAon {
    pub const PTR: *mut u32 = 0x40500000 as *mut u32;
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Caller must ensure that all concurrent use of this"]
    #[doc = r" peripheral in the firmware is done so in a compatible"]
    #[doc = r" way. The simplest way to enforce this is to only call"]
    #[doc = r" this function once."]
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self { _priv: () }
    }
    #[doc = r" Returns a register block that can be used to read"]
    #[doc = r" registers from this peripheral, but cannot write."]
    #[inline(always)]
    pub fn regs(&self) -> RegisterBlock<ureg::RealMmio<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
    #[doc = r" Return a register block that can be used to read and"]
    #[doc = r" write this peripheral's registers."]
    #[inline(always)]
    pub fn regs_mut(&mut self) -> RegisterBlock<ureg::RealMmioMut<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
}
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SramCtrlMain {
    _priv: (),
}
impl SramCtrlMain {
    pub const PTR: *mut u32 = 0x411c0000 as *mut u32;
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Caller must ensure that all concurrent use of this"]
    #[doc = r" peripheral in the firmware is done so in a compatible"]
    #[doc = r" way. The simplest way to enforce this is to only call"]
    #[doc = r" this function once."]
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self { _priv: () }
    }
    #[doc = r" Returns a register block that can be used to read"]
    #[doc = r" registers from this peripheral, but cannot write."]
    #[inline(always)]
    pub fn regs(&self) -> RegisterBlock<ureg::RealMmio<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
    #[doc = r" Return a register block that can be used to read and"]
    #[doc = r" write this peripheral's registers."]
    #[inline(always)]
    pub fn regs_mut(&mut self) -> RegisterBlock<ureg::RealMmioMut<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct RegisterBlock<TMmio: ureg::Mmio + core::borrow::Borrow<TMmio>> {
    ptr: *mut u32,
    mmio: TMmio,
}
impl<TMmio: ureg::Mmio + core::default::Default> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new(ptr: *mut u32) -> Self {
        Self {
            ptr,
            mmio: core::default::Default::default(),
        }
    }
}
impl<TMmio: ureg::Mmio> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new_with_mmio(ptr: *mut u32, mmio: TMmio) -> Self {
        Self { ptr, mmio }
    }
    #[doc = "Alert Test Register\n\nRead value: [`regs::AlertTestReadVal`]; Write value: [`regs::AlertTestWriteVal`]"]
    #[inline(always)]
    pub fn alert_test(&self) -> ureg::RegRef<crate::meta::AlertTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SRAM status register.\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock register for execution enable register.\n\nRead value: [`regs::ExecRegwenReadVal`]; Write value: [`regs::ExecRegwenWriteVal`]"]
    #[inline(always)]
    pub fn exec_regwen(&self) -> ureg::RegRef<crate::meta::ExecRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Sram execution enable.\n\nRead value: [`regs::ExecReadVal`]; Write value: [`regs::ExecWriteVal`]"]
    #[inline(always)]
    pub fn exec(&self) -> ureg::RegRef<crate::meta::Exec, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock register for control register.\n\nRead value: [`regs::CtrlRegwenReadVal`]; Write value: [`regs::CtrlRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_regwen(&self) -> ureg::RegRef<crate::meta::CtrlRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SRAM ctrl register.\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clearable SRAM key request status.\n\nRead value: [`regs::ScrKeyRotatedReadVal`]; Write value: [`regs::ScrKeyRotatedWriteVal`]"]
    #[inline(always)]
    pub fn scr_key_rotated(&self) -> ureg::RegRef<crate::meta::ScrKeyRotated, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock register for readback enable register.\n\nRead value: [`regs::ReadbackRegwenReadVal`]; Write value: [`regs::ReadbackRegwenWriteVal`]"]
    #[inline(always)]
    pub fn readback_regwen(&self) -> ureg::RegRef<crate::meta::ReadbackRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "readback enable.\n\nRead value: [`regs::ReadbackReadVal`]; Write value: [`regs::ReadbackWriteVal`]"]
    #[inline(always)]
    pub fn readback(&self) -> ureg::RegRef<crate::meta::Readback, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for AlertTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlWriteVal(u32);
    impl CtrlWriteVal {
        #[doc = "Write 1 to request a new scrambling key from OTP. After writing to this register, SRAM transactions will\nbe blocked until !!STATUS.SCR_KEY_VALID has been set to 1. If !!STATUS.SCR_KEY_VALID was already 1\nbefore triggering a key renewal, hardware will automatically clear that status bit such that software\ncan poll its status. Note that requesting a new scrambling key takes ~200 OTP cycles, which translates\nto ~800 CPU cycles (OTP runs at 24MHz, CPU runs at 100MHz). Note that writing 1 to this register while\na key request or a memory initialization request is already pending has no effect."]
        #[inline(always)]
        pub fn renew_scr_key(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to request memory init.\nThe init mechanism uses an LFSR that is seeded with a part of the nonce supplied when requesting a scrambling key.\nOnce seeded, the memory is initialized with pseudo-random data pulled from the LFSR.\nNote that !!CTRL.RENEW_SCR_KEY takes priority when writing 1 to both !!CTRL.RENEW_SCR_KEY and !!CTRL.INIT with the same write transaction.\nThis means that the key request will complete first, followed by SRAM initialization. Note that writing 1 to this register while\nan init request is already pending has no effect."]
        #[inline(always)]
        pub fn init(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for CtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlRegwenReadVal(u32);
    impl CtrlRegwenReadVal {
        #[doc = "When cleared to zero, !!CTRL can not be written anymore."]
        #[inline(always)]
        pub fn ctrl_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlRegwenWriteVal {
            CtrlRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlRegwenWriteVal(u32);
    impl CtrlRegwenWriteVal {
        #[doc = "When cleared to zero, !!CTRL can not be written anymore."]
        #[inline(always)]
        pub fn ctrl_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CtrlRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExecReadVal(u32);
    impl ExecReadVal {
        #[doc = "Write kMultiBitBool4True to this field to enable execution from SRAM.\nNote that this register only takes effect if the EN_SRAM_IFETCH switch\nin the OTP HW_CFG1 partition is set to kMultiBitBool8True. Otherwise execution\nfrom SRAM cannot be enabled via this register."]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ExecWriteVal {
            ExecWriteVal(self.0)
        }
    }
    impl From<u32> for ExecReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExecReadVal> for u32 {
        #[inline(always)]
        fn from(val: ExecReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExecWriteVal(u32);
    impl ExecWriteVal {
        #[doc = "Write kMultiBitBool4True to this field to enable execution from SRAM.\nNote that this register only takes effect if the EN_SRAM_IFETCH switch\nin the OTP HW_CFG1 partition is set to kMultiBitBool8True. Otherwise execution\nfrom SRAM cannot be enabled via this register."]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for ExecWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExecWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ExecWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExecRegwenReadVal(u32);
    impl ExecRegwenReadVal {
        #[doc = "When cleared to zero, !!EXEC can not be written anymore."]
        #[inline(always)]
        pub fn exec_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ExecRegwenWriteVal {
            ExecRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ExecRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExecRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ExecRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExecRegwenWriteVal(u32);
    impl ExecRegwenWriteVal {
        #[doc = "When cleared to zero, !!EXEC can not be written anymore."]
        #[inline(always)]
        pub fn exec_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ExecRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExecRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ExecRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadbackReadVal(u32);
    impl ReadbackReadVal {
        #[doc = "Write kMultiBitBool4True to this field to enable the readback security feature for the SRAM.\nA readback of each memory write or read request will be performed and a comparison happens.\nAny other value than kMultiBitBool4False written to this field is interpreted as kMultiBitBool4True."]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ReadbackWriteVal {
            ReadbackWriteVal(self.0)
        }
    }
    impl From<u32> for ReadbackReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadbackReadVal> for u32 {
        #[inline(always)]
        fn from(val: ReadbackReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadbackWriteVal(u32);
    impl ReadbackWriteVal {
        #[doc = "Write kMultiBitBool4True to this field to enable the readback security feature for the SRAM.\nA readback of each memory write or read request will be performed and a comparison happens.\nAny other value than kMultiBitBool4False written to this field is interpreted as kMultiBitBool4True."]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for ReadbackWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadbackWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ReadbackWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadbackRegwenReadVal(u32);
    impl ReadbackRegwenReadVal {
        #[doc = "When cleared to zero, !!READBACK can not be written anymore."]
        #[inline(always)]
        pub fn readback_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ReadbackRegwenWriteVal {
            ReadbackRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ReadbackRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadbackRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ReadbackRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadbackRegwenWriteVal(u32);
    impl ReadbackRegwenWriteVal {
        #[doc = "When cleared to zero, !!READBACK can not be written anymore."]
        #[inline(always)]
        pub fn readback_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ReadbackRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadbackRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ReadbackRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ScrKeyRotatedReadVal(u32);
    impl ScrKeyRotatedReadVal {
        #[doc = "This status register is similar to !!SCR_KEY_VALID with the difference that the status is multibit encoded,\nSW clearable and sticky (i.e., HW does not auto-clear the register except during escalation). That way,\nSW can use this for a hardened acknowledgement mechanism where it clears the register before requesting a key.\n\nkMultiBitBool4True indicates that a valid scrambling key has been obtained from OTP.\nWrite kMultiBitBool4True to clear."]
        #[inline(always)]
        pub fn success(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ScrKeyRotatedWriteVal {
            ScrKeyRotatedWriteVal(self.0)
        }
    }
    impl From<u32> for ScrKeyRotatedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ScrKeyRotatedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ScrKeyRotatedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ScrKeyRotatedWriteVal(u32);
    impl ScrKeyRotatedWriteVal {
        #[doc = "This status register is similar to !!SCR_KEY_VALID with the difference that the status is multibit encoded,\nSW clearable and sticky (i.e., HW does not auto-clear the register except during escalation). That way,\nSW can use this for a hardened acknowledgement mechanism where it clears the register before requesting a key.\n\nkMultiBitBool4True indicates that a valid scrambling key has been obtained from OTP.\nWrite kMultiBitBool4True to clear."]
        #[inline(always)]
        pub fn success(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for ScrKeyRotatedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ScrKeyRotatedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ScrKeyRotatedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "This bit is set to 1 if a fatal bus integrity fault is detected.\nThis error triggers a fatal_error alert.\nThis condition is terminal."]
        #[inline(always)]
        pub fn bus_integ_error(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit is set to 1 if a the initialization counter has reached an invalid state.\nThis error triggers a fatal_error alert.\nThis condition is terminal."]
        #[inline(always)]
        pub fn init_error(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Set to 1 if the sram controller has received an escalate request.\nIf this is set to 1, the scrambling keys have been reset to the default values\nand all subsequent memory requests will be blocked.\nThis condition is terminal."]
        #[inline(always)]
        pub fn escalated(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Set to 1 if a new scrambling key has been successfully obtained from OTP.\nNote that if this is set to 0, the SRAM contents are still scrambled, but a\ndefault all-zero key and nonce are used to do so."]
        #[inline(always)]
        pub fn scr_key_valid(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Set to 1 if the scrambling key has been derived from a valid key seed in OTP.\nIf !!STATUS.SCR_KEY_VALID is set to 1, !!STATUS.SCR_KEY_SEED_VALID should be 1\nexcept for cases where the scrambling key seeds have not yet been provisioned to\nOTP. In such a case, the scrambling key is still ephemeral (i.e., it is derived\nusing entropy from CSRNG), but a default all-zero value is used as the key seed."]
        #[inline(always)]
        pub fn scr_key_seed_valid(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Set to 1 if the hardware initialization triggered via !!CTRL.INIT has completed."]
        #[inline(always)]
        pub fn init_done(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit is set to 1 if a SRAM readback check failed.\nThis error triggers a fatal_error alert.\nThis condition is terminal."]
        #[inline(always)]
        pub fn readback_error(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit is set to 1 if a multi bit encoding error has been detected inside the RAM modules.\nThis error triggers a fatal_error alert.\nThis condition is terminal."]
        #[inline(always)]
        pub fn sram_alert(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
    }
    impl From<u32> for StatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: StatusReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    pub mod selector {}
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ExecRegwen =
        ureg::ReadWriteReg32<1, crate::regs::ExecRegwenReadVal, crate::regs::ExecRegwenWriteVal>;
    pub type Exec = ureg::ReadWriteReg32<9, crate::regs::ExecReadVal, crate::regs::ExecWriteVal>;
    pub type CtrlRegwen =
        ureg::ReadWriteReg32<1, crate::regs::CtrlRegwenReadVal, crate::regs::CtrlRegwenWriteVal>;
    pub type Ctrl = ureg::WriteOnlyReg32<0, crate::regs::CtrlWriteVal>;
    pub type ScrKeyRotated = ureg::ReadWriteReg32<
        9,
        crate::regs::ScrKeyRotatedReadVal,
        crate::regs::ScrKeyRotatedWriteVal,
    >;
    pub type ReadbackRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ReadbackRegwenReadVal,
        crate::regs::ReadbackRegwenWriteVal,
    >;
    pub type Readback =
        ureg::ReadWriteReg32<9, crate::regs::ReadbackReadVal, crate::regs::ReadbackWriteVal>;
}
