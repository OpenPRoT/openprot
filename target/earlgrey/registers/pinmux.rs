#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct PinmuxAon {
    _priv: (),
}
impl PinmuxAon {
    pub const PTR: *mut u32 = 0x40460000 as *mut u32;
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
    #[doc = "Register write enable for MIO peripheral input selects.\n\nRead value: [`regs::MioPeriphInselRegwenReadVal`]; Write value: [`regs::MioPeriphInselRegwenWriteVal`]"]
    #[inline(always)]
    pub fn mio_periph_insel_regwen(
        &self,
    ) -> ureg::Array<57, ureg::RegRef<crate::meta::MioPeriphInselRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "For each peripheral input, this selects the muxable pad input.\n\nRead value: [`regs::MioPeriphInselReadVal`]; Write value: [`regs::MioPeriphInselWriteVal`]"]
    #[inline(always)]
    pub fn mio_periph_insel(
        &self,
    ) -> ureg::Array<57, ureg::RegRef<crate::meta::MioPeriphInsel, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MIO output selects.\n\nRead value: [`regs::MioOutselRegwenReadVal`]; Write value: [`regs::MioOutselRegwenWriteVal`]"]
    #[inline(always)]
    pub fn mio_outsel_regwen(
        &self,
    ) -> ureg::Array<47, ureg::RegRef<crate::meta::MioOutselRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1cc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "For each muxable pad, this selects the peripheral output.\n\nRead value: [`regs::MioOutselReadVal`]; Write value: [`regs::MioOutselWriteVal`]"]
    #[inline(always)]
    pub fn mio_outsel(&self) -> ureg::Array<47, ureg::RegRef<crate::meta::MioOutsel, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x288 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MIO PAD attributes.\n\nRead value: [`regs::MioPadAttrRegwenReadVal`]; Write value: [`regs::MioPadAttrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_attr_regwen(
        &self,
    ) -> ureg::Array<47, ureg::RegRef<crate::meta::MioPadAttrRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x344 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Muxed pad attributes.\nThis register has WARL behavior since not each pad type may support\nall attributes.\nThe muxed pad that is used for TAP strap 0 has a different reset value, with `pull_en` set to 1.\n\nRead value: [`regs::IoPadAttrReadVal`]; Write value: [`regs::IoPadAttrWriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_attr(&self) -> ureg::Array<47, ureg::RegRef<crate::meta::MioPadAttr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x400 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for DIO PAD attributes.\n\nRead value: [`regs::DioPadAttrRegwenReadVal`]; Write value: [`regs::DioPadAttrRegwenWriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_attr_regwen(
        &self,
    ) -> ureg::Array<16, ureg::RegRef<crate::meta::DioPadAttrRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Dedicated pad attributes.\nThis register has WARL behavior since not each pad type may support\nall attributes.\n\nRead value: [`regs::IoPadAttrReadVal`]; Write value: [`regs::IoPadAttrWriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_attr(&self) -> ureg::Array<16, ureg::RegRef<crate::meta::DioPadAttr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4fc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register indicating whether the corresponding pad is in sleep mode.\n\nRead value: [`regs::MioPadSleepStatus0ReadVal`]; Write value: [`regs::MioPadSleepStatus0WriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_sleep_status0(&self) -> ureg::RegRef<crate::meta::MioPadSleepStatus0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x53c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register indicating whether the corresponding pad is in sleep mode.\n\nRead value: [`regs::MioPadSleepStatus1ReadVal`]; Write value: [`regs::MioPadSleepStatus1WriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_sleep_status1(&self) -> ureg::RegRef<crate::meta::MioPadSleepStatus1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x540 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MIO sleep value configuration.\n\nRead value: [`regs::MioPadSleepRegwenReadVal`]; Write value: [`regs::MioPadSleepRegwenWriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_sleep_regwen(
        &self,
    ) -> ureg::Array<47, ureg::RegRef<crate::meta::MioPadSleepRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x544 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enables the sleep mode of the corresponding muxed pad.\n\nRead value: [`regs::MioPadSleepEnReadVal`]; Write value: [`regs::MioPadSleepEnWriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_sleep_en(
        &self,
    ) -> ureg::Array<47, ureg::RegRef<crate::meta::MioPadSleepEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x600 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Defines sleep behavior of the corresponding muxed pad.\n\nRead value: [`regs::IoPadSleepModeReadVal`]; Write value: [`regs::IoPadSleepModeWriteVal`]"]
    #[inline(always)]
    pub fn mio_pad_sleep_mode(
        &self,
    ) -> ureg::Array<47, ureg::RegRef<crate::meta::MioPadSleepMode, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x6bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register indicating whether the corresponding pad is in sleep mode.\n\nRead value: [`regs::DioPadSleepStatus0ReadVal`]; Write value: [`regs::DioPadSleepStatus0WriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_sleep_status0(&self) -> ureg::RegRef<crate::meta::DioPadSleepStatus0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x778 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for DIO sleep value configuration.\n\nRead value: [`regs::DioPadSleepRegwenReadVal`]; Write value: [`regs::DioPadSleepRegwenWriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_sleep_regwen(
        &self,
    ) -> ureg::Array<16, ureg::RegRef<crate::meta::DioPadSleepRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x77c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enables the sleep mode of the corresponding dedicated pad.\n\nRead value: [`regs::DioPadSleepEnReadVal`]; Write value: [`regs::DioPadSleepEnWriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_sleep_en(
        &self,
    ) -> ureg::Array<16, ureg::RegRef<crate::meta::DioPadSleepEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x7bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Defines sleep behavior of the corresponding dedicated pad.\n\nRead value: [`regs::IoPadSleepModeReadVal`]; Write value: [`regs::IoPadSleepModeWriteVal`]"]
    #[inline(always)]
    pub fn dio_pad_sleep_mode(
        &self,
    ) -> ureg::Array<16, ureg::RegRef<crate::meta::DioPadSleepMode, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x7fc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for wakeup detectors.\n\nRead value: [`regs::WkupDetectorRegwenReadVal`]; Write value: [`regs::WkupDetectorRegwenWriteVal`]"]
    #[inline(always)]
    pub fn wkup_detector_regwen(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::WkupDetectorRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x83c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enables for the wakeup detectors.\nNote that these registers are synced to the always-on clock.\nThe first write access always completes immediately.\nHowever, read/write accesses following a write will block until that write has completed.\n\nRead value: [`regs::WkupDetectorEnReadVal`]; Write value: [`regs::WkupDetectorEnWriteVal`]"]
    #[inline(always)]
    pub fn wkup_detector_en(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::WkupDetectorEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x85c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration of wakeup condition detectors.\nNote that these registers are synced to the always-on clock.\nThe first write access always completes immediately.\nHowever, read/write accesses following a write will block until that write has completed.\n\nNote that the wkup detector should be disabled by setting !!WKUP_DETECTOR_EN_0 before changing the detection mode.\nThe reason for that is that the pulse width counter is NOT cleared upon a mode change while the detector is enabled.\n\nRead value: [`regs::WkupDetectorReadVal`]; Write value: [`regs::WkupDetectorWriteVal`]"]
    #[inline(always)]
    pub fn wkup_detector(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::WkupDetector, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x87c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Counter thresholds for wakeup condition detectors.\nNote that these registers are synced to the always-on clock.\nThe first write access always completes immediately.\nHowever, read/write accesses following a write will block until that write has completed.\n\nRead value: [`regs::WkupDetectorCntThReadVal`]; Write value: [`regs::WkupDetectorCntThWriteVal`]"]
    #[inline(always)]
    pub fn wkup_detector_cnt_th(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::WkupDetectorCntTh, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x89c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Pad selects for pad wakeup condition detectors.\nThis register is NOT synced to the AON domain since the muxing mechanism is implemented in the same way as the pinmux muxing matrix.\n\nRead value: [`regs::WkupDetectorPadselReadVal`]; Write value: [`regs::WkupDetectorPadselWriteVal`]"]
    #[inline(always)]
    pub fn wkup_detector_padsel(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::WkupDetectorPadsel, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x8bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Cause registers for wakeup detectors.\nNote that these registers are synced to the always-on clock.\nThe first write access always completes immediately.\nHowever, read/write accesses following a write will block until that write has completed.\n\nRead value: [`regs::WkupCause0ReadVal`]; Write value: [`regs::WkupCause0WriteVal`]"]
    #[inline(always)]
    pub fn wkup_cause0(&self) -> ureg::RegRef<crate::meta::WkupCause0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8dc / core::mem::size_of::<u32>()),
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
    pub struct DioPadAttrRegwenReadVal(u32);
    impl DioPadAttrRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!DIO_PAD_ATTR\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DioPadAttrRegwenWriteVal {
            DioPadAttrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for DioPadAttrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadAttrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadAttrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadAttrRegwenWriteVal(u32);
    impl DioPadAttrRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!DIO_PAD_ATTR\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for DioPadAttrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadAttrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadAttrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepEnReadVal(u32);
    impl DioPadSleepEnReadVal {
        #[doc = "Deep sleep mode enable.\nIf this bit is set to 1 the corresponding pad will enable the sleep behavior\nspecified in !!DIO_PAD_SLEEP_MODE upon deep sleep entry, and the corresponding bit\nin !!DIO_PAD_SLEEP_STATUS will be set to 1.\nThe pad remains in deep sleep mode until the corresponding bit in\n!!DIO_PAD_SLEEP_STATUS is cleared by SW.\nNote that if an always on peripheral is connected to a specific DIO pad,\nthe corresponding !!DIO_PAD_SLEEP_EN bit should be set to 0."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DioPadSleepEnWriteVal {
            DioPadSleepEnWriteVal(self.0)
        }
    }
    impl From<u32> for DioPadSleepEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepEnWriteVal(u32);
    impl DioPadSleepEnWriteVal {
        #[doc = "Deep sleep mode enable.\nIf this bit is set to 1 the corresponding pad will enable the sleep behavior\nspecified in !!DIO_PAD_SLEEP_MODE upon deep sleep entry, and the corresponding bit\nin !!DIO_PAD_SLEEP_STATUS will be set to 1.\nThe pad remains in deep sleep mode until the corresponding bit in\n!!DIO_PAD_SLEEP_STATUS is cleared by SW.\nNote that if an always on peripheral is connected to a specific DIO pad,\nthe corresponding !!DIO_PAD_SLEEP_EN bit should be set to 0."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for DioPadSleepEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepRegwenReadVal(u32);
    impl DioPadSleepRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!DIO_PAD_SLEEP_MODE\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DioPadSleepRegwenWriteVal {
            DioPadSleepRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for DioPadSleepRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepRegwenWriteVal(u32);
    impl DioPadSleepRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!DIO_PAD_SLEEP_MODE\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for DioPadSleepRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepStatus0ReadVal(u32);
    impl DioPadSleepStatus0ReadVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en12(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en13(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en14(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en15(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DioPadSleepStatus0WriteVal {
            DioPadSleepStatus0WriteVal(self.0)
        }
    }
    impl From<u32> for DioPadSleepStatus0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepStatus0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepStatus0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DioPadSleepStatus0WriteVal(u32);
    impl DioPadSleepStatus0WriteVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en0_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en1_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en2_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en3_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en4_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en5_clear(self) -> Self {
            Self(self.0 & !(1 << 5))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en6_clear(self) -> Self {
            Self(self.0 & !(1 << 6))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en7_clear(self) -> Self {
            Self(self.0 & !(1 << 7))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en8_clear(self) -> Self {
            Self(self.0 & !(1 << 8))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en9_clear(self) -> Self {
            Self(self.0 & !(1 << 9))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en10_clear(self) -> Self {
            Self(self.0 & !(1 << 10))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en11_clear(self) -> Self {
            Self(self.0 & !(1 << 11))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en12_clear(self) -> Self {
            Self(self.0 & !(1 << 12))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en13_clear(self) -> Self {
            Self(self.0 & !(1 << 13))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en14_clear(self) -> Self {
            Self(self.0 & !(1 << 14))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!DIO_PAD_SLEEP_MODE) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en15_clear(self) -> Self {
            Self(self.0 & !(1 << 15))
        }
    }
    impl From<u32> for DioPadSleepStatus0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DioPadSleepStatus0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: DioPadSleepStatus0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoPadAttrReadVal(u32);
    impl IoPadAttrReadVal {
        #[doc = "Invert input and output levels."]
        #[inline(always)]
        pub fn invert(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable virtual open drain."]
        #[inline(always)]
        pub fn virtual_od_en(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
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
        #[doc = "Enable keeper termination. This weakly drives the previous pad output value when output is disabled, similar to a verilog `trireg`."]
        #[inline(always)]
        pub fn keeper_en(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable the schmitt trigger."]
        #[inline(always)]
        pub fn schmitt_en(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Enable open drain."]
        #[inline(always)]
        pub fn od_en(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Disable input drivers.\nSetting this to 1 for pads that are not used as input can reduce their leakage current."]
        #[inline(always)]
        pub fn input_disable(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Slew rate (0x0: slowest, 0x3: fastest)."]
        #[inline(always)]
        pub fn slew_rate(&self) -> u32 {
            (self.0 >> 16) & 3
        }
        #[doc = "Drive strength (0x0: weakest, 0xf: strongest)"]
        #[inline(always)]
        pub fn drive_strength(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoPadAttrWriteVal {
            IoPadAttrWriteVal(self.0)
        }
    }
    impl From<u32> for IoPadAttrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoPadAttrReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoPadAttrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoPadAttrWriteVal(u32);
    impl IoPadAttrWriteVal {
        #[doc = "Invert input and output levels."]
        #[inline(always)]
        pub fn invert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable virtual open drain."]
        #[inline(always)]
        pub fn virtual_od_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
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
        #[doc = "Enable keeper termination. This weakly drives the previous pad output value when output is disabled, similar to a verilog `trireg`."]
        #[inline(always)]
        pub fn keeper_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable the schmitt trigger."]
        #[inline(always)]
        pub fn schmitt_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Enable open drain."]
        #[inline(always)]
        pub fn od_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Disable input drivers.\nSetting this to 1 for pads that are not used as input can reduce their leakage current."]
        #[inline(always)]
        pub fn input_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Slew rate (0x0: slowest, 0x3: fastest)."]
        #[inline(always)]
        pub fn slew_rate(self, val: u32) -> Self {
            Self((self.0 & !(3 << 16)) | ((val & 3) << 16))
        }
        #[doc = "Drive strength (0x0: weakest, 0xf: strongest)"]
        #[inline(always)]
        pub fn drive_strength(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 20)) | ((val & 0xf) << 20))
        }
    }
    impl From<u32> for IoPadAttrWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoPadAttrWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoPadAttrWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoPadSleepModeReadVal(u32);
    impl IoPadSleepModeReadVal {
        #[doc = "Value to drive in deep sleep."]
        #[inline(always)]
        pub fn out(&self) -> super::enums::Out {
            super::enums::Out::try_from((self.0 >> 0) & 3).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IoPadSleepModeWriteVal {
            IoPadSleepModeWriteVal(self.0)
        }
    }
    impl From<u32> for IoPadSleepModeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoPadSleepModeReadVal> for u32 {
        #[inline(always)]
        fn from(val: IoPadSleepModeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IoPadSleepModeWriteVal(u32);
    impl IoPadSleepModeWriteVal {
        #[doc = "Value to drive in deep sleep."]
        #[inline(always)]
        pub fn out(
            self,
            f: impl FnOnce(super::enums::selector::OutSelector) -> super::enums::Out,
        ) -> Self {
            Self((self.0 & !(3 << 0)) | (u32::from(f(super::enums::selector::OutSelector())) << 0))
        }
    }
    impl From<u32> for IoPadSleepModeWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IoPadSleepModeWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IoPadSleepModeWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioOutselReadVal(u32);
    impl MioOutselReadVal {
        #[doc = "0: tie constantly to zero, 1: tie constantly to 1, 2: high-Z,\n>=3: peripheral outputs (i.e., add 3 to the native peripheral pad index)."]
        #[inline(always)]
        pub fn out(&self) -> u32 {
            (self.0 >> 0) & 0x7f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioOutselWriteVal {
            MioOutselWriteVal(self.0)
        }
    }
    impl From<u32> for MioOutselReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioOutselReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioOutselReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioOutselWriteVal(u32);
    impl MioOutselWriteVal {
        #[doc = "0: tie constantly to zero, 1: tie constantly to 1, 2: high-Z,\n>=3: peripheral outputs (i.e., add 3 to the native peripheral pad index)."]
        #[inline(always)]
        pub fn out(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 0)) | ((val & 0x7f) << 0))
        }
    }
    impl From<u32> for MioOutselWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioOutselWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioOutselWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioOutselRegwenReadVal(u32);
    impl MioOutselRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding MIO_OUTSEL\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioOutselRegwenWriteVal {
            MioOutselRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MioOutselRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioOutselRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioOutselRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioOutselRegwenWriteVal(u32);
    impl MioOutselRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding MIO_OUTSEL\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MioOutselRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioOutselRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioOutselRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadAttrRegwenReadVal(u32);
    impl MioPadAttrRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MIO_PAD_ATTR\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPadAttrRegwenWriteVal {
            MioPadAttrRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MioPadAttrRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadAttrRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadAttrRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadAttrRegwenWriteVal(u32);
    impl MioPadAttrRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MIO_PAD_ATTR\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MioPadAttrRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadAttrRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadAttrRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepEnReadVal(u32);
    impl MioPadSleepEnReadVal {
        #[doc = "Deep sleep mode enable.\nIf this bit is set to 1 the corresponding pad will enable the sleep behavior\nspecified in !!MIO_PAD_SLEEP_MODE upon deep sleep entry, and the corresponding bit\nin !!MIO_PAD_SLEEP_STATUS will be set to 1.\nThe pad remains in deep sleep mode until the corresponding bit in\n!!MIO_PAD_SLEEP_STATUS is cleared by SW.\nNote that if an always on peripheral is connected to a specific MIO pad,\nthe corresponding !!MIO_PAD_SLEEP_EN bit should be set to 0."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPadSleepEnWriteVal {
            MioPadSleepEnWriteVal(self.0)
        }
    }
    impl From<u32> for MioPadSleepEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepEnWriteVal(u32);
    impl MioPadSleepEnWriteVal {
        #[doc = "Deep sleep mode enable.\nIf this bit is set to 1 the corresponding pad will enable the sleep behavior\nspecified in !!MIO_PAD_SLEEP_MODE upon deep sleep entry, and the corresponding bit\nin !!MIO_PAD_SLEEP_STATUS will be set to 1.\nThe pad remains in deep sleep mode until the corresponding bit in\n!!MIO_PAD_SLEEP_STATUS is cleared by SW.\nNote that if an always on peripheral is connected to a specific MIO pad,\nthe corresponding !!MIO_PAD_SLEEP_EN bit should be set to 0."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for MioPadSleepEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepRegwenReadVal(u32);
    impl MioPadSleepRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MIO_PAD_SLEEP_MODE\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPadSleepRegwenWriteVal {
            MioPadSleepRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MioPadSleepRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepRegwenWriteVal(u32);
    impl MioPadSleepRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding !!MIO_PAD_SLEEP_MODE\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MioPadSleepRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepStatus0ReadVal(u32);
    impl MioPadSleepStatus0ReadVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en12(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en13(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en14(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en15(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en16(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en17(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en18(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en19(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en20(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en21(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en22(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en23(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en24(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en25(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en26(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en27(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en28(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en29(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en30(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en31(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPadSleepStatus0WriteVal {
            MioPadSleepStatus0WriteVal(self.0)
        }
    }
    impl From<u32> for MioPadSleepStatus0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepStatus0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepStatus0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepStatus0WriteVal(u32);
    impl MioPadSleepStatus0WriteVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en0_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en1_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en2_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en3_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en4_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en5_clear(self) -> Self {
            Self(self.0 & !(1 << 5))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en6_clear(self) -> Self {
            Self(self.0 & !(1 << 6))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en7_clear(self) -> Self {
            Self(self.0 & !(1 << 7))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en8_clear(self) -> Self {
            Self(self.0 & !(1 << 8))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en9_clear(self) -> Self {
            Self(self.0 & !(1 << 9))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en10_clear(self) -> Self {
            Self(self.0 & !(1 << 10))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en11_clear(self) -> Self {
            Self(self.0 & !(1 << 11))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en12_clear(self) -> Self {
            Self(self.0 & !(1 << 12))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en13_clear(self) -> Self {
            Self(self.0 & !(1 << 13))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en14_clear(self) -> Self {
            Self(self.0 & !(1 << 14))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en15_clear(self) -> Self {
            Self(self.0 & !(1 << 15))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en16_clear(self) -> Self {
            Self(self.0 & !(1 << 16))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en17_clear(self) -> Self {
            Self(self.0 & !(1 << 17))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en18_clear(self) -> Self {
            Self(self.0 & !(1 << 18))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en19_clear(self) -> Self {
            Self(self.0 & !(1 << 19))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en20_clear(self) -> Self {
            Self(self.0 & !(1 << 20))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en21_clear(self) -> Self {
            Self(self.0 & !(1 << 21))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en22_clear(self) -> Self {
            Self(self.0 & !(1 << 22))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en23_clear(self) -> Self {
            Self(self.0 & !(1 << 23))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en24_clear(self) -> Self {
            Self(self.0 & !(1 << 24))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en25_clear(self) -> Self {
            Self(self.0 & !(1 << 25))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en26_clear(self) -> Self {
            Self(self.0 & !(1 << 26))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en27_clear(self) -> Self {
            Self(self.0 & !(1 << 27))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en28_clear(self) -> Self {
            Self(self.0 & !(1 << 28))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en29_clear(self) -> Self {
            Self(self.0 & !(1 << 29))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en30_clear(self) -> Self {
            Self(self.0 & !(1 << 30))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en31_clear(self) -> Self {
            Self(self.0 & !(1 << 31))
        }
    }
    impl From<u32> for MioPadSleepStatus0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepStatus0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepStatus0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepStatus1ReadVal(u32);
    impl MioPadSleepStatus1ReadVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en32(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en33(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en34(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en35(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en36(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en37(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en38(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en39(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en40(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en41(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en42(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en43(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en44(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en45(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en46(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPadSleepStatus1WriteVal {
            MioPadSleepStatus1WriteVal(self.0)
        }
    }
    impl From<u32> for MioPadSleepStatus1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepStatus1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepStatus1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPadSleepStatus1WriteVal(u32);
    impl MioPadSleepStatus1WriteVal {
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en32_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en33_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en34_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en35_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en36_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en37_clear(self) -> Self {
            Self(self.0 & !(1 << 5))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en38_clear(self) -> Self {
            Self(self.0 & !(1 << 6))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en39_clear(self) -> Self {
            Self(self.0 & !(1 << 7))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en40_clear(self) -> Self {
            Self(self.0 & !(1 << 8))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en41_clear(self) -> Self {
            Self(self.0 & !(1 << 9))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en42_clear(self) -> Self {
            Self(self.0 & !(1 << 10))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en43_clear(self) -> Self {
            Self(self.0 & !(1 << 11))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en44_clear(self) -> Self {
            Self(self.0 & !(1 << 12))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en45_clear(self) -> Self {
            Self(self.0 & !(1 << 13))
        }
        #[doc = "This register is set to 1 if the deep sleep mode of the corresponding\npad has been enabled (!!MIO_PAD_SLEEP_EN) upon deep sleep entry.\nThe sleep mode of the corresponding pad will remain active until SW\nclears this bit."]
        #[inline(always)]
        pub fn en46_clear(self) -> Self {
            Self(self.0 & !(1 << 14))
        }
    }
    impl From<u32> for MioPadSleepStatus1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPadSleepStatus1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPadSleepStatus1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPeriphInselReadVal(u32);
    impl MioPeriphInselReadVal {
        #[doc = "0: tie constantly to zero, 1: tie constantly to 1,\n>=2: MIO pads (i.e., add 2 to the native MIO pad index)."]
        #[inline(always)]
        pub fn in_(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPeriphInselWriteVal {
            MioPeriphInselWriteVal(self.0)
        }
    }
    impl From<u32> for MioPeriphInselReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPeriphInselReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPeriphInselReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPeriphInselWriteVal(u32);
    impl MioPeriphInselWriteVal {
        #[doc = "0: tie constantly to zero, 1: tie constantly to 1,\n>=2: MIO pads (i.e., add 2 to the native MIO pad index)."]
        #[inline(always)]
        pub fn in_(self, val: u32) -> Self {
            Self((self.0 & !(0x3f << 0)) | ((val & 0x3f) << 0))
        }
    }
    impl From<u32> for MioPeriphInselWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPeriphInselWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPeriphInselWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPeriphInselRegwenReadVal(u32);
    impl MioPeriphInselRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding MIO_PERIPH_INSEL\nis not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MioPeriphInselRegwenWriteVal {
            MioPeriphInselRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MioPeriphInselRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPeriphInselRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MioPeriphInselRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MioPeriphInselRegwenWriteVal(u32);
    impl MioPeriphInselRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding MIO_PERIPH_INSEL\nis not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MioPeriphInselRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MioPeriphInselRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MioPeriphInselRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCause0ReadVal(u32);
    impl WkupCause0ReadVal {
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupCause0WriteVal {
            WkupCause0WriteVal(self.0)
        }
    }
    impl From<u32> for WkupCause0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCause0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCause0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupCause0WriteVal(u32);
    impl WkupCause0WriteVal {
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause0_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause1_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause2_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause3_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause4_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause5_clear(self) -> Self {
            Self(self.0 & !(1 << 5))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause6_clear(self) -> Self {
            Self(self.0 & !(1 << 6))
        }
        #[doc = "Set to 1 if the corresponding detector has detected a wakeup pattern. Write 0 to clear."]
        #[inline(always)]
        pub fn cause7_clear(self) -> Self {
            Self(self.0 & !(1 << 7))
        }
    }
    impl From<u32> for WkupCause0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupCause0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupCause0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorReadVal(u32);
    impl WkupDetectorReadVal {
        #[doc = "Wakeup detection mode. Out of range values default to Posedge."]
        #[inline(always)]
        pub fn mode(&self) -> super::enums::Mode {
            super::enums::Mode::try_from((self.0 >> 0) & 7).unwrap()
        }
        #[doc = "0: signal filter disabled, 1: signal filter enabled. the signal must\nbe stable for 4 always-on clock cycles before the value is being forwarded.\ncan be used for debouncing."]
        #[inline(always)]
        pub fn filter(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "0: select index !!WKUP_DETECTOR_PADSEL from MIO pads,\n1: select index !!WKUP_DETECTOR_PADSEL from DIO pads."]
        #[inline(always)]
        pub fn miodio(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupDetectorWriteVal {
            WkupDetectorWriteVal(self.0)
        }
    }
    impl From<u32> for WkupDetectorReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorWriteVal(u32);
    impl WkupDetectorWriteVal {
        #[doc = "Wakeup detection mode. Out of range values default to Posedge."]
        #[inline(always)]
        pub fn mode(
            self,
            f: impl FnOnce(super::enums::selector::ModeSelector) -> super::enums::Mode,
        ) -> Self {
            Self((self.0 & !(7 << 0)) | (u32::from(f(super::enums::selector::ModeSelector())) << 0))
        }
        #[doc = "0: signal filter disabled, 1: signal filter enabled. the signal must\nbe stable for 4 always-on clock cycles before the value is being forwarded.\ncan be used for debouncing."]
        #[inline(always)]
        pub fn filter(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "0: select index !!WKUP_DETECTOR_PADSEL from MIO pads,\n1: select index !!WKUP_DETECTOR_PADSEL from DIO pads."]
        #[inline(always)]
        pub fn miodio(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
    }
    impl From<u32> for WkupDetectorWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorCntThReadVal(u32);
    impl WkupDetectorCntThReadVal {
        #[doc = "Counter threshold for TimedLow and TimedHigh wakeup detector modes (see !!WKUP_DETECTOR).\nThe threshold is in terms of always-on clock cycles."]
        #[inline(always)]
        pub fn th(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupDetectorCntThWriteVal {
            WkupDetectorCntThWriteVal(self.0)
        }
    }
    impl From<u32> for WkupDetectorCntThReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorCntThReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorCntThReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorCntThWriteVal(u32);
    impl WkupDetectorCntThWriteVal {
        #[doc = "Counter threshold for TimedLow and TimedHigh wakeup detector modes (see !!WKUP_DETECTOR).\nThe threshold is in terms of always-on clock cycles."]
        #[inline(always)]
        pub fn th(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for WkupDetectorCntThWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorCntThWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorCntThWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorEnReadVal(u32);
    impl WkupDetectorEnReadVal {
        #[doc = "Setting this bit activates the corresponding wakeup detector.\nThe behavior is as specified in !!WKUP_DETECTOR,\n!!WKUP_DETECTOR_CNT_TH and !!WKUP_DETECTOR_PADSEL."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupDetectorEnWriteVal {
            WkupDetectorEnWriteVal(self.0)
        }
    }
    impl From<u32> for WkupDetectorEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorEnWriteVal(u32);
    impl WkupDetectorEnWriteVal {
        #[doc = "Setting this bit activates the corresponding wakeup detector.\nThe behavior is as specified in !!WKUP_DETECTOR,\n!!WKUP_DETECTOR_CNT_TH and !!WKUP_DETECTOR_PADSEL."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for WkupDetectorEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorPadselReadVal(u32);
    impl WkupDetectorPadselReadVal {
        #[doc = "Selects a specific MIO or DIO pad (depending on !!WKUP_DETECTOR configuration).\nIn case of MIO, the pad select index is the same as used for !!MIO_PERIPH_INSEL, meaning that index\n0 and 1 just select constants 0 and 1, and the MIO pads live at indices >= 2. In case of DIO pads,\nthe pad select index corresponds 1:1 to the DIO pad to be selected."]
        #[inline(always)]
        pub fn sel(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupDetectorPadselWriteVal {
            WkupDetectorPadselWriteVal(self.0)
        }
    }
    impl From<u32> for WkupDetectorPadselReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorPadselReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorPadselReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorPadselWriteVal(u32);
    impl WkupDetectorPadselWriteVal {
        #[doc = "Selects a specific MIO or DIO pad (depending on !!WKUP_DETECTOR configuration).\nIn case of MIO, the pad select index is the same as used for !!MIO_PERIPH_INSEL, meaning that index\n0 and 1 just select constants 0 and 1, and the MIO pads live at indices >= 2. In case of DIO pads,\nthe pad select index corresponds 1:1 to the DIO pad to be selected."]
        #[inline(always)]
        pub fn sel(self, val: u32) -> Self {
            Self((self.0 & !(0x3f << 0)) | ((val & 0x3f) << 0))
        }
    }
    impl From<u32> for WkupDetectorPadselWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorPadselWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorPadselWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorRegwenReadVal(u32);
    impl WkupDetectorRegwenReadVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding WKUP_DETECTOR\nconfiguration is not writable anymore."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> WkupDetectorRegwenWriteVal {
            WkupDetectorRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for WkupDetectorRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WkupDetectorRegwenWriteVal(u32);
    impl WkupDetectorRegwenWriteVal {
        #[doc = "Register write enable bit.\nIf this is cleared to 0, the corresponding WKUP_DETECTOR\nconfiguration is not writable anymore."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for WkupDetectorRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WkupDetectorRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WkupDetectorRegwenWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Mode {
        Posedge = 0,
        Negedge = 1,
        Edge = 2,
        Timedhigh = 3,
        Timedlow = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl Mode {
        #[inline(always)]
        pub fn posedge(&self) -> bool {
            *self == Self::Posedge
        }
        #[inline(always)]
        pub fn negedge(&self) -> bool {
            *self == Self::Negedge
        }
        #[inline(always)]
        pub fn edge(&self) -> bool {
            *self == Self::Edge
        }
        #[inline(always)]
        pub fn timed_high(&self) -> bool {
            *self == Self::Timedhigh
        }
        #[inline(always)]
        pub fn timed_low(&self) -> bool {
            *self == Self::Timedlow
        }
    }
    impl TryFrom<u32> for Mode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Mode, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Mode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Mode> for u32 {
        fn from(val: Mode) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Out {
        TieLow = 0,
        TieHigh = 1,
        HighZ = 2,
        Keep = 3,
    }
    impl Out {
        #[inline(always)]
        pub fn tie_low(&self) -> bool {
            *self == Self::TieLow
        }
        #[inline(always)]
        pub fn tie_high(&self) -> bool {
            *self == Self::TieHigh
        }
        #[inline(always)]
        pub fn high_z(&self) -> bool {
            *self == Self::HighZ
        }
        #[inline(always)]
        pub fn keep(&self) -> bool {
            *self == Self::Keep
        }
    }
    impl TryFrom<u32> for Out {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Out, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Out>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Out> for u32 {
        fn from(val: Out) -> Self {
            val as u32
        }
    }
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
        pub struct ModeSelector();
        impl ModeSelector {
            #[inline(always)]
            pub fn posedge(&self) -> super::Mode {
                super::Mode::Posedge
            }
            #[inline(always)]
            pub fn negedge(&self) -> super::Mode {
                super::Mode::Negedge
            }
            #[inline(always)]
            pub fn edge(&self) -> super::Mode {
                super::Mode::Edge
            }
            #[inline(always)]
            pub fn timed_high(&self) -> super::Mode {
                super::Mode::Timedhigh
            }
            #[inline(always)]
            pub fn timed_low(&self) -> super::Mode {
                super::Mode::Timedlow
            }
        }
        pub struct OutSelector();
        impl OutSelector {
            #[inline(always)]
            pub fn tie_low(&self) -> super::Out {
                super::Out::TieLow
            }
            #[inline(always)]
            pub fn tie_high(&self) -> super::Out {
                super::Out::TieHigh
            }
            #[inline(always)]
            pub fn high_z(&self) -> super::Out {
                super::Out::HighZ
            }
            #[inline(always)]
            pub fn keep(&self) -> super::Out {
                super::Out::Keep
            }
        }
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
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type MioPeriphInselRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MioPeriphInselRegwenReadVal,
        crate::regs::MioPeriphInselRegwenWriteVal,
    >;
    pub type MioPeriphInsel = ureg::ReadWriteReg32<
        0,
        crate::regs::MioPeriphInselReadVal,
        crate::regs::MioPeriphInselWriteVal,
    >;
    pub type MioOutselRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MioOutselRegwenReadVal,
        crate::regs::MioOutselRegwenWriteVal,
    >;
    pub type MioOutsel =
        ureg::ReadWriteReg32<2, crate::regs::MioOutselReadVal, crate::regs::MioOutselWriteVal>;
    pub type MioPadAttrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MioPadAttrRegwenReadVal,
        crate::regs::MioPadAttrRegwenWriteVal,
    >;
    pub type MioPadAttr =
        ureg::ReadWriteReg32<0, crate::regs::IoPadAttrReadVal, crate::regs::IoPadAttrWriteVal>;
    pub type DioPadAttrRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::DioPadAttrRegwenReadVal,
        crate::regs::DioPadAttrRegwenWriteVal,
    >;
    pub type DioPadAttr =
        ureg::ReadWriteReg32<0, crate::regs::IoPadAttrReadVal, crate::regs::IoPadAttrWriteVal>;
    pub type MioPadSleepStatus0 = ureg::ReadWriteReg32<
        0,
        crate::regs::MioPadSleepStatus0ReadVal,
        crate::regs::MioPadSleepStatus0WriteVal,
    >;
    pub type MioPadSleepStatus1 = ureg::ReadWriteReg32<
        0,
        crate::regs::MioPadSleepStatus1ReadVal,
        crate::regs::MioPadSleepStatus1WriteVal,
    >;
    pub type MioPadSleepRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MioPadSleepRegwenReadVal,
        crate::regs::MioPadSleepRegwenWriteVal,
    >;
    pub type MioPadSleepEn = ureg::ReadWriteReg32<
        0,
        crate::regs::MioPadSleepEnReadVal,
        crate::regs::MioPadSleepEnWriteVal,
    >;
    pub type MioPadSleepMode = ureg::ReadWriteReg32<
        2,
        crate::regs::IoPadSleepModeReadVal,
        crate::regs::IoPadSleepModeWriteVal,
    >;
    pub type DioPadSleepStatus0 = ureg::ReadWriteReg32<
        0,
        crate::regs::DioPadSleepStatus0ReadVal,
        crate::regs::DioPadSleepStatus0WriteVal,
    >;
    pub type DioPadSleepRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::DioPadSleepRegwenReadVal,
        crate::regs::DioPadSleepRegwenWriteVal,
    >;
    pub type DioPadSleepEn = ureg::ReadWriteReg32<
        0,
        crate::regs::DioPadSleepEnReadVal,
        crate::regs::DioPadSleepEnWriteVal,
    >;
    pub type DioPadSleepMode = ureg::ReadWriteReg32<
        2,
        crate::regs::IoPadSleepModeReadVal,
        crate::regs::IoPadSleepModeWriteVal,
    >;
    pub type WkupDetectorRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::WkupDetectorRegwenReadVal,
        crate::regs::WkupDetectorRegwenWriteVal,
    >;
    pub type WkupDetectorEn = ureg::ReadWriteReg32<
        0,
        crate::regs::WkupDetectorEnReadVal,
        crate::regs::WkupDetectorEnWriteVal,
    >;
    pub type WkupDetector = ureg::ReadWriteReg32<
        0,
        crate::regs::WkupDetectorReadVal,
        crate::regs::WkupDetectorWriteVal,
    >;
    pub type WkupDetectorCntTh = ureg::ReadWriteReg32<
        0,
        crate::regs::WkupDetectorCntThReadVal,
        crate::regs::WkupDetectorCntThWriteVal,
    >;
    pub type WkupDetectorPadsel = ureg::ReadWriteReg32<
        0,
        crate::regs::WkupDetectorPadselReadVal,
        crate::regs::WkupDetectorPadselWriteVal,
    >;
    pub type WkupCause0 =
        ureg::ReadWriteReg32<0, crate::regs::WkupCause0ReadVal, crate::regs::WkupCause0WriteVal>;
}
