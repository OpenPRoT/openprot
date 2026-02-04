#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SysrstCtrlAon {
    _priv: (),
}
impl SysrstCtrlAon {
    pub const PTR: *mut u32 = 0x40430000 as *mut u32;
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
    #[doc = "Configuration write enable control register\n\nRead value: [`regs::RegwenReadVal`]; Write value: [`regs::RegwenWriteVal`]"]
    #[inline(always)]
    pub fn regwen(&self) -> ureg::RegRef<crate::meta::Regwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EC reset control register\n\nRead value: [`regs::EcRstCtlReadVal`]; Write value: [`regs::EcRstCtlWriteVal`]"]
    #[inline(always)]
    pub fn ec_rst_ctl(&self) -> ureg::RegRef<crate::meta::EcRstCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ultra low power AC debounce control register\n\nRead value: [`regs::UlpAcDebounceCtlReadVal`]; Write value: [`regs::UlpAcDebounceCtlWriteVal`]"]
    #[inline(always)]
    pub fn ulp_ac_debounce_ctl(&self) -> ureg::RegRef<crate::meta::UlpAcDebounceCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ultra low power lid debounce control register\n\nRead value: [`regs::UlpLidDebounceCtlReadVal`]; Write value: [`regs::UlpLidDebounceCtlWriteVal`]"]
    #[inline(always)]
    pub fn ulp_lid_debounce_ctl(&self) -> ureg::RegRef<crate::meta::UlpLidDebounceCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ultra low power pwrb debounce control register\n\nRead value: [`regs::UlpPwrbDebounceCtlReadVal`]; Write value: [`regs::UlpPwrbDebounceCtlWriteVal`]"]
    #[inline(always)]
    pub fn ulp_pwrb_debounce_ctl(&self) -> ureg::RegRef<crate::meta::UlpPwrbDebounceCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ultra low power control register\n\nRead value: [`regs::UlpCtlReadVal`]; Write value: [`regs::UlpCtlWriteVal`]"]
    #[inline(always)]
    pub fn ulp_ctl(&self) -> ureg::RegRef<crate::meta::UlpCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ultra low power status\n\nRead value: [`regs::UlpStatusReadVal`]; Write value: [`regs::UlpStatusWriteVal`]"]
    #[inline(always)]
    pub fn ulp_status(&self) -> ureg::RegRef<crate::meta::UlpStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "wakeup status\n\nRead value: [`regs::WkupStatusReadVal`]; Write value: [`regs::WkupStatusWriteVal`]"]
    #[inline(always)]
    pub fn wkup_status(&self) -> ureg::RegRef<crate::meta::WkupStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "configure key input output invert property\n\nRead value: [`regs::KeyInvertCtlReadVal`]; Write value: [`regs::KeyInvertCtlWriteVal`]"]
    #[inline(always)]
    pub fn key_invert_ctl(&self) -> ureg::RegRef<crate::meta::KeyInvertCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register determines which override values are allowed for a given output.\nIf an override value programmed via !!PIN_OUT_VALUE is not configured as an allowed value,\nit will not have any effect.\n\nRead value: [`regs::PinAllowedCtlReadVal`]; Write value: [`regs::PinAllowedCtlWriteVal`]"]
    #[inline(always)]
    pub fn pin_allowed_ctl(&self) -> ureg::RegRef<crate::meta::PinAllowedCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enables the override function for a specific pin.\n\nRead value: [`regs::PinOutCtlReadVal`]; Write value: [`regs::PinOutCtlWriteVal`]"]
    #[inline(always)]
    pub fn pin_out_ctl(&self) -> ureg::RegRef<crate::meta::PinOutCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Sets the pin override value. Note that only the values\nconfigured as 'allowed' in !!PIN_ALLOWED_CTL will have\nan effect. Otherwise the pin value will not be overridden.\n\nRead value: [`regs::PinOutValueReadVal`]; Write value: [`regs::PinOutValueWriteVal`]"]
    #[inline(always)]
    pub fn pin_out_value(&self) -> ureg::RegRef<crate::meta::PinOutValue, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "For SW to read the sysrst_ctrl inputs like GPIO\n\nRead value: [`regs::PinInValueReadVal`]; Write value: [`regs::PinInValueWriteVal`]"]
    #[inline(always)]
    pub fn pin_in_value(&self) -> ureg::RegRef<crate::meta::PinInValue, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Define the keys or inputs that can trigger the interrupt\n\nRead value: [`regs::KeyIntrCtlReadVal`]; Write value: [`regs::KeyIntrCtlWriteVal`]"]
    #[inline(always)]
    pub fn key_intr_ctl(&self) -> ureg::RegRef<crate::meta::KeyIntrCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Debounce timer control register for key-triggered interrupt\n\nRead value: [`regs::KeyIntrDebounceCtlReadVal`]; Write value: [`regs::KeyIntrDebounceCtlWriteVal`]"]
    #[inline(always)]
    pub fn key_intr_debounce_ctl(&self) -> ureg::RegRef<crate::meta::KeyIntrDebounceCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Debounce timer control register for pwrb_in H2L transition\n\nRead value: [`regs::AutoBlockDebounceCtlReadVal`]; Write value: [`regs::AutoBlockDebounceCtlWriteVal`]"]
    #[inline(always)]
    pub fn auto_block_debounce_ctl(
        &self,
    ) -> ureg::RegRef<crate::meta::AutoBlockDebounceCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "configure the key outputs to auto-override and their value\n\nRead value: [`regs::AutoBlockOutCtlReadVal`]; Write value: [`regs::AutoBlockOutCtlWriteVal`]"]
    #[inline(always)]
    pub fn auto_block_out_ctl(&self) -> ureg::RegRef<crate::meta::AutoBlockOutCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "To define the keys that define the pre-condition of the combo\n[0]: key0_in_sel\n[1]: key1_in_sel\n[2]: key2_in_sel\n[3]: pwrb_in_sel\n[4]: ac_present_sel\nHW will start matching the combo as defined by !!COM_SEL_CTL if this precondition is fulfilled.\n\nIf no keys are configured for the pre-condition, the pre-condition always evaluates to true.\n\nThe debounce timing is defined via !!KEY_INTR_DEBOUNCE_CTL whereas the pre-condition pressed timing is defined via !!COM_PRE_DET_CTL.\n\nRead value: [`regs::SelCtlReadVal`]; Write value: [`regs::SelCtlWriteVal`]"]
    #[inline(always)]
    pub fn com_pre_sel_ctl(
        &self,
    ) -> ureg::Array<4, ureg::RegRef<crate::meta::ComPreSelCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "To define the duration that the combo pre-condition should be pressed\n0-60s, each step is 5us(200KHz clock)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn com_pre_det_ctl(
        &self,
    ) -> ureg::Array<4, ureg::RegRef<crate::meta::ComPreDetCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "To define the keys that trigger the combo\n[0]: key0_in_sel\n[1]: key1_in_sel\n[2]: key2_in_sel\n[3]: pwrb_in_sel\n[4]: ac_present_sel\nHW will detect H2L transition in the combo use case.\n\nOptionally, a pre-condition can be configured for the combo detection via !!COM_PRE_SEL_CTL.\n\nIf no keys are configured for the combo, the combo detection is disabled.\n\nThe debounce timing is defined via !!KEY_INTR_DEBOUNCE_CTL whereas the key-pressed timing is defined via !!COM_DET_CTL.\n\nRead value: [`regs::SelCtlReadVal`]; Write value: [`regs::SelCtlWriteVal`]"]
    #[inline(always)]
    pub fn com_sel_ctl(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::ComSelCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "To define the duration that the combo should be pressed\n0-60s, each step is 5us(200KHz clock)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn com_det_ctl(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::ComDetCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "To define the actions once the combo is detected\n[0]: bat_disable\n[1]: interrupt (to OpenTitan processor)\n[2]: ec_rst (for Embedded Controller)\n[3]: rst_req (to OpenTitan reset manager)\n\nRead value: [`regs::ComOutCtlReadVal`]; Write value: [`regs::ComOutCtlWriteVal`]"]
    #[inline(always)]
    pub fn com_out_ctl(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::ComOutCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Combo interrupt source. These registers will only be set if the\ninterrupt action is configured in the corresponding !!COM_OUT_CTL register.\n\nRead value: [`regs::ComboIntrStatusReadVal`]; Write value: [`regs::ComboIntrStatusWriteVal`]"]
    #[inline(always)]
    pub fn combo_intr_status(&self) -> ureg::RegRef<crate::meta::ComboIntrStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "key interrupt source\n\nRead value: [`regs::KeyIntrStatusReadVal`]; Write value: [`regs::KeyIntrStatusWriteVal`]"]
    #[inline(always)]
    pub fn key_intr_status(&self) -> ureg::RegRef<crate::meta::KeyIntrStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa8 / core::mem::size_of::<u32>()),
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
    pub struct AutoBlockDebounceCtlReadVal(u32);
    impl AutoBlockDebounceCtlReadVal {
        #[doc = "Define the timer value so that the pwrb_in is not oscillating in clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn debounce_timer(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn auto_block_enable(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AutoBlockDebounceCtlWriteVal {
            AutoBlockDebounceCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AutoBlockDebounceCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AutoBlockDebounceCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AutoBlockDebounceCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AutoBlockDebounceCtlWriteVal(u32);
    impl AutoBlockDebounceCtlWriteVal {
        #[doc = "Define the timer value so that the pwrb_in is not oscillating in clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn debounce_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn auto_block_enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
    }
    impl From<u32> for AutoBlockDebounceCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AutoBlockDebounceCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AutoBlockDebounceCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AutoBlockOutCtlReadVal(u32);
    impl AutoBlockOutCtlReadVal {
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key0_out_sel(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key1_out_sel(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key2_out_sel(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key0_out_value(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key1_out_value(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key2_out_value(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AutoBlockOutCtlWriteVal {
            AutoBlockOutCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AutoBlockOutCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AutoBlockOutCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AutoBlockOutCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AutoBlockOutCtlWriteVal(u32);
    impl AutoBlockOutCtlWriteVal {
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key0_out_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key1_out_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: disable auto-block; 1: enable auto-block"]
        #[inline(always)]
        pub fn key2_out_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key0_out_value(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key1_out_value(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: override to 1'b0; 1: override to 1'b1"]
        #[inline(always)]
        pub fn key2_out_value(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
    }
    impl From<u32> for AutoBlockOutCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AutoBlockOutCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AutoBlockOutCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ComboIntrStatusReadVal(u32);
    impl ComboIntrStatusReadVal {
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo0_h2_l(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo1_h2_l(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo2_h2_l(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo3_h2_l(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ComboIntrStatusWriteVal {
            ComboIntrStatusWriteVal(self.0)
        }
    }
    impl From<u32> for ComboIntrStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ComboIntrStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: ComboIntrStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ComboIntrStatusWriteVal(u32);
    impl ComboIntrStatusWriteVal {
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo0_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo1_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo2_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn combo3_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
    }
    impl From<u32> for ComboIntrStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ComboIntrStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ComboIntrStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ComOutCtlReadVal(u32);
    impl ComOutCtlReadVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn bat_disable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn interrupt(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn rst_req(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ComOutCtlWriteVal {
            ComOutCtlWriteVal(self.0)
        }
    }
    impl From<u32> for ComOutCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ComOutCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: ComOutCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ComOutCtlWriteVal(u32);
    impl ComOutCtlWriteVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn bat_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn interrupt(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn rst_req(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for ComOutCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ComOutCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ComOutCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EcRstCtlReadVal(u32);
    impl EcRstCtlReadVal {
        #[doc = "Configure the pulse width of ec_rst_l.\nEach step is 5 us for a 200 kHz clock."]
        #[inline(always)]
        pub fn ec_rst_pulse(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EcRstCtlWriteVal {
            EcRstCtlWriteVal(self.0)
        }
    }
    impl From<u32> for EcRstCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EcRstCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: EcRstCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EcRstCtlWriteVal(u32);
    impl EcRstCtlWriteVal {
        #[doc = "Configure the pulse width of ec_rst_l.\nEach step is 5 us for a 200 kHz clock."]
        #[inline(always)]
        pub fn ec_rst_pulse(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for EcRstCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EcRstCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: EcRstCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.event_detected is set."]
        #[inline(always)]
        pub fn event_detected(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.event_detected is set."]
        #[inline(always)]
        pub fn event_detected(self, val: bool) -> Self {
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
        #[doc = "Common interrupt triggered by combo or keyboard events."]
        #[inline(always)]
        pub fn event_detected(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
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
    pub struct IntrTestWriteVal(u32);
    impl IntrTestWriteVal {
        #[doc = "Write 1 to force !!INTR_STATE.event_detected to 1."]
        #[inline(always)]
        pub fn event_detected(self, val: bool) -> Self {
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
    pub struct KeyIntrCtlReadVal(u32);
    impl KeyIntrCtlReadVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_h2_l(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_h2_l(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_h2_l(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_h2_l(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_h2_l(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst_l_h2_l(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn flash_wp_l_h2_l(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_l2_h(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_l2_h(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_l2_h(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_l2_h(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_l2_h(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst_l_l2_h(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn flash_wp_l_l2_h(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> KeyIntrCtlWriteVal {
            KeyIntrCtlWriteVal(self.0)
        }
    }
    impl From<u32> for KeyIntrCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyIntrCtlWriteVal(u32);
    impl KeyIntrCtlWriteVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst_l_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn flash_wp_l_h2_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ec_rst_l_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn flash_wp_l_l2_h(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
    }
    impl From<u32> for KeyIntrCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyIntrDebounceCtlReadVal(u32);
    impl KeyIntrDebounceCtlReadVal {
        #[doc = "Define the timer value so that the key or input is not oscillating in clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn debounce_timer(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> KeyIntrDebounceCtlWriteVal {
            KeyIntrDebounceCtlWriteVal(self.0)
        }
    }
    impl From<u32> for KeyIntrDebounceCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrDebounceCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrDebounceCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyIntrDebounceCtlWriteVal(u32);
    impl KeyIntrDebounceCtlWriteVal {
        #[doc = "Define the timer value so that the key or input is not oscillating in clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn debounce_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for KeyIntrDebounceCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrDebounceCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrDebounceCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyIntrStatusReadVal(u32);
    impl KeyIntrStatusReadVal {
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn pwrb_h2_l(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key0_in_h2_l(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key1_in_h2_l(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key2_in_h2_l(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ac_present_h2_l(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ec_rst_l_h2_l(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn flash_wp_l_h2_l(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn pwrb_l2_h(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key0_in_l2_h(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key1_in_l2_h(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key2_in_l2_h(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ac_present_l2_h(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ec_rst_l_l2_h(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn flash_wp_l_l2_h(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> KeyIntrStatusWriteVal {
            KeyIntrStatusWriteVal(self.0)
        }
    }
    impl From<u32> for KeyIntrStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyIntrStatusWriteVal(u32);
    impl KeyIntrStatusWriteVal {
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn pwrb_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key0_in_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key1_in_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key2_in_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ac_present_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ec_rst_l_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn flash_wp_l_h2_l_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn pwrb_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key0_in_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key1_in_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn key2_in_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ac_present_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 11))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn ec_rst_l_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 12))
        }
        #[doc = "0: case not detected;1: case detected"]
        #[inline(always)]
        pub fn flash_wp_l_l2_h_clear(self) -> Self {
            Self(self.0 | (1 << 13))
        }
    }
    impl From<u32> for KeyIntrStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyIntrStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: KeyIntrStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyInvertCtlReadVal(u32);
    impl KeyInvertCtlReadVal {
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key0_in(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key0_out(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key1_in(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key1_out(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key2_in(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key2_out(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn pwrb_in(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn pwrb_out(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn ac_present(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn bat_disable(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn lid_open(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn z3_wakeup(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> KeyInvertCtlWriteVal {
            KeyInvertCtlWriteVal(self.0)
        }
    }
    impl From<u32> for KeyInvertCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyInvertCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: KeyInvertCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct KeyInvertCtlWriteVal(u32);
    impl KeyInvertCtlWriteVal {
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key0_in(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key0_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key1_in(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key1_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key2_in(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn key2_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn pwrb_in(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn pwrb_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn ac_present(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn bat_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn lid_open(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "0: don't invert; 1: invert"]
        #[inline(always)]
        pub fn z3_wakeup(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for KeyInvertCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyInvertCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: KeyInvertCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinAllowedCtlReadVal(u32);
    impl PinAllowedCtlReadVal {
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn bat_disable_0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn ec_rst_l_0(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn pwrb_out_0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key0_out_0(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key1_out_0(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key2_out_0(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn z3_wakeup_0(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn flash_wp_l_0(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn bat_disable_1(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn ec_rst_l_1(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn pwrb_out_1(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key0_out_1(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key1_out_1(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key2_out_1(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn z3_wakeup_1(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn flash_wp_l_1(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PinAllowedCtlWriteVal {
            PinAllowedCtlWriteVal(self.0)
        }
    }
    impl From<u32> for PinAllowedCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinAllowedCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: PinAllowedCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinAllowedCtlWriteVal(u32);
    impl PinAllowedCtlWriteVal {
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn bat_disable_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn ec_rst_l_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn pwrb_out_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key0_out_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key1_out_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key2_out_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn z3_wakeup_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn flash_wp_l_0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn bat_disable_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn ec_rst_l_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn pwrb_out_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key0_out_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key1_out_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn key2_out_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn z3_wakeup_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "0: not allowed; 1: allowed"]
        #[inline(always)]
        pub fn flash_wp_l_1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
    }
    impl From<u32> for PinAllowedCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinAllowedCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PinAllowedCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinInValueReadVal(u32);
    impl PinInValueReadVal {
        #[doc = "raw pwrb_in value; before the invert logic"]
        #[inline(always)]
        pub fn pwrb_in(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "raw key0_in value; before the invert logic"]
        #[inline(always)]
        pub fn key0_in(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "raw key1_in value; before the invert logic"]
        #[inline(always)]
        pub fn key1_in(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "raw key2_in value; before the invert logic"]
        #[inline(always)]
        pub fn key2_in(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "raw lid_open value; before the invert logic"]
        #[inline(always)]
        pub fn lid_open(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "raw ac_present value; before the invert logic"]
        #[inline(always)]
        pub fn ac_present(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "raw ec_rst_l value; before the invert logic"]
        #[inline(always)]
        pub fn ec_rst_l(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "raw flash_wp_l value; before the invert logic"]
        #[inline(always)]
        pub fn flash_wp_l(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
    }
    impl From<u32> for PinInValueReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinInValueReadVal> for u32 {
        #[inline(always)]
        fn from(val: PinInValueReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinOutCtlReadVal(u32);
    impl PinOutCtlReadVal {
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn bat_disable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn ec_rst_l(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn pwrb_out(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key0_out(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key1_out(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key2_out(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn z3_wakeup(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn flash_wp_l(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PinOutCtlWriteVal {
            PinOutCtlWriteVal(self.0)
        }
    }
    impl From<u32> for PinOutCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinOutCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: PinOutCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinOutCtlWriteVal(u32);
    impl PinOutCtlWriteVal {
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn bat_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn ec_rst_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn pwrb_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key0_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key1_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn key2_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn z3_wakeup(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "0: disable override; 1: enable override"]
        #[inline(always)]
        pub fn flash_wp_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for PinOutCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinOutCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PinOutCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinOutValueReadVal(u32);
    impl PinOutValueReadVal {
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn bat_disable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn ec_rst_l(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn pwrb_out(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key0_out(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key1_out(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key2_out(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn z3_wakeup(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn flash_wp_l(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PinOutValueWriteVal {
            PinOutValueWriteVal(self.0)
        }
    }
    impl From<u32> for PinOutValueReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinOutValueReadVal> for u32 {
        #[inline(always)]
        fn from(val: PinOutValueReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PinOutValueWriteVal(u32);
    impl PinOutValueWriteVal {
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn bat_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn ec_rst_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn pwrb_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key0_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key1_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn key2_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn z3_wakeup(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "0: override to 1b0; 1: override to 1b1"]
        #[inline(always)]
        pub fn flash_wp_l(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for PinOutValueWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PinOutValueWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PinOutValueWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegwenReadVal(u32);
    impl RegwenReadVal {
        #[doc = "config write enable.\n0: cfg is locked(not writable); 1: cfg is not locked(writable)"]
        #[inline(always)]
        pub fn write_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RegwenWriteVal {
            RegwenWriteVal(self.0)
        }
    }
    impl From<u32> for RegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: RegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegwenWriteVal(u32);
    impl RegwenWriteVal {
        #[doc = "config write enable.\n0: cfg is locked(not writable); 1: cfg is not locked(writable)"]
        #[inline(always)]
        pub fn write_en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for RegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpAcDebounceCtlReadVal(u32);
    impl UlpAcDebounceCtlReadVal {
        #[doc = "Configure the debounce timer for the AC input in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_ac_debounce_timer(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UlpAcDebounceCtlWriteVal {
            UlpAcDebounceCtlWriteVal(self.0)
        }
    }
    impl From<u32> for UlpAcDebounceCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpAcDebounceCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: UlpAcDebounceCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpAcDebounceCtlWriteVal(u32);
    impl UlpAcDebounceCtlWriteVal {
        #[doc = "Configure the debounce timer for the AC input in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_ac_debounce_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for UlpAcDebounceCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpAcDebounceCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UlpAcDebounceCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpCtlReadVal(u32);
    impl UlpCtlReadVal {
        #[doc = "0: disable ULP wakeup feature and reset the ULP FSM; 1: enable ULP wakeup feature"]
        #[inline(always)]
        pub fn ulp_enable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UlpCtlWriteVal {
            UlpCtlWriteVal(self.0)
        }
    }
    impl From<u32> for UlpCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: UlpCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpCtlWriteVal(u32);
    impl UlpCtlWriteVal {
        #[doc = "0: disable ULP wakeup feature and reset the ULP FSM; 1: enable ULP wakeup feature"]
        #[inline(always)]
        pub fn ulp_enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for UlpCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UlpCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpLidDebounceCtlReadVal(u32);
    impl UlpLidDebounceCtlReadVal {
        #[doc = "Configure the debounce timer for the lid in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_lid_debounce_timer(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UlpLidDebounceCtlWriteVal {
            UlpLidDebounceCtlWriteVal(self.0)
        }
    }
    impl From<u32> for UlpLidDebounceCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpLidDebounceCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: UlpLidDebounceCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpLidDebounceCtlWriteVal(u32);
    impl UlpLidDebounceCtlWriteVal {
        #[doc = "Configure the debounce timer for the lid in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_lid_debounce_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for UlpLidDebounceCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpLidDebounceCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UlpLidDebounceCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpPwrbDebounceCtlReadVal(u32);
    impl UlpPwrbDebounceCtlReadVal {
        #[doc = "Configure the debounce timer for the power button in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_pwrb_debounce_timer(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UlpPwrbDebounceCtlWriteVal {
            UlpPwrbDebounceCtlWriteVal(self.0)
        }
    }
    impl From<u32> for UlpPwrbDebounceCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpPwrbDebounceCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: UlpPwrbDebounceCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpPwrbDebounceCtlWriteVal(u32);
    impl UlpPwrbDebounceCtlWriteVal {
        #[doc = "Configure the debounce timer for the power button in number of clock cycles.\nEach step is 5 us for a 200 kHz clock.\nThe signal must exceed the debounce time by at least one clock cycle to be detected."]
        #[inline(always)]
        pub fn ulp_pwrb_debounce_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for UlpPwrbDebounceCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpPwrbDebounceCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UlpPwrbDebounceCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpStatusReadVal(u32);
    impl UlpStatusReadVal {
        #[doc = "0: ULP wakeup not detected; 1: ULP wakeup event is detected"]
        #[inline(always)]
        pub fn ulp_wakeup(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UlpStatusWriteVal {
            UlpStatusWriteVal(self.0)
        }
    }
    impl From<u32> for UlpStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: UlpStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UlpStatusWriteVal(u32);
    impl UlpStatusWriteVal {
        #[doc = "0: ULP wakeup not detected; 1: ULP wakeup event is detected"]
        #[inline(always)]
        pub fn ulp_wakeup_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for UlpStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UlpStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UlpStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupStatusReadVal(u32);
    impl WkupStatusReadVal {
        #[doc = "0: wakeup event not detected; 1: wakeup event is detected"]
        #[inline(always)]
        pub fn wakeup_sts(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupStatusWriteVal {
            WkupStatusWriteVal(self.0)
        }
    }
    impl From<u32> for WkupStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupStatusWriteVal(u32);
    impl WkupStatusWriteVal {
        #[doc = "0: wakeup event not detected; 1: wakeup event is detected"]
        #[inline(always)]
        pub fn wakeup_sts_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for WkupStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SelCtlReadVal(u32);
    impl SelCtlReadVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_sel(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_sel(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_sel(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_sel(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_sel(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SelCtlWriteVal {
            SelCtlWriteVal(self.0)
        }
    }
    impl From<u32> for SelCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SelCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: SelCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SelCtlWriteVal(u32);
    impl SelCtlWriteVal {
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key0_in_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key1_in_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn key2_in_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn pwrb_in_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: disable, 1: enable"]
        #[inline(always)]
        pub fn ac_present_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
    }
    impl From<u32> for SelCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SelCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SelCtlWriteVal) -> u32 {
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
    pub type IntrState = ureg::ReadOnlyReg32<crate::regs::IntrStateReadVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Regwen =
        ureg::ReadWriteReg32<1, crate::regs::RegwenReadVal, crate::regs::RegwenWriteVal>;
    pub type EcRstCtl =
        ureg::ReadWriteReg32<0x7d0, crate::regs::EcRstCtlReadVal, crate::regs::EcRstCtlWriteVal>;
    pub type UlpAcDebounceCtl = ureg::ReadWriteReg32<
        0x1f40,
        crate::regs::UlpAcDebounceCtlReadVal,
        crate::regs::UlpAcDebounceCtlWriteVal,
    >;
    pub type UlpLidDebounceCtl = ureg::ReadWriteReg32<
        0x1f40,
        crate::regs::UlpLidDebounceCtlReadVal,
        crate::regs::UlpLidDebounceCtlWriteVal,
    >;
    pub type UlpPwrbDebounceCtl = ureg::ReadWriteReg32<
        0x1f40,
        crate::regs::UlpPwrbDebounceCtlReadVal,
        crate::regs::UlpPwrbDebounceCtlWriteVal,
    >;
    pub type UlpCtl =
        ureg::ReadWriteReg32<0, crate::regs::UlpCtlReadVal, crate::regs::UlpCtlWriteVal>;
    pub type UlpStatus =
        ureg::ReadWriteReg32<0, crate::regs::UlpStatusReadVal, crate::regs::UlpStatusWriteVal>;
    pub type WkupStatus =
        ureg::ReadWriteReg32<0, crate::regs::WkupStatusReadVal, crate::regs::WkupStatusWriteVal>;
    pub type KeyInvertCtl = ureg::ReadWriteReg32<
        0,
        crate::regs::KeyInvertCtlReadVal,
        crate::regs::KeyInvertCtlWriteVal,
    >;
    pub type PinAllowedCtl = ureg::ReadWriteReg32<
        0x82,
        crate::regs::PinAllowedCtlReadVal,
        crate::regs::PinAllowedCtlWriteVal,
    >;
    pub type PinOutCtl =
        ureg::ReadWriteReg32<0x82, crate::regs::PinOutCtlReadVal, crate::regs::PinOutCtlWriteVal>;
    pub type PinOutValue =
        ureg::ReadWriteReg32<0, crate::regs::PinOutValueReadVal, crate::regs::PinOutValueWriteVal>;
    pub type PinInValue = ureg::ReadOnlyReg32<crate::regs::PinInValueReadVal>;
    pub type KeyIntrCtl =
        ureg::ReadWriteReg32<0, crate::regs::KeyIntrCtlReadVal, crate::regs::KeyIntrCtlWriteVal>;
    pub type KeyIntrDebounceCtl = ureg::ReadWriteReg32<
        0x7d0,
        crate::regs::KeyIntrDebounceCtlReadVal,
        crate::regs::KeyIntrDebounceCtlWriteVal,
    >;
    pub type AutoBlockDebounceCtl = ureg::ReadWriteReg32<
        0x7d0,
        crate::regs::AutoBlockDebounceCtlReadVal,
        crate::regs::AutoBlockDebounceCtlWriteVal,
    >;
    pub type AutoBlockOutCtl = ureg::ReadWriteReg32<
        0,
        crate::regs::AutoBlockOutCtlReadVal,
        crate::regs::AutoBlockOutCtlWriteVal,
    >;
    pub type ComPreSelCtl =
        ureg::ReadWriteReg32<0, crate::regs::SelCtlReadVal, crate::regs::SelCtlWriteVal>;
    pub type ComPreDetCtl = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ComSelCtl =
        ureg::ReadWriteReg32<0, crate::regs::SelCtlReadVal, crate::regs::SelCtlWriteVal>;
    pub type ComDetCtl = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ComOutCtl =
        ureg::ReadWriteReg32<0, crate::regs::ComOutCtlReadVal, crate::regs::ComOutCtlWriteVal>;
    pub type ComboIntrStatus = ureg::ReadWriteReg32<
        0,
        crate::regs::ComboIntrStatusReadVal,
        crate::regs::ComboIntrStatusWriteVal,
    >;
    pub type KeyIntrStatus = ureg::ReadWriteReg32<
        0,
        crate::regs::KeyIntrStatusReadVal,
        crate::regs::KeyIntrStatusWriteVal,
    >;
}
