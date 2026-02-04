#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct PwrmgrAon {
    _priv: (),
}
impl PwrmgrAon {
    pub const PTR: *mut u32 = 0x40400000 as *mut u32;
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
    #[doc = "Controls the configurability of the !!CONTROL register.\n\nThis register ensures the contents do not change once a low power hint and\nWFI has occurred.\n\nIt unlocks whenever a low power transition has completed (transition back to the\nACTIVE state) for any reason.\n\nRead value: [`regs::CtrlCfgRegwenReadVal`]; Write value: [`regs::CtrlCfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_cfg_regwen(&self) -> ureg::RegRef<crate::meta::CtrlCfgRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control register\n\nRead value: [`regs::ControlReadVal`]; Write value: [`regs::ControlWriteVal`]"]
    #[inline(always)]
    pub fn control(&self) -> ureg::RegRef<crate::meta::Control, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "The configuration registers CONTROL, WAKEUP_EN, RESET_EN are all written in the\nfast clock domain but used in the slow clock domain.\n\nThe configuration are not propagated across the clock boundary until this\nregister is triggered and read.  See fields below for more details\n\nRead value: [`regs::CfgCdcSyncReadVal`]; Write value: [`regs::CfgCdcSyncWriteVal`]"]
    #[inline(always)]
    pub fn cfg_cdc_sync(&self) -> ureg::RegRef<crate::meta::CfgCdcSync, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration enable for wakeup_en register\n\nRead value: [`regs::WakeupEnRegwenReadVal`]; Write value: [`regs::WakeupEnRegwenWriteVal`]"]
    #[inline(always)]
    pub fn wakeup_en_regwen(&self) -> ureg::RegRef<crate::meta::WakeupEnRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Bit mask for enabled wakeups\n\nRead value: [`regs::WakeupEn0ReadVal`]; Write value: [`regs::WakeupEn0WriteVal`]"]
    #[inline(always)]
    pub fn wakeup_en0(&self) -> ureg::RegRef<crate::meta::WakeupEn0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A read only register of all current wake requests post enable mask\n\nRead value: [`regs::WakeStatus0ReadVal`]; Write value: [`regs::WakeStatus0WriteVal`]"]
    #[inline(always)]
    pub fn wake_status0(&self) -> ureg::RegRef<crate::meta::WakeStatus0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration enable for reset_en register\n\nRead value: [`regs::ResetEnRegwenReadVal`]; Write value: [`regs::ResetEnRegwenWriteVal`]"]
    #[inline(always)]
    pub fn reset_en_regwen(&self) -> ureg::RegRef<crate::meta::ResetEnRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Bit mask for enabled reset requests\n\nRead value: [`regs::ResetEn0ReadVal`]; Write value: [`regs::ResetEn0WriteVal`]"]
    #[inline(always)]
    pub fn reset_en0(&self) -> ureg::RegRef<crate::meta::ResetEn0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A read only register of all current reset requests post enable mask\n\nRead value: [`regs::ResetStatus0ReadVal`]; Write value: [`regs::ResetStatus0WriteVal`]"]
    #[inline(always)]
    pub fn reset_status0(&self) -> ureg::RegRef<crate::meta::ResetStatus0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A read only register of escalation reset request\n\nRead value: [`regs::EscalateResetStatusReadVal`]; Write value: [`regs::EscalateResetStatusWriteVal`]"]
    #[inline(always)]
    pub fn escalate_reset_status(&self) -> ureg::RegRef<crate::meta::EscalateResetStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Indicates which functions caused the chip to wakeup\n\nRead value: [`regs::WakeInfoCaptureDisReadVal`]; Write value: [`regs::WakeInfoCaptureDisWriteVal`]"]
    #[inline(always)]
    pub fn wake_info_capture_dis(&self) -> ureg::RegRef<crate::meta::WakeInfoCaptureDis, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Indicates which functions caused the chip to wakeup.\nThe wake info recording begins whenever the device begins a valid low power entry.\n\nThis capture is continued until it is explicitly disabled through WAKE_INFO_CAPTURE_DIS.\nThis means it is possible to capture multiple wakeup reasons.\n\nRead value: [`regs::WakeInfoReadVal`]; Write value: [`regs::WakeInfoWriteVal`]"]
    #[inline(always)]
    pub fn wake_info(&self) -> ureg::RegRef<crate::meta::WakeInfo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A read only register that shows the existing faults\n\nRead value: [`regs::FaultStatusReadVal`]; Write value: [`regs::FaultStatusWriteVal`]"]
    #[inline(always)]
    pub fn fault_status(&self) -> ureg::RegRef<crate::meta::FaultStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
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
    pub struct CfgCdcSyncReadVal(u32);
    impl CfgCdcSyncReadVal {
        #[doc = "Configuration sync.  When this bit is written to 1, a sync pulse is generated.  When\nthe sync completes, this bit then self clears.\n\nSoftware should write this bit to 1, wait for it to clear, before assuming the slow clock\ndomain has accepted the programmed values."]
        #[inline(always)]
        pub fn sync(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgCdcSyncWriteVal {
            CfgCdcSyncWriteVal(self.0)
        }
    }
    impl From<u32> for CfgCdcSyncReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgCdcSyncReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgCdcSyncReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgCdcSyncWriteVal(u32);
    impl CfgCdcSyncWriteVal {
        #[doc = "Configuration sync.  When this bit is written to 1, a sync pulse is generated.  When\nthe sync completes, this bit then self clears.\n\nSoftware should write this bit to 1, wait for it to clear, before assuming the slow clock\ndomain has accepted the programmed values."]
        #[inline(always)]
        pub fn sync(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for CfgCdcSyncWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgCdcSyncWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgCdcSyncWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlReadVal(u32);
    impl ControlReadVal {
        #[doc = "The low power hint to power manager.\nThe hint is an indication for how the manager should treat the next WFI.\nOnce the power manager begins a low power transition, or if a valid reset request is registered,\nthis bit is automatically cleared by HW."]
        #[inline(always)]
        pub fn low_power_hint(&self) -> super::enums::LowPowerHint {
            super::enums::LowPowerHint::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = "core clock enable during low power state"]
        #[inline(always)]
        pub fn core_clk_en(&self) -> super::enums::Enum7305284c1ae92db5 {
            super::enums::Enum7305284c1ae92db5::try_from((self.0 >> 4) & 1).unwrap()
        }
        #[doc = "IO clock enable during low power state"]
        #[inline(always)]
        pub fn io_clk_en(&self) -> super::enums::Enum7305284c1ae92db5 {
            super::enums::Enum7305284c1ae92db5::try_from((self.0 >> 5) & 1).unwrap()
        }
        #[doc = "USB clock enable during low power state"]
        #[inline(always)]
        pub fn usb_clk_en_lp(&self) -> super::enums::Enum7305284c1ae92db5 {
            super::enums::Enum7305284c1ae92db5::try_from((self.0 >> 6) & 1).unwrap()
        }
        #[doc = "USB clock enable during active power state"]
        #[inline(always)]
        pub fn usb_clk_en_active(&self) -> super::enums::Enum7305284c1ae92db5 {
            super::enums::Enum7305284c1ae92db5::try_from((self.0 >> 7) & 1).unwrap()
        }
        #[doc = "Active low, main power domain power down"]
        #[inline(always)]
        pub fn main_pd_n(&self) -> super::enums::MainPdN {
            super::enums::MainPdN::try_from((self.0 >> 8) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ControlWriteVal {
            ControlWriteVal(self.0)
        }
    }
    impl From<u32> for ControlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlReadVal> for u32 {
        #[inline(always)]
        fn from(val: ControlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlWriteVal(u32);
    impl ControlWriteVal {
        #[doc = "The low power hint to power manager.\nThe hint is an indication for how the manager should treat the next WFI.\nOnce the power manager begins a low power transition, or if a valid reset request is registered,\nthis bit is automatically cleared by HW."]
        #[inline(always)]
        pub fn low_power_hint(
            self,
            f: impl FnOnce(super::enums::selector::LowPowerHintSelector) -> super::enums::LowPowerHint,
        ) -> Self {
            Self(
                (self.0 & !(1 << 0))
                    | (u32::from(f(super::enums::selector::LowPowerHintSelector())) << 0),
            )
        }
        #[doc = "core clock enable during low power state"]
        #[inline(always)]
        pub fn core_clk_en(
            self,
            f: impl FnOnce(
                super::enums::selector::Enum7305284c1ae92db5Selector,
            ) -> super::enums::Enum7305284c1ae92db5,
        ) -> Self {
            Self(
                (self.0 & !(1 << 4))
                    | (u32::from(f(super::enums::selector::Enum7305284c1ae92db5Selector())) << 4),
            )
        }
        #[doc = "IO clock enable during low power state"]
        #[inline(always)]
        pub fn io_clk_en(
            self,
            f: impl FnOnce(
                super::enums::selector::Enum7305284c1ae92db5Selector,
            ) -> super::enums::Enum7305284c1ae92db5,
        ) -> Self {
            Self(
                (self.0 & !(1 << 5))
                    | (u32::from(f(super::enums::selector::Enum7305284c1ae92db5Selector())) << 5),
            )
        }
        #[doc = "USB clock enable during low power state"]
        #[inline(always)]
        pub fn usb_clk_en_lp(
            self,
            f: impl FnOnce(
                super::enums::selector::Enum7305284c1ae92db5Selector,
            ) -> super::enums::Enum7305284c1ae92db5,
        ) -> Self {
            Self(
                (self.0 & !(1 << 6))
                    | (u32::from(f(super::enums::selector::Enum7305284c1ae92db5Selector())) << 6),
            )
        }
        #[doc = "USB clock enable during active power state"]
        #[inline(always)]
        pub fn usb_clk_en_active(
            self,
            f: impl FnOnce(
                super::enums::selector::Enum7305284c1ae92db5Selector,
            ) -> super::enums::Enum7305284c1ae92db5,
        ) -> Self {
            Self(
                (self.0 & !(1 << 7))
                    | (u32::from(f(super::enums::selector::Enum7305284c1ae92db5Selector())) << 7),
            )
        }
        #[doc = "Active low, main power domain power down"]
        #[inline(always)]
        pub fn main_pd_n(
            self,
            f: impl FnOnce(super::enums::selector::MainPdNSelector) -> super::enums::MainPdN,
        ) -> Self {
            Self(
                (self.0 & !(1 << 8))
                    | (u32::from(f(super::enums::selector::MainPdNSelector())) << 8),
            )
        }
    }
    impl From<u32> for ControlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ControlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlCfgRegwenReadVal(u32);
    impl CtrlCfgRegwenReadVal {
        #[doc = "Configuration enable.\n\nThis bit defaults to 1 and is set to 0 by hardware when low power entry is initiated.\nWhen the device transitions back from low power state to active state, this bit is set\nback to 1 to allow software configuration of !!CONTROL"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
    }
    impl From<u32> for CtrlCfgRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlCfgRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlCfgRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EscalateResetStatusReadVal(u32);
    impl EscalateResetStatusReadVal {
        #[doc = "When 1, an escalation reset has been seen.\nWhen 0, there is no escalation reset."]
        #[inline(always)]
        pub fn val(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
    }
    impl From<u32> for EscalateResetStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EscalateResetStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: EscalateResetStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FaultStatusReadVal(u32);
    impl FaultStatusReadVal {
        #[doc = "When 1, an integrity error has occurred."]
        #[inline(always)]
        pub fn reg_intg_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "When 1, an escalation clock / reset timeout has occurred."]
        #[inline(always)]
        pub fn esc_timeout(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "When 1, unexpected power glitch was observed on main PD."]
        #[inline(always)]
        pub fn main_pd_glitch(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
    }
    impl From<u32> for FaultStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FaultStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: FaultStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.wakeup is set."]
        #[inline(always)]
        pub fn wakeup(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.wakeup is set."]
        #[inline(always)]
        pub fn wakeup(self, val: bool) -> Self {
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
        #[doc = "Wake from low power state. See wake info for more details"]
        #[inline(always)]
        pub fn wakeup(&self) -> bool {
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
        #[doc = "Wake from low power state. See wake info for more details"]
        #[inline(always)]
        pub fn wakeup_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.wakeup to 1."]
        #[inline(always)]
        pub fn wakeup(self, val: bool) -> Self {
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
    pub struct ResetEn0ReadVal(u32);
    impl ResetEn0ReadVal {
        #[doc = "Whenever a particular bit is set to 1, that reset request is enabled.\nWhenever a particular bit is set to 0, that reset request cannot reset the device."]
        #[inline(always)]
        pub fn en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that reset request is enabled.\nWhenever a particular bit is set to 0, that reset request cannot reset the device."]
        #[inline(always)]
        pub fn en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ResetEn0WriteVal {
            ResetEn0WriteVal(self.0)
        }
    }
    impl From<u32> for ResetEn0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetEn0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: ResetEn0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetEn0WriteVal(u32);
    impl ResetEn0WriteVal {
        #[doc = "Whenever a particular bit is set to 1, that reset request is enabled.\nWhenever a particular bit is set to 0, that reset request cannot reset the device."]
        #[inline(always)]
        pub fn en0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Whenever a particular bit is set to 1, that reset request is enabled.\nWhenever a particular bit is set to 0, that reset request cannot reset the device."]
        #[inline(always)]
        pub fn en1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for ResetEn0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetEn0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: ResetEn0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetEnRegwenReadVal(u32);
    impl ResetEnRegwenReadVal {
        #[doc = "When 1, RESET_EN register can be configured.\nWhen 0, RESET_EN register cannot be configured."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ResetEnRegwenWriteVal {
            ResetEnRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ResetEnRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetEnRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ResetEnRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetEnRegwenWriteVal(u32);
    impl ResetEnRegwenWriteVal {
        #[doc = "When 1, RESET_EN register can be configured.\nWhen 0, RESET_EN register cannot be configured."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ResetEnRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetEnRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ResetEnRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetStatus0ReadVal(u32);
    impl ResetStatus0ReadVal {
        #[doc = "Current value of reset request"]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Current value of reset request"]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
    }
    impl From<u32> for ResetStatus0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetStatus0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: ResetStatus0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeupEn0ReadVal(u32);
    impl WakeupEn0ReadVal {
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WakeupEn0WriteVal {
            WakeupEn0WriteVal(self.0)
        }
    }
    impl From<u32> for WakeupEn0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeupEn0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeupEn0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeupEn0WriteVal(u32);
    impl WakeupEn0WriteVal {
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Whenever a particular bit is set to 1, that wakeup is also enabled.\nWhenever a particular bit is set to 0, that wakeup cannot wake the device from low power."]
        #[inline(always)]
        pub fn en5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
    }
    impl From<u32> for WakeupEn0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeupEn0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: WakeupEn0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeupEnRegwenReadVal(u32);
    impl WakeupEnRegwenReadVal {
        #[doc = "When 1, WAKEUP_EN register can be configured.\nWhen 0, WAKEUP_EN register cannot be configured."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WakeupEnRegwenWriteVal {
            WakeupEnRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for WakeupEnRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeupEnRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeupEnRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeupEnRegwenWriteVal(u32);
    impl WakeupEnRegwenWriteVal {
        #[doc = "When 1, WAKEUP_EN register can be configured.\nWhen 0, WAKEUP_EN register cannot be configured."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for WakeupEnRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeupEnRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WakeupEnRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeInfoReadVal(u32);
    impl WakeInfoReadVal {
        #[doc = "Various peripheral wake reasons"]
        #[inline(always)]
        pub fn reasons(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[doc = "The fall through wakeup reason indicates that despite setting a WFI and providing a low power\nhint, an interrupt arrived at just the right time to break the executing core out of WFI.\n\nThe power manager detects this condition, halts low power entry and reports as a wakeup reason"]
        #[inline(always)]
        pub fn fall_through(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "The abort wakeup reason indicates that despite setting a WFI and providing a low power\nhint, an active flash / lifecycle / otp transaction was ongoing when the power controller\nattempted to initiate low power entry.\n\nThe power manager detects this condition, halts low power entry and reports as a wakeup reason"]
        #[inline(always)]
        pub fn abort(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WakeInfoWriteVal {
            WakeInfoWriteVal(self.0)
        }
    }
    impl From<u32> for WakeInfoReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeInfoReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeInfoReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeInfoWriteVal(u32);
    impl WakeInfoWriteVal {
        #[doc = "The fall through wakeup reason indicates that despite setting a WFI and providing a low power\nhint, an interrupt arrived at just the right time to break the executing core out of WFI.\n\nThe power manager detects this condition, halts low power entry and reports as a wakeup reason"]
        #[inline(always)]
        pub fn fall_through_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "The abort wakeup reason indicates that despite setting a WFI and providing a low power\nhint, an active flash / lifecycle / otp transaction was ongoing when the power controller\nattempted to initiate low power entry.\n\nThe power manager detects this condition, halts low power entry and reports as a wakeup reason"]
        #[inline(always)]
        pub fn abort_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
    }
    impl From<u32> for WakeInfoWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeInfoWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WakeInfoWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeInfoCaptureDisReadVal(u32);
    impl WakeInfoCaptureDisReadVal {
        #[doc = "When written to 1, this actively suppresses the wakeup info capture.\nWhen written to 0, wakeup info capture timing is controlled by HW."]
        #[inline(always)]
        pub fn val(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WakeInfoCaptureDisWriteVal {
            WakeInfoCaptureDisWriteVal(self.0)
        }
    }
    impl From<u32> for WakeInfoCaptureDisReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeInfoCaptureDisReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeInfoCaptureDisReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeInfoCaptureDisWriteVal(u32);
    impl WakeInfoCaptureDisWriteVal {
        #[doc = "When written to 1, this actively suppresses the wakeup info capture.\nWhen written to 0, wakeup info capture timing is controlled by HW."]
        #[inline(always)]
        pub fn val(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for WakeInfoCaptureDisWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeInfoCaptureDisWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WakeInfoCaptureDisWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeStatus0ReadVal(u32);
    impl WakeStatus0ReadVal {
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Current value of wake requests"]
        #[inline(always)]
        pub fn val5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
    }
    impl From<u32> for WakeStatus0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeStatus0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeStatus0ReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Enum7305284c1ae92db5 {
        Disabled = 0,
        Enabled = 1,
    }
    impl Enum7305284c1ae92db5 {
        #[inline(always)]
        pub fn disabled(&self) -> bool {
            *self == Self::Disabled
        }
        #[inline(always)]
        pub fn enabled(&self) -> bool {
            *self == Self::Enabled
        }
    }
    impl TryFrom<u32> for Enum7305284c1ae92db5 {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Enum7305284c1ae92db5, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, Enum7305284c1ae92db5>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Enum7305284c1ae92db5> for u32 {
        fn from(val: Enum7305284c1ae92db5) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum LowPowerHint {
        None = 0,
        LowPower = 1,
    }
    impl LowPowerHint {
        #[inline(always)]
        pub fn none(&self) -> bool {
            *self == Self::None
        }
        #[inline(always)]
        pub fn low_power(&self) -> bool {
            *self == Self::LowPower
        }
    }
    impl TryFrom<u32> for LowPowerHint {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<LowPowerHint, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, LowPowerHint>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<LowPowerHint> for u32 {
        fn from(val: LowPowerHint) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum MainPdN {
        PowerDown = 0,
        PowerUp = 1,
    }
    impl MainPdN {
        #[inline(always)]
        pub fn power_down(&self) -> bool {
            *self == Self::PowerDown
        }
        #[inline(always)]
        pub fn power_up(&self) -> bool {
            *self == Self::PowerUp
        }
    }
    impl TryFrom<u32> for MainPdN {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<MainPdN, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, MainPdN>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<MainPdN> for u32 {
        fn from(val: MainPdN) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct Enum7305284c1ae92db5Selector();
        impl Enum7305284c1ae92db5Selector {
            #[inline(always)]
            pub fn disabled(&self) -> super::Enum7305284c1ae92db5 {
                super::Enum7305284c1ae92db5::Disabled
            }
            #[inline(always)]
            pub fn enabled(&self) -> super::Enum7305284c1ae92db5 {
                super::Enum7305284c1ae92db5::Enabled
            }
        }
        pub struct LowPowerHintSelector();
        impl LowPowerHintSelector {
            #[inline(always)]
            pub fn none(&self) -> super::LowPowerHint {
                super::LowPowerHint::None
            }
            #[inline(always)]
            pub fn low_power(&self) -> super::LowPowerHint {
                super::LowPowerHint::LowPower
            }
        }
        pub struct MainPdNSelector();
        impl MainPdNSelector {
            #[inline(always)]
            pub fn power_down(&self) -> super::MainPdN {
                super::MainPdN::PowerDown
            }
            #[inline(always)]
            pub fn power_up(&self) -> super::MainPdN {
                super::MainPdN::PowerUp
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
    pub type CtrlCfgRegwen = ureg::ReadOnlyReg32<crate::regs::CtrlCfgRegwenReadVal>;
    pub type Control =
        ureg::ReadWriteReg32<0x180, crate::regs::ControlReadVal, crate::regs::ControlWriteVal>;
    pub type CfgCdcSync =
        ureg::ReadWriteReg32<0, crate::regs::CfgCdcSyncReadVal, crate::regs::CfgCdcSyncWriteVal>;
    pub type WakeupEnRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::WakeupEnRegwenReadVal,
        crate::regs::WakeupEnRegwenWriteVal,
    >;
    pub type WakeupEn0 =
        ureg::ReadWriteReg32<0, crate::regs::WakeupEn0ReadVal, crate::regs::WakeupEn0WriteVal>;
    pub type WakeStatus0 = ureg::ReadOnlyReg32<crate::regs::WakeStatus0ReadVal>;
    pub type ResetEnRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ResetEnRegwenReadVal,
        crate::regs::ResetEnRegwenWriteVal,
    >;
    pub type ResetEn0 =
        ureg::ReadWriteReg32<0, crate::regs::ResetEn0ReadVal, crate::regs::ResetEn0WriteVal>;
    pub type ResetStatus0 = ureg::ReadOnlyReg32<crate::regs::ResetStatus0ReadVal>;
    pub type EscalateResetStatus = ureg::ReadOnlyReg32<crate::regs::EscalateResetStatusReadVal>;
    pub type WakeInfoCaptureDis = ureg::ReadWriteReg32<
        0,
        crate::regs::WakeInfoCaptureDisReadVal,
        crate::regs::WakeInfoCaptureDisWriteVal,
    >;
    pub type WakeInfo =
        ureg::ReadWriteReg32<0, crate::regs::WakeInfoReadVal, crate::regs::WakeInfoWriteVal>;
    pub type FaultStatus = ureg::ReadOnlyReg32<crate::regs::FaultStatusReadVal>;
}
