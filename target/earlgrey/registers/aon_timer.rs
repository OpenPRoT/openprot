#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct AonTimerAon {
    _priv: (),
}
impl AonTimerAon {
    pub const PTR: *mut u32 = 0x40470000 as *mut u32;
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
    #[doc = "Wakeup Timer Control register\n\nRead value: [`regs::WkupCtrlReadVal`]; Write value: [`regs::WkupCtrlWriteVal`]"]
    #[inline(always)]
    pub fn wkup_ctrl(&self) -> ureg::RegRef<crate::meta::WkupCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Wakeup Timer Threshold Register (bits 63 - 32)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wkup_thold_hi(&self) -> ureg::RegRef<crate::meta::WkupTholdHi, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Wakeup Timer Threshold Register (bits 31 - 0)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wkup_thold_lo(&self) -> ureg::RegRef<crate::meta::WkupTholdLo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Wakeup Timer Count Register (bits 63 - 32)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wkup_count_hi(&self) -> ureg::RegRef<crate::meta::WkupCountHi, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Wakeup Timer Count Register (bits 31 - 0)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wkup_count_lo(&self) -> ureg::RegRef<crate::meta::WkupCountLo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Watchdog Timer Write Enable Register\n\nRead value: [`regs::WdogRegwenReadVal`]; Write value: [`regs::WdogRegwenWriteVal`]"]
    #[inline(always)]
    pub fn wdog_regwen(&self) -> ureg::RegRef<crate::meta::WdogRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Watchdog Timer Control register\n\nRead value: [`regs::WdogCtrlReadVal`]; Write value: [`regs::WdogCtrlWriteVal`]"]
    #[inline(always)]
    pub fn wdog_ctrl(&self) -> ureg::RegRef<crate::meta::WdogCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Watchdog Timer Bark Threshold Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wdog_bark_thold(&self) -> ureg::RegRef<crate::meta::WdogBarkThold, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Watchdog Timer Bite Threshold Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wdog_bite_thold(&self) -> ureg::RegRef<crate::meta::WdogBiteThold, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Watchdog Timer Count Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wdog_count(&self) -> ureg::RegRef<crate::meta::WdogCount, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt State Register\n\nRead value: [`regs::IntrStateReadVal`]; Write value: [`regs::IntrStateWriteVal`]"]
    #[inline(always)]
    pub fn intr_state(&self) -> ureg::RegRef<crate::meta::IntrState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Test Register\n\nRead value: [`regs::IntrTestReadVal`]; Write value: [`regs::IntrTestWriteVal`]"]
    #[inline(always)]
    pub fn intr_test(&self) -> ureg::RegRef<crate::meta::IntrTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Wakeup request status\n\nRead value: [`regs::WkupCauseReadVal`]; Write value: [`regs::WkupCauseWriteVal`]"]
    #[inline(always)]
    pub fn wkup_cause(&self) -> ureg::RegRef<crate::meta::WkupCause, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
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
    pub struct IntrStateReadVal(u32);
    impl IntrStateReadVal {
        #[doc = "Raised if the wakeup timer has hit the specified threshold"]
        #[inline(always)]
        pub fn wkup_timer_expired(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Raised if the watchdog timer has hit the bark threshold"]
        #[inline(always)]
        pub fn wdog_timer_bark(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
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
        #[doc = "Raised if the wakeup timer has hit the specified threshold"]
        #[inline(always)]
        pub fn wkup_timer_expired_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Raised if the watchdog timer has hit the bark threshold"]
        #[inline(always)]
        pub fn wdog_timer_bark_clear(self) -> Self {
            Self(self.0 | (1 << 1))
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
        #[doc = "Write 1 to force wkup_timer_expired interrupt"]
        #[inline(always)]
        pub fn wkup_timer_expired(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force wdog_timer_bark interrupt"]
        #[inline(always)]
        pub fn wdog_timer_bark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
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
    pub struct WdogCtrlReadVal(u32);
    impl WdogCtrlReadVal {
        #[doc = "When set to 1, the watchdog timer will count"]
        #[inline(always)]
        pub fn enable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "When set to 1, the watchdog timer will not count during sleep"]
        #[inline(always)]
        pub fn pause_in_sleep(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WdogCtrlWriteVal {
            WdogCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for WdogCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WdogCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: WdogCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WdogCtrlWriteVal(u32);
    impl WdogCtrlWriteVal {
        #[doc = "When set to 1, the watchdog timer will count"]
        #[inline(always)]
        pub fn enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "When set to 1, the watchdog timer will not count during sleep"]
        #[inline(always)]
        pub fn pause_in_sleep(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for WdogCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WdogCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WdogCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WdogRegwenReadVal(u32);
    impl WdogRegwenReadVal {
        #[doc = "Once cleared, the watchdog configuration will be locked until the next reset"]
        #[inline(always)]
        pub fn regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WdogRegwenWriteVal {
            WdogRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for WdogRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WdogRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: WdogRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WdogRegwenWriteVal(u32);
    impl WdogRegwenWriteVal {
        #[doc = "Once cleared, the watchdog configuration will be locked until the next reset"]
        #[inline(always)]
        pub fn regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for WdogRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WdogRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WdogRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCauseReadVal(u32);
    impl WkupCauseReadVal {
        #[doc = "AON timer requested wakeup, write 0 to clear"]
        #[inline(always)]
        pub fn cause(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupCauseWriteVal {
            WkupCauseWriteVal(self.0)
        }
    }
    impl From<u32> for WkupCauseReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCauseReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCauseReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCauseWriteVal(u32);
    impl WkupCauseWriteVal {
        #[doc = "AON timer requested wakeup, write 0 to clear"]
        #[inline(always)]
        pub fn cause_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for WkupCauseWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCauseWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCauseWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCtrlReadVal(u32);
    impl WkupCtrlReadVal {
        #[doc = "When set to 1, the wakeup timer will count"]
        #[inline(always)]
        pub fn enable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Pre-scaler value for wakeup timer count"]
        #[inline(always)]
        pub fn prescaler(&self) -> u32 {
            (self.0 >> 1) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupCtrlWriteVal {
            WkupCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for WkupCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCtrlWriteVal(u32);
    impl WkupCtrlWriteVal {
        #[doc = "When set to 1, the wakeup timer will count"]
        #[inline(always)]
        pub fn enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Pre-scaler value for wakeup timer count"]
        #[inline(always)]
        pub fn prescaler(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 1)) | ((val & 0xfff) << 1))
        }
    }
    impl From<u32> for WkupCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCtrlWriteVal) -> u32 {
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
    pub type WkupCtrl =
        ureg::ReadWriteReg32<0, crate::regs::WkupCtrlReadVal, crate::regs::WkupCtrlWriteVal>;
    pub type WkupTholdHi = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WkupTholdLo = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WkupCountHi = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WkupCountLo = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WdogRegwen =
        ureg::ReadWriteReg32<1, crate::regs::WdogRegwenReadVal, crate::regs::WdogRegwenWriteVal>;
    pub type WdogCtrl =
        ureg::ReadWriteReg32<0, crate::regs::WdogCtrlReadVal, crate::regs::WdogCtrlWriteVal>;
    pub type WdogBarkThold = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WdogBiteThold = ureg::ReadWriteReg32<0, u32, u32>;
    pub type WdogCount = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type WkupCause =
        ureg::ReadWriteReg32<0, crate::regs::WkupCauseReadVal, crate::regs::WkupCauseWriteVal>;
}
