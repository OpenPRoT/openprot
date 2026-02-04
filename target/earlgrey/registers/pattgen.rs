#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Pattgen {
    _priv: (),
}
impl Pattgen {
    pub const PTR: *mut u32 = 0x400e0000 as *mut u32;
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
    #[doc = "PATTGEN control register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "PATTGEN pre-divider register for Channel 0\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn prediv_ch0(&self) -> ureg::RegRef<crate::meta::PredivCh0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "PATTGEN pre-divider register for Channel 1\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn prediv_ch1(&self) -> ureg::RegRef<crate::meta::PredivCh1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "PATTGEN seed pattern multi-registers for Channel 0.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn data_ch0(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::DataCh0, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "PATTGEN seed pattern multi-registers for Channel 1.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn data_ch1(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::DataCh1, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "PATTGEN pattern length\n\nRead value: [`regs::SizeReadVal`]; Write value: [`regs::SizeWriteVal`]"]
    #[inline(always)]
    pub fn size(&self) -> ureg::RegRef<crate::meta::Size, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
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
    pub struct CtrlReadVal(u32);
    impl CtrlReadVal {
        #[doc = "Enable pattern generator functionality for Channel 0"]
        #[inline(always)]
        pub fn enable_ch0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable pattern generator functionality for Channel 1"]
        #[inline(always)]
        pub fn enable_ch1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Clock (`pcl`) polarity for Channel 0.  If low, `pda` signal changes on falling edge of pcl signal, otherwise pda changes on rising edge. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn polarity_ch0(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Clock (`pcl`) polarity for Channel 1.  If low, `pda` signal changes on falling edge of `pcl` signal, otherwise pda changes on rising edge. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn polarity_ch1(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 0, `pcl` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pcl` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pcl_ch0(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 0, `pda` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pda` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pda_ch0(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 0, `pcl` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pcl` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pcl_ch1(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 0, `pda` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pda` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pda_ch1(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlWriteVal {
            CtrlWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlWriteVal(u32);
    impl CtrlWriteVal {
        #[doc = "Enable pattern generator functionality for Channel 0"]
        #[inline(always)]
        pub fn enable_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable pattern generator functionality for Channel 1"]
        #[inline(always)]
        pub fn enable_ch1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Clock (`pcl`) polarity for Channel 0.  If low, `pda` signal changes on falling edge of pcl signal, otherwise pda changes on rising edge. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn polarity_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Clock (`pcl`) polarity for Channel 1.  If low, `pda` signal changes on falling edge of `pcl` signal, otherwise pda changes on rising edge. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn polarity_ch1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 0, `pcl` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pcl` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pcl_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 0, `pda` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pda` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pda_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 0, `pcl` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pcl` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pcl_ch1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 0, `pda` is low when pattgen is not actively sending data bits (i.e., when pattgen is disabled or all data bits have been sent).\nIf 1, `pda` is high when pattgen is not actively sending data bits."]
        #[inline(always)]
        pub fn inactive_level_pda_ch1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for CtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.done_ch0 is set."]
        #[inline(always)]
        pub fn done_ch0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.done_ch1 is set."]
        #[inline(always)]
        pub fn done_ch1(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.done_ch0 is set."]
        #[inline(always)]
        pub fn done_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.done_ch1 is set."]
        #[inline(always)]
        pub fn done_ch1(self, val: bool) -> Self {
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
        #[doc = "raise if pattern generation on Channel 0 is complete"]
        #[inline(always)]
        pub fn done_ch0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "raise if pattern generation on Channel 1 is complete"]
        #[inline(always)]
        pub fn done_ch1(&self) -> bool {
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
        #[doc = "raise if pattern generation on Channel 0 is complete"]
        #[inline(always)]
        pub fn done_ch0_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "raise if pattern generation on Channel 1 is complete"]
        #[inline(always)]
        pub fn done_ch1_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.done_ch0 to 1."]
        #[inline(always)]
        pub fn done_ch0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.done_ch1 to 1."]
        #[inline(always)]
        pub fn done_ch1(self, val: bool) -> Self {
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
    pub struct SizeReadVal(u32);
    impl SizeReadVal {
        #[doc = "Length of the seed pattern for Channel 0, minus 1. Valid values: 0..63. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn len_ch0(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[doc = "Number of pattern repetitions for Channel 0, minus 1. Valid values: 0..1023. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn reps_ch0(&self) -> u32 {
            (self.0 >> 6) & 0x3ff
        }
        #[doc = "Length of the seed pattern for Channel 1, minus 1. Valid values: 0..63. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn len_ch1(&self) -> u32 {
            (self.0 >> 16) & 0x3f
        }
        #[doc = "Number of pattern repetitions for Channel 1, minus 1. Valid values: 0..1023. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn reps_ch1(&self) -> u32 {
            (self.0 >> 22) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SizeWriteVal {
            SizeWriteVal(self.0)
        }
    }
    impl From<u32> for SizeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SizeReadVal> for u32 {
        #[inline(always)]
        fn from(val: SizeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SizeWriteVal(u32);
    impl SizeWriteVal {
        #[doc = "Length of the seed pattern for Channel 0, minus 1. Valid values: 0..63. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn len_ch0(self, val: u32) -> Self {
            Self((self.0 & !(0x3f << 0)) | ((val & 0x3f) << 0))
        }
        #[doc = "Number of pattern repetitions for Channel 0, minus 1. Valid values: 0..1023. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn reps_ch0(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 6)) | ((val & 0x3ff) << 6))
        }
        #[doc = "Length of the seed pattern for Channel 1, minus 1. Valid values: 0..63. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn len_ch1(self, val: u32) -> Self {
            Self((self.0 & !(0x3f << 16)) | ((val & 0x3f) << 16))
        }
        #[doc = "Number of pattern repetitions for Channel 1, minus 1. Valid values: 0..1023. Note that writes to a channel's configuration registers have no effect while the channel is enabled."]
        #[inline(always)]
        pub fn reps_ch1(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 22)) | ((val & 0x3ff) << 22))
        }
    }
    impl From<u32> for SizeWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SizeWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SizeWriteVal) -> u32 {
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
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Ctrl = ureg::ReadWriteReg32<0, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type PredivCh0 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type PredivCh1 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DataCh0 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DataCh1 = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Size = ureg::ReadWriteReg32<0, crate::regs::SizeReadVal, crate::regs::SizeWriteVal>;
}
