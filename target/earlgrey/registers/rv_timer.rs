#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RvTimer {
    _priv: (),
}
impl RvTimer {
    pub const PTR: *mut u32 = 0x40100000 as *mut u32;
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
    #[doc = "Control register\n\nRead value: [`regs::Ctrl0ReadVal`]; Write value: [`regs::Ctrl0WriteVal`]"]
    #[inline(always)]
    pub fn ctrl0(&self) -> ureg::RegRef<crate::meta::Ctrl0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable\n\nRead value: [`regs::IntrEnable0ReadVal`]; Write value: [`regs::IntrEnable0WriteVal`]"]
    #[inline(always)]
    pub fn intr_enable0(&self) -> ureg::RegRef<crate::meta::IntrEnable0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x100 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Status\n\nRead value: [`regs::IntrState0ReadVal`]; Write value: [`regs::IntrState0WriteVal`]"]
    #[inline(always)]
    pub fn intr_state0(&self) -> ureg::RegRef<crate::meta::IntrState0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x104 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt test register\n\nRead value: [`regs::IntrTest0ReadVal`]; Write value: [`regs::IntrTest0WriteVal`]"]
    #[inline(always)]
    pub fn intr_test0(&self) -> ureg::RegRef<crate::meta::IntrTest0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x108 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration for Hart 0\n\nRead value: [`regs::Cfg0ReadVal`]; Write value: [`regs::Cfg0WriteVal`]"]
    #[inline(always)]
    pub fn cfg0(&self) -> ureg::RegRef<crate::meta::Cfg0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timer value Lower\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn timer_v_lower0(&self) -> ureg::RegRef<crate::meta::TimerVLower0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x110 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timer value Upper\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn timer_v_upper0(&self) -> ureg::RegRef<crate::meta::TimerVUpper0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x114 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timer value Lower\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn compare_lower0_0(&self) -> ureg::RegRef<crate::meta::CompareLower00, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x118 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timer value Upper\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn compare_upper0_0(&self) -> ureg::RegRef<crate::meta::CompareUpper00, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x11c / core::mem::size_of::<u32>()),
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
        pub fn fatal_fault(self, val: bool) -> Self {
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
    pub struct Cfg0ReadVal(u32);
    impl Cfg0ReadVal {
        #[doc = "Prescaler to generate tick"]
        #[inline(always)]
        pub fn prescale(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Incremental value for each tick"]
        #[inline(always)]
        pub fn step(&self) -> u32 {
            (self.0 >> 16) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Cfg0WriteVal {
            Cfg0WriteVal(self.0)
        }
    }
    impl From<u32> for Cfg0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Cfg0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Cfg0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Cfg0WriteVal(u32);
    impl Cfg0WriteVal {
        #[doc = "Prescaler to generate tick"]
        #[inline(always)]
        pub fn prescale(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 0)) | ((val & 0xfff) << 0))
        }
        #[doc = "Incremental value for each tick"]
        #[inline(always)]
        pub fn step(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 16)) | ((val & 0xff) << 16))
        }
    }
    impl From<u32> for Cfg0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Cfg0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Cfg0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ctrl0ReadVal(u32);
    impl Ctrl0ReadVal {
        #[doc = "If 1, timer operates"]
        #[inline(always)]
        pub fn active0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ctrl0WriteVal {
            Ctrl0WriteVal(self.0)
        }
    }
    impl From<u32> for Ctrl0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ctrl0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ctrl0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ctrl0WriteVal(u32);
    impl Ctrl0WriteVal {
        #[doc = "If 1, timer operates"]
        #[inline(always)]
        pub fn active0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for Ctrl0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ctrl0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ctrl0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnable0ReadVal(u32);
    impl IntrEnable0ReadVal {
        #[doc = "Interrupt Enable for timer"]
        #[inline(always)]
        pub fn ie0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrEnable0WriteVal {
            IntrEnable0WriteVal(self.0)
        }
    }
    impl From<u32> for IntrEnable0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnable0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnable0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnable0WriteVal(u32);
    impl IntrEnable0WriteVal {
        #[doc = "Interrupt Enable for timer"]
        #[inline(always)]
        pub fn ie0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for IntrEnable0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnable0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnable0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrState0ReadVal(u32);
    impl IntrState0ReadVal {
        #[doc = "Interrupt status for timer"]
        #[inline(always)]
        pub fn is0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrState0WriteVal {
            IntrState0WriteVal(self.0)
        }
    }
    impl From<u32> for IntrState0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrState0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrState0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrState0WriteVal(u32);
    impl IntrState0WriteVal {
        #[doc = "Interrupt status for timer"]
        #[inline(always)]
        pub fn is0_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for IntrState0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrState0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrState0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrTest0WriteVal(u32);
    impl IntrTest0WriteVal {
        #[doc = "Interrupt test for timer"]
        #[inline(always)]
        pub fn t0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for IntrTest0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrTest0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrTest0WriteVal) -> u32 {
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
    pub type Ctrl0 = ureg::ReadWriteReg32<0, crate::regs::Ctrl0ReadVal, crate::regs::Ctrl0WriteVal>;
    pub type IntrEnable0 =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnable0ReadVal, crate::regs::IntrEnable0WriteVal>;
    pub type IntrState0 =
        ureg::ReadWriteReg32<0, crate::regs::IntrState0ReadVal, crate::regs::IntrState0WriteVal>;
    pub type IntrTest0 = ureg::WriteOnlyReg32<0, crate::regs::IntrTest0WriteVal>;
    pub type Cfg0 =
        ureg::ReadWriteReg32<0x10000, crate::regs::Cfg0ReadVal, crate::regs::Cfg0WriteVal>;
    pub type TimerVLower0 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TimerVUpper0 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type CompareLower00 = ureg::ReadWriteReg32<0xffffffff, u32, u32>;
    pub type CompareUpper00 = ureg::ReadWriteReg32<0xffffffff, u32, u32>;
}
