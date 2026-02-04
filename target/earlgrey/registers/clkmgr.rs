#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct ClkmgrAon {
    _priv: (),
}
impl ClkmgrAon {
    pub const PTR: *mut u32 = 0x40420000 as *mut u32;
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
    #[doc = "External clock control write enable\n\nRead value: [`regs::ExtclkCtrlRegwenReadVal`]; Write value: [`regs::ExtclkCtrlRegwenWriteVal`]"]
    #[inline(always)]
    pub fn extclk_ctrl_regwen(&self) -> ureg::RegRef<crate::meta::ExtclkCtrlRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Select external clock\n\nRead value: [`regs::ExtclkCtrlReadVal`]; Write value: [`regs::ExtclkCtrlWriteVal`]"]
    #[inline(always)]
    pub fn extclk_ctrl(&self) -> ureg::RegRef<crate::meta::ExtclkCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status of requested external clock switch\n\nRead value: [`regs::ExtclkStatusReadVal`]; Write value: [`regs::ExtclkStatusWriteVal`]"]
    #[inline(always)]
    pub fn extclk_status(&self) -> ureg::RegRef<crate::meta::ExtclkStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Jitter write enable\n\nRead value: [`regs::JitterRegwenReadVal`]; Write value: [`regs::JitterRegwenWriteVal`]"]
    #[inline(always)]
    pub fn jitter_regwen(&self) -> ureg::RegRef<crate::meta::JitterRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable jittery clock\n\nRead value: [`regs::JitterEnableReadVal`]; Write value: [`regs::JitterEnableWriteVal`]"]
    #[inline(always)]
    pub fn jitter_enable(&self) -> ureg::RegRef<crate::meta::JitterEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clock enable for software gateable clocks.\nThese clocks are directly controlled by software.\n\nRead value: [`regs::ClkEnablesReadVal`]; Write value: [`regs::ClkEnablesWriteVal`]"]
    #[inline(always)]
    pub fn clk_enables(&self) -> ureg::RegRef<crate::meta::ClkEnables, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clock hint for software gateable transactional clocks during active mode.\nDuring low power mode, all clocks are gated off regardless of the software hint.\n\nTransactional clocks are not fully controlled by software.  Instead software provides only a disable hint.\n\nWhen software provides a disable hint, the clock manager checks to see if the associated hardware block is idle.\nIf the hardware block is idle, then the clock is disabled.\nIf the hardware block is not idle, the clock is kept on.\n\nFor the enable case, the software hint is immediately honored and the clock turned on.  Hardware does not provide any\nfeedback in this case.\n\nRead value: [`regs::ClkHintsReadVal`]; Write value: [`regs::ClkHintsWriteVal`]"]
    #[inline(always)]
    pub fn clk_hints(&self) -> ureg::RegRef<crate::meta::ClkHints, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Since the final state of !!CLK_HINTS is not always determined by software,\nthis register provides read feedback for the current clock state.\n\n\nRead value: [`regs::ClkHintsStatusReadVal`]; Write value: [`regs::ClkHintsStatusWriteVal`]"]
    #[inline(always)]
    pub fn clk_hints_status(&self) -> ureg::RegRef<crate::meta::ClkHintsStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Measurement control write enable\n\nRead value: [`regs::MeasureCtrlRegwenReadVal`]; Write value: [`regs::MeasureCtrlRegwenWriteVal`]"]
    #[inline(always)]
    pub fn measure_ctrl_regwen(&self) -> ureg::RegRef<crate::meta::MeasureCtrlRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable for measurement control\n\nRead value: [`regs::IoMeasCtrlEnReadVal`]; Write value: [`regs::IoMeasCtrlEnWriteVal`]"]
    #[inline(always)]
    pub fn io_meas_ctrl_en(&self) -> ureg::RegRef<crate::meta::IoMeasCtrlEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration controls for io measurement.\n\nThe threshold fields are made wider than required (by 1 bit) to ensure\nthere is room to adjust for measurement inaccuracies.\n\nRead value: [`regs::IoMeasCtrlShadowedReadVal`]; Write value: [`regs::IoMeasCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn io_meas_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::IoMeasCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable for measurement control\n\nRead value: [`regs::IoDiv2MeasCtrlEnReadVal`]; Write value: [`regs::IoDiv2MeasCtrlEnWriteVal`]"]
    #[inline(always)]
    pub fn io_div2_meas_ctrl_en(&self) -> ureg::RegRef<crate::meta::IoDiv2MeasCtrlEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration controls for io_div2 measurement.\n\nThe threshold fields are made wider than required (by 1 bit) to ensure\nthere is room to adjust for measurement inaccuracies.\n\nRead value: [`regs::IoDiv2MeasCtrlShadowedReadVal`]; Write value: [`regs::IoDiv2MeasCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn io_div2_meas_ctrl_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::IoDiv2MeasCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable for measurement control\n\nRead value: [`regs::IoDiv4MeasCtrlEnReadVal`]; Write value: [`regs::IoDiv4MeasCtrlEnWriteVal`]"]
    #[inline(always)]
    pub fn io_div4_meas_ctrl_en(&self) -> ureg::RegRef<crate::meta::IoDiv4MeasCtrlEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration controls for io_div4 measurement.\n\nThe threshold fields are made wider than required (by 1 bit) to ensure\nthere is room to adjust for measurement inaccuracies.\n\nRead value: [`regs::IoDiv4MeasCtrlShadowedReadVal`]; Write value: [`regs::IoDiv4MeasCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn io_div4_meas_ctrl_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::IoDiv4MeasCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable for measurement control\n\nRead value: [`regs::MainMeasCtrlEnReadVal`]; Write value: [`regs::MainMeasCtrlEnWriteVal`]"]
    #[inline(always)]
    pub fn main_meas_ctrl_en(&self) -> ureg::RegRef<crate::meta::MainMeasCtrlEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration controls for main measurement.\n\nThe threshold fields are made wider than required (by 1 bit) to ensure\nthere is room to adjust for measurement inaccuracies.\n\nRead value: [`regs::MainMeasCtrlShadowedReadVal`]; Write value: [`regs::MainMeasCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn main_meas_ctrl_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::MainMeasCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable for measurement control\n\nRead value: [`regs::UsbMeasCtrlEnReadVal`]; Write value: [`regs::UsbMeasCtrlEnWriteVal`]"]
    #[inline(always)]
    pub fn usb_meas_ctrl_en(&self) -> ureg::RegRef<crate::meta::UsbMeasCtrlEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration controls for usb measurement.\n\nThe threshold fields are made wider than required (by 1 bit) to ensure\nthere is room to adjust for measurement inaccuracies.\n\nRead value: [`regs::UsbMeasCtrlShadowedReadVal`]; Write value: [`regs::UsbMeasCtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn usb_meas_ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::UsbMeasCtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Recoverable Error code\n\nRead value: [`regs::RecovErrCodeReadVal`]; Write value: [`regs::RecovErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn recov_err_code(&self) -> ureg::RegRef<crate::meta::RecovErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Error code\n\nRead value: [`regs::FatalErrCodeReadVal`]; Write value: [`regs::FatalErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn fatal_err_code(&self) -> ureg::RegRef<crate::meta::FatalErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
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
        pub fn recov_fault(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_fault(self, val: bool) -> Self {
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
    pub struct ClkEnablesReadVal(u32);
    impl ClkEnablesReadVal {
        #[doc = "0 CLK_IO_DIV4_PERI is disabled.\n1 CLK_IO_DIV4_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_div4_peri_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0 CLK_IO_DIV2_PERI is disabled.\n1 CLK_IO_DIV2_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_div2_peri_en(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0 CLK_IO_PERI is disabled.\n1 CLK_IO_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_peri_en(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0 CLK_USB_PERI is disabled.\n1 CLK_USB_PERI is enabled."]
        #[inline(always)]
        pub fn clk_usb_peri_en(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClkEnablesWriteVal {
            ClkEnablesWriteVal(self.0)
        }
    }
    impl From<u32> for ClkEnablesReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClkEnablesReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClkEnablesReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClkEnablesWriteVal(u32);
    impl ClkEnablesWriteVal {
        #[doc = "0 CLK_IO_DIV4_PERI is disabled.\n1 CLK_IO_DIV4_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_div4_peri_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0 CLK_IO_DIV2_PERI is disabled.\n1 CLK_IO_DIV2_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_div2_peri_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0 CLK_IO_PERI is disabled.\n1 CLK_IO_PERI is enabled."]
        #[inline(always)]
        pub fn clk_io_peri_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0 CLK_USB_PERI is disabled.\n1 CLK_USB_PERI is enabled."]
        #[inline(always)]
        pub fn clk_usb_peri_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for ClkEnablesWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClkEnablesWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClkEnablesWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClkHintsReadVal(u32);
    impl ClkHintsReadVal {
        #[doc = "0 CLK_MAIN_AES can be disabled.\n1 CLK_MAIN_AES is enabled."]
        #[inline(always)]
        pub fn clk_main_aes_hint(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_HMAC can be disabled.\n1 CLK_MAIN_HMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_hmac_hint(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_KMAC can be disabled.\n1 CLK_MAIN_KMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_kmac_hint(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_OTBN can be disabled.\n1 CLK_MAIN_OTBN is enabled."]
        #[inline(always)]
        pub fn clk_main_otbn_hint(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClkHintsWriteVal {
            ClkHintsWriteVal(self.0)
        }
    }
    impl From<u32> for ClkHintsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClkHintsReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClkHintsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClkHintsWriteVal(u32);
    impl ClkHintsWriteVal {
        #[doc = "0 CLK_MAIN_AES can be disabled.\n1 CLK_MAIN_AES is enabled."]
        #[inline(always)]
        pub fn clk_main_aes_hint(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0 CLK_MAIN_HMAC can be disabled.\n1 CLK_MAIN_HMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_hmac_hint(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0 CLK_MAIN_KMAC can be disabled.\n1 CLK_MAIN_KMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_kmac_hint(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0 CLK_MAIN_OTBN can be disabled.\n1 CLK_MAIN_OTBN is enabled."]
        #[inline(always)]
        pub fn clk_main_otbn_hint(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for ClkHintsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClkHintsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClkHintsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClkHintsStatusReadVal(u32);
    impl ClkHintsStatusReadVal {
        #[doc = "0 CLK_MAIN_AES is disabled.\n1 CLK_MAIN_AES is enabled."]
        #[inline(always)]
        pub fn clk_main_aes_val(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_HMAC is disabled.\n1 CLK_MAIN_HMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_hmac_val(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_KMAC is disabled.\n1 CLK_MAIN_KMAC is enabled."]
        #[inline(always)]
        pub fn clk_main_kmac_val(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0 CLK_MAIN_OTBN is disabled.\n1 CLK_MAIN_OTBN is enabled."]
        #[inline(always)]
        pub fn clk_main_otbn_val(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
    }
    impl From<u32> for ClkHintsStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClkHintsStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClkHintsStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExtclkCtrlReadVal(u32);
    impl ExtclkCtrlReadVal {
        #[doc = "When the current value is not kMultiBitBool4True, writing a value of kMultiBitBool4True\nselects external clock as clock for the system.  Writing any other value has\nno impact.\n\nWhen the current value is kMultiBitBool4True, writing a value of kMultiBitBool4False\nselects internal clock as clock for the system. Writing any other value during this stage\nhas no impact.\n\nWhile this register can always be programmed, it only takes effect when debug functions are enabled\nin life cycle TEST, DEV or RMA states."]
        #[inline(always)]
        pub fn sel(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "A value of kMultiBitBool4True selects nominal speed external clock.\nAll other values selects low speed clocks.\n\nNote this field only has an effect when the !!EXTCLK_CTRL.SEL field is set to\nkMultiBitBool4True.\n\nNominal speed means the external clock is approximately the same frequency as\nthe internal oscillator source.  When this option is used, all clocks operate\nat roughly the nominal frequency.\n\nLow speed means the external clock is approximately half the frequency of the\ninternal oscillator source. When this option is used, the internal dividers are\nstepped down.  As a result, previously undivided clocks now run at half frequency,\nwhile previously divided clocks run at roughly the nominal frequency.\n\nSee external clock switch support in documentation for more details."]
        #[inline(always)]
        pub fn hi_speed_sel(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ExtclkCtrlWriteVal {
            ExtclkCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for ExtclkCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExtclkCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: ExtclkCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExtclkCtrlWriteVal(u32);
    impl ExtclkCtrlWriteVal {
        #[doc = "When the current value is not kMultiBitBool4True, writing a value of kMultiBitBool4True\nselects external clock as clock for the system.  Writing any other value has\nno impact.\n\nWhen the current value is kMultiBitBool4True, writing a value of kMultiBitBool4False\nselects internal clock as clock for the system. Writing any other value during this stage\nhas no impact.\n\nWhile this register can always be programmed, it only takes effect when debug functions are enabled\nin life cycle TEST, DEV or RMA states."]
        #[inline(always)]
        pub fn sel(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "A value of kMultiBitBool4True selects nominal speed external clock.\nAll other values selects low speed clocks.\n\nNote this field only has an effect when the !!EXTCLK_CTRL.SEL field is set to\nkMultiBitBool4True.\n\nNominal speed means the external clock is approximately the same frequency as\nthe internal oscillator source.  When this option is used, all clocks operate\nat roughly the nominal frequency.\n\nLow speed means the external clock is approximately half the frequency of the\ninternal oscillator source. When this option is used, the internal dividers are\nstepped down.  As a result, previously undivided clocks now run at half frequency,\nwhile previously divided clocks run at roughly the nominal frequency.\n\nSee external clock switch support in documentation for more details."]
        #[inline(always)]
        pub fn hi_speed_sel(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
    }
    impl From<u32> for ExtclkCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExtclkCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ExtclkCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExtclkCtrlRegwenReadVal(u32);
    impl ExtclkCtrlRegwenReadVal {
        #[doc = "When 1, the value of !!EXTCLK_CTRL can be set.  When 0, writes to !!EXTCLK_CTRL have no\neffect."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ExtclkCtrlRegwenWriteVal {
            ExtclkCtrlRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ExtclkCtrlRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExtclkCtrlRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ExtclkCtrlRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExtclkCtrlRegwenWriteVal(u32);
    impl ExtclkCtrlRegwenWriteVal {
        #[doc = "When 1, the value of !!EXTCLK_CTRL can be set.  When 0, writes to !!EXTCLK_CTRL have no\neffect."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ExtclkCtrlRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExtclkCtrlRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ExtclkCtrlRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ExtclkStatusReadVal(u32);
    impl ExtclkStatusReadVal {
        #[doc = "When !!EXTCLK_CTRL.SEL is set to kMultiBitBool4True, this field reflects\nwhether the clock has been switched the external source.\n\nkMultiBitBool4True indicates the switch is complete.\nkMultiBitBool4False indicates the switch is either not possible or still ongoing."]
        #[inline(always)]
        pub fn ack(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
    }
    impl From<u32> for ExtclkStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ExtclkStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: ExtclkStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FatalErrCodeReadVal(u32);
    impl FatalErrCodeReadVal {
        #[doc = "Register file has experienced a fatal integrity error."]
        #[inline(always)]
        pub fn reg_intg(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "One of the idle counts encountered a duplicate error."]
        #[inline(always)]
        pub fn idle_cnt(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "One of the shadow registers encountered a storage error."]
        #[inline(always)]
        pub fn shadow_storage_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
    }
    impl From<u32> for FatalErrCodeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FatalErrCodeReadVal> for u32 {
        #[inline(always)]
        fn from(val: FatalErrCodeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv2MeasCtrlEnReadVal(u32);
    impl IoDiv2MeasCtrlEnReadVal {
        #[doc = "Enable measurement for io_div2"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoDiv2MeasCtrlEnWriteVal {
            IoDiv2MeasCtrlEnWriteVal(self.0)
        }
    }
    impl From<u32> for IoDiv2MeasCtrlEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv2MeasCtrlEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv2MeasCtrlEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv2MeasCtrlEnWriteVal(u32);
    impl IoDiv2MeasCtrlEnWriteVal {
        #[doc = "Enable measurement for io_div2"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for IoDiv2MeasCtrlEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv2MeasCtrlEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv2MeasCtrlEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv2MeasCtrlShadowedReadVal(u32);
    impl IoDiv2MeasCtrlShadowedReadVal {
        #[doc = "Max threshold for io_div2 measurement"]
        #[inline(always)]
        pub fn hi(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = "Min threshold for io_div2 measurement"]
        #[inline(always)]
        pub fn lo(&self) -> u32 {
            (self.0 >> 9) & 0x1ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoDiv2MeasCtrlShadowedWriteVal {
            IoDiv2MeasCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for IoDiv2MeasCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv2MeasCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv2MeasCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv2MeasCtrlShadowedWriteVal(u32);
    impl IoDiv2MeasCtrlShadowedWriteVal {
        #[doc = "Max threshold for io_div2 measurement"]
        #[inline(always)]
        pub fn hi(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "Min threshold for io_div2 measurement"]
        #[inline(always)]
        pub fn lo(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 9)) | ((val & 0x1ff) << 9))
        }
    }
    impl From<u32> for IoDiv2MeasCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv2MeasCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv2MeasCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv4MeasCtrlEnReadVal(u32);
    impl IoDiv4MeasCtrlEnReadVal {
        #[doc = "Enable measurement for io_div4"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoDiv4MeasCtrlEnWriteVal {
            IoDiv4MeasCtrlEnWriteVal(self.0)
        }
    }
    impl From<u32> for IoDiv4MeasCtrlEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv4MeasCtrlEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv4MeasCtrlEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv4MeasCtrlEnWriteVal(u32);
    impl IoDiv4MeasCtrlEnWriteVal {
        #[doc = "Enable measurement for io_div4"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for IoDiv4MeasCtrlEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv4MeasCtrlEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv4MeasCtrlEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv4MeasCtrlShadowedReadVal(u32);
    impl IoDiv4MeasCtrlShadowedReadVal {
        #[doc = "Max threshold for io_div4 measurement"]
        #[inline(always)]
        pub fn hi(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Min threshold for io_div4 measurement"]
        #[inline(always)]
        pub fn lo(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoDiv4MeasCtrlShadowedWriteVal {
            IoDiv4MeasCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for IoDiv4MeasCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv4MeasCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv4MeasCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoDiv4MeasCtrlShadowedWriteVal(u32);
    impl IoDiv4MeasCtrlShadowedWriteVal {
        #[doc = "Max threshold for io_div4 measurement"]
        #[inline(always)]
        pub fn hi(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "Min threshold for io_div4 measurement"]
        #[inline(always)]
        pub fn lo(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 8)) | ((val & 0xff) << 8))
        }
    }
    impl From<u32> for IoDiv4MeasCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoDiv4MeasCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoDiv4MeasCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoMeasCtrlEnReadVal(u32);
    impl IoMeasCtrlEnReadVal {
        #[doc = "Enable measurement for io"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoMeasCtrlEnWriteVal {
            IoMeasCtrlEnWriteVal(self.0)
        }
    }
    impl From<u32> for IoMeasCtrlEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoMeasCtrlEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoMeasCtrlEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoMeasCtrlEnWriteVal(u32);
    impl IoMeasCtrlEnWriteVal {
        #[doc = "Enable measurement for io"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for IoMeasCtrlEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoMeasCtrlEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoMeasCtrlEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoMeasCtrlShadowedReadVal(u32);
    impl IoMeasCtrlShadowedReadVal {
        #[doc = "Max threshold for io measurement"]
        #[inline(always)]
        pub fn hi(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = "Min threshold for io measurement"]
        #[inline(always)]
        pub fn lo(&self) -> u32 {
            (self.0 >> 10) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoMeasCtrlShadowedWriteVal {
            IoMeasCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for IoMeasCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoMeasCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoMeasCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoMeasCtrlShadowedWriteVal(u32);
    impl IoMeasCtrlShadowedWriteVal {
        #[doc = "Max threshold for io measurement"]
        #[inline(always)]
        pub fn hi(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[doc = "Min threshold for io measurement"]
        #[inline(always)]
        pub fn lo(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 10)) | ((val & 0x3ff) << 10))
        }
    }
    impl From<u32> for IoMeasCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoMeasCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoMeasCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JitterEnableReadVal(u32);
    impl JitterEnableReadVal {
        #[doc = "Enable jittery clock.\nAt reset, this register reads as kMultiBitBool4False and the jittery clock is disabled.\nAny write to the register turns the value to kMultiBitBool4True and enables the jittery clock.\nThe value written doesn't matter.\nThe value then remains kMultiBitBool4True until reset."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> JitterEnableWriteVal {
            JitterEnableWriteVal(self.0)
        }
    }
    impl From<u32> for JitterEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JitterEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: JitterEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JitterEnableWriteVal(u32);
    impl JitterEnableWriteVal {
        #[doc = "Enable jittery clock.\nAt reset, this register reads as kMultiBitBool4False and the jittery clock is disabled.\nAny write to the register turns the value to kMultiBitBool4True and enables the jittery clock.\nThe value written doesn't matter.\nThe value then remains kMultiBitBool4True until reset."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for JitterEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JitterEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: JitterEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JitterRegwenReadVal(u32);
    impl JitterRegwenReadVal {
        #[doc = "This register has no effect."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> JitterRegwenWriteVal {
            JitterRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for JitterRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JitterRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: JitterRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JitterRegwenWriteVal(u32);
    impl JitterRegwenWriteVal {
        #[doc = "This register has no effect."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for JitterRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JitterRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: JitterRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MainMeasCtrlEnReadVal(u32);
    impl MainMeasCtrlEnReadVal {
        #[doc = "Enable measurement for main"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MainMeasCtrlEnWriteVal {
            MainMeasCtrlEnWriteVal(self.0)
        }
    }
    impl From<u32> for MainMeasCtrlEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MainMeasCtrlEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: MainMeasCtrlEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MainMeasCtrlEnWriteVal(u32);
    impl MainMeasCtrlEnWriteVal {
        #[doc = "Enable measurement for main"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for MainMeasCtrlEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MainMeasCtrlEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MainMeasCtrlEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MainMeasCtrlShadowedReadVal(u32);
    impl MainMeasCtrlShadowedReadVal {
        #[doc = "Max threshold for main measurement"]
        #[inline(always)]
        pub fn hi(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = "Min threshold for main measurement"]
        #[inline(always)]
        pub fn lo(&self) -> u32 {
            (self.0 >> 10) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MainMeasCtrlShadowedWriteVal {
            MainMeasCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for MainMeasCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MainMeasCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: MainMeasCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MainMeasCtrlShadowedWriteVal(u32);
    impl MainMeasCtrlShadowedWriteVal {
        #[doc = "Max threshold for main measurement"]
        #[inline(always)]
        pub fn hi(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[doc = "Min threshold for main measurement"]
        #[inline(always)]
        pub fn lo(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 10)) | ((val & 0x3ff) << 10))
        }
    }
    impl From<u32> for MainMeasCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MainMeasCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MainMeasCtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MeasureCtrlRegwenReadVal(u32);
    impl MeasureCtrlRegwenReadVal {
        #[doc = "When 1, the value of the measurement control can be set.  When 0, writes have no\neffect."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MeasureCtrlRegwenWriteVal {
            MeasureCtrlRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MeasureCtrlRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MeasureCtrlRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MeasureCtrlRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MeasureCtrlRegwenWriteVal(u32);
    impl MeasureCtrlRegwenWriteVal {
        #[doc = "When 1, the value of the measurement control can be set.  When 0, writes have no\neffect."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MeasureCtrlRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MeasureCtrlRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MeasureCtrlRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovErrCodeReadVal(u32);
    impl RecovErrCodeReadVal {
        #[doc = "One of the shadow registers encountered an update error."]
        #[inline(always)]
        pub fn shadow_update_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "io has encountered a measurement error."]
        #[inline(always)]
        pub fn io_measure_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "io_div2 has encountered a measurement error."]
        #[inline(always)]
        pub fn io_div2_measure_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "io_div4 has encountered a measurement error."]
        #[inline(always)]
        pub fn io_div4_measure_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "main has encountered a measurement error."]
        #[inline(always)]
        pub fn main_measure_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "usb has encountered a measurement error."]
        #[inline(always)]
        pub fn usb_measure_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "io has timed out."]
        #[inline(always)]
        pub fn io_timeout_err(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "io_div2 has timed out."]
        #[inline(always)]
        pub fn io_div2_timeout_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "io_div4 has timed out."]
        #[inline(always)]
        pub fn io_div4_timeout_err(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "main has timed out."]
        #[inline(always)]
        pub fn main_timeout_err(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "usb has timed out."]
        #[inline(always)]
        pub fn usb_timeout_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RecovErrCodeWriteVal {
            RecovErrCodeWriteVal(self.0)
        }
    }
    impl From<u32> for RecovErrCodeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovErrCodeReadVal> for u32 {
        #[inline(always)]
        fn from(val: RecovErrCodeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovErrCodeWriteVal(u32);
    impl RecovErrCodeWriteVal {
        #[doc = "One of the shadow registers encountered an update error."]
        #[inline(always)]
        pub fn shadow_update_err_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "io has encountered a measurement error."]
        #[inline(always)]
        pub fn io_measure_err_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "io_div2 has encountered a measurement error."]
        #[inline(always)]
        pub fn io_div2_measure_err_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "io_div4 has encountered a measurement error."]
        #[inline(always)]
        pub fn io_div4_measure_err_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "main has encountered a measurement error."]
        #[inline(always)]
        pub fn main_measure_err_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "usb has encountered a measurement error."]
        #[inline(always)]
        pub fn usb_measure_err_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "io has timed out."]
        #[inline(always)]
        pub fn io_timeout_err_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "io_div2 has timed out."]
        #[inline(always)]
        pub fn io_div2_timeout_err_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
        #[doc = "io_div4 has timed out."]
        #[inline(always)]
        pub fn io_div4_timeout_err_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "main has timed out."]
        #[inline(always)]
        pub fn main_timeout_err_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "usb has timed out."]
        #[inline(always)]
        pub fn usb_timeout_err_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
    }
    impl From<u32> for RecovErrCodeWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovErrCodeWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RecovErrCodeWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbMeasCtrlEnReadVal(u32);
    impl UsbMeasCtrlEnReadVal {
        #[doc = "Enable measurement for usb"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UsbMeasCtrlEnWriteVal {
            UsbMeasCtrlEnWriteVal(self.0)
        }
    }
    impl From<u32> for UsbMeasCtrlEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbMeasCtrlEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: UsbMeasCtrlEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbMeasCtrlEnWriteVal(u32);
    impl UsbMeasCtrlEnWriteVal {
        #[doc = "Enable measurement for usb"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for UsbMeasCtrlEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbMeasCtrlEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UsbMeasCtrlEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbMeasCtrlShadowedReadVal(u32);
    impl UsbMeasCtrlShadowedReadVal {
        #[doc = "Max threshold for usb measurement"]
        #[inline(always)]
        pub fn hi(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = "Min threshold for usb measurement"]
        #[inline(always)]
        pub fn lo(&self) -> u32 {
            (self.0 >> 9) & 0x1ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UsbMeasCtrlShadowedWriteVal {
            UsbMeasCtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for UsbMeasCtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbMeasCtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: UsbMeasCtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbMeasCtrlShadowedWriteVal(u32);
    impl UsbMeasCtrlShadowedWriteVal {
        #[doc = "Max threshold for usb measurement"]
        #[inline(always)]
        pub fn hi(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "Min threshold for usb measurement"]
        #[inline(always)]
        pub fn lo(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 9)) | ((val & 0x1ff) << 9))
        }
    }
    impl From<u32> for UsbMeasCtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbMeasCtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UsbMeasCtrlShadowedWriteVal) -> u32 {
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
    pub type ExtclkCtrlRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ExtclkCtrlRegwenReadVal,
        crate::regs::ExtclkCtrlRegwenWriteVal,
    >;
    pub type ExtclkCtrl =
        ureg::ReadWriteReg32<0x99, crate::regs::ExtclkCtrlReadVal, crate::regs::ExtclkCtrlWriteVal>;
    pub type ExtclkStatus = ureg::ReadOnlyReg32<crate::regs::ExtclkStatusReadVal>;
    pub type JitterRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::JitterRegwenReadVal,
        crate::regs::JitterRegwenWriteVal,
    >;
    pub type JitterEnable = ureg::ReadWriteReg32<
        9,
        crate::regs::JitterEnableReadVal,
        crate::regs::JitterEnableWriteVal,
    >;
    pub type ClkEnables =
        ureg::ReadWriteReg32<0xf, crate::regs::ClkEnablesReadVal, crate::regs::ClkEnablesWriteVal>;
    pub type ClkHints =
        ureg::ReadWriteReg32<0xf, crate::regs::ClkHintsReadVal, crate::regs::ClkHintsWriteVal>;
    pub type ClkHintsStatus = ureg::ReadOnlyReg32<crate::regs::ClkHintsStatusReadVal>;
    pub type MeasureCtrlRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MeasureCtrlRegwenReadVal,
        crate::regs::MeasureCtrlRegwenWriteVal,
    >;
    pub type IoMeasCtrlEn = ureg::ReadWriteReg32<
        9,
        crate::regs::IoMeasCtrlEnReadVal,
        crate::regs::IoMeasCtrlEnWriteVal,
    >;
    pub type IoMeasCtrlShadowed = ureg::ReadWriteReg32<
        0x759ea,
        crate::regs::IoMeasCtrlShadowedReadVal,
        crate::regs::IoMeasCtrlShadowedWriteVal,
    >;
    pub type IoDiv2MeasCtrlEn = ureg::ReadWriteReg32<
        9,
        crate::regs::IoDiv2MeasCtrlEnReadVal,
        crate::regs::IoDiv2MeasCtrlEnWriteVal,
    >;
    pub type IoDiv2MeasCtrlShadowed = ureg::ReadWriteReg32<
        0x1ccfa,
        crate::regs::IoDiv2MeasCtrlShadowedReadVal,
        crate::regs::IoDiv2MeasCtrlShadowedWriteVal,
    >;
    pub type IoDiv4MeasCtrlEn = ureg::ReadWriteReg32<
        9,
        crate::regs::IoDiv4MeasCtrlEnReadVal,
        crate::regs::IoDiv4MeasCtrlEnWriteVal,
    >;
    pub type IoDiv4MeasCtrlShadowed = ureg::ReadWriteReg32<
        0x6e82,
        crate::regs::IoDiv4MeasCtrlShadowedReadVal,
        crate::regs::IoDiv4MeasCtrlShadowedWriteVal,
    >;
    pub type MainMeasCtrlEn = ureg::ReadWriteReg32<
        9,
        crate::regs::MainMeasCtrlEnReadVal,
        crate::regs::MainMeasCtrlEnWriteVal,
    >;
    pub type MainMeasCtrlShadowed = ureg::ReadWriteReg32<
        0x7a9fe,
        crate::regs::MainMeasCtrlShadowedReadVal,
        crate::regs::MainMeasCtrlShadowedWriteVal,
    >;
    pub type UsbMeasCtrlEn = ureg::ReadWriteReg32<
        9,
        crate::regs::UsbMeasCtrlEnReadVal,
        crate::regs::UsbMeasCtrlEnWriteVal,
    >;
    pub type UsbMeasCtrlShadowed = ureg::ReadWriteReg32<
        0x1ccfa,
        crate::regs::UsbMeasCtrlShadowedReadVal,
        crate::regs::UsbMeasCtrlShadowedWriteVal,
    >;
    pub type RecovErrCode = ureg::ReadWriteReg32<
        0,
        crate::regs::RecovErrCodeReadVal,
        crate::regs::RecovErrCodeWriteVal,
    >;
    pub type FatalErrCode = ureg::ReadOnlyReg32<crate::regs::FatalErrCodeReadVal>;
}
