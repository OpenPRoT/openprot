#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RvDm {
    _priv: (),
}
impl RvDm {
    pub const PTR: *mut u32 = 0x41200000 as *mut u32;
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
    #[doc = "Lock bit for !!LATE_DEBUG_ENABLE register.\n\nRead value: [`regs::LateDebugEnableRegwenReadVal`]; Write value: [`regs::LateDebugEnableRegwenWriteVal`]"]
    #[inline(always)]
    pub fn late_debug_enable_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::LateDebugEnableRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Debug enable register.\n\nIf the device is in the DEV lifecycle state and the\nDIS_RV_DM_LATE_DEBUG_IN_DEV has been programmed to kMuBi8False\n(or an invalid value), the RV_DM gating mechanisms are by default\nnot ungated until SW writes kMuBi32True to this register.\n\nThis can be leveraged to implement a \"late debug enable in DEV\"\npolicy, whereby ROM_EXT first locks out any sensitive areas and\nfunctionalities of the device before enabling debug access via\nRV_DM.\n\nThis register can be locked out via !!LATE_DEBUG_ENABLE_REGWEN.\n\nThis register does not have any effect in the following cases:\n  - If the device is in a DFT-enabled life cycle state (TEST_UNLOCKED*, RMA)\n  - If the device is in the DEV life cycle state and DIS_RV_DM_LATE_DEBUG_IN_DEV has been programmed to kMuBi8True\n  - If the device is in a life cycle state where hardware debugging is disabled (TEST_LOCKED*, PROD*, invalid states).\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn late_debug_enable(&self) -> ureg::RegRef<crate::meta::LateDebugEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
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
    pub struct LateDebugEnableRegwenReadVal(u32);
    impl LateDebugEnableRegwenReadVal {
        #[doc = "LATE_DEBUG_ENABLE register configuration enable bit. If\n  this is cleared to 0, the !!LATE_DEBUG_ENABLE register\n  cannot be written anymore."]
        #[inline(always)]
        pub fn late_debug_enable_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> LateDebugEnableRegwenWriteVal {
            LateDebugEnableRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for LateDebugEnableRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LateDebugEnableRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: LateDebugEnableRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LateDebugEnableRegwenWriteVal(u32);
    impl LateDebugEnableRegwenWriteVal {
        #[doc = "LATE_DEBUG_ENABLE register configuration enable bit. If\n  this is cleared to 0, the !!LATE_DEBUG_ENABLE register\n  cannot be written anymore."]
        #[inline(always)]
        pub fn late_debug_enable_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for LateDebugEnableRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LateDebugEnableRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: LateDebugEnableRegwenWriteVal) -> u32 {
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
    pub type LateDebugEnableRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::LateDebugEnableRegwenReadVal,
        crate::regs::LateDebugEnableRegwenWriteVal,
    >;
    pub type LateDebugEnable = ureg::ReadWriteReg32<0x69696969, u32, u32>;
}
