#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Gpio {
    _priv: (),
}
impl Gpio {
    pub const PTR: *mut u32 = 0x40040000 as *mut u32;
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
    #[doc = "Interrupt State Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_state(&self) -> ureg::RegRef<crate::meta::IntrState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_enable(&self) -> ureg::RegRef<crate::meta::IntrEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Test Register\n\nRead value: [`u32`]; Write value: [`u32`]"]
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
    #[doc = "GPIO Input data read value\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn data_in(&self) -> ureg::RegRef<crate::meta::DataIn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO direct output data write value\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn direct_out(&self) -> ureg::RegRef<crate::meta::DirectOut, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO write data lower with mask.\n\nMasked write for DATA_OUT[15:0].\n\nUpper 16 bits of this register are used as mask. Writing\nlower 16 bits of the register changes DATA_OUT[15:0] value\nif mask bits are set.\n\nRead-back of this register returns upper 16 bits as zero\nand lower 16 bits as DATA_OUT[15:0].\n\nRead value: [`regs::MaskedOutLowerReadVal`]; Write value: [`regs::MaskedOutLowerWriteVal`]"]
    #[inline(always)]
    pub fn masked_out_lower(&self) -> ureg::RegRef<crate::meta::MaskedOutLower, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO write data upper with mask.\n\nMasked write for DATA_OUT[31:16].\n\nUpper 16 bits of this register are used as mask. Writing\nlower 16 bits of the register changes DATA_OUT[31:16] value\nif mask bits are set.\n\nRead-back of this register returns upper 16 bits as zero\nand lower 16 bits as DATA_OUT[31:16].\n\nRead value: [`regs::MaskedOutUpperReadVal`]; Write value: [`regs::MaskedOutUpperWriteVal`]"]
    #[inline(always)]
    pub fn masked_out_upper(&self) -> ureg::RegRef<crate::meta::MaskedOutUpper, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO Output Enable.\n\nSetting direct_oe[i] to 1 enables output mode for GPIO[i]\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn direct_oe(&self) -> ureg::RegRef<crate::meta::DirectOe, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO write Output Enable lower with mask.\n\nMasked write for DATA_OE[15:0], the register that controls\noutput mode for GPIO pins [15:0].\n\nUpper 16 bits of this register are used as mask. Writing\nlower 16 bits of the register changes DATA_OE[15:0] value\nif mask bits are set.\n\nRead-back of this register returns upper 16 bits as zero\nand lower 16 bits as DATA_OE[15:0].\n\nRead value: [`regs::MaskedOeLowerReadVal`]; Write value: [`regs::MaskedOeLowerWriteVal`]"]
    #[inline(always)]
    pub fn masked_oe_lower(&self) -> ureg::RegRef<crate::meta::MaskedOeLower, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO write Output Enable upper with mask.\n\nMasked write for DATA_OE[31:16], the register that controls\noutput mode for GPIO pins [31:16].\n\nUpper 16 bits of this register are used as mask. Writing\nlower 16 bits of the register changes DATA_OE[31:16] value\nif mask bits are set.\n\nRead-back of this register returns upper 16 bits as zero\nand lower 16 bits as DATA_OE[31:16].\n\nRead value: [`regs::MaskedOeUpperReadVal`]; Write value: [`regs::MaskedOeUpperWriteVal`]"]
    #[inline(always)]
    pub fn masked_oe_upper(&self) -> ureg::RegRef<crate::meta::MaskedOeUpper, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO interrupt enable for GPIO, rising edge.\n\nIf !!INTR_ENABLE[i] is true, a value of 1 on !!INTR_CTRL_EN_RISING[i]\nenables rising-edge interrupt detection on GPIO[i].\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_ctrl_en_rising(&self) -> ureg::RegRef<crate::meta::IntrCtrlEnRising, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO interrupt enable for GPIO, falling edge.\n\nIf !!INTR_ENABLE[i] is true, a value of 1 on !!INTR_CTRL_EN_FALLING[i]\nenables falling-edge interrupt detection on GPIO[i].\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_ctrl_en_falling(&self) -> ureg::RegRef<crate::meta::IntrCtrlEnFalling, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO interrupt enable for GPIO, level high.\n\nIf !!INTR_ENABLE[i] is true, a value of 1 on !!INTR_CTRL_EN_LVLHIGH[i]\nenables level high interrupt detection on GPIO[i].\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_ctrl_en_lvlhigh(&self) -> ureg::RegRef<crate::meta::IntrCtrlEnLvlhigh, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "GPIO interrupt enable for GPIO, level low.\n\nIf !!INTR_ENABLE[i] is true, a value of 1 on !!INTR_CTRL_EN_LVLLOW[i]\nenables level low interrupt detection on GPIO[i].\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn intr_ctrl_en_lvllow(&self) -> ureg::RegRef<crate::meta::IntrCtrlEnLvllow, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "filter enable for GPIO input bits.\n\nIf !!CTRL_EN_INPUT_FILTER[i] is true, a value of input bit [i]\nmust be stable for 16 cycles before transitioning.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn ctrl_en_input_filter(&self) -> ureg::RegRef<crate::meta::CtrlEnInputFilter, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
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
    pub struct MaskedOeLowerReadVal(u32);
    impl MaskedOeLowerReadVal {
        #[doc = "Write OE value[15:0].\n\nValue to write into DATA_OE[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Write OE mask[15:0].\n\nA value of 1 in mask[i] allows the updating of DATA_OE[i], 0 <= i <= 15"]
        #[inline(always)]
        pub fn mask(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaskedOeLowerWriteVal {
            MaskedOeLowerWriteVal(self.0)
        }
    }
    impl From<u32> for MaskedOeLowerReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOeLowerReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOeLowerReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOeLowerWriteVal(u32);
    impl MaskedOeLowerWriteVal {
        #[doc = "Write OE value[15:0].\n\nValue to write into DATA_OE[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Write OE mask[15:0].\n\nA value of 1 in mask[i] allows the updating of DATA_OE[i], 0 <= i <= 15"]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for MaskedOeLowerWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOeLowerWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOeLowerWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOeUpperReadVal(u32);
    impl MaskedOeUpperReadVal {
        #[doc = "Write OE value[31:16].\n\nValue to write into DATA_OE[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Write OE mask[31:16].\n\nA value of 1 in mask[i] allows the updating of DATA_OE[i], 16 <= i <= 31"]
        #[inline(always)]
        pub fn mask(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaskedOeUpperWriteVal {
            MaskedOeUpperWriteVal(self.0)
        }
    }
    impl From<u32> for MaskedOeUpperReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOeUpperReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOeUpperReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOeUpperWriteVal(u32);
    impl MaskedOeUpperWriteVal {
        #[doc = "Write OE value[31:16].\n\nValue to write into DATA_OE[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Write OE mask[31:16].\n\nA value of 1 in mask[i] allows the updating of DATA_OE[i], 16 <= i <= 31"]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for MaskedOeUpperWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOeUpperWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOeUpperWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOutLowerReadVal(u32);
    impl MaskedOutLowerReadVal {
        #[doc = "Write data value[15:0].\n\nValue to write into DATA_OUT[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaskedOutLowerWriteVal {
            MaskedOutLowerWriteVal(self.0)
        }
    }
    impl From<u32> for MaskedOutLowerReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOutLowerReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOutLowerReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOutLowerWriteVal(u32);
    impl MaskedOutLowerWriteVal {
        #[doc = "Write data value[15:0].\n\nValue to write into DATA_OUT[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Write data mask[15:0].\n\nA value of 1 in mask[i] allows the updating of DATA_OUT[i], 0 <= i <= 15"]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for MaskedOutLowerWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOutLowerWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOutLowerWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOutUpperReadVal(u32);
    impl MaskedOutUpperReadVal {
        #[doc = "Write data value[31:16].\n\n   Value to write into DATA_OUT[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaskedOutUpperWriteVal {
            MaskedOutUpperWriteVal(self.0)
        }
    }
    impl From<u32> for MaskedOutUpperReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOutUpperReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOutUpperReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaskedOutUpperWriteVal(u32);
    impl MaskedOutUpperWriteVal {
        #[doc = "Write data value[31:16].\n\n   Value to write into DATA_OUT[i], valid in the presence of mask[i]==1"]
        #[inline(always)]
        pub fn data(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Write data mask[31:16].\n\nA value of 1 in mask[i] allows the updating of DATA_OUT[i], 16 <= i <= 31"]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for MaskedOutUpperWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaskedOutUpperWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaskedOutUpperWriteVal) -> u32 {
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
    pub type IntrState = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrEnable = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, u32>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type DataIn = ureg::ReadOnlyReg32<u32>;
    pub type DirectOut = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MaskedOutLower = ureg::ReadWriteReg32<
        0,
        crate::regs::MaskedOutLowerReadVal,
        crate::regs::MaskedOutLowerWriteVal,
    >;
    pub type MaskedOutUpper = ureg::ReadWriteReg32<
        0,
        crate::regs::MaskedOutUpperReadVal,
        crate::regs::MaskedOutUpperWriteVal,
    >;
    pub type DirectOe = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MaskedOeLower = ureg::ReadWriteReg32<
        0,
        crate::regs::MaskedOeLowerReadVal,
        crate::regs::MaskedOeLowerWriteVal,
    >;
    pub type MaskedOeUpper = ureg::ReadWriteReg32<
        0,
        crate::regs::MaskedOeUpperReadVal,
        crate::regs::MaskedOeUpperWriteVal,
    >;
    pub type IntrCtrlEnRising = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrCtrlEnFalling = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrCtrlEnLvlhigh = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntrCtrlEnLvllow = ureg::ReadWriteReg32<0, u32, u32>;
    pub type CtrlEnInputFilter = ureg::ReadWriteReg32<0, u32, u32>;
}
