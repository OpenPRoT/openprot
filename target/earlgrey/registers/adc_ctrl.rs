#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct AdcCtrlAon {
    _priv: (),
}
impl AdcCtrlAon {
    pub const PTR: *mut u32 = 0x40440000 as *mut u32;
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
    #[doc = "ADC enable control register\n\nRead value: [`regs::AdcEnCtlReadVal`]; Write value: [`regs::AdcEnCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_en_ctl(&self) -> ureg::RegRef<crate::meta::AdcEnCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC PowerDown(PD) control register\n\nRead value: [`regs::AdcPdCtlReadVal`]; Write value: [`regs::AdcPdCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_pd_ctl(&self) -> ureg::RegRef<crate::meta::AdcPdCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC Low-Power(LP) sample control register\n\nRead value: [`regs::AdcLpSampleCtlReadVal`]; Write value: [`regs::AdcLpSampleCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_lp_sample_ctl(&self) -> ureg::RegRef<crate::meta::AdcLpSampleCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC sample control register\n\nRead value: [`regs::AdcSampleCtlReadVal`]; Write value: [`regs::AdcSampleCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_sample_ctl(&self) -> ureg::RegRef<crate::meta::AdcSampleCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC FSM reset control\n\nRead value: [`regs::AdcFsmRstReadVal`]; Write value: [`regs::AdcFsmRstWriteVal`]"]
    #[inline(always)]
    pub fn adc_fsm_rst(&self) -> ureg::RegRef<crate::meta::AdcFsmRst, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC channel0 filter range\n\nUp to 8 filters can be configured per channel and each filter has an associated [min, max] range.\nThe condition bit then defines whether the sample values of that channel need to lie within the range or outside to create a match.\nThe filter range bounds can be configured with a granularity of 2.148mV.\n\nRead value: [`regs::AdcChnxFilterCtlReadVal`]; Write value: [`regs::AdcChnxFilterCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_chn0_filter_ctl(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::AdcChn0FilterCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC channel1 filter range\n\nUp to 8 filters can be configured per channel and each filter has an associated [min, max] range.\nThe condition bit then defines whether the sample values of that channel need to lie within the range or outside to create a match.\nThe filter range bounds can be configured with a granularity of 2.148mV.\n\nRead value: [`regs::AdcChnxFilterCtlReadVal`]; Write value: [`regs::AdcChnxFilterCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_chn1_filter_ctl(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::AdcChn1FilterCtl, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "ADC value sampled on channel\n\nRead value: [`regs::AdcChnValReadVal`]; Write value: [`regs::AdcChnValWriteVal`]"]
    #[inline(always)]
    pub fn adc_chn_val(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::AdcChnVal, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable filter matches as wakeups\n\nRead value: [`regs::AdcWakeupCtlReadVal`]; Write value: [`regs::AdcWakeupCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_wakeup_ctl(&self) -> ureg::RegRef<crate::meta::AdcWakeupCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Adc filter match status\n\nIndicates whether a particular filter has matched on all channels.\n\nRead value: [`regs::FilterStatusReadVal`]; Write value: [`regs::FilterStatusWriteVal`]"]
    #[inline(always)]
    pub fn filter_status(&self) -> ureg::RegRef<crate::meta::FilterStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt enable controls.\n\nadc_ctrl sends out only 1 interrupt, so this register controls\nwhich internal sources are actually registered.\n\nThis register uses the same bit enumeration as !!ADC_INTR_STATUS\n\nRead value: [`regs::AdcIntrCtlReadVal`]; Write value: [`regs::AdcIntrCtlWriteVal`]"]
    #[inline(always)]
    pub fn adc_intr_ctl(&self) -> ureg::RegRef<crate::meta::AdcIntrCtl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Debug cable internal status\n\nRead value: [`regs::AdcIntrStatusReadVal`]; Write value: [`regs::AdcIntrStatusWriteVal`]"]
    #[inline(always)]
    pub fn adc_intr_status(&self) -> ureg::RegRef<crate::meta::AdcIntrStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "State of the internal state machine\n\nRead value: [`regs::AdcFsmStateReadVal`]; Write value: [`regs::AdcFsmStateWriteVal`]"]
    #[inline(always)]
    pub fn adc_fsm_state(&self) -> ureg::RegRef<crate::meta::AdcFsmState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
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
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.match_pending is set."]
        #[inline(always)]
        pub fn match_pending(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.match_pending is set."]
        #[inline(always)]
        pub fn match_pending(self, val: bool) -> Self {
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
        #[doc = "ADC match or measurement event has occurred"]
        #[inline(always)]
        pub fn match_pending(&self) -> bool {
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
        #[doc = "Write 1 to force !!INTR_STATE.match_pending to 1."]
        #[inline(always)]
        pub fn match_pending(self, val: bool) -> Self {
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
    pub struct AdcChnValReadVal(u32);
    impl AdcChnValReadVal {
        #[doc = "2-bit extension; RO 0"]
        #[inline(always)]
        pub fn adc_chn_value_ext(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = "Latest ADC value sampled on channel. each step is 2.148mV"]
        #[inline(always)]
        pub fn adc_chn_value(&self) -> u32 {
            (self.0 >> 2) & 0x3ff
        }
        #[doc = "2-bit extension; RO 0"]
        #[inline(always)]
        pub fn adc_chn_value_intr_ext(&self) -> u32 {
            (self.0 >> 16) & 3
        }
        #[doc = "ADC value sampled on channel when the interrupt is raised(debug cable is attached or disconnected), each step is 2.148mV"]
        #[inline(always)]
        pub fn adc_chn_value_intr(&self) -> u32 {
            (self.0 >> 18) & 0x3ff
        }
    }
    impl From<u32> for AdcChnValReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcChnValReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcChnValReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcChnxFilterCtlReadVal(u32);
    impl AdcChnxFilterCtlReadVal {
        #[doc = "10-bit for chn0 filter min value "]
        #[inline(always)]
        pub fn min_v(&self) -> u32 {
            (self.0 >> 2) & 0x3ff
        }
        #[doc = "1-bit for the condition; 1'b0 means min<=ADC<=max, 1'b1 means ADC>max or ADC<min "]
        #[inline(always)]
        pub fn cond(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "10-bit for chn0 filter max value "]
        #[inline(always)]
        pub fn max_v(&self) -> u32 {
            (self.0 >> 18) & 0x3ff
        }
        #[doc = "Enable for filter"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcChnxFilterCtlWriteVal {
            AdcChnxFilterCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcChnxFilterCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcChnxFilterCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcChnxFilterCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcChnxFilterCtlWriteVal(u32);
    impl AdcChnxFilterCtlWriteVal {
        #[doc = "10-bit for chn0 filter min value "]
        #[inline(always)]
        pub fn min_v(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 2)) | ((val & 0x3ff) << 2))
        }
        #[doc = "1-bit for the condition; 1'b0 means min<=ADC<=max, 1'b1 means ADC>max or ADC<min "]
        #[inline(always)]
        pub fn cond(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "10-bit for chn0 filter max value "]
        #[inline(always)]
        pub fn max_v(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 18)) | ((val & 0x3ff) << 18))
        }
        #[doc = "Enable for filter"]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for AdcChnxFilterCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcChnxFilterCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcChnxFilterCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcEnCtlReadVal(u32);
    impl AdcEnCtlReadVal {
        #[doc = "1'b0: to power down ADC and ADC_CTRL FSM will enter the reset state; 1'b1: to power up ADC and ADC_CTRL FSM will start"]
        #[inline(always)]
        pub fn adc_enable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Oneshot mode does not care about the filter value. 1'b0: disable; 1'b1: enable"]
        #[inline(always)]
        pub fn oneshot_mode(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcEnCtlWriteVal {
            AdcEnCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcEnCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcEnCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcEnCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcEnCtlWriteVal(u32);
    impl AdcEnCtlWriteVal {
        #[doc = "1'b0: to power down ADC and ADC_CTRL FSM will enter the reset state; 1'b1: to power up ADC and ADC_CTRL FSM will start"]
        #[inline(always)]
        pub fn adc_enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Oneshot mode does not care about the filter value. 1'b0: disable; 1'b1: enable"]
        #[inline(always)]
        pub fn oneshot_mode(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for AdcEnCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcEnCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcEnCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcFsmRstReadVal(u32);
    impl AdcFsmRstReadVal {
        #[doc = "1'b0: Normal functional mode. 1'b1: SW to reset all the FSMs and timers"]
        #[inline(always)]
        pub fn rst_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcFsmRstWriteVal {
            AdcFsmRstWriteVal(self.0)
        }
    }
    impl From<u32> for AdcFsmRstReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcFsmRstReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcFsmRstReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcFsmRstWriteVal(u32);
    impl AdcFsmRstWriteVal {
        #[doc = "1'b0: Normal functional mode. 1'b1: SW to reset all the FSMs and timers"]
        #[inline(always)]
        pub fn rst_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for AdcFsmRstWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcFsmRstWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcFsmRstWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcFsmStateReadVal(u32);
    impl AdcFsmStateReadVal {
        #[doc = "Current FSM state (for debug purposes)"]
        #[inline(always)]
        pub fn state(&self) -> super::enums::State {
            super::enums::State::try_from((self.0 >> 0) & 0x1f).unwrap()
        }
    }
    impl From<u32> for AdcFsmStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcFsmStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcFsmStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcIntrCtlReadVal(u32);
    impl AdcIntrCtlReadVal {
        #[doc = "Filter interrupt source.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn match_en(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Interrupt due to FSM transition from low power sampling\nmode to normal sampling mode. This is mainly intended for debug.\n\nNote that this interrupt is primarily intended for debug purposes.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn trans_en(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt due to oneshot sampling.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn oneshot_en(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcIntrCtlWriteVal {
            AdcIntrCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcIntrCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcIntrCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcIntrCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcIntrCtlWriteVal(u32);
    impl AdcIntrCtlWriteVal {
        #[doc = "Filter interrupt source.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn match_en(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "Interrupt due to FSM transition from low power sampling\nmode to normal sampling mode. This is mainly intended for debug.\n\nNote that this interrupt is primarily intended for debug purposes.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn trans_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt due to oneshot sampling.\n\n0: interrupt source is not enabled; 1: interrupt source is enabled"]
        #[inline(always)]
        pub fn oneshot_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
    }
    impl From<u32> for AdcIntrCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcIntrCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcIntrCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcIntrStatusReadVal(u32);
    impl AdcIntrStatusReadVal {
        #[doc = "0: filter condition is not met; 1: filter condition is met"]
        #[inline(always)]
        pub fn match_(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "0: transition did not occur; 1: transition occurred"]
        #[inline(always)]
        pub fn trans(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "0: oneshot sample is not done ; 1: oneshot sample is done"]
        #[inline(always)]
        pub fn oneshot(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcIntrStatusWriteVal {
            AdcIntrStatusWriteVal(self.0)
        }
    }
    impl From<u32> for AdcIntrStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcIntrStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcIntrStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcIntrStatusWriteVal(u32);
    impl AdcIntrStatusWriteVal {
        #[doc = "0: transition did not occur; 1: transition occurred"]
        #[inline(always)]
        pub fn trans_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "0: oneshot sample is not done ; 1: oneshot sample is done"]
        #[inline(always)]
        pub fn oneshot_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
    }
    impl From<u32> for AdcIntrStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcIntrStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcIntrStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcLpSampleCtlReadVal(u32);
    impl AdcLpSampleCtlReadVal {
        #[doc = "The number of samples in low-power mode when the low-power mode is enabled.\nAfter the programmed number is met, ADC won't be powered down any more.\nThis value must be 1 or larger."]
        #[inline(always)]
        pub fn lp_sample_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcLpSampleCtlWriteVal {
            AdcLpSampleCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcLpSampleCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcLpSampleCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcLpSampleCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcLpSampleCtlWriteVal(u32);
    impl AdcLpSampleCtlWriteVal {
        #[doc = "The number of samples in low-power mode when the low-power mode is enabled.\nAfter the programmed number is met, ADC won't be powered down any more.\nThis value must be 1 or larger."]
        #[inline(always)]
        pub fn lp_sample_cnt(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for AdcLpSampleCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcLpSampleCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcLpSampleCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcPdCtlReadVal(u32);
    impl AdcPdCtlReadVal {
        #[doc = "1'b0: adc_pd is disabled, use adc_sample_ctl. 1'b1: adc_pd is enabled, use both adc_lp_sample_ctl & adc_sample_ctl"]
        #[inline(always)]
        pub fn lp_mode(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "ADC power up time, measured in always on clock cycles.\nAfter power up time is reached, the ADC controller needs one additional cycle before an ADC channel is selected for access."]
        #[inline(always)]
        pub fn pwrup_time(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "How often FSM wakes up from ADC PD mode to take a sample, measured in always on clock cycles."]
        #[inline(always)]
        pub fn wakeup_time(&self) -> u32 {
            (self.0 >> 8) & 0xffffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcPdCtlWriteVal {
            AdcPdCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcPdCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcPdCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcPdCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcPdCtlWriteVal(u32);
    impl AdcPdCtlWriteVal {
        #[doc = "1'b0: adc_pd is disabled, use adc_sample_ctl. 1'b1: adc_pd is enabled, use both adc_lp_sample_ctl & adc_sample_ctl"]
        #[inline(always)]
        pub fn lp_mode(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "ADC power up time, measured in always on clock cycles.\nAfter power up time is reached, the ADC controller needs one additional cycle before an ADC channel is selected for access."]
        #[inline(always)]
        pub fn pwrup_time(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "How often FSM wakes up from ADC PD mode to take a sample, measured in always on clock cycles."]
        #[inline(always)]
        pub fn wakeup_time(self, val: u32) -> Self {
            Self((self.0 & !(0xffffff << 8)) | ((val & 0xffffff) << 8))
        }
    }
    impl From<u32> for AdcPdCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcPdCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcPdCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcSampleCtlReadVal(u32);
    impl AdcSampleCtlReadVal {
        #[doc = "The number of samples in normal-power mode to meet the debounce spec.\nUsed after the low-power mode condition is met or in the normal power mode.\nThis value must be 1 or larger."]
        #[inline(always)]
        pub fn np_sample_cnt(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcSampleCtlWriteVal {
            AdcSampleCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcSampleCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcSampleCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcSampleCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcSampleCtlWriteVal(u32);
    impl AdcSampleCtlWriteVal {
        #[doc = "The number of samples in normal-power mode to meet the debounce spec.\nUsed after the low-power mode condition is met or in the normal power mode.\nThis value must be 1 or larger."]
        #[inline(always)]
        pub fn np_sample_cnt(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for AdcSampleCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcSampleCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcSampleCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcWakeupCtlReadVal(u32);
    impl AdcWakeupCtlReadVal {
        #[doc = "Filter wakeup source.\n\n0: filter match will not generate wakeup;\n1: filter match will generate wakeup"]
        #[inline(always)]
        pub fn match_en(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Wakeup due to FSM transition from low power sampling\nmode to normal sampling mode.\n\nNote that this wakeup source is primarily intended for debug purposes.\nIf enabled all the time, this can lead to many wakeups due to false\npositives that are ruled out automatically by adc_ctrl after\ntransitioning from LP -> NP.\n\n0: transition match will not generate wakeup;\n1: transition match will generate wakeup"]
        #[inline(always)]
        pub fn trans_en(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AdcWakeupCtlWriteVal {
            AdcWakeupCtlWriteVal(self.0)
        }
    }
    impl From<u32> for AdcWakeupCtlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcWakeupCtlReadVal> for u32 {
        #[inline(always)]
        fn from(val: AdcWakeupCtlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AdcWakeupCtlWriteVal(u32);
    impl AdcWakeupCtlWriteVal {
        #[doc = "Filter wakeup source.\n\n0: filter match will not generate wakeup;\n1: filter match will generate wakeup"]
        #[inline(always)]
        pub fn match_en(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "Wakeup due to FSM transition from low power sampling\nmode to normal sampling mode.\n\nNote that this wakeup source is primarily intended for debug purposes.\nIf enabled all the time, this can lead to many wakeups due to false\npositives that are ruled out automatically by adc_ctrl after\ntransitioning from LP -> NP.\n\n0: transition match will not generate wakeup;\n1: transition match will generate wakeup"]
        #[inline(always)]
        pub fn trans_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
    }
    impl From<u32> for AdcWakeupCtlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AdcWakeupCtlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AdcWakeupCtlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FilterStatusReadVal(u32);
    impl FilterStatusReadVal {
        #[doc = "0: filter condition is not met; 1: filter condition is met"]
        #[inline(always)]
        pub fn match_(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "0: transition did not occur; 1: transition occurred"]
        #[inline(always)]
        pub fn trans(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FilterStatusWriteVal {
            FilterStatusWriteVal(self.0)
        }
    }
    impl From<u32> for FilterStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FilterStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: FilterStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FilterStatusWriteVal(u32);
    impl FilterStatusWriteVal {
        #[doc = "0: transition did not occur; 1: transition occurred"]
        #[inline(always)]
        pub fn trans_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
    }
    impl From<u32> for FilterStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FilterStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FilterStatusWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum State {
        Pwrdn = 0,
        Pwrup = 1,
        Onest0 = 2,
        Onest021 = 3,
        Onest1 = 4,
        OnestDone = 5,
        Lp0 = 6,
        Lp021 = 7,
        Lp1 = 8,
        LpEval = 9,
        LpSlp = 10,
        LpPwrup = 11,
        Np0 = 12,
        Np021 = 13,
        Np1 = 14,
        NpEval = 15,
        NpDone = 16,
        Reserved17 = 17,
        Reserved18 = 18,
        Reserved19 = 19,
        Reserved20 = 20,
        Reserved21 = 21,
        Reserved22 = 22,
        Reserved23 = 23,
        Reserved24 = 24,
        Reserved25 = 25,
        Reserved26 = 26,
        Reserved27 = 27,
        Reserved28 = 28,
        Reserved29 = 29,
        Reserved30 = 30,
        Reserved31 = 31,
    }
    impl State {
        #[inline(always)]
        pub fn pwrdn(&self) -> bool {
            *self == Self::Pwrdn
        }
        #[inline(always)]
        pub fn pwrup(&self) -> bool {
            *self == Self::Pwrup
        }
        #[inline(always)]
        pub fn onest_0(&self) -> bool {
            *self == Self::Onest0
        }
        #[inline(always)]
        pub fn onest_021(&self) -> bool {
            *self == Self::Onest021
        }
        #[inline(always)]
        pub fn onest_1(&self) -> bool {
            *self == Self::Onest1
        }
        #[inline(always)]
        pub fn onest_done(&self) -> bool {
            *self == Self::OnestDone
        }
        #[inline(always)]
        pub fn lp_0(&self) -> bool {
            *self == Self::Lp0
        }
        #[inline(always)]
        pub fn lp_021(&self) -> bool {
            *self == Self::Lp021
        }
        #[inline(always)]
        pub fn lp_1(&self) -> bool {
            *self == Self::Lp1
        }
        #[inline(always)]
        pub fn lp_eval(&self) -> bool {
            *self == Self::LpEval
        }
        #[inline(always)]
        pub fn lp_slp(&self) -> bool {
            *self == Self::LpSlp
        }
        #[inline(always)]
        pub fn lp_pwrup(&self) -> bool {
            *self == Self::LpPwrup
        }
        #[inline(always)]
        pub fn np_0(&self) -> bool {
            *self == Self::Np0
        }
        #[inline(always)]
        pub fn np_021(&self) -> bool {
            *self == Self::Np021
        }
        #[inline(always)]
        pub fn np_1(&self) -> bool {
            *self == Self::Np1
        }
        #[inline(always)]
        pub fn np_eval(&self) -> bool {
            *self == Self::NpEval
        }
        #[inline(always)]
        pub fn np_done(&self) -> bool {
            *self == Self::NpDone
        }
    }
    impl TryFrom<u32> for State {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<State, ()> {
            if val < 0x20 {
                Ok(unsafe { core::mem::transmute::<u32, State>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<State> for u32 {
        fn from(val: State) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct StateSelector();
        impl StateSelector {
            #[inline(always)]
            pub fn pwrdn(&self) -> super::State {
                super::State::Pwrdn
            }
            #[inline(always)]
            pub fn pwrup(&self) -> super::State {
                super::State::Pwrup
            }
            #[inline(always)]
            pub fn onest_0(&self) -> super::State {
                super::State::Onest0
            }
            #[inline(always)]
            pub fn onest_021(&self) -> super::State {
                super::State::Onest021
            }
            #[inline(always)]
            pub fn onest_1(&self) -> super::State {
                super::State::Onest1
            }
            #[inline(always)]
            pub fn onest_done(&self) -> super::State {
                super::State::OnestDone
            }
            #[inline(always)]
            pub fn lp_0(&self) -> super::State {
                super::State::Lp0
            }
            #[inline(always)]
            pub fn lp_021(&self) -> super::State {
                super::State::Lp021
            }
            #[inline(always)]
            pub fn lp_1(&self) -> super::State {
                super::State::Lp1
            }
            #[inline(always)]
            pub fn lp_eval(&self) -> super::State {
                super::State::LpEval
            }
            #[inline(always)]
            pub fn lp_slp(&self) -> super::State {
                super::State::LpSlp
            }
            #[inline(always)]
            pub fn lp_pwrup(&self) -> super::State {
                super::State::LpPwrup
            }
            #[inline(always)]
            pub fn np_0(&self) -> super::State {
                super::State::Np0
            }
            #[inline(always)]
            pub fn np_021(&self) -> super::State {
                super::State::Np021
            }
            #[inline(always)]
            pub fn np_1(&self) -> super::State {
                super::State::Np1
            }
            #[inline(always)]
            pub fn np_eval(&self) -> super::State {
                super::State::NpEval
            }
            #[inline(always)]
            pub fn np_done(&self) -> super::State {
                super::State::NpDone
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState = ureg::ReadOnlyReg32<crate::regs::IntrStateReadVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type AdcEnCtl =
        ureg::ReadWriteReg32<0, crate::regs::AdcEnCtlReadVal, crate::regs::AdcEnCtlWriteVal>;
    pub type AdcPdCtl =
        ureg::ReadWriteReg32<0x64070, crate::regs::AdcPdCtlReadVal, crate::regs::AdcPdCtlWriteVal>;
    pub type AdcLpSampleCtl = ureg::ReadWriteReg32<
        4,
        crate::regs::AdcLpSampleCtlReadVal,
        crate::regs::AdcLpSampleCtlWriteVal,
    >;
    pub type AdcSampleCtl = ureg::ReadWriteReg32<
        0x9b,
        crate::regs::AdcSampleCtlReadVal,
        crate::regs::AdcSampleCtlWriteVal,
    >;
    pub type AdcFsmRst =
        ureg::ReadWriteReg32<0, crate::regs::AdcFsmRstReadVal, crate::regs::AdcFsmRstWriteVal>;
    pub type AdcChn0FilterCtl = ureg::ReadWriteReg32<
        0,
        crate::regs::AdcChnxFilterCtlReadVal,
        crate::regs::AdcChnxFilterCtlWriteVal,
    >;
    pub type AdcChn1FilterCtl = ureg::ReadWriteReg32<
        0,
        crate::regs::AdcChnxFilterCtlReadVal,
        crate::regs::AdcChnxFilterCtlWriteVal,
    >;
    pub type AdcChnVal = ureg::ReadOnlyReg32<crate::regs::AdcChnValReadVal>;
    pub type AdcWakeupCtl = ureg::ReadWriteReg32<
        0,
        crate::regs::AdcWakeupCtlReadVal,
        crate::regs::AdcWakeupCtlWriteVal,
    >;
    pub type FilterStatus = ureg::ReadWriteReg32<
        0,
        crate::regs::FilterStatusReadVal,
        crate::regs::FilterStatusWriteVal,
    >;
    pub type AdcIntrCtl =
        ureg::ReadWriteReg32<0, crate::regs::AdcIntrCtlReadVal, crate::regs::AdcIntrCtlWriteVal>;
    pub type AdcIntrStatus = ureg::ReadWriteReg32<
        0,
        crate::regs::AdcIntrStatusReadVal,
        crate::regs::AdcIntrStatusWriteVal,
    >;
    pub type AdcFsmState = ureg::ReadOnlyReg32<crate::regs::AdcFsmStateReadVal>;
}
