#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Otbn {
    _priv: (),
}
impl Otbn {
    pub const PTR: *mut u32 = 0x41130000 as *mut u32;
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
    #[doc = "Interrupt State Register\n\nRead value: [`regs::IntrStateReadVal`]; Write value: [`regs::IntrStateWriteVal`]"]
    #[inline(always)]
    pub fn intr_state(&self) -> ureg::RegRef<crate::meta::IntrState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable Register\n\nRead value: [`regs::IntrEnableReadVal`]; Write value: [`regs::IntrEnableWriteVal`]"]
    #[inline(always)]
    pub fn intr_enable(&self) -> ureg::RegRef<crate::meta::IntrEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Test Register\n\nRead value: [`regs::IntrTestReadVal`]; Write value: [`regs::IntrTestWriteVal`]"]
    #[inline(always)]
    pub fn intr_test(&self) -> ureg::RegRef<crate::meta::IntrTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert Test Register\n\nRead value: [`regs::AlertTestReadVal`]; Write value: [`regs::AlertTestWriteVal`]"]
    #[inline(always)]
    pub fn alert_test(&self) -> ureg::RegRef<crate::meta::AlertTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Register\n\nA command initiates an OTBN operation. While performing the operation,\nOTBN is busy; the !!STATUS register reflects that.\n\nAll operations signal their completion by raising the done\ninterrupt; alternatively, software may poll the !!STATUS register.\n\nWrites are ignored if OTBN is not idle.\nUnrecognized commands are ignored.\n\nRead value: [`regs::CmdReadVal`]; Write value: [`regs::CmdWriteVal`]"]
    #[inline(always)]
    pub fn cmd(&self) -> ureg::RegRef<crate::meta::Cmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control Register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status Register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Operation Result Register\n\nDescribes the errors detected during an operation.\n\nRefer to the \"List of Errors\" section for a detailed description of the\nerrors.\n\nThe host CPU can clear this register when OTBN is not running,\nby writing any value. Write attempts while OTBN is running are ignored.\n\nRead value: [`regs::ErrBitsReadVal`]; Write value: [`regs::ErrBitsWriteVal`]"]
    #[inline(always)]
    pub fn err_bits(&self) -> ureg::RegRef<crate::meta::ErrBits, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Fatal Alert Cause Register\n\nDescribes any errors that led to a fatal alert.\nA fatal error puts OTBN in locked state; the value of this register\ndoes not change until OTBN is reset.\n\nRefer to the \"List of Errors\" section for a detailed description of the\nerrors.\n\nRead value: [`regs::FatalAlertCauseReadVal`]; Write value: [`regs::FatalAlertCauseWriteVal`]"]
    #[inline(always)]
    pub fn fatal_alert_cause(&self) -> ureg::RegRef<crate::meta::FatalAlertCause, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Instruction Count Register\n\nReturns the number of instructions executed in the current or last\noperation. The counter saturates at 2^32-1 and is reset to 0 at the\nstart of a new operation.\n\nOnly the EXECUTE operation counts instructions; for all other operations\nthis register remains at 0. Instructions triggering an error do not\ncount towards the total.\n\nAlways reads as 0 if OTBN is locked.\n\nThe host CPU can clear this register when OTBN is not running,\nby writing any value. Write attempts while OTBN is running are ignored.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn insn_cnt(&self) -> ureg::RegRef<crate::meta::InsnCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A 32-bit CRC checksum of data written to memory\n\nSee the \"Memory Load Integrity\" section of the manual for full details.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn load_checksum(&self) -> ureg::RegRef<crate::meta::LoadChecksum, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Instruction Memory Access\n\nThe instruction memory may only be accessed through this window\nwhile OTBN is idle.\n\nIf OTBN is busy or locked, read accesses return 0 and write accesses\nare ignored.\nIf OTBN is busy, any access additionally triggers an\nILLEGAL_BUS_ACCESS fatal error.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn imem(&self) -> ureg::Array<2048, ureg::RegRef<crate::meta::Imem, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Data Memory Access\n\nThe data memory may only be accessed through this window while OTBN\nis idle.\n\nIf OTBN is busy or locked, read accesses return 0 and write accesses\nare ignored.\nIf OTBN is busy, any access additionally triggers an\nILLEGAL_BUS_ACCESS fatal error.\n\nNote that DMEM is actually 4kiB in size, but only the first 3kiB of\nthe memory is visible through this register interface.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn dmem(&self) -> ureg::Array<768, ureg::RegRef<crate::meta::Dmem, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x8000 / core::mem::size_of::<u32>()),
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
        pub fn fatal(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
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
    pub struct CmdWriteVal(u32);
    impl CmdWriteVal {
        #[doc = "The operation to perform.\n\n| Value | Name          | Description |\n|:------|:--------------|:------------|\n| 0xd8  | EXECUTE       | Starts the execution of the program stored in the instruction memory, starting at address zero. |\n| 0xc3  | SEC_WIPE_DMEM | Securely removes all contents from the data memory. |\n| 0x1e  | SEC_WIPE_IMEM | Securely removes all contents from the instruction  memory. |"]
        #[inline(always)]
        pub fn cmd(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for CmdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlReadVal(u32);
    impl CtrlReadVal {
        #[doc = "Controls the reaction to software errors.\n\nWhen set software errors produce fatal errors, rather than\nrecoverable errors.\n\nWrites are ignored if OTBN is not idle."]
        #[inline(always)]
        pub fn software_errs_fatal(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlWriteVal {
            CtrlWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlWriteVal(u32);
    impl CtrlWriteVal {
        #[doc = "Controls the reaction to software errors.\n\nWhen set software errors produce fatal errors, rather than\nrecoverable errors.\n\nWrites are ignored if OTBN is not idle."]
        #[inline(always)]
        pub fn software_errs_fatal(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
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
    pub struct ErrBitsReadVal(u32);
    impl ErrBitsReadVal {
        #[doc = "A `BAD_DATA_ADDR` error was observed."]
        #[inline(always)]
        pub fn bad_data_addr(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "A `BAD_INSN_ADDR` error was observed."]
        #[inline(always)]
        pub fn bad_insn_addr(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "A `CALL_STACK` error was observed."]
        #[inline(always)]
        pub fn call_stack(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "An `ILLEGAL_INSN` error was observed."]
        #[inline(always)]
        pub fn illegal_insn(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "A `LOOP` error was observed."]
        #[inline(always)]
        pub fn loop_(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "A `KEY_INVALID` error was observed."]
        #[inline(always)]
        pub fn key_invalid(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "An `RND_REP_CHK_FAIL` error was observed."]
        #[inline(always)]
        pub fn rnd_rep_chk_fail(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "An `RND_FIPS_CHK_FAIL` error was observed."]
        #[inline(always)]
        pub fn rnd_fips_chk_fail(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "A `IMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn imem_intg_violation(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "A `DMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn dmem_intg_violation(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "A `REG_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn reg_intg_violation(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "A `BUS_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn bus_intg_violation(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "A `BAD_INTERNAL_STATE` error was observed."]
        #[inline(always)]
        pub fn bad_internal_state(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "An `ILLEGAL_BUS_ACCESS` error was observed."]
        #[inline(always)]
        pub fn illegal_bus_access(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "A `LIFECYCLE_ESCALATION` error was observed."]
        #[inline(always)]
        pub fn lifecycle_escalation(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "A `FATAL_SOFTWARE` error was observed."]
        #[inline(always)]
        pub fn fatal_software(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrBitsWriteVal {
            ErrBitsWriteVal(self.0)
        }
    }
    impl From<u32> for ErrBitsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrBitsReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrBitsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrBitsWriteVal(u32);
    impl ErrBitsWriteVal {
        #[doc = "A `BAD_DATA_ADDR` error was observed."]
        #[inline(always)]
        pub fn bad_data_addr(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "A `BAD_INSN_ADDR` error was observed."]
        #[inline(always)]
        pub fn bad_insn_addr(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "A `CALL_STACK` error was observed."]
        #[inline(always)]
        pub fn call_stack(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "An `ILLEGAL_INSN` error was observed."]
        #[inline(always)]
        pub fn illegal_insn(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "A `LOOP` error was observed."]
        #[inline(always)]
        pub fn loop_(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "A `KEY_INVALID` error was observed."]
        #[inline(always)]
        pub fn key_invalid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "An `RND_REP_CHK_FAIL` error was observed."]
        #[inline(always)]
        pub fn rnd_rep_chk_fail(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "An `RND_FIPS_CHK_FAIL` error was observed."]
        #[inline(always)]
        pub fn rnd_fips_chk_fail(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "A `IMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn imem_intg_violation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "A `DMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn dmem_intg_violation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "A `REG_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn reg_intg_violation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "A `BUS_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn bus_intg_violation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "A `BAD_INTERNAL_STATE` error was observed."]
        #[inline(always)]
        pub fn bad_internal_state(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "An `ILLEGAL_BUS_ACCESS` error was observed."]
        #[inline(always)]
        pub fn illegal_bus_access(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "A `LIFECYCLE_ESCALATION` error was observed."]
        #[inline(always)]
        pub fn lifecycle_escalation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "A `FATAL_SOFTWARE` error was observed."]
        #[inline(always)]
        pub fn fatal_software(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
    }
    impl From<u32> for ErrBitsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrBitsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrBitsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FatalAlertCauseReadVal(u32);
    impl FatalAlertCauseReadVal {
        #[doc = "A `IMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn imem_intg_violation(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "A `DMEM_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn dmem_intg_violation(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "A `REG_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn reg_intg_violation(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "A `BUS_INTG_VIOLATION` error was observed."]
        #[inline(always)]
        pub fn bus_intg_violation(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "A `BAD_INTERNAL_STATE` error was observed."]
        #[inline(always)]
        pub fn bad_internal_state(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "A `ILLEGAL_BUS_ACCESS` error was observed."]
        #[inline(always)]
        pub fn illegal_bus_access(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "A `LIFECYCLE_ESCALATION` error was observed."]
        #[inline(always)]
        pub fn lifecycle_escalation(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "A `FATAL_SOFTWARE` error was observed."]
        #[inline(always)]
        pub fn fatal_software(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
    }
    impl From<u32> for FatalAlertCauseReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FatalAlertCauseReadVal> for u32 {
        #[inline(always)]
        fn from(val: FatalAlertCauseReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.done is set."]
        #[inline(always)]
        pub fn done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrEnableWriteVal {
            IntrEnableWriteVal(self.0)
        }
    }
    impl From<u32> for IntrEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableWriteVal(u32);
    impl IntrEnableWriteVal {
        #[doc = "Enable interrupt when !!INTR_STATE.done is set."]
        #[inline(always)]
        pub fn done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for IntrEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateReadVal(u32);
    impl IntrStateReadVal {
        #[doc = "OTBN has completed the operation."]
        #[inline(always)]
        pub fn done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrStateWriteVal {
            IntrStateWriteVal(self.0)
        }
    }
    impl From<u32> for IntrStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateWriteVal(u32);
    impl IntrStateWriteVal {
        #[doc = "OTBN has completed the operation."]
        #[inline(always)]
        pub fn done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for IntrStateWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrTestWriteVal(u32);
    impl IntrTestWriteVal {
        #[doc = "Write 1 to force !!INTR_STATE.done to 1."]
        #[inline(always)]
        pub fn done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for IntrTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "Indicates the current operational state OTBN is in.\n\nAll BUSY values represent an operation started by a write to the\n!!CMD register.\n\n| Value | Name               | Description                                           |\n|:------|:-------------------|:------------------------------------------------------|\n| 0x00  | IDLE               | OTBN is idle: it is not performing any action.        |\n| 0x01  | BUSY_EXECUTE       | OTBN is busy executing software.                      |\n| 0x02  | BUSY_SEC_WIPE_DMEM | OTBN is busy securely wiping the data memory.         |\n| 0x03  | BUSY_SEC_WIPE_IMEM | OTBN is busy securely wiping the instruction memory.  |\n| 0x04  | BUSY_SEC_WIPE_INT  | OTBN is busy securely wiping the internal state.      |\n| 0xFF  | LOCKED             | OTBN is locked as reaction to a fatal error, and must be reset to unlock it again. See also the section \"Reaction to Fatal Errors\". |\n"]
        #[inline(always)]
        pub fn status(&self) -> u32 {
            (self.0 >> 0) & 0xff
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
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Cmd = ureg::WriteOnlyReg32<0, crate::regs::CmdWriteVal>;
    pub type Ctrl = ureg::ReadWriteReg32<0, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ErrBits =
        ureg::ReadWriteReg32<0, crate::regs::ErrBitsReadVal, crate::regs::ErrBitsWriteVal>;
    pub type FatalAlertCause = ureg::ReadOnlyReg32<crate::regs::FatalAlertCauseReadVal>;
    pub type InsnCnt = ureg::ReadWriteReg32<0, u32, u32>;
    pub type LoadChecksum = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Imem = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Dmem = ureg::ReadWriteReg32<0, u32, u32>;
}
