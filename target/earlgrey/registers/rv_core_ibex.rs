#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RvCoreIbex {
    _priv: (),
}
impl RvCoreIbex {
    pub const PTR: *mut u32 = 0x411f0000 as *mut u32;
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
    #[doc = "Software recoverable error\n\nRead value: [`regs::SwRecovErrReadVal`]; Write value: [`regs::SwRecovErrWriteVal`]"]
    #[inline(always)]
    pub fn sw_recov_err(&self) -> ureg::RegRef<crate::meta::SwRecovErr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Software fatal error\n\nRead value: [`regs::SwFatalErrReadVal`]; Write value: [`regs::SwFatalErrWriteVal`]"]
    #[inline(always)]
    pub fn sw_fatal_err(&self) -> ureg::RegRef<crate::meta::SwFatalErr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Ibus address control regwen.\n\nRead value: [`regs::BusRegwenReadVal`]; Write value: [`regs::BusRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ibus_regwen(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::IbusRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable Ibus address matching\n\nRead value: [`regs::IbusAddrEnReadVal`]; Write value: [`regs::IbusAddrEnWriteVal`]"]
    #[inline(always)]
    pub fn ibus_addr_en(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::IbusAddrEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Matching region programming for ibus.\n\n  The value programmed is done at power-of-2 alignment.\n  For example, if the intended matching region is 0x8000_0000 to 0x8000_FFFF, the value programmed is 0x8000_7FFF.\n\n  The value programmed can be determined from the translation granule.\n  Assume the user wishes to translate a specific 64KB block to a different address:\n  64KB has a hex value of 0x10000.\n  Subtract 1 from this value and then right shift by one to obtain 0x7FFF.\n  This value is then logically OR'd with the upper address bits that would select which 64KB to translate.\n\n  In this example, the user wishes to translate the 0x8000-th 64KB block.\n  The value programmed is then 0x8000_7FFF.\n\n  If the user were to translate the 0x8001-th 64KB block, the value programmed would be 0x8001_7FFF.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn ibus_addr_matching(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::IbusAddrMatching, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  The remap address after a match has been made.\n  The remap bits apply only to top portion of address bits not covered by the matching region.\n\n  For example, if the translation region is 64KB, the remapped address applies only to the upper\n  address bits that select which 64KB to be translated.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn ibus_remap_addr(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::IbusRemapAddr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Dbus address control regwen.\n\nRead value: [`regs::BusRegwenReadVal`]; Write value: [`regs::BusRegwenWriteVal`]"]
    #[inline(always)]
    pub fn dbus_regwen(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::DbusRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable dbus address matching\n\nRead value: [`regs::DbusAddrEnReadVal`]; Write value: [`regs::DbusAddrEnWriteVal`]"]
    #[inline(always)]
    pub fn dbus_addr_en(&self) -> ureg::Array<2, ureg::RegRef<crate::meta::DbusAddrEn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "See !!IBUS_ADDR_MATCHING_0 for detailed description.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn dbus_addr_matching(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::DbusAddrMatching, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "See !!IBUS_REMAP_ADDR_0 for a detailed description.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn dbus_remap_addr(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::DbusRemapAddr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable mask for NMI.\nOnce an enable mask is set, it cannot be disabled.\n\nRead value: [`regs::NmiEnableReadVal`]; Write value: [`regs::NmiEnableWriteVal`]"]
    #[inline(always)]
    pub fn nmi_enable(&self) -> ureg::RegRef<crate::meta::NmiEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current NMI state\n\nRead value: [`regs::NmiStateReadVal`]; Write value: [`regs::NmiStateWriteVal`]"]
    #[inline(always)]
    pub fn nmi_state(&self) -> ureg::RegRef<crate::meta::NmiState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "error status\n\nRead value: [`regs::ErrStatusReadVal`]; Write value: [`regs::ErrStatusWriteVal`]"]
    #[inline(always)]
    pub fn err_status(&self) -> ureg::RegRef<crate::meta::ErrStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Random data from EDN\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rnd_data(&self) -> ureg::RegRef<crate::meta::RndData, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status of random data in !!RND_DATA\n\nRead value: [`regs::RndStatusReadVal`]; Write value: [`regs::RndStatusWriteVal`]"]
    #[inline(always)]
    pub fn rnd_status(&self) -> ureg::RegRef<crate::meta::RndStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "FPGA build timestamp info.\nThis register only contains valid data for fpga, for all other variants it is simply 0.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn fpga_info(&self) -> ureg::RegRef<crate::meta::FpgaInfo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x60 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Exposed tlul window for DV only purposes.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn dv_sim_window(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::DvSimWindow, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
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
        pub fn fatal_sw_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_sw_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_hw_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_hw_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
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
    pub struct BusRegwenReadVal(u32);
    impl BusRegwenReadVal {
        #[doc = "Ibus address controls write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn en(&self) -> super::enums::En {
            super::enums::En::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BusRegwenWriteVal {
            BusRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for BusRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BusRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: BusRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BusRegwenWriteVal(u32);
    impl BusRegwenWriteVal {
        #[doc = "Ibus address controls write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for BusRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BusRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BusRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DbusAddrEnReadVal(u32);
    impl DbusAddrEnReadVal {
        #[doc = "Enable dbus address matching."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DbusAddrEnWriteVal {
            DbusAddrEnWriteVal(self.0)
        }
    }
    impl From<u32> for DbusAddrEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DbusAddrEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: DbusAddrEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DbusAddrEnWriteVal(u32);
    impl DbusAddrEnWriteVal {
        #[doc = "Enable dbus address matching."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for DbusAddrEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DbusAddrEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DbusAddrEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrStatusReadVal(u32);
    impl ErrStatusReadVal {
        #[doc = "rv_core_ibex_peri detected a register transmission integrity error"]
        #[inline(always)]
        pub fn reg_intg_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "rv_core_ibex detected a response integrity error"]
        #[inline(always)]
        pub fn fatal_intg_err(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "rv_core_ibex detected a fatal internal error\n(``alert_major_internal_o`` from Ibex seen)"]
        #[inline(always)]
        pub fn fatal_core_err(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "rv_core_ibex detected a recoverable internal error\n(``alert_minor`` from Ibex seen)"]
        #[inline(always)]
        pub fn recov_core_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrStatusWriteVal {
            ErrStatusWriteVal(self.0)
        }
    }
    impl From<u32> for ErrStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrStatusWriteVal(u32);
    impl ErrStatusWriteVal {
        #[doc = "rv_core_ibex_peri detected a register transmission integrity error"]
        #[inline(always)]
        pub fn reg_intg_err_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "rv_core_ibex detected a response integrity error"]
        #[inline(always)]
        pub fn fatal_intg_err_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "rv_core_ibex detected a fatal internal error\n(``alert_major_internal_o`` from Ibex seen)"]
        #[inline(always)]
        pub fn fatal_core_err_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "rv_core_ibex detected a recoverable internal error\n(``alert_minor`` from Ibex seen)"]
        #[inline(always)]
        pub fn recov_core_err_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
    }
    impl From<u32> for ErrStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IbusAddrEnReadVal(u32);
    impl IbusAddrEnReadVal {
        #[doc = "Enable ibus address matching."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IbusAddrEnWriteVal {
            IbusAddrEnWriteVal(self.0)
        }
    }
    impl From<u32> for IbusAddrEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IbusAddrEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: IbusAddrEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IbusAddrEnWriteVal(u32);
    impl IbusAddrEnWriteVal {
        #[doc = "Enable ibus address matching."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for IbusAddrEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IbusAddrEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IbusAddrEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct NmiEnableReadVal(u32);
    impl NmiEnableReadVal {
        #[doc = "Enable mask for alert NMI"]
        #[inline(always)]
        pub fn alert_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable mask for watchdog NMI"]
        #[inline(always)]
        pub fn wdog_en(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> NmiEnableWriteVal {
            NmiEnableWriteVal(self.0)
        }
    }
    impl From<u32> for NmiEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<NmiEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: NmiEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct NmiEnableWriteVal(u32);
    impl NmiEnableWriteVal {
        #[doc = "Enable mask for alert NMI"]
        #[inline(always)]
        pub fn alert_en_set(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Enable mask for watchdog NMI"]
        #[inline(always)]
        pub fn wdog_en_set(self) -> Self {
            Self(self.0 | (1 << 1))
        }
    }
    impl From<u32> for NmiEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<NmiEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: NmiEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct NmiStateReadVal(u32);
    impl NmiStateReadVal {
        #[doc = "Current state for alert NMI"]
        #[inline(always)]
        pub fn alert(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Current state for watchdog NMI"]
        #[inline(always)]
        pub fn wdog(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> NmiStateWriteVal {
            NmiStateWriteVal(self.0)
        }
    }
    impl From<u32> for NmiStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<NmiStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: NmiStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct NmiStateWriteVal(u32);
    impl NmiStateWriteVal {
        #[doc = "Current state for alert NMI"]
        #[inline(always)]
        pub fn alert_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Current state for watchdog NMI"]
        #[inline(always)]
        pub fn wdog_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
    }
    impl From<u32> for NmiStateWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<NmiStateWriteVal> for u32 {
        #[inline(always)]
        fn from(val: NmiStateWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RndStatusReadVal(u32);
    impl RndStatusReadVal {
        #[doc = "When set, the data in !!RND_DATA is valid. When clear an EDN\nrequest for new data for !!RND_DATA is pending."]
        #[inline(always)]
        pub fn rnd_data_valid(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "When !!RND_STATUS.RND_DATA_VALID is 1, this bit indicates whether\n!!RND_DATA is fips quality.\n\nWhen !!RND_STATUS.RND_DATA_VALID is 0, this bit has no meaning."]
        #[inline(always)]
        pub fn rnd_data_fips(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
    }
    impl From<u32> for RndStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RndStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: RndStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwFatalErrReadVal(u32);
    impl SwFatalErrReadVal {
        #[doc = "Software fatal alert.\nWhen set to any value other than kMultiBitBool4False, a fatal alert is sent.\nNote, this field once cleared cannot be set and will continuously cause alert events."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SwFatalErrWriteVal {
            SwFatalErrWriteVal(self.0)
        }
    }
    impl From<u32> for SwFatalErrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwFatalErrReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwFatalErrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwFatalErrWriteVal(u32);
    impl SwFatalErrWriteVal {
        #[doc = "Software fatal alert.\nWhen set to any value other than kMultiBitBool4False, a fatal alert is sent.\nNote, this field once cleared cannot be set and will continuously cause alert events."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for SwFatalErrWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwFatalErrWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SwFatalErrWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRecovErrReadVal(u32);
    impl SwRecovErrReadVal {
        #[doc = "Software recoverable alert.\nWhen set to any value other than kMultiBitBool4False, a recoverable alert is sent.\nOnce the alert is sent, the field is then reset to kMultiBitBool4False."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SwRecovErrWriteVal {
            SwRecovErrWriteVal(self.0)
        }
    }
    impl From<u32> for SwRecovErrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRecovErrReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwRecovErrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwRecovErrWriteVal(u32);
    impl SwRecovErrWriteVal {
        #[doc = "Software recoverable alert.\nWhen set to any value other than kMultiBitBool4False, a recoverable alert is sent.\nOnce the alert is sent, the field is then reset to kMultiBitBool4False."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for SwRecovErrWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwRecovErrWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SwRecovErrWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum En {
        Locked = 0,
        Enabled = 1,
    }
    impl En {
        #[inline(always)]
        pub fn locked(&self) -> bool {
            *self == Self::Locked
        }
        #[inline(always)]
        pub fn enabled(&self) -> bool {
            *self == Self::Enabled
        }
    }
    impl TryFrom<u32> for En {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<En, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, En>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<En> for u32 {
        fn from(val: En) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct EnSelector();
        impl EnSelector {
            #[inline(always)]
            pub fn locked(&self) -> super::En {
                super::En::Locked
            }
            #[inline(always)]
            pub fn enabled(&self) -> super::En {
                super::En::Enabled
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type SwRecovErr =
        ureg::ReadWriteReg32<9, crate::regs::SwRecovErrReadVal, crate::regs::SwRecovErrWriteVal>;
    pub type SwFatalErr =
        ureg::ReadWriteReg32<9, crate::regs::SwFatalErrReadVal, crate::regs::SwFatalErrWriteVal>;
    pub type IbusRegwen =
        ureg::ReadWriteReg32<1, crate::regs::BusRegwenReadVal, crate::regs::BusRegwenWriteVal>;
    pub type IbusAddrEn =
        ureg::ReadWriteReg32<0, crate::regs::IbusAddrEnReadVal, crate::regs::IbusAddrEnWriteVal>;
    pub type IbusAddrMatching = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IbusRemapAddr = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DbusRegwen =
        ureg::ReadWriteReg32<1, crate::regs::BusRegwenReadVal, crate::regs::BusRegwenWriteVal>;
    pub type DbusAddrEn =
        ureg::ReadWriteReg32<0, crate::regs::DbusAddrEnReadVal, crate::regs::DbusAddrEnWriteVal>;
    pub type DbusAddrMatching = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DbusRemapAddr = ureg::ReadWriteReg32<0, u32, u32>;
    pub type NmiEnable =
        ureg::ReadWriteReg32<0, crate::regs::NmiEnableReadVal, crate::regs::NmiEnableWriteVal>;
    pub type NmiState =
        ureg::ReadWriteReg32<0, crate::regs::NmiStateReadVal, crate::regs::NmiStateWriteVal>;
    pub type ErrStatus =
        ureg::ReadWriteReg32<0, crate::regs::ErrStatusReadVal, crate::regs::ErrStatusWriteVal>;
    pub type RndData = ureg::ReadOnlyReg32<u32>;
    pub type RndStatus = ureg::ReadOnlyReg32<crate::regs::RndStatusReadVal>;
    pub type FpgaInfo = ureg::ReadOnlyReg32<u32>;
    pub type DvSimWindow = ureg::ReadWriteReg32<0, u32, u32>;
}
