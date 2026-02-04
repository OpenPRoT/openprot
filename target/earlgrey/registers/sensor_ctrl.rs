#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SensorCtrlAon {
    _priv: (),
}
impl SensorCtrlAon {
    pub const PTR: *mut u32 = 0x40490000 as *mut u32;
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
    #[doc = "Controls the configurability of !!FATAL_ALERT_EN register.\n\nRead value: [`regs::CfgRegwenReadVal`]; Write value: [`regs::CfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn cfg_regwen(&self) -> ureg::RegRef<crate::meta::CfgRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert trigger test\n\nRead value: [`regs::AlertTrig0ReadVal`]; Write value: [`regs::AlertTrig0WriteVal`]"]
    #[inline(always)]
    pub fn alert_trig0(&self) -> ureg::RegRef<crate::meta::AlertTrig0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Each multibit value enables a corresponding alert.\n\nRead value: [`regs::AlertEnReadVal`]; Write value: [`regs::AlertEnWriteVal`]"]
    #[inline(always)]
    pub fn alert_en(&self) -> ureg::Array<11, ureg::RegRef<crate::meta::AlertEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Each bit marks a corresponding alert as fatal or recoverable.\n\nNote that alerts are ignored if they are not enabled in !!ALERT_EN.\n\nRead value: [`regs::FatalAlertEn0ReadVal`]; Write value: [`regs::FatalAlertEn0WriteVal`]"]
    #[inline(always)]
    pub fn fatal_alert_en0(&self) -> ureg::RegRef<crate::meta::FatalAlertEn0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Each bit represents a recoverable alert that has been triggered by AST.\nSince these are recoverable alerts, they can be cleared by software.\n\nRead value: [`regs::RecovAlert0ReadVal`]; Write value: [`regs::RecovAlert0WriteVal`]"]
    #[inline(always)]
    pub fn recov_alert0(&self) -> ureg::RegRef<crate::meta::RecovAlert0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Each bit represents a fatal alert that has been triggered by AST.\nSince these registers represent fatal alerts, they cannot be cleared.\n\nThe lower bits are used for ast alert events.\nThe upper bits are used for local events.\n\nRead value: [`regs::FatalAlert0ReadVal`]; Write value: [`regs::FatalAlert0WriteVal`]"]
    #[inline(always)]
    pub fn fatal_alert0(&self) -> ureg::RegRef<crate::meta::FatalAlert0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status readback for ast\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for attributes of manual pads\n\nRead value: [`regs::ManualPadAttrRegwenReadVal`]; Write value: [`regs::ManualPadAttrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn manual_pad_attr_regwen(
        &self,
    ) -> ureg::Array<4, ureg::RegRef<crate::meta::ManualPadAttrRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Attributes of manual pads.\nThis register has WARL behavior since not every pad may support each attribute.\nThe mapping of registers to pads is as follows (only supported for targets that instantiate `chip_earlgrey_asic`):\n- MANUAL_PAD_ATTR_0: CC1\n- MANUAL_PAD_ATTR_1: CC2\n- MANUAL_PAD_ATTR_2: FLASH_TEST_MODE0\n- MANUAL_PAD_ATTR_3: FLASH_TEST_MODE1\n\nRead value: [`regs::ManualPadAttrReadVal`]; Write value: [`regs::ManualPadAttrWriteVal`]"]
    #[inline(always)]
    pub fn manual_pad_attr(
        &self,
    ) -> ureg::Array<4, ureg::RegRef<crate::meta::ManualPadAttr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AlertEnReadVal(u32);
    impl AlertEnReadVal {
        #[doc = "kMultiBitBool4True - An alert event is enabled.\nkMultiBitBool4False - An alert event is disabled.\n\nAt reset, all alerts are enabled.\nThis is by design so that no alerts get missed unless they get disabled explicitly.\nFirmware can disable alerts that may be problematic for the designated use case."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertEnWriteVal {
            AlertEnWriteVal(self.0)
        }
    }
    impl From<u32> for AlertEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertEnWriteVal(u32);
    impl AlertEnWriteVal {
        #[doc = "kMultiBitBool4True - An alert event is enabled.\nkMultiBitBool4False - An alert event is disabled.\n\nAt reset, all alerts are enabled.\nThis is by design so that no alerts get missed unless they get disabled explicitly.\nFirmware can disable alerts that may be problematic for the designated use case."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for AlertEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_alert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_alert(self, val: bool) -> Self {
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
    pub struct AlertTrig0ReadVal(u32);
    impl AlertTrig0ReadVal {
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertTrig0WriteVal {
            AlertTrig0WriteVal(self.0)
        }
    }
    impl From<u32> for AlertTrig0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertTrig0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertTrig0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertTrig0WriteVal(u32);
    impl AlertTrig0WriteVal {
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Alert trigger for testing\n0 No alerts triggered\n1 Continuously trigger alert until disabled\nFor bit mapping, please see !!ALERT_TEST"]
        #[inline(always)]
        pub fn val10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
    }
    impl From<u32> for AlertTrig0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertTrig0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertTrig0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgRegwenReadVal(u32);
    impl CfgRegwenReadVal {
        #[doc = "Configuration enable."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgRegwenWriteVal {
            CfgRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CfgRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgRegwenWriteVal(u32);
    impl CfgRegwenWriteVal {
        #[doc = "Configuration enable."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CfgRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FatalAlert0ReadVal(u32);
    impl FatalAlert0ReadVal {
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
    }
    impl From<u32> for FatalAlert0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FatalAlert0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: FatalAlert0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FatalAlertEn0ReadVal(u32);
    impl FatalAlertEn0ReadVal {
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FatalAlertEn0WriteVal {
            FatalAlertEn0WriteVal(self.0)
        }
    }
    impl From<u32> for FatalAlertEn0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FatalAlertEn0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: FatalAlertEn0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FatalAlertEn0WriteVal(u32);
    impl FatalAlertEn0WriteVal {
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "1 - An alert event is fatal.\n0 - An alert event is recoverable.\n\nAt reset, all alerts are recoverable.\nThis is by design so that a false-positive alert event early in the reset sequence doesn't jam the alert until the next reset.\nFirmware can define alerts that are critical for the designated use case as fatal."]
        #[inline(always)]
        pub fn val10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
    }
    impl From<u32> for FatalAlertEn0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FatalAlertEn0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: FatalAlertEn0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.io_status_change is set."]
        #[inline(always)]
        pub fn io_status_change(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.init_status_change is set."]
        #[inline(always)]
        pub fn init_status_change(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.io_status_change is set."]
        #[inline(always)]
        pub fn io_status_change(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.init_status_change is set."]
        #[inline(always)]
        pub fn init_status_change(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
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
        #[doc = "io power status has changed"]
        #[inline(always)]
        pub fn io_status_change(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "ast init status has changed"]
        #[inline(always)]
        pub fn init_status_change(&self) -> bool {
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
        #[doc = "io power status has changed"]
        #[inline(always)]
        pub fn io_status_change_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "ast init status has changed"]
        #[inline(always)]
        pub fn init_status_change_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.io_status_change to 1."]
        #[inline(always)]
        pub fn io_status_change(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.init_status_change to 1."]
        #[inline(always)]
        pub fn init_status_change(self, val: bool) -> Self {
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
    pub struct ManualPadAttrReadVal(u32);
    impl ManualPadAttrReadVal {
        #[doc = "Enable pull-up or pull-down resistor."]
        #[inline(always)]
        pub fn pull_en(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Pull select (0: pull-down, 1: pull-up)."]
        #[inline(always)]
        pub fn pull_select(&self) -> super::enums::PullSelect {
            super::enums::PullSelect::try_from((self.0 >> 3) & 1).unwrap()
        }
        #[doc = "Disable input drivers.\nSetting this to 1 for pads that are not used as input can reduce their leakage current."]
        #[inline(always)]
        pub fn input_disable(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ManualPadAttrWriteVal {
            ManualPadAttrWriteVal(self.0)
        }
    }
    impl From<u32> for ManualPadAttrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ManualPadAttrReadVal> for u32 {
        #[inline(always)]
        fn from(val: ManualPadAttrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ManualPadAttrWriteVal(u32);
    impl ManualPadAttrWriteVal {
        #[doc = "Enable pull-up or pull-down resistor."]
        #[inline(always)]
        pub fn pull_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Pull select (0: pull-down, 1: pull-up)."]
        #[inline(always)]
        pub fn pull_select(
            self,
            f: impl FnOnce(super::enums::selector::PullSelectSelector) -> super::enums::PullSelect,
        ) -> Self {
            Self(
                (self.0 & !(1 << 3))
                    | (u32::from(f(super::enums::selector::PullSelectSelector())) << 3),
            )
        }
        #[doc = "Disable input drivers.\nSetting this to 1 for pads that are not used as input can reduce their leakage current."]
        #[inline(always)]
        pub fn input_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for ManualPadAttrWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ManualPadAttrWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ManualPadAttrWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ManualPadAttrRegwenReadVal(u32);
    impl ManualPadAttrRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MANUAL_PAD_ATTR is not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ManualPadAttrRegwenWriteVal {
            ManualPadAttrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ManualPadAttrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ManualPadAttrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ManualPadAttrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ManualPadAttrRegwenWriteVal(u32);
    impl ManualPadAttrRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MANUAL_PAD_ATTR is not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ManualPadAttrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ManualPadAttrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ManualPadAttrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovAlert0ReadVal(u32);
    impl RecovAlert0ReadVal {
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RecovAlert0WriteVal {
            RecovAlert0WriteVal(self.0)
        }
    }
    impl From<u32> for RecovAlert0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovAlert0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: RecovAlert0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovAlert0WriteVal(u32);
    impl RecovAlert0WriteVal {
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val0_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val1_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val2_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val3_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val4_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val5_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val6_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val7_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val8_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val9_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "1 - An alert event has been set\n0 - No alert event has been set"]
        #[inline(always)]
        pub fn val10_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
    }
    impl From<u32> for RecovAlert0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovAlert0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: RecovAlert0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "AST has finished initializing"]
        #[inline(always)]
        pub fn ast_init_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "IO power is ready"]
        #[inline(always)]
        pub fn io_pok(&self) -> u32 {
            (self.0 >> 1) & 3
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
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum PullSelect {
        PullDown = 0,
        PullUp = 1,
    }
    impl PullSelect {
        #[inline(always)]
        pub fn pull_down(&self) -> bool {
            *self == Self::PullDown
        }
        #[inline(always)]
        pub fn pull_up(&self) -> bool {
            *self == Self::PullUp
        }
    }
    impl TryFrom<u32> for PullSelect {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<PullSelect, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, PullSelect>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<PullSelect> for u32 {
        fn from(val: PullSelect) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct PullSelectSelector();
        impl PullSelectSelector {
            #[inline(always)]
            pub fn pull_down(&self) -> super::PullSelect {
                super::PullSelect::PullDown
            }
            #[inline(always)]
            pub fn pull_up(&self) -> super::PullSelect {
                super::PullSelect::PullUp
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type CfgRegwen =
        ureg::ReadWriteReg32<1, crate::regs::CfgRegwenReadVal, crate::regs::CfgRegwenWriteVal>;
    pub type AlertTrig0 =
        ureg::ReadWriteReg32<0, crate::regs::AlertTrig0ReadVal, crate::regs::AlertTrig0WriteVal>;
    pub type AlertEn =
        ureg::ReadWriteReg32<6, crate::regs::AlertEnReadVal, crate::regs::AlertEnWriteVal>;
    pub type FatalAlertEn0 = ureg::ReadWriteReg32<
        0,
        crate::regs::FatalAlertEn0ReadVal,
        crate::regs::FatalAlertEn0WriteVal,
    >;
    pub type RecovAlert0 =
        ureg::ReadWriteReg32<0, crate::regs::RecovAlert0ReadVal, crate::regs::RecovAlert0WriteVal>;
    pub type FatalAlert0 = ureg::ReadOnlyReg32<crate::regs::FatalAlert0ReadVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ManualPadAttrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ManualPadAttrRegwenReadVal,
        crate::regs::ManualPadAttrRegwenWriteVal,
    >;
    pub type ManualPadAttr = ureg::ReadWriteReg32<
        0,
        crate::regs::ManualPadAttrReadVal,
        crate::regs::ManualPadAttrWriteVal,
    >;
}
