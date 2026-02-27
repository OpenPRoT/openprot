#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RvDm {
    _priv: (),
}
impl RvDm {
    pub const PTR: *mut u32 = 0x10000 as *mut u32;
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
    #[doc = "Written by a hart whenever it enters debug mode.\n\nA hart entering debug mode must write its ID to this address to indicate that it has halted.\nWhen the debug module triggers a debug mode (aka halt) request to the hart, the hart will jump to the debug ROM.\nIn that debug ROM, the hart must write its ID here to acknowledge completion of the request.\nWhen the write is received, the debug module will record that the hart is halted in its status register.\nIn addition, the debug module may begin to accept abstract commands that run on that hart.\n\nNote that this write upon entering debug mode is also important for indicating that a sequence of debug mode instructions completed.\nIn that case, the hart would write to this address while it was already halted.\n\nRead value: [`regs::HaltedReadVal`]; Write value: [`regs::HaltedWriteVal`]"]
    #[inline(always)]
    pub fn halted(&self) -> ureg::RegRef<crate::meta::Halted, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x100 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Written by a hart to acknowledge a command.\n\nA hart that receives an abstract command (indicated by its corresponds !!FLAGS register) must write to this address to acknowledge it received the command.\nThe value written is unused, but it is conventionally 0.\n\nUpon receiving the write, the debug module will reset the GO field in the selected hart's !!FLAGS register.\nThe debug module will transition to a state where it awaits the write to !!HALTED to indicate the command has completed.\n\nRead value: [`regs::GoingReadVal`]; Write value: [`regs::GoingWriteVal`]"]
    #[inline(always)]
    pub fn going(&self) -> ureg::RegRef<crate::meta::Going, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x108 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Written by a hart to acknowledge a resume request.\n\nA hart that receives the command to resume from debug mode (via the RESUME flag in its !!FLAGS register) must write its ID to this address.\n\nThis write tells the debug module that the command has been acknowledged, and the hart is no longer halted.\n\nRead value: [`regs::ResumingReadVal`]; Write value: [`regs::ResumingWriteVal`]"]
    #[inline(always)]
    pub fn resuming(&self) -> ureg::RegRef<crate::meta::Resuming, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x110 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "An exception was triggered while the core was in debug mode.\n\nRead value: [`regs::ExceptionReadVal`]; Write value: [`regs::ExceptionWriteVal`]"]
    #[inline(always)]
    pub fn exception(&self) -> ureg::RegRef<crate::meta::Exception, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x118 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A jump instruction the hart executes to begin a command.\n\nWhen a debugger sends an abstract command to the debug module, the debug module indicates the instruction to run here, which is invariably a jump.\nThe hart receiving the command must execute the instruction at this address after acknowledging the command with the write to !!GOING.\n\nSimilarly, when a debugger requests that a hart resume, the debug module supplies a jump instruction to execute here.\nIn the resume request case, the hart must execute the indicated instruction after acknolwedging the request with the write to !!RESUMING.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn whereto(&self) -> ureg::RegRef<crate::meta::Whereto, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x300 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A ROM containing instructions for implementing abstract commands.\n\nThe hart executes these instructions at the debug modules behest.\nThe debug module's jump instruction at !!WHERETO will land here, except for the AccessRegister command with the \"postexec\" bit set and the \"transfer\" bit unset.\nSee the RISC-V Debug Specification for more information on the encoding of abstract commands.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn abstractcmd(&self) -> ureg::Array<10, ureg::RegRef<crate::meta::Abstractcmd, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x338 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A buffer for the debugger to write small debug mode programs.\n\nThe hart may run these programs by command from the debugger.\nSee the RISC-V Debug Specification for more information about the Program Buffer and how it is used with abstract commands and the \"postexec\" bit.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn program_buffer(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::ProgramBuffer, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x360 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Message Registers for passing arguments and/or return values for abstract commands.\n\nSee the RISC-V Debug Specification for more information about Message Registers and their relationship to abstract commands.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn dataaddr(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::Dataaddr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x380 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flags indicating what a hart in debug mode should do.\n\nThese flags are how a debug module signals whether a hart should execute an abstract command, resume from debug mode, or remain idle.\n\nEach hart has its own FLAGS register that is a single byte.\nBit 0 is the GO flag, indicating a request for the selected hart to execute the command.\nBit 1 is the RESUME flag, indication a request for the selected hart to resume from halt/ debug mode.\nThe other bits are reserved.\n\nThe hart finds its own FLAGS register by taking the base address of this group and adding the hart's ID to the byte address.\n\nThese are written by the debug module.\nWhen a selected hart writes the !!GOING register, the corresponding GO flag is cleared.\nWhen a selected hart writes the !!RESUMING register, the corresponding RESUME flag is cleared.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn flags(&self) -> ureg::Array<256, ureg::RegRef<crate::meta::Flags, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x400 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Access window into the debug ROM.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rom(&self) -> ureg::Array<512, ureg::RegRef<crate::meta::Rom, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x800 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct ExceptionWriteVal(u32);
    impl ExceptionWriteVal {
        #[inline(always)]
        pub fn exception(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ExceptionWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExceptionWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ExceptionWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct GoingWriteVal(u32);
    impl GoingWriteVal {
        #[inline(always)]
        pub fn going(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for GoingWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<GoingWriteVal> for u32 {
        #[inline(always)]
        fn from(val: GoingWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HaltedWriteVal(u32);
    impl HaltedWriteVal {
        #[inline(always)]
        pub fn halted(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for HaltedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HaltedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HaltedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResumingWriteVal(u32);
    impl ResumingWriteVal {
        #[inline(always)]
        pub fn resuming(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for ResumingWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResumingWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ResumingWriteVal) -> u32 {
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
    pub type Halted = ureg::WriteOnlyReg32<0, crate::regs::HaltedWriteVal>;
    pub type Going = ureg::WriteOnlyReg32<0, crate::regs::GoingWriteVal>;
    pub type Resuming = ureg::WriteOnlyReg32<0, crate::regs::ResumingWriteVal>;
    pub type Exception = ureg::WriteOnlyReg32<0, crate::regs::ExceptionWriteVal>;
    pub type Whereto = ureg::ReadOnlyReg32<u32>;
    pub type Abstractcmd = ureg::ReadOnlyReg32<u32>;
    pub type ProgramBuffer = ureg::ReadOnlyReg32<u32>;
    pub type Dataaddr = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Flags = ureg::ReadOnlyReg32<u32>;
    pub type Rom = ureg::ReadOnlyReg32<u32>;
}
