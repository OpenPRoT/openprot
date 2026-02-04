#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RstmgrAon {
    _priv: (),
}
impl RstmgrAon {
    pub const PTR: *mut u32 = 0x40410000 as *mut u32;
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
    #[doc = "Software requested system reset.\n\nRead value: [`regs::ResetReqReadVal`]; Write value: [`regs::ResetReqWriteVal`]"]
    #[inline(always)]
    pub fn reset_req(&self) -> ureg::RegRef<crate::meta::ResetReq, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Device reset reason.\n\nRead value: [`regs::ResetInfoReadVal`]; Write value: [`regs::ResetInfoWriteVal`]"]
    #[inline(always)]
    pub fn reset_info(&self) -> ureg::RegRef<crate::meta::ResetInfo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert write enable\n\nRead value: [`regs::AlertRegwenReadVal`]; Write value: [`regs::AlertRegwenWriteVal`]"]
    #[inline(always)]
    pub fn alert_regwen(&self) -> ureg::RegRef<crate::meta::AlertRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert info dump controls.\n\nRead value: [`regs::AlertInfoCtrlReadVal`]; Write value: [`regs::AlertInfoCtrlWriteVal`]"]
    #[inline(always)]
    pub fn alert_info_ctrl(&self) -> ureg::RegRef<crate::meta::AlertInfoCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert info dump attributes.\n\nRead value: [`regs::AlertInfoAttrReadVal`]; Write value: [`regs::AlertInfoAttrWriteVal`]"]
    #[inline(always)]
    pub fn alert_info_attr(&self) -> ureg::RegRef<crate::meta::AlertInfoAttr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Alert dump information prior to last reset.\n  Which value read is controlled by the !!ALERT_INFO_CTRL register.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn alert_info(&self) -> ureg::RegRef<crate::meta::AlertInfo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Cpu write enable\n\nRead value: [`regs::CpuRegwenReadVal`]; Write value: [`regs::CpuRegwenWriteVal`]"]
    #[inline(always)]
    pub fn cpu_regwen(&self) -> ureg::RegRef<crate::meta::CpuRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Cpu info dump controls.\n\nRead value: [`regs::CpuInfoCtrlReadVal`]; Write value: [`regs::CpuInfoCtrlWriteVal`]"]
    #[inline(always)]
    pub fn cpu_info_ctrl(&self) -> ureg::RegRef<crate::meta::CpuInfoCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Cpu info dump attributes.\n\nRead value: [`regs::CpuInfoAttrReadVal`]; Write value: [`regs::CpuInfoAttrWriteVal`]"]
    #[inline(always)]
    pub fn cpu_info_attr(&self) -> ureg::RegRef<crate::meta::CpuInfoAttr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Cpu dump information prior to last reset.\n  Which value read is controlled by the !!CPU_INFO_CTRL register.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn cpu_info(&self) -> ureg::RegRef<crate::meta::CpuInfo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for software controllable resets.\nWhen a particular bit value is 0, the corresponding value in !!SW_RST_CTRL_N can no longer be changed.\nWhen a particular bit value is 1, the corresponding value in !!SW_RST_CTRL_N can be changed.\n\nRead value: [`regs::SwRstRegwenReadVal`]; Write value: [`regs::SwRstRegwenWriteVal`]"]
    #[inline(always)]
    pub fn sw_rst_regwen(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::SwRstRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Software controllable resets.\nWhen a particular bit value is 0, the corresponding module is held in reset.\nWhen a particular bit value is 1, the corresponding module is not held in reset.\n\nRead value: [`regs::SwRstCtrlNReadVal`]; Write value: [`regs::SwRstCtrlNWriteVal`]"]
    #[inline(always)]
    pub fn sw_rst_ctrl_n(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::SwRstCtrlN, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "A bit vector of all the errors that have occurred in reset manager\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AlertInfoAttrReadVal(u32);
    impl AlertInfoAttrReadVal {
        #[doc = "The number of 32-bit values contained in the alert info dump."]
        #[inline(always)]
        pub fn cnt_avail(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
    }
    impl From<u32> for AlertInfoAttrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertInfoAttrReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertInfoAttrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertInfoCtrlReadVal(u32);
    impl AlertInfoCtrlReadVal {
        #[doc = "Enable alert dump to capture new information.\nThis field is automatically set to 0 upon system reset (even if rstmgr is not reset)."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Controls which 32-bit value to read."]
        #[inline(always)]
        pub fn index(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertInfoCtrlWriteVal {
            AlertInfoCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for AlertInfoCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertInfoCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertInfoCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertInfoCtrlWriteVal(u32);
    impl AlertInfoCtrlWriteVal {
        #[doc = "Enable alert dump to capture new information.\nThis field is automatically set to 0 upon system reset (even if rstmgr is not reset)."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Controls which 32-bit value to read."]
        #[inline(always)]
        pub fn index(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
    }
    impl From<u32> for AlertInfoCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertInfoCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertInfoCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertRegwenReadVal(u32);
    impl AlertRegwenReadVal {
        #[doc = "When 1, !!ALERT_INFO_CTRL can be modified."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AlertRegwenWriteVal {
            AlertRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for AlertRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: AlertRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertRegwenWriteVal(u32);
    impl AlertRegwenWriteVal {
        #[doc = "When 1, !!ALERT_INFO_CTRL can be modified."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for AlertRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_fault(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_cnsty_fault(self, val: bool) -> Self {
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
    pub struct CpuInfoAttrReadVal(u32);
    impl CpuInfoAttrReadVal {
        #[doc = "The number of 32-bit values contained in the cpu info dump."]
        #[inline(always)]
        pub fn cnt_avail(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
    }
    impl From<u32> for CpuInfoAttrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CpuInfoAttrReadVal> for u32 {
        #[inline(always)]
        fn from(val: CpuInfoAttrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CpuInfoCtrlReadVal(u32);
    impl CpuInfoCtrlReadVal {
        #[doc = "Enable cpu dump to capture new information.\nThis field is automatically set to 0 upon system reset (even if rstmgr is not reset)."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Controls which 32-bit value to read."]
        #[inline(always)]
        pub fn index(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CpuInfoCtrlWriteVal {
            CpuInfoCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for CpuInfoCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CpuInfoCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: CpuInfoCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CpuInfoCtrlWriteVal(u32);
    impl CpuInfoCtrlWriteVal {
        #[doc = "Enable cpu dump to capture new information.\nThis field is automatically set to 0 upon system reset (even if rstmgr is not reset)."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Controls which 32-bit value to read."]
        #[inline(always)]
        pub fn index(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
    }
    impl From<u32> for CpuInfoCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CpuInfoCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CpuInfoCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CpuRegwenReadVal(u32);
    impl CpuRegwenReadVal {
        #[doc = "When 1, !!CPU_INFO_CTRL can be modified."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CpuRegwenWriteVal {
            CpuRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CpuRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CpuRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CpuRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CpuRegwenWriteVal(u32);
    impl CpuRegwenWriteVal {
        #[doc = "When 1, !!CPU_INFO_CTRL can be modified."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CpuRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CpuRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CpuRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrCodeReadVal(u32);
    impl ErrCodeReadVal {
        #[doc = "The register file has experienced an integrity error."]
        #[inline(always)]
        pub fn reg_intg_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "A inconsistent parent / child reset was observed."]
        #[inline(always)]
        pub fn reset_consistency_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Sparsely encoded fsm error."]
        #[inline(always)]
        pub fn fsm_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
    }
    impl From<u32> for ErrCodeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrCodeReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrCodeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetInfoReadVal(u32);
    impl ResetInfoReadVal {
        #[doc = "Indicates when a device has reset due to power up."]
        #[inline(always)]
        pub fn por(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Indicates when a device has reset due low power exit."]
        #[inline(always)]
        pub fn low_power_exit(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Indicates when a device has reset due to !!RESET_REQ."]
        #[inline(always)]
        pub fn sw_reset(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Indicates when a device has reset due to a hardware requested reset.\nThe bit mapping is as follows:\nb3: sysrst_ctrl_aon: OpenTitan reset request to `rstmgr` (running on AON clock).\nb4: aon_timer_aon: watchdog reset requestt\nb5: pwrmgr_aon: main power glitch reset request\nb6: alert_handler: escalation reset request\nb7: rv_dm: non-debug-module reset request"]
        #[inline(always)]
        pub fn hw_req(&self) -> u32 {
            (self.0 >> 3) & 0x1f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ResetInfoWriteVal {
            ResetInfoWriteVal(self.0)
        }
    }
    impl From<u32> for ResetInfoReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetInfoReadVal> for u32 {
        #[inline(always)]
        fn from(val: ResetInfoReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetInfoWriteVal(u32);
    impl ResetInfoWriteVal {
        #[doc = "Indicates when a device has reset due to power up."]
        #[inline(always)]
        pub fn por_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Indicates when a device has reset due low power exit."]
        #[inline(always)]
        pub fn low_power_exit_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Indicates when a device has reset due to !!RESET_REQ."]
        #[inline(always)]
        pub fn sw_reset_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
    }
    impl From<u32> for ResetInfoWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetInfoWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ResetInfoWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetReqReadVal(u32);
    impl ResetReqReadVal {
        #[doc = "When set to kMultiBitBool4True, a reset to power manager is requested.\nUpon completion of reset, this bit is automatically cleared by hardware."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ResetReqWriteVal {
            ResetReqWriteVal(self.0)
        }
    }
    impl From<u32> for ResetReqReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetReqReadVal> for u32 {
        #[inline(always)]
        fn from(val: ResetReqReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ResetReqWriteVal(u32);
    impl ResetReqWriteVal {
        #[doc = "When set to kMultiBitBool4True, a reset to power manager is requested.\nUpon completion of reset, this bit is automatically cleared by hardware."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for ResetReqWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ResetReqWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ResetReqWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRstCtrlNReadVal(u32);
    impl SwRstCtrlNReadVal {
        #[doc = "Software reset value"]
        #[inline(always)]
        pub fn val(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SwRstCtrlNWriteVal {
            SwRstCtrlNWriteVal(self.0)
        }
    }
    impl From<u32> for SwRstCtrlNReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRstCtrlNReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwRstCtrlNReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRstCtrlNWriteVal(u32);
    impl SwRstCtrlNWriteVal {
        #[doc = "Software reset value"]
        #[inline(always)]
        pub fn val(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for SwRstCtrlNWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRstCtrlNWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SwRstCtrlNWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRstRegwenReadVal(u32);
    impl SwRstRegwenReadVal {
        #[doc = "Register write enable for software controllable resets"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SwRstRegwenWriteVal {
            SwRstRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for SwRstRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRstRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwRstRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRstRegwenWriteVal(u32);
    impl SwRstRegwenWriteVal {
        #[doc = "Register write enable for software controllable resets"]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for SwRstRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRstRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SwRstRegwenWriteVal) -> u32 {
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
    pub type ResetReq =
        ureg::ReadWriteReg32<9, crate::regs::ResetReqReadVal, crate::regs::ResetReqWriteVal>;
    pub type ResetInfo =
        ureg::ReadWriteReg32<1, crate::regs::ResetInfoReadVal, crate::regs::ResetInfoWriteVal>;
    pub type AlertRegwen =
        ureg::ReadWriteReg32<1, crate::regs::AlertRegwenReadVal, crate::regs::AlertRegwenWriteVal>;
    pub type AlertInfoCtrl = ureg::ReadWriteReg32<
        0,
        crate::regs::AlertInfoCtrlReadVal,
        crate::regs::AlertInfoCtrlWriteVal,
    >;
    pub type AlertInfoAttr = ureg::ReadOnlyReg32<crate::regs::AlertInfoAttrReadVal>;
    pub type AlertInfo = ureg::ReadOnlyReg32<u32>;
    pub type CpuRegwen =
        ureg::ReadWriteReg32<1, crate::regs::CpuRegwenReadVal, crate::regs::CpuRegwenWriteVal>;
    pub type CpuInfoCtrl =
        ureg::ReadWriteReg32<0, crate::regs::CpuInfoCtrlReadVal, crate::regs::CpuInfoCtrlWriteVal>;
    pub type CpuInfoAttr = ureg::ReadOnlyReg32<crate::regs::CpuInfoAttrReadVal>;
    pub type CpuInfo = ureg::ReadOnlyReg32<u32>;
    pub type SwRstRegwen =
        ureg::ReadWriteReg32<1, crate::regs::SwRstRegwenReadVal, crate::regs::SwRstRegwenWriteVal>;
    pub type SwRstCtrlN =
        ureg::ReadWriteReg32<1, crate::regs::SwRstCtrlNReadVal, crate::regs::SwRstCtrlNWriteVal>;
    pub type ErrCode = ureg::ReadOnlyReg32<crate::regs::ErrCodeReadVal>;
}
