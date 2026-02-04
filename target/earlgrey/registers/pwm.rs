#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct PwmAon {
    _priv: (),
}
impl PwmAon {
    pub const PTR: *mut u32 = 0x40450000 as *mut u32;
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
    #[doc = "Register write enable for all control registers\n\nRead value: [`regs::RegwenReadVal`]; Write value: [`regs::RegwenWriteVal`]"]
    #[inline(always)]
    pub fn regwen(&self) -> ureg::RegRef<crate::meta::Regwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration register\n\nRead value: [`regs::CfgReadVal`]; Write value: [`regs::CfgWriteVal`]"]
    #[inline(always)]
    pub fn cfg(&self) -> ureg::RegRef<crate::meta::Cfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable PWM operation for each channel\n\nRead value: [`regs::PwmEn0ReadVal`]; Write value: [`regs::PwmEn0WriteVal`]"]
    #[inline(always)]
    pub fn pwm_en0(&self) -> ureg::RegRef<crate::meta::PwmEn0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Invert the PWM output for each channel\n\nRead value: [`regs::Invert0ReadVal`]; Write value: [`regs::Invert0WriteVal`]"]
    #[inline(always)]
    pub fn invert0(&self) -> ureg::RegRef<crate::meta::Invert0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Basic PWM Channel Parameters\n\nRead value: [`regs::PwmParamReadVal`]; Write value: [`regs::PwmParamWriteVal`]"]
    #[inline(always)]
    pub fn pwm_param(&self) -> ureg::Array<6, ureg::RegRef<crate::meta::PwmParam, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls the duty_cycle of each channel.\n\nRead value: [`regs::DutyCycleReadVal`]; Write value: [`regs::DutyCycleWriteVal`]"]
    #[inline(always)]
    pub fn duty_cycle(&self) -> ureg::Array<6, ureg::RegRef<crate::meta::DutyCycle, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Hardware controlled blink/heartbeat parameters.\n\nRead value: [`regs::BlinkParamReadVal`]; Write value: [`regs::BlinkParamWriteVal`]"]
    #[inline(always)]
    pub fn blink_param(&self) -> ureg::Array<6, ureg::RegRef<crate::meta::BlinkParam, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
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
    pub struct BlinkParamReadVal(u32);
    impl BlinkParamReadVal {
        #[doc = "This blink-rate timing parameter has two different\n   interpretations depending on whether or not the heartbeat\n   feature is enabled. If heartbeat is disabled, a blinking\n   PWM will pulse at duty cycle A for (X+1) pulses before\n   switching to duty cycle B. If heartbeat is enabled\n   the duty-cycle will start at the duty cycle A, but\n   will be incremented (or decremented) every (X+1) cycles.\n   In heartbeat mode is enabled, the size of each step is\n   controlled by BLINK_PARAM.Y."]
        #[inline(always)]
        pub fn x(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "This blink-rate timing parameter has two different\n   interpretations depending on whether or not the heartbeat\n   feature is enabled. If heartbeat is disabled, a blinking\n   PWM will pulse at duty cycle B for (Y+1) pulse cycles\n   before returning to duty cycle A. If heartbeat is enabled\n   the duty cycle will increase (or decrease) by (Y+1) units\n   every time it is incremented (or decremented)"]
        #[inline(always)]
        pub fn y(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BlinkParamWriteVal {
            BlinkParamWriteVal(self.0)
        }
    }
    impl From<u32> for BlinkParamReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BlinkParamReadVal> for u32 {
        #[inline(always)]
        fn from(val: BlinkParamReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BlinkParamWriteVal(u32);
    impl BlinkParamWriteVal {
        #[doc = "This blink-rate timing parameter has two different\n   interpretations depending on whether or not the heartbeat\n   feature is enabled. If heartbeat is disabled, a blinking\n   PWM will pulse at duty cycle A for (X+1) pulses before\n   switching to duty cycle B. If heartbeat is enabled\n   the duty-cycle will start at the duty cycle A, but\n   will be incremented (or decremented) every (X+1) cycles.\n   In heartbeat mode is enabled, the size of each step is\n   controlled by BLINK_PARAM.Y."]
        #[inline(always)]
        pub fn x(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "This blink-rate timing parameter has two different\n   interpretations depending on whether or not the heartbeat\n   feature is enabled. If heartbeat is disabled, a blinking\n   PWM will pulse at duty cycle B for (Y+1) pulse cycles\n   before returning to duty cycle A. If heartbeat is enabled\n   the duty cycle will increase (or decrease) by (Y+1) units\n   every time it is incremented (or decremented)"]
        #[inline(always)]
        pub fn y(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for BlinkParamWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BlinkParamWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BlinkParamWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgReadVal(u32);
    impl CfgReadVal {
        #[doc = "Sets the period of each PWM beat to be (CLK_DIV+1)\n   input clock periods.  Since PWM pulses are generated once\n   every 2^(DC_RESN+1) beats, the period between output\n   pulses is 2^(DC_RESN+1)*(CLK_DIV+1) times longer than the\n   input clock period."]
        #[inline(always)]
        pub fn clk_div(&self) -> u32 {
            (self.0 >> 0) & 0x7ffffff
        }
        #[doc = "Phase Resolution (logarithmic). All duty-cycle and phase\n   shift registers represent fractional PWM cycles, expressed in\n   units of 2^16 PWM cycles. Each PWM cycle  is divided\n   into 2^(DC_RESN+1) time slices, and thus only the (DC_RESN+1)\n   most significant bits of each phase or duty cycle register\n   are relevant."]
        #[inline(always)]
        pub fn dc_resn(&self) -> u32 {
            (self.0 >> 27) & 0xf
        }
        #[doc = "Assert this bit to enable the PWM phase counter.\n   Clearing this bit disables and resets the phase counter."]
        #[inline(always)]
        pub fn cntr_en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgWriteVal {
            CfgWriteVal(self.0)
        }
    }
    impl From<u32> for CfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgWriteVal(u32);
    impl CfgWriteVal {
        #[doc = "Sets the period of each PWM beat to be (CLK_DIV+1)\n   input clock periods.  Since PWM pulses are generated once\n   every 2^(DC_RESN+1) beats, the period between output\n   pulses is 2^(DC_RESN+1)*(CLK_DIV+1) times longer than the\n   input clock period."]
        #[inline(always)]
        pub fn clk_div(self, val: u32) -> Self {
            Self((self.0 & !(0x7ffffff << 0)) | ((val & 0x7ffffff) << 0))
        }
        #[doc = "Phase Resolution (logarithmic). All duty-cycle and phase\n   shift registers represent fractional PWM cycles, expressed in\n   units of 2^16 PWM cycles. Each PWM cycle  is divided\n   into 2^(DC_RESN+1) time slices, and thus only the (DC_RESN+1)\n   most significant bits of each phase or duty cycle register\n   are relevant."]
        #[inline(always)]
        pub fn dc_resn(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 27)) | ((val & 0xf) << 27))
        }
        #[doc = "Assert this bit to enable the PWM phase counter.\n   Clearing this bit disables and resets the phase counter."]
        #[inline(always)]
        pub fn cntr_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DutyCycleReadVal(u32);
    impl DutyCycleReadVal {
        #[doc = "The initial duty cycle for PWM output, in units\n   of 2^(-16)ths of a pulse cycle. The actual precision is\n   however limited to the (DC_RESN+1) most significant bits.\n   This setting applies continuously when not blinking\n   and determines the initial duty cycle when blinking."]
        #[inline(always)]
        pub fn a(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "The target duty cycle for PWM output, in units\n   of 2^(-16)ths of a pulse cycle. The actual precision is\n   however limited to the (DC_RESN+1) most significant bits.\n   This setting only applies when blinking, and determines\n   the target duty cycle."]
        #[inline(always)]
        pub fn b(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DutyCycleWriteVal {
            DutyCycleWriteVal(self.0)
        }
    }
    impl From<u32> for DutyCycleReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DutyCycleReadVal> for u32 {
        #[inline(always)]
        fn from(val: DutyCycleReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DutyCycleWriteVal(u32);
    impl DutyCycleWriteVal {
        #[doc = "The initial duty cycle for PWM output, in units\n   of 2^(-16)ths of a pulse cycle. The actual precision is\n   however limited to the (DC_RESN+1) most significant bits.\n   This setting applies continuously when not blinking\n   and determines the initial duty cycle when blinking."]
        #[inline(always)]
        pub fn a(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "The target duty cycle for PWM output, in units\n   of 2^(-16)ths of a pulse cycle. The actual precision is\n   however limited to the (DC_RESN+1) most significant bits.\n   This setting only applies when blinking, and determines\n   the target duty cycle."]
        #[inline(always)]
        pub fn b(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for DutyCycleWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DutyCycleWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DutyCycleWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Invert0ReadVal(u32);
    impl Invert0ReadVal {
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Invert0WriteVal {
            Invert0WriteVal(self.0)
        }
    }
    impl From<u32> for Invert0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Invert0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Invert0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Invert0WriteVal(u32);
    impl Invert0WriteVal {
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to this bit to invert the output for each channel,\n   so that the corresponding output is active-low."]
        #[inline(always)]
        pub fn invert5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
    }
    impl From<u32> for Invert0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Invert0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Invert0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PwmEn0ReadVal(u32);
    impl PwmEn0ReadVal {
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PwmEn0WriteVal {
            PwmEn0WriteVal(self.0)
        }
    }
    impl From<u32> for PwmEn0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PwmEn0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: PwmEn0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PwmEn0WriteVal(u32);
    impl PwmEn0WriteVal {
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to this bit to enable PWM pulses on the\n   corresponding channel."]
        #[inline(always)]
        pub fn en5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
    }
    impl From<u32> for PwmEn0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PwmEn0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: PwmEn0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PwmParamReadVal(u32);
    impl PwmParamReadVal {
        #[doc = "Phase delay of the PWM rising edge, in units of 2^(-16) PWM\n   cycles"]
        #[inline(always)]
        pub fn phase_delay(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Modulates blink behavior to create a heartbeat effect. When\n   HTBT_EN is set, the duty cycle increases (or decreases)\n   linearly from DUTY_CYCLE.A to DUTY_CYCLE.B and back, in\n   steps of (BLINK_PARAM.Y+1), with an increment (decrement)\n   once every (BLINK_PARAM.X+1) PWM cycles. When HTBT_EN is\n   cleared, the standard blink behavior applies, meaning that\n   the output duty cycle alternates between DUTY_CYCLE.A for\n   (BLINK_PARAM.X+1) pulses and DUTY_CYCLE.B for\n   (BLINK_PARAM.Y+1) pulses."]
        #[inline(always)]
        pub fn htbt_en(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Enables blink (or heartbeat).  If cleared, the output duty\n   cycle will remain constant at DUTY_CYCLE.A. Enabling this\n   bit  causes the PWM duty cycle to fluctuate between\n   DUTY_CYCLE.A and DUTY_CYCLE.B"]
        #[inline(always)]
        pub fn blink_en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PwmParamWriteVal {
            PwmParamWriteVal(self.0)
        }
    }
    impl From<u32> for PwmParamReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PwmParamReadVal> for u32 {
        #[inline(always)]
        fn from(val: PwmParamReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PwmParamWriteVal(u32);
    impl PwmParamWriteVal {
        #[doc = "Phase delay of the PWM rising edge, in units of 2^(-16) PWM\n   cycles"]
        #[inline(always)]
        pub fn phase_delay(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Modulates blink behavior to create a heartbeat effect. When\n   HTBT_EN is set, the duty cycle increases (or decreases)\n   linearly from DUTY_CYCLE.A to DUTY_CYCLE.B and back, in\n   steps of (BLINK_PARAM.Y+1), with an increment (decrement)\n   once every (BLINK_PARAM.X+1) PWM cycles. When HTBT_EN is\n   cleared, the standard blink behavior applies, meaning that\n   the output duty cycle alternates between DUTY_CYCLE.A for\n   (BLINK_PARAM.X+1) pulses and DUTY_CYCLE.B for\n   (BLINK_PARAM.Y+1) pulses."]
        #[inline(always)]
        pub fn htbt_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Enables blink (or heartbeat).  If cleared, the output duty\n   cycle will remain constant at DUTY_CYCLE.A. Enabling this\n   bit  causes the PWM duty cycle to fluctuate between\n   DUTY_CYCLE.A and DUTY_CYCLE.B"]
        #[inline(always)]
        pub fn blink_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for PwmParamWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PwmParamWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PwmParamWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegwenReadVal(u32);
    impl RegwenReadVal {
        #[doc = "When true, all writable registers can be modified.\n    When false, they become read-only. Defaults true, write\n    zero to clear. This can be cleared after initial\n    configuration at boot in order to lock in the listed\n    register settings."]
        #[inline(always)]
        pub fn regwen(&self) -> bool {
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
        #[doc = "When true, all writable registers can be modified.\n    When false, they become read-only. Defaults true, write\n    zero to clear. This can be cleared after initial\n    configuration at boot in order to lock in the listed\n    register settings."]
        #[inline(always)]
        pub fn regwen_clear(self) -> Self {
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
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    pub mod selector {}
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Regwen =
        ureg::ReadWriteReg32<1, crate::regs::RegwenReadVal, crate::regs::RegwenWriteVal>;
    pub type Cfg =
        ureg::ReadWriteReg32<0x38008000, crate::regs::CfgReadVal, crate::regs::CfgWriteVal>;
    pub type PwmEn0 =
        ureg::ReadWriteReg32<0, crate::regs::PwmEn0ReadVal, crate::regs::PwmEn0WriteVal>;
    pub type Invert0 =
        ureg::ReadWriteReg32<0, crate::regs::Invert0ReadVal, crate::regs::Invert0WriteVal>;
    pub type PwmParam =
        ureg::ReadWriteReg32<0, crate::regs::PwmParamReadVal, crate::regs::PwmParamWriteVal>;
    pub type DutyCycle = ureg::ReadWriteReg32<
        0x7fff7fff,
        crate::regs::DutyCycleReadVal,
        crate::regs::DutyCycleWriteVal,
    >;
    pub type BlinkParam =
        ureg::ReadWriteReg32<0, crate::regs::BlinkParamReadVal, crate::regs::BlinkParamWriteVal>;
}
