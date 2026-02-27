#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct FlashCtrl {
    _priv: (),
}
impl FlashCtrl {
    pub const PTR: *mut u32 = 0x41000000 as *mut u32;
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
    #[doc = "Disable flash functionality\n\nRead value: [`regs::DisReadVal`]; Write value: [`regs::DisWriteVal`]"]
    #[inline(always)]
    pub fn dis(&self) -> ureg::RegRef<crate::meta::Dis, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls whether flash can be used for code execution fetches\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn exec(&self) -> ureg::RegRef<crate::meta::Exec, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controller init register\n\nRead value: [`regs::InitReadVal`]; Write value: [`regs::InitWriteVal`]"]
    #[inline(always)]
    pub fn init(&self) -> ureg::RegRef<crate::meta::Init, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls the configurability of the !!CONTROL register.\n\nThis register ensures the contents of !!CONTROL cannot be changed by software once a flash\noperation has begun.\n\nIt unlocks whenever the existing flash operation completes, regardless of success or error.\n\nRead value: [`regs::CtrlRegwenReadVal`]; Write value: [`regs::CtrlRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_regwen(&self) -> ureg::RegRef<crate::meta::CtrlRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control register\n\nRead value: [`regs::ControlReadVal`]; Write value: [`regs::ControlWriteVal`]"]
    #[inline(always)]
    pub fn control(&self) -> ureg::RegRef<crate::meta::Control, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Address for flash operation\n\nRead value: [`regs::AddrReadVal`]; Write value: [`regs::AddrWriteVal`]"]
    #[inline(always)]
    pub fn addr(&self) -> ureg::RegRef<crate::meta::Addr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable different program types\n\nRead value: [`regs::ProgTypeEnReadVal`]; Write value: [`regs::ProgTypeEnWriteVal`]"]
    #[inline(always)]
    pub fn prog_type_en(&self) -> ureg::RegRef<crate::meta::ProgTypeEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Suspend erase\n\nRead value: [`regs::EraseSuspendReadVal`]; Write value: [`regs::EraseSuspendWriteVal`]"]
    #[inline(always)]
    pub fn erase_suspend(&self) -> ureg::RegRef<crate::meta::EraseSuspend, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::RegionCfgRegwenReadVal`]; Write value: [`regs::RegionCfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn region_cfg_regwen(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::RegionCfgRegwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory property configuration for data partition\n\nRead value: [`regs::MpRegionCfgReadVal`]; Write value: [`regs::MpRegionCfgWriteVal`]"]
    #[inline(always)]
    pub fn mp_region_cfg(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::MpRegionCfg, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory base and size configuration for data partition\n\nRead value: [`regs::MpRegionReadVal`]; Write value: [`regs::MpRegionWriteVal`]"]
    #[inline(always)]
    pub fn mp_region(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::MpRegion, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Default region properties\n\nRead value: [`regs::DefaultRegionReadVal`]; Write value: [`regs::DefaultRegionWriteVal`]"]
    #[inline(always)]
    pub fn default_region(&self) -> ureg::RegRef<crate::meta::DefaultRegion, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo0RegwenReadVal`]; Write value: [`regs::BankxInfo0RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info0_regwen(
        &self,
    ) -> ureg::Array<10, ureg::RegRef<crate::meta::Bank0Info0Regwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank0,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info0_page_cfg(
        &self,
    ) -> ureg::Array<10, ureg::RegRef<crate::meta::Bank0Info0PageCfg, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xbc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo1RegwenReadVal`]; Write value: [`regs::BankxInfo1RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info1_regwen(&self) -> ureg::RegRef<crate::meta::Bank0Info1Regwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank0,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info1_page_cfg(&self) -> ureg::RegRef<crate::meta::Bank0Info1PageCfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo2RegwenReadVal`]; Write value: [`regs::BankxInfo2RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info2_regwen(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Bank0Info2Regwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xec / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank0,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank0_info2_page_cfg(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Bank0Info2PageCfg, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xf4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo0RegwenReadVal`]; Write value: [`regs::BankxInfo0RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info0_regwen(
        &self,
    ) -> ureg::Array<10, ureg::RegRef<crate::meta::Bank1Info0Regwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xfc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank1,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info0_page_cfg(
        &self,
    ) -> ureg::Array<10, ureg::RegRef<crate::meta::Bank1Info0PageCfg, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x124 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo1RegwenReadVal`]; Write value: [`regs::BankxInfo1RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info1_regwen(&self) -> ureg::RegRef<crate::meta::Bank1Info1Regwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank1,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info1_page_cfg(&self) -> ureg::RegRef<crate::meta::Bank1Info1PageCfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x150 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory region registers configuration enable.\n\nRead value: [`regs::BankxInfo2RegwenReadVal`]; Write value: [`regs::BankxInfo2RegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info2_regwen(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Bank1Info2Regwen, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x154 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "  Memory property configuration for info partition in bank1,\n  Unlike data partition, each page is individually configured.\n\nRead value: [`regs::BankxInfoxPageCfgReadVal`]; Write value: [`regs::BankxInfoxPageCfgWriteVal`]"]
    #[inline(always)]
    pub fn bank1_info2_page_cfg(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Bank1Info2PageCfg, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x15c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "HW interface info configuration rule overrides\n\nRead value: [`regs::HwInfoCfgOverrideReadVal`]; Write value: [`regs::HwInfoCfgOverrideWriteVal`]"]
    #[inline(always)]
    pub fn hw_info_cfg_override(&self) -> ureg::RegRef<crate::meta::HwInfoCfgOverride, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x164 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Bank configuration registers configuration enable.\n\nRead value: [`regs::BankCfgRegwenReadVal`]; Write value: [`regs::BankCfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn bank_cfg_regwen(&self) -> ureg::RegRef<crate::meta::BankCfgRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x168 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Memory properties bank configuration\n\nRead value: [`regs::MpBankCfgShadowed0ReadVal`]; Write value: [`regs::MpBankCfgShadowed0WriteVal`]"]
    #[inline(always)]
    pub fn mp_bank_cfg_shadowed0(&self) -> ureg::RegRef<crate::meta::MpBankCfgShadowed0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x16c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash Operation Status\n\nRead value: [`regs::OpStatusReadVal`]; Write value: [`regs::OpStatusWriteVal`]"]
    #[inline(always)]
    pub fn op_status(&self) -> ureg::RegRef<crate::meta::OpStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x170 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash Controller Status\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x174 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current flash fsm state\n\nRead value: [`regs::DebugStateReadVal`]; Write value: [`regs::DebugStateWriteVal`]"]
    #[inline(always)]
    pub fn debug_state(&self) -> ureg::RegRef<crate::meta::DebugState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x178 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash error code register.\nThis register tabulates detailed error status of the flash.\nThis is separate from !!OP_STATUS, which is used to indicate the current state of the software initiated\nflash operation.\n\nNote, all errors in this register are considered recoverable errors, ie, errors that could have been\ngenerated by software.\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x17c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register tabulates standard fault status of the flash.\n\nThese represent errors that occur in the standard structures of the design.\nFor example fsm integrity, counter integrity and tlul integrity.\n\nRead value: [`regs::StdFaultStatusReadVal`]; Write value: [`regs::StdFaultStatusWriteVal`]"]
    #[inline(always)]
    pub fn std_fault_status(&self) -> ureg::RegRef<crate::meta::StdFaultStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x180 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register tabulates customized fault status of the flash.\n\nThese are errors that are impossible to have been caused by software or unrecoverable in nature.\n\nAll errors except for multi-bit ECC errors (!!FAULT_STATUS.PHY_RELBL_ERR) and ICV (!!FAULT_STATUS.PHY_STORAGE_ERR) trigger a fatal alert.\nOnce set, they remain set until reset.\n\nRead value: [`regs::FaultStatusReadVal`]; Write value: [`regs::FaultStatusWriteVal`]"]
    #[inline(always)]
    pub fn fault_status(&self) -> ureg::RegRef<crate::meta::FaultStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x184 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Synchronous error address\n\nRead value: [`regs::ErrAddrReadVal`]; Write value: [`regs::ErrAddrWriteVal`]"]
    #[inline(always)]
    pub fn err_addr(&self) -> ureg::RegRef<crate::meta::ErrAddr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x188 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Total number of single bit ECC error count\n\nRead value: [`regs::EccSingleErrCnt0ReadVal`]; Write value: [`regs::EccSingleErrCnt0WriteVal`]"]
    #[inline(always)]
    pub fn ecc_single_err_cnt0(&self) -> ureg::RegRef<crate::meta::EccSingleErrCnt0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Latest address of ECC single err\n\nRead value: [`regs::EccSingleErrAddrReadVal`]; Write value: [`regs::EccSingleErrAddrWriteVal`]"]
    #[inline(always)]
    pub fn ecc_single_err_addr(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::EccSingleErrAddr, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x190 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Phy alert configuration\n\nRead value: [`regs::PhyAlertCfgReadVal`]; Write value: [`regs::PhyAlertCfgWriteVal`]"]
    #[inline(always)]
    pub fn phy_alert_cfg(&self) -> ureg::RegRef<crate::meta::PhyAlertCfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x198 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash Phy Status\n\nRead value: [`regs::PhyStatusReadVal`]; Write value: [`regs::PhyStatusWriteVal`]"]
    #[inline(always)]
    pub fn phy_status(&self) -> ureg::RegRef<crate::meta::PhyStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x19c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash Controller Scratch\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn scratch(&self) -> ureg::RegRef<crate::meta::Scratch, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Programmable depth where FIFOs should generate interrupts\n\nRead value: [`regs::FifoLvlReadVal`]; Write value: [`regs::FifoLvlWriteVal`]"]
    #[inline(always)]
    pub fn fifo_lvl(&self) -> ureg::RegRef<crate::meta::FifoLvl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Reset for flash controller FIFOs\n\nRead value: [`regs::FifoRstReadVal`]; Write value: [`regs::FifoRstWriteVal`]"]
    #[inline(always)]
    pub fn fifo_rst(&self) -> ureg::RegRef<crate::meta::FifoRst, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Current program and read fifo depth\n\nRead value: [`regs::CurrFifoLvlReadVal`]; Write value: [`regs::CurrFifoLvlWriteVal`]"]
    #[inline(always)]
    pub fn curr_fifo_lvl(&self) -> ureg::RegRef<crate::meta::CurrFifoLvl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1ac / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash program FIFO.\n\nThe FIFO is 16 entries of 4B flash words. This FIFO can only be programmed\nby software after a program operation has been initiated via the !!CONTROL register.\nThis ensures accidental programming of the program FIFO cannot lock up the system.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn prog_fifo(&self) -> ureg::RegRef<crate::meta::ProgFifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1b0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash read FIFO.\n\nThe FIFO is 16 entries of 4B flash words\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rd_fifo(&self) -> ureg::RegRef<crate::meta::RdFifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1b4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AddrReadVal(u32);
    impl AddrReadVal {
        #[doc = "Start address of a flash transaction.  This is a byte address relative to the flash\nonly.  Ie, an address of 0 will access address 0 of the requested partition.\n\nFor read operations, the flash controller will truncate to the closest, lower word\naligned address.  For example, if 0x13 is supplied, the controller will perform a\nread at address 0x10.\n\nProgram operations behave similarly, the controller does not have read modified write\nsupport.\n\nFor page erases, the controller will truncate to the closest lower page aligned\naddress.  Similarly for bank erases, the controller will truncate to the closest\nlower bank aligned address."]
        #[inline(always)]
        pub fn start(&self) -> u32 {
            (self.0 >> 0) & 0xfffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AddrWriteVal {
            AddrWriteVal(self.0)
        }
    }
    impl From<u32> for AddrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AddrReadVal> for u32 {
        #[inline(always)]
        fn from(val: AddrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AddrWriteVal(u32);
    impl AddrWriteVal {
        #[doc = "Start address of a flash transaction.  This is a byte address relative to the flash\nonly.  Ie, an address of 0 will access address 0 of the requested partition.\n\nFor read operations, the flash controller will truncate to the closest, lower word\naligned address.  For example, if 0x13 is supplied, the controller will perform a\nread at address 0x10.\n\nProgram operations behave similarly, the controller does not have read modified write\nsupport.\n\nFor page erases, the controller will truncate to the closest lower page aligned\naddress.  Similarly for bank erases, the controller will truncate to the closest\nlower bank aligned address."]
        #[inline(always)]
        pub fn start(self, val: u32) -> Self {
            Self((self.0 & !(0xfffff << 0)) | ((val & 0xfffff) << 0))
        }
    }
    impl From<u32> for AddrWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AddrWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AddrWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_std_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_prim_flash_alert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_prim_flash_alert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
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
    pub struct BankxInfo0RegwenReadVal(u32);
    impl BankxInfo0RegwenReadVal {
        #[doc = "Info0 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region(&self) -> super::enums::Inforegionwen {
            super::enums::Inforegionwen::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BankxInfo0RegwenWriteVal {
            BankxInfo0RegwenWriteVal(self.0)
        }
    }
    impl From<u32> for BankxInfo0RegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo0RegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo0RegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfo0RegwenWriteVal(u32);
    impl BankxInfo0RegwenWriteVal {
        #[doc = "Info0 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for BankxInfo0RegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo0RegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo0RegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfo1RegwenReadVal(u32);
    impl BankxInfo1RegwenReadVal {
        #[doc = "Info1 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region(&self) -> super::enums::Inforegionwen {
            super::enums::Inforegionwen::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BankxInfo1RegwenWriteVal {
            BankxInfo1RegwenWriteVal(self.0)
        }
    }
    impl From<u32> for BankxInfo1RegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo1RegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo1RegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfo1RegwenWriteVal(u32);
    impl BankxInfo1RegwenWriteVal {
        #[doc = "Info1 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for BankxInfo1RegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo1RegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo1RegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfo2RegwenReadVal(u32);
    impl BankxInfo2RegwenReadVal {
        #[doc = "Info2 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region(&self) -> super::enums::Inforegionwen {
            super::enums::Inforegionwen::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BankxInfo2RegwenWriteVal {
            BankxInfo2RegwenWriteVal(self.0)
        }
    }
    impl From<u32> for BankxInfo2RegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo2RegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo2RegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfo2RegwenWriteVal(u32);
    impl BankxInfo2RegwenWriteVal {
        #[doc = "Info2 page write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for BankxInfo2RegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfo2RegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfo2RegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfoxPageCfgReadVal(u32);
    impl BankxInfoxPageCfgReadVal {
        #[doc = "Region enabled, following fields apply"]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(&self) -> u32 {
            (self.0 >> 8) & 0xf
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(&self) -> u32 {
            (self.0 >> 12) & 0xf
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "Region is ECC enabled (both integrity and reliability ECC)."]
        #[inline(always)]
        pub fn ecc_en(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(&self) -> u32 {
            (self.0 >> 24) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BankxInfoxPageCfgWriteVal {
            BankxInfoxPageCfgWriteVal(self.0)
        }
    }
    impl From<u32> for BankxInfoxPageCfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfoxPageCfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfoxPageCfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankxInfoxPageCfgWriteVal(u32);
    impl BankxInfoxPageCfgWriteVal {
        #[doc = "Region enabled, following fields apply"]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 8)) | ((val & 0xf) << 8))
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 12)) | ((val & 0xf) << 12))
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 16)) | ((val & 0xf) << 16))
        }
        #[doc = "Region is ECC enabled (both integrity and reliability ECC)."]
        #[inline(always)]
        pub fn ecc_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 20)) | ((val & 0xf) << 20))
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 24)) | ((val & 0xf) << 24))
        }
    }
    impl From<u32> for BankxInfoxPageCfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankxInfoxPageCfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BankxInfoxPageCfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankCfgRegwenReadVal(u32);
    impl BankCfgRegwenReadVal {
        #[doc = "Bank register write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn bank(&self) -> super::enums::Bank {
            super::enums::Bank::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> BankCfgRegwenWriteVal {
            BankCfgRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for BankCfgRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankCfgRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: BankCfgRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct BankCfgRegwenWriteVal(u32);
    impl BankCfgRegwenWriteVal {
        #[doc = "Bank register write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn bank_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for BankCfgRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<BankCfgRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: BankCfgRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlReadVal(u32);
    impl ControlReadVal {
        #[doc = "Start flash transaction.  This bit shall only be set at the same time or after the other\nfields of the !!CONTROL register and !!ADDR have been programmed."]
        #[inline(always)]
        pub fn start(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Flash operation selection"]
        #[inline(always)]
        pub fn op(&self) -> super::enums::Op {
            super::enums::Op::try_from((self.0 >> 4) & 3).unwrap()
        }
        #[doc = "Flash program operation type selection"]
        #[inline(always)]
        pub fn prog_sel(&self) -> super::enums::ProgSel {
            super::enums::ProgSel::try_from((self.0 >> 6) & 1).unwrap()
        }
        #[doc = "Flash erase operation type selection"]
        #[inline(always)]
        pub fn erase_sel(&self) -> super::enums::EraseSel {
            super::enums::EraseSel::try_from((self.0 >> 7) & 1).unwrap()
        }
        #[doc = "When doing a read, program or page erase operation, selects either info or data partition for operation.\nWhen 0, select data partition - this is the portion of flash that is accessible both by the host and by the controller.\nWhen 1, select info partition - this is the portion of flash that is only accessible by the controller.\n\nWhen doing a bank erase operation, selects info partition also for erase.\nWhen 0, bank erase only erases data partition.\nWhen 1, bank erase erases data partition and info partition."]
        #[inline(always)]
        pub fn partition_sel(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Informational partions can have multiple types.\n\nThis field selects the info type to be accessed."]
        #[inline(always)]
        pub fn info_sel(&self) -> u32 {
            (self.0 >> 9) & 3
        }
        #[doc = "One fewer than the number of bus words the flash operation should read or program.\nFor example, to read 10 words, software should program this field with the value 9."]
        #[inline(always)]
        pub fn num(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ControlWriteVal {
            ControlWriteVal(self.0)
        }
    }
    impl From<u32> for ControlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlReadVal> for u32 {
        #[inline(always)]
        fn from(val: ControlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlWriteVal(u32);
    impl ControlWriteVal {
        #[doc = "Start flash transaction.  This bit shall only be set at the same time or after the other\nfields of the !!CONTROL register and !!ADDR have been programmed."]
        #[inline(always)]
        pub fn start(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Flash operation selection"]
        #[inline(always)]
        pub fn op(
            self,
            f: impl FnOnce(super::enums::selector::OpSelector) -> super::enums::Op,
        ) -> Self {
            Self((self.0 & !(3 << 4)) | (u32::from(f(super::enums::selector::OpSelector())) << 4))
        }
        #[doc = "Flash program operation type selection"]
        #[inline(always)]
        pub fn prog_sel(
            self,
            f: impl FnOnce(super::enums::selector::ProgSelSelector) -> super::enums::ProgSel,
        ) -> Self {
            Self(
                (self.0 & !(1 << 6))
                    | (u32::from(f(super::enums::selector::ProgSelSelector())) << 6),
            )
        }
        #[doc = "Flash erase operation type selection"]
        #[inline(always)]
        pub fn erase_sel(
            self,
            f: impl FnOnce(super::enums::selector::EraseSelSelector) -> super::enums::EraseSel,
        ) -> Self {
            Self(
                (self.0 & !(1 << 7))
                    | (u32::from(f(super::enums::selector::EraseSelSelector())) << 7),
            )
        }
        #[doc = "When doing a read, program or page erase operation, selects either info or data partition for operation.\nWhen 0, select data partition - this is the portion of flash that is accessible both by the host and by the controller.\nWhen 1, select info partition - this is the portion of flash that is only accessible by the controller.\n\nWhen doing a bank erase operation, selects info partition also for erase.\nWhen 0, bank erase only erases data partition.\nWhen 1, bank erase erases data partition and info partition."]
        #[inline(always)]
        pub fn partition_sel(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Informational partions can have multiple types.\n\nThis field selects the info type to be accessed."]
        #[inline(always)]
        pub fn info_sel(self, val: u32) -> Self {
            Self((self.0 & !(3 << 9)) | ((val & 3) << 9))
        }
        #[doc = "One fewer than the number of bus words the flash operation should read or program.\nFor example, to read 10 words, software should program this field with the value 9."]
        #[inline(always)]
        pub fn num(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
    }
    impl From<u32> for ControlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ControlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlRegwenReadVal(u32);
    impl CtrlRegwenReadVal {
        #[doc = "Configuration enable.\n\nThis bit defaults to 1 and is set to 0 by hardware when flash operation is initiated.\nWhen the controller completes the flash operation, this bit is set\nback to 1 to allow software configuration of !!CONTROL"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
    }
    impl From<u32> for CtrlRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CurrFifoLvlReadVal(u32);
    impl CurrFifoLvlReadVal {
        #[doc = "Current program fifo depth"]
        #[inline(always)]
        pub fn prog(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = "Current read fifo depth"]
        #[inline(always)]
        pub fn rd(&self) -> u32 {
            (self.0 >> 8) & 0x1f
        }
    }
    impl From<u32> for CurrFifoLvlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CurrFifoLvlReadVal> for u32 {
        #[inline(always)]
        fn from(val: CurrFifoLvlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DebugStateReadVal(u32);
    impl DebugStateReadVal {
        #[doc = "Current lcmgr interface staet "]
        #[inline(always)]
        pub fn lcmgr_state(&self) -> u32 {
            (self.0 >> 0) & 0x7ff
        }
    }
    impl From<u32> for DebugStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DebugStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: DebugStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DefaultRegionReadVal(u32);
    impl DefaultRegionReadVal {
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(&self) -> u32 {
            (self.0 >> 8) & 0xf
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(&self) -> u32 {
            (self.0 >> 12) & 0xf
        }
        #[doc = "Region is ECC enabled (both integrity and reliability ECC)."]
        #[inline(always)]
        pub fn ecc_en(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DefaultRegionWriteVal {
            DefaultRegionWriteVal(self.0)
        }
    }
    impl From<u32> for DefaultRegionReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DefaultRegionReadVal> for u32 {
        #[inline(always)]
        fn from(val: DefaultRegionReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DefaultRegionWriteVal(u32);
    impl DefaultRegionWriteVal {
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 8)) | ((val & 0xf) << 8))
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 12)) | ((val & 0xf) << 12))
        }
        #[doc = "Region is ECC enabled (both integrity and reliability ECC)."]
        #[inline(always)]
        pub fn ecc_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 16)) | ((val & 0xf) << 16))
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 20)) | ((val & 0xf) << 20))
        }
    }
    impl From<u32> for DefaultRegionWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DefaultRegionWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DefaultRegionWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DisReadVal(u32);
    impl DisReadVal {
        #[doc = "Disables flash functionality completely.\nThis is a shortcut mechanism used by the software to completely\nkill flash in case of emergency.\n\nSince this register is rw1s instead of rw, to disable, write the value kMuBi4True\nto the register to disable the flash."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DisWriteVal {
            DisWriteVal(self.0)
        }
    }
    impl From<u32> for DisReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DisReadVal> for u32 {
        #[inline(always)]
        fn from(val: DisReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DisWriteVal(u32);
    impl DisWriteVal {
        #[doc = "Disables flash functionality completely.\nThis is a shortcut mechanism used by the software to completely\nkill flash in case of emergency.\n\nSince this register is rw1s instead of rw, to disable, write the value kMuBi4True\nto the register to disable the flash."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for DisWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DisWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DisWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EccSingleErrAddrReadVal(u32);
    impl EccSingleErrAddrReadVal {
        #[doc = "Latest single error address for this bank"]
        #[inline(always)]
        pub fn ecc_single_err_addr(&self) -> u32 {
            (self.0 >> 0) & 0xfffff
        }
    }
    impl From<u32> for EccSingleErrAddrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EccSingleErrAddrReadVal> for u32 {
        #[inline(always)]
        fn from(val: EccSingleErrAddrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EccSingleErrCnt0ReadVal(u32);
    impl EccSingleErrCnt0ReadVal {
        #[doc = "This count will not wrap when saturated"]
        #[inline(always)]
        pub fn ecc_single_err_cnt0(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "This count will not wrap when saturated"]
        #[inline(always)]
        pub fn ecc_single_err_cnt1(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EccSingleErrCnt0WriteVal {
            EccSingleErrCnt0WriteVal(self.0)
        }
    }
    impl From<u32> for EccSingleErrCnt0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EccSingleErrCnt0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: EccSingleErrCnt0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EccSingleErrCnt0WriteVal(u32);
    impl EccSingleErrCnt0WriteVal {
        #[doc = "This count will not wrap when saturated"]
        #[inline(always)]
        pub fn ecc_single_err_cnt0(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "This count will not wrap when saturated"]
        #[inline(always)]
        pub fn ecc_single_err_cnt1(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 8)) | ((val & 0xff) << 8))
        }
    }
    impl From<u32> for EccSingleErrCnt0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EccSingleErrCnt0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: EccSingleErrCnt0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EraseSuspendReadVal(u32);
    impl EraseSuspendReadVal {
        #[doc = "When 1, request erase suspend.\nIf no erase ongoing, the request is immediately cleared by hardware\nIf erase ongoing, the request is fed to the flash_phy and cleared when the suspend is handled."]
        #[inline(always)]
        pub fn req(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EraseSuspendWriteVal {
            EraseSuspendWriteVal(self.0)
        }
    }
    impl From<u32> for EraseSuspendReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EraseSuspendReadVal> for u32 {
        #[inline(always)]
        fn from(val: EraseSuspendReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EraseSuspendWriteVal(u32);
    impl EraseSuspendWriteVal {
        #[doc = "When 1, request erase suspend.\nIf no erase ongoing, the request is immediately cleared by hardware\nIf erase ongoing, the request is fed to the flash_phy and cleared when the suspend is handled."]
        #[inline(always)]
        pub fn req(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for EraseSuspendWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EraseSuspendWriteVal> for u32 {
        #[inline(always)]
        fn from(val: EraseSuspendWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrAddrReadVal(u32);
    impl ErrAddrReadVal {
        #[inline(always)]
        pub fn err_addr(&self) -> u32 {
            (self.0 >> 0) & 0xfffff
        }
    }
    impl From<u32> for ErrAddrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrAddrReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrAddrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrCodeReadVal(u32);
    impl ErrCodeReadVal {
        #[doc = "Software has supplied an undefined operation.\nSee !!CONTROL.OP for list of valid operations."]
        #[inline(always)]
        pub fn op_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Flash access has encountered an access permission error.\nPlease see !!ERR_ADDR for exact address.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn mp_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Flash read has an error.\nThis could be a reliability ECC error or an storage integrity error\nencountered during a software issued controller read, see !!STD_FAULT_STATUS.\nSee !!ERR_ADDR for exact address.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn rd_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Flash program has an error.\nThis could be a program integrity error, see !!STD_FAULT_STATUS.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Flash program has a window resolution error.  Ie, the start of program\nand end of program are in different windows.  Please check !!ERR_ADDR.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_win_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Flash program selected unavailable type, see !!PROG_TYPE_EN.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_type_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "A shadow register encountered an update error.\nThis is an asynchronous error."]
        #[inline(always)]
        pub fn update_err(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "A recoverable error has been encountered in the flash macro.\nPlease read the flash macro status registers for more details."]
        #[inline(always)]
        pub fn macro_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrCodeWriteVal {
            ErrCodeWriteVal(self.0)
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
    pub struct ErrCodeWriteVal(u32);
    impl ErrCodeWriteVal {
        #[doc = "Software has supplied an undefined operation.\nSee !!CONTROL.OP for list of valid operations."]
        #[inline(always)]
        pub fn op_err_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Flash access has encountered an access permission error.\nPlease see !!ERR_ADDR for exact address.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn mp_err_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Flash read has an error.\nThis could be a reliability ECC error or an storage integrity error\nencountered during a software issued controller read, see !!STD_FAULT_STATUS.\nSee !!ERR_ADDR for exact address.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn rd_err_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Flash program has an error.\nThis could be a program integrity error, see !!STD_FAULT_STATUS.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_err_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "Flash program has a window resolution error.  Ie, the start of program\nand end of program are in different windows.  Please check !!ERR_ADDR.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_win_err_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "Flash program selected unavailable type, see !!PROG_TYPE_EN.\nThis is a synchronous error."]
        #[inline(always)]
        pub fn prog_type_err_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "A shadow register encountered an update error.\nThis is an asynchronous error."]
        #[inline(always)]
        pub fn update_err_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "A recoverable error has been encountered in the flash macro.\nPlease read the flash macro status registers for more details."]
        #[inline(always)]
        pub fn macro_err_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
    }
    impl From<u32> for ErrCodeWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrCodeWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrCodeWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FaultStatusReadVal(u32);
    impl FaultStatusReadVal {
        #[doc = "The flash life cycle management interface has supplied an undefined operation.\nSee !!CONTROL.OP for list of valid operations."]
        #[inline(always)]
        pub fn op_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The flash life cycle management interface encountered a memory permission error."]
        #[inline(always)]
        pub fn mp_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "The flash life cycle management interface encountered a read error.\nThis could be a reliability ECC error or an integrity ECC error\nencountered during a read, see !!STD_FAULT_STATUS for more details."]
        #[inline(always)]
        pub fn rd_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "The flash life cycle management interface encountered a program error.\nThis could be a program integirty eror, see !!STD_FAULT_STATUS for more details."]
        #[inline(always)]
        pub fn prog_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "The flash life cycle management interface encountered a program resolution error."]
        #[inline(always)]
        pub fn prog_win_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "The flash life cycle management interface encountered a program type error.\nA program type not supported by the flash macro was issued."]
        #[inline(always)]
        pub fn prog_type_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "The seed reading process encountered an unexpected error."]
        #[inline(always)]
        pub fn seed_err(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "The flash macro encountered a storage reliability ECC error.\n\nNote that this error bit can be cleared to allow firmware dealing with multi-bit ECC errors during firmware selection and verification.\nAfter passing this stage, it is recommended that firmware classifies the corresponding alert as fatal on the receiver end, i.e, inside the alert handler."]
        #[inline(always)]
        pub fn phy_relbl_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "The flash macro encountered a storage integrity ECC error.\n\nNote that this error bit can be cleared to allow firmware dealing with ICV errors during firmware selection and verification.\nAfter passing this stage, it is recommended that firmware classifies the corresponding alert as fatal on the receiver end, i.e, inside the alert handler."]
        #[inline(always)]
        pub fn phy_storage_err(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "The flash emitted an unexpected acknowledgement."]
        #[inline(always)]
        pub fn spurious_ack(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "The phy arbiter encountered inconsistent results."]
        #[inline(always)]
        pub fn arb_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "A host transaction was granted with illegal properties."]
        #[inline(always)]
        pub fn host_gnt_err(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FaultStatusWriteVal {
            FaultStatusWriteVal(self.0)
        }
    }
    impl From<u32> for FaultStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FaultStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: FaultStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FaultStatusWriteVal(u32);
    impl FaultStatusWriteVal {
        #[doc = "The flash macro encountered a storage reliability ECC error.\n\nNote that this error bit can be cleared to allow firmware dealing with multi-bit ECC errors during firmware selection and verification.\nAfter passing this stage, it is recommended that firmware classifies the corresponding alert as fatal on the receiver end, i.e, inside the alert handler."]
        #[inline(always)]
        pub fn phy_relbl_err_clear(self) -> Self {
            Self(self.0 & !(1 << 7))
        }
        #[doc = "The flash macro encountered a storage integrity ECC error.\n\nNote that this error bit can be cleared to allow firmware dealing with ICV errors during firmware selection and verification.\nAfter passing this stage, it is recommended that firmware classifies the corresponding alert as fatal on the receiver end, i.e, inside the alert handler."]
        #[inline(always)]
        pub fn phy_storage_err_clear(self) -> Self {
            Self(self.0 & !(1 << 8))
        }
    }
    impl From<u32> for FaultStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FaultStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FaultStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoLvlReadVal(u32);
    impl FifoLvlReadVal {
        #[doc = "When the program FIFO drains to this level, trigger an interrupt.\nDefault value is set such that interrupt does not trigger at reset."]
        #[inline(always)]
        pub fn prog(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = "When the read FIFO fills to this level, trigger an interrupt.\nDefault value is set such that interrupt does not trigger at reset."]
        #[inline(always)]
        pub fn rd(&self) -> u32 {
            (self.0 >> 8) & 0x1f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FifoLvlWriteVal {
            FifoLvlWriteVal(self.0)
        }
    }
    impl From<u32> for FifoLvlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoLvlReadVal> for u32 {
        #[inline(always)]
        fn from(val: FifoLvlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoLvlWriteVal(u32);
    impl FifoLvlWriteVal {
        #[doc = "When the program FIFO drains to this level, trigger an interrupt.\nDefault value is set such that interrupt does not trigger at reset."]
        #[inline(always)]
        pub fn prog(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 0)) | ((val & 0x1f) << 0))
        }
        #[doc = "When the read FIFO fills to this level, trigger an interrupt.\nDefault value is set such that interrupt does not trigger at reset."]
        #[inline(always)]
        pub fn rd(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 8)) | ((val & 0x1f) << 8))
        }
    }
    impl From<u32> for FifoLvlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoLvlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FifoLvlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoRstReadVal(u32);
    impl FifoRstReadVal {
        #[doc = "Active high resets for both program and read FIFOs.  This is especially useful after the controller\nencounters an error of some kind.\nThis bit will hold the FIFO in reset as long as it is set."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FifoRstWriteVal {
            FifoRstWriteVal(self.0)
        }
    }
    impl From<u32> for FifoRstReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoRstReadVal> for u32 {
        #[inline(always)]
        fn from(val: FifoRstReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoRstWriteVal(u32);
    impl FifoRstWriteVal {
        #[doc = "Active high resets for both program and read FIFOs.  This is especially useful after the controller\nencounters an error of some kind.\nThis bit will hold the FIFO in reset as long as it is set."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for FifoRstWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoRstWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FifoRstWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwInfoCfgOverrideReadVal(u32);
    impl HwInfoCfgOverrideReadVal {
        #[doc = "The hardwired hardware info configuration rules for scramble enable are logically AND'd with\nthis field.\nIf the hardware rules hardwires scramble to enable, we can disable via software if needed.\n\nBy default this field is false."]
        #[inline(always)]
        pub fn scramble_dis(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "The hardwired hardware info configuration rules for ECC enable are logically AND'd with\nthis field.\nIf the hardware rules hardwires ECC to enable, we can disable via software if needed.\n\nBy default this field is false."]
        #[inline(always)]
        pub fn ecc_dis(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> HwInfoCfgOverrideWriteVal {
            HwInfoCfgOverrideWriteVal(self.0)
        }
    }
    impl From<u32> for HwInfoCfgOverrideReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwInfoCfgOverrideReadVal> for u32 {
        #[inline(always)]
        fn from(val: HwInfoCfgOverrideReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwInfoCfgOverrideWriteVal(u32);
    impl HwInfoCfgOverrideWriteVal {
        #[doc = "The hardwired hardware info configuration rules for scramble enable are logically AND'd with\nthis field.\nIf the hardware rules hardwires scramble to enable, we can disable via software if needed.\n\nBy default this field is false."]
        #[inline(always)]
        pub fn scramble_dis(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "The hardwired hardware info configuration rules for ECC enable are logically AND'd with\nthis field.\nIf the hardware rules hardwires ECC to enable, we can disable via software if needed.\n\nBy default this field is false."]
        #[inline(always)]
        pub fn ecc_dis(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
    }
    impl From<u32> for HwInfoCfgOverrideWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwInfoCfgOverrideWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HwInfoCfgOverrideWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InitReadVal(u32);
    impl InitReadVal {
        #[doc = "Initializes the flash controller.\n\nDuring the initialization process, the flash controller requests the address and data\nscramble keys and reads out the root seeds stored in flash before allowing other usage\nof the flash controller.\n\nWhen the initialization sequence is complete, the flash read buffers are enabled\nand turned on."]
        #[inline(always)]
        pub fn val(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InitWriteVal {
            InitWriteVal(self.0)
        }
    }
    impl From<u32> for InitReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InitReadVal> for u32 {
        #[inline(always)]
        fn from(val: InitReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InitWriteVal(u32);
    impl InitWriteVal {
        #[doc = "Initializes the flash controller.\n\nDuring the initialization process, the flash controller requests the address and data\nscramble keys and reads out the root seeds stored in flash before allowing other usage\nof the flash controller.\n\nWhen the initialization sequence is complete, the flash read buffers are enabled\nand turned on."]
        #[inline(always)]
        pub fn val_set(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for InitWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InitWriteVal> for u32 {
        #[inline(always)]
        fn from(val: InitWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.prog_empty is set."]
        #[inline(always)]
        pub fn prog_empty(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.prog_lvl is set."]
        #[inline(always)]
        pub fn prog_lvl(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rd_full is set."]
        #[inline(always)]
        pub fn rd_full(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rd_lvl is set."]
        #[inline(always)]
        pub fn rd_lvl(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.op_done is set."]
        #[inline(always)]
        pub fn op_done(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.corr_err is set."]
        #[inline(always)]
        pub fn corr_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.prog_empty is set."]
        #[inline(always)]
        pub fn prog_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.prog_lvl is set."]
        #[inline(always)]
        pub fn prog_lvl(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rd_full is set."]
        #[inline(always)]
        pub fn rd_full(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rd_lvl is set."]
        #[inline(always)]
        pub fn rd_lvl(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.op_done is set."]
        #[inline(always)]
        pub fn op_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.corr_err is set."]
        #[inline(always)]
        pub fn corr_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
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
        #[doc = "Program FIFO empty"]
        #[inline(always)]
        pub fn prog_empty(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Program FIFO drained to level"]
        #[inline(always)]
        pub fn prog_lvl(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Read FIFO full"]
        #[inline(always)]
        pub fn rd_full(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Read FIFO filled to level"]
        #[inline(always)]
        pub fn rd_lvl(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Operation complete"]
        #[inline(always)]
        pub fn op_done(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Correctable error encountered"]
        #[inline(always)]
        pub fn corr_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
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
        #[doc = "Operation complete"]
        #[inline(always)]
        pub fn op_done_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "Correctable error encountered"]
        #[inline(always)]
        pub fn corr_err_clear(self) -> Self {
            Self(self.0 | (1 << 5))
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
        #[doc = "Write 1 to force !!INTR_STATE.prog_empty to 1."]
        #[inline(always)]
        pub fn prog_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.prog_lvl to 1."]
        #[inline(always)]
        pub fn prog_lvl(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rd_full to 1."]
        #[inline(always)]
        pub fn rd_full(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rd_lvl to 1."]
        #[inline(always)]
        pub fn rd_lvl(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to force !!INTR_STATE.op_done to 1."]
        #[inline(always)]
        pub fn op_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to force !!INTR_STATE.corr_err to 1."]
        #[inline(always)]
        pub fn corr_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
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
    pub struct MpBankCfgShadowed0ReadVal(u32);
    impl MpBankCfgShadowed0ReadVal {
        #[doc = "Bank wide erase enable"]
        #[inline(always)]
        pub fn erase_en0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Bank wide erase enable"]
        #[inline(always)]
        pub fn erase_en1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MpBankCfgShadowed0WriteVal {
            MpBankCfgShadowed0WriteVal(self.0)
        }
    }
    impl From<u32> for MpBankCfgShadowed0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpBankCfgShadowed0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: MpBankCfgShadowed0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MpBankCfgShadowed0WriteVal(u32);
    impl MpBankCfgShadowed0WriteVal {
        #[doc = "Bank wide erase enable"]
        #[inline(always)]
        pub fn erase_en0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Bank wide erase enable"]
        #[inline(always)]
        pub fn erase_en1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for MpBankCfgShadowed0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpBankCfgShadowed0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: MpBankCfgShadowed0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MpRegionReadVal(u32);
    impl MpRegionReadVal {
        #[doc = "Region base page. Note the granularity is page, not byte or word"]
        #[inline(always)]
        pub fn base(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = "Region size in number of pages.\nFor example, if base is 0 and size is 1, then the region is defined by page 0.\nIf base is 0 and size is 2, then the region is defined by pages 0 and 1."]
        #[inline(always)]
        pub fn size(&self) -> u32 {
            (self.0 >> 9) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MpRegionWriteVal {
            MpRegionWriteVal(self.0)
        }
    }
    impl From<u32> for MpRegionReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpRegionReadVal> for u32 {
        #[inline(always)]
        fn from(val: MpRegionReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MpRegionWriteVal(u32);
    impl MpRegionWriteVal {
        #[doc = "Region base page. Note the granularity is page, not byte or word"]
        #[inline(always)]
        pub fn base(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "Region size in number of pages.\nFor example, if base is 0 and size is 1, then the region is defined by page 0.\nIf base is 0 and size is 2, then the region is defined by pages 0 and 1."]
        #[inline(always)]
        pub fn size(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 9)) | ((val & 0x3ff) << 9))
        }
    }
    impl From<u32> for MpRegionWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpRegionWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MpRegionWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MpRegionCfgReadVal(u32);
    impl MpRegionCfgReadVal {
        #[doc = "Region enabled, following fields apply.\nIf region is disabled, it is not matched against any incoming transaction."]
        #[inline(always)]
        pub fn en(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(&self) -> u32 {
            (self.0 >> 8) & 0xf
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(&self) -> u32 {
            (self.0 >> 12) & 0xf
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "Region is integrity checked and reliability ECC enabled."]
        #[inline(always)]
        pub fn ecc_en(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(&self) -> u32 {
            (self.0 >> 24) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MpRegionCfgWriteVal {
            MpRegionCfgWriteVal(self.0)
        }
    }
    impl From<u32> for MpRegionCfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpRegionCfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: MpRegionCfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MpRegionCfgWriteVal(u32);
    impl MpRegionCfgWriteVal {
        #[doc = "Region enabled, following fields apply.\nIf region is disabled, it is not matched against any incoming transaction."]
        #[inline(always)]
        pub fn en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "Region can be read"]
        #[inline(always)]
        pub fn rd_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "Region can be programmed"]
        #[inline(always)]
        pub fn prog_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 8)) | ((val & 0xf) << 8))
        }
        #[doc = "Region can be erased"]
        #[inline(always)]
        pub fn erase_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 12)) | ((val & 0xf) << 12))
        }
        #[doc = "Region is scramble enabled."]
        #[inline(always)]
        pub fn scramble_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 16)) | ((val & 0xf) << 16))
        }
        #[doc = "Region is integrity checked and reliability ECC enabled."]
        #[inline(always)]
        pub fn ecc_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 20)) | ((val & 0xf) << 20))
        }
        #[doc = "Region is high endurance enabled."]
        #[inline(always)]
        pub fn he_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 24)) | ((val & 0xf) << 24))
        }
    }
    impl From<u32> for MpRegionCfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MpRegionCfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MpRegionCfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OpStatusReadVal(u32);
    impl OpStatusReadVal {
        #[doc = "Flash operation done. Set by HW, cleared by SW"]
        #[inline(always)]
        pub fn done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Flash operation error. Set by HW, cleared by SW. See !!ERR_CODE for more details."]
        #[inline(always)]
        pub fn err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OpStatusWriteVal {
            OpStatusWriteVal(self.0)
        }
    }
    impl From<u32> for OpStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OpStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: OpStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OpStatusWriteVal(u32);
    impl OpStatusWriteVal {
        #[doc = "Flash operation done. Set by HW, cleared by SW"]
        #[inline(always)]
        pub fn done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Flash operation error. Set by HW, cleared by SW. See !!ERR_CODE for more details."]
        #[inline(always)]
        pub fn err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for OpStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OpStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: OpStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyAlertCfgReadVal(u32);
    impl PhyAlertCfgReadVal {
        #[doc = "Acknowledge flash phy alert"]
        #[inline(always)]
        pub fn alert_ack(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Trigger flash phy alert"]
        #[inline(always)]
        pub fn alert_trig(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PhyAlertCfgWriteVal {
            PhyAlertCfgWriteVal(self.0)
        }
    }
    impl From<u32> for PhyAlertCfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyAlertCfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: PhyAlertCfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyAlertCfgWriteVal(u32);
    impl PhyAlertCfgWriteVal {
        #[doc = "Acknowledge flash phy alert"]
        #[inline(always)]
        pub fn alert_ack(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Trigger flash phy alert"]
        #[inline(always)]
        pub fn alert_trig(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for PhyAlertCfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyAlertCfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PhyAlertCfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyStatusReadVal(u32);
    impl PhyStatusReadVal {
        #[doc = "Flash phy controller initializing"]
        #[inline(always)]
        pub fn init_wip(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Normal program supported"]
        #[inline(always)]
        pub fn prog_normal_avail(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Program repair supported"]
        #[inline(always)]
        pub fn prog_repair_avail(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
    }
    impl From<u32> for PhyStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: PhyStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ProgTypeEnReadVal(u32);
    impl ProgTypeEnReadVal {
        #[doc = "Normal prog type available"]
        #[inline(always)]
        pub fn normal(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Repair prog type available"]
        #[inline(always)]
        pub fn repair(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ProgTypeEnWriteVal {
            ProgTypeEnWriteVal(self.0)
        }
    }
    impl From<u32> for ProgTypeEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ProgTypeEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: ProgTypeEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ProgTypeEnWriteVal(u32);
    impl ProgTypeEnWriteVal {
        #[doc = "Normal prog type available"]
        #[inline(always)]
        pub fn normal_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "Repair prog type available"]
        #[inline(always)]
        pub fn repair_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
    }
    impl From<u32> for ProgTypeEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ProgTypeEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ProgTypeEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegionCfgRegwenReadVal(u32);
    impl RegionCfgRegwenReadVal {
        #[doc = "Region register write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region(&self) -> super::enums::Cfgregionwen {
            super::enums::Cfgregionwen::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RegionCfgRegwenWriteVal {
            RegionCfgRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for RegionCfgRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RegionCfgRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: RegionCfgRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegionCfgRegwenWriteVal(u32);
    impl RegionCfgRegwenWriteVal {
        #[doc = "Region register write enable.  Once set to 0, it can longer be configured to 1"]
        #[inline(always)]
        pub fn region_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for RegionCfgRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RegionCfgRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RegionCfgRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "Flash read FIFO full, software must consume data"]
        #[inline(always)]
        pub fn rd_full(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Flash read FIFO empty"]
        #[inline(always)]
        pub fn rd_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Flash program FIFO full"]
        #[inline(always)]
        pub fn prog_full(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Flash program FIFO empty, software must provide data"]
        #[inline(always)]
        pub fn prog_empty(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Flash controller undergoing init, inclusive of phy init"]
        #[inline(always)]
        pub fn init_wip(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Flash controller initialized"]
        #[inline(always)]
        pub fn initialized(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
    }
    impl From<u32> for StatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: StatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StdFaultStatusReadVal(u32);
    impl StdFaultStatusReadVal {
        #[doc = "The flash controller encountered a register integrity error."]
        #[inline(always)]
        pub fn reg_intg_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The flash controller encountered a program data transmission integrity error."]
        #[inline(always)]
        pub fn prog_intg_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "The life cycle management interface has encountered a fatal error.\nThe error is either an FSM sparse encoding error or a count error."]
        #[inline(always)]
        pub fn lcmgr_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "The life cycle management interface has encountered a transmission\nintegrity error.  This is an integrity error on the generated integrity\nduring a life cycle management interface read."]
        #[inline(always)]
        pub fn lcmgr_intg_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "The arbiter fsm has encountered a sparse encoding error."]
        #[inline(always)]
        pub fn arb_fsm_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "A shadow register encountered a storage error."]
        #[inline(always)]
        pub fn storage_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "A flash phy fsm has encountered a sparse encoding error."]
        #[inline(always)]
        pub fn phy_fsm_err(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Flash ctrl read/prog has encountered a count error."]
        #[inline(always)]
        pub fn ctrl_cnt_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Flash primitive fifo's have encountered a count error."]
        #[inline(always)]
        pub fn fifo_err(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
    }
    impl From<u32> for StdFaultStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StdFaultStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: StdFaultStatusReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Bank {
        BankLocked = 0,
        BankEnabled = 1,
    }
    impl Bank {
        #[inline(always)]
        pub fn bank_locked(&self) -> bool {
            *self == Self::BankLocked
        }
        #[inline(always)]
        pub fn bank_enabled(&self) -> bool {
            *self == Self::BankEnabled
        }
    }
    impl TryFrom<u32> for Bank {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Bank, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, Bank>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Bank> for u32 {
        fn from(val: Bank) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Cfgregionwen {
        RegionLocked = 0,
        RegionEnabled = 1,
    }
    impl Cfgregionwen {
        #[inline(always)]
        pub fn region_locked(&self) -> bool {
            *self == Self::RegionLocked
        }
        #[inline(always)]
        pub fn region_enabled(&self) -> bool {
            *self == Self::RegionEnabled
        }
    }
    impl TryFrom<u32> for Cfgregionwen {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Cfgregionwen, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, Cfgregionwen>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Cfgregionwen> for u32 {
        fn from(val: Cfgregionwen) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum EraseSel {
        PageErase = 0,
        BankErase = 1,
    }
    impl EraseSel {
        #[inline(always)]
        pub fn page_erase(&self) -> bool {
            *self == Self::PageErase
        }
        #[inline(always)]
        pub fn bank_erase(&self) -> bool {
            *self == Self::BankErase
        }
    }
    impl TryFrom<u32> for EraseSel {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<EraseSel, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, EraseSel>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<EraseSel> for u32 {
        fn from(val: EraseSel) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Inforegionwen {
        PageLocked = 0,
        PageEnabled = 1,
    }
    impl Inforegionwen {
        #[inline(always)]
        pub fn page_locked(&self) -> bool {
            *self == Self::PageLocked
        }
        #[inline(always)]
        pub fn page_enabled(&self) -> bool {
            *self == Self::PageEnabled
        }
    }
    impl TryFrom<u32> for Inforegionwen {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Inforegionwen, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, Inforegionwen>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Inforegionwen> for u32 {
        fn from(val: Inforegionwen) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Op {
        Read = 0,
        Prog = 1,
        Erase = 2,
        Reserved3 = 3,
    }
    impl Op {
        #[inline(always)]
        pub fn read(&self) -> bool {
            *self == Self::Read
        }
        #[inline(always)]
        pub fn prog(&self) -> bool {
            *self == Self::Prog
        }
        #[inline(always)]
        pub fn erase(&self) -> bool {
            *self == Self::Erase
        }
    }
    impl TryFrom<u32> for Op {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Op, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Op>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Op> for u32 {
        fn from(val: Op) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum ProgSel {
        NormalProgram = 0,
        ProgramRepair = 1,
    }
    impl ProgSel {
        #[inline(always)]
        pub fn normal_program(&self) -> bool {
            *self == Self::NormalProgram
        }
        #[inline(always)]
        pub fn program_repair(&self) -> bool {
            *self == Self::ProgramRepair
        }
    }
    impl TryFrom<u32> for ProgSel {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<ProgSel, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, ProgSel>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<ProgSel> for u32 {
        fn from(val: ProgSel) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct BankSelector();
        impl BankSelector {
            #[inline(always)]
            pub fn bank_locked(&self) -> super::Bank {
                super::Bank::BankLocked
            }
            #[inline(always)]
            pub fn bank_enabled(&self) -> super::Bank {
                super::Bank::BankEnabled
            }
        }
        pub struct CfgregionwenSelector();
        impl CfgregionwenSelector {
            #[inline(always)]
            pub fn region_locked(&self) -> super::Cfgregionwen {
                super::Cfgregionwen::RegionLocked
            }
            #[inline(always)]
            pub fn region_enabled(&self) -> super::Cfgregionwen {
                super::Cfgregionwen::RegionEnabled
            }
        }
        pub struct EraseSelSelector();
        impl EraseSelSelector {
            #[inline(always)]
            pub fn page_erase(&self) -> super::EraseSel {
                super::EraseSel::PageErase
            }
            #[inline(always)]
            pub fn bank_erase(&self) -> super::EraseSel {
                super::EraseSel::BankErase
            }
        }
        pub struct InforegionwenSelector();
        impl InforegionwenSelector {
            #[inline(always)]
            pub fn page_locked(&self) -> super::Inforegionwen {
                super::Inforegionwen::PageLocked
            }
            #[inline(always)]
            pub fn page_enabled(&self) -> super::Inforegionwen {
                super::Inforegionwen::PageEnabled
            }
        }
        pub struct OpSelector();
        impl OpSelector {
            #[inline(always)]
            pub fn read(&self) -> super::Op {
                super::Op::Read
            }
            #[inline(always)]
            pub fn prog(&self) -> super::Op {
                super::Op::Prog
            }
            #[inline(always)]
            pub fn erase(&self) -> super::Op {
                super::Op::Erase
            }
        }
        pub struct ProgSelSelector();
        impl ProgSelSelector {
            #[inline(always)]
            pub fn normal_program(&self) -> super::ProgSel {
                super::ProgSel::NormalProgram
            }
            #[inline(always)]
            pub fn program_repair(&self) -> super::ProgSel {
                super::ProgSel::ProgramRepair
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<3, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Dis = ureg::ReadWriteReg32<9, crate::regs::DisReadVal, crate::regs::DisWriteVal>;
    pub type Exec = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Init = ureg::ReadWriteReg32<0, crate::regs::InitReadVal, crate::regs::InitWriteVal>;
    pub type CtrlRegwen = ureg::ReadOnlyReg32<crate::regs::CtrlRegwenReadVal>;
    pub type Control =
        ureg::ReadWriteReg32<0, crate::regs::ControlReadVal, crate::regs::ControlWriteVal>;
    pub type Addr = ureg::ReadWriteReg32<0, crate::regs::AddrReadVal, crate::regs::AddrWriteVal>;
    pub type ProgTypeEn =
        ureg::ReadWriteReg32<3, crate::regs::ProgTypeEnReadVal, crate::regs::ProgTypeEnWriteVal>;
    pub type EraseSuspend = ureg::ReadWriteReg32<
        0,
        crate::regs::EraseSuspendReadVal,
        crate::regs::EraseSuspendWriteVal,
    >;
    pub type RegionCfgRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::RegionCfgRegwenReadVal,
        crate::regs::RegionCfgRegwenWriteVal,
    >;
    pub type MpRegionCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::MpRegionCfgReadVal,
        crate::regs::MpRegionCfgWriteVal,
    >;
    pub type MpRegion =
        ureg::ReadWriteReg32<0, crate::regs::MpRegionReadVal, crate::regs::MpRegionWriteVal>;
    pub type DefaultRegion = ureg::ReadWriteReg32<
        0x999999,
        crate::regs::DefaultRegionReadVal,
        crate::regs::DefaultRegionWriteVal,
    >;
    pub type Bank0Info0Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo0RegwenReadVal,
        crate::regs::BankxInfo0RegwenWriteVal,
    >;
    pub type Bank0Info0PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type Bank0Info1Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo1RegwenReadVal,
        crate::regs::BankxInfo1RegwenWriteVal,
    >;
    pub type Bank0Info1PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type Bank0Info2Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo2RegwenReadVal,
        crate::regs::BankxInfo2RegwenWriteVal,
    >;
    pub type Bank0Info2PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type Bank1Info0Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo0RegwenReadVal,
        crate::regs::BankxInfo0RegwenWriteVal,
    >;
    pub type Bank1Info0PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type Bank1Info1Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo1RegwenReadVal,
        crate::regs::BankxInfo1RegwenWriteVal,
    >;
    pub type Bank1Info1PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type Bank1Info2Regwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankxInfo2RegwenReadVal,
        crate::regs::BankxInfo2RegwenWriteVal,
    >;
    pub type Bank1Info2PageCfg = ureg::ReadWriteReg32<
        0x9999999,
        crate::regs::BankxInfoxPageCfgReadVal,
        crate::regs::BankxInfoxPageCfgWriteVal,
    >;
    pub type HwInfoCfgOverride = ureg::ReadWriteReg32<
        0x99,
        crate::regs::HwInfoCfgOverrideReadVal,
        crate::regs::HwInfoCfgOverrideWriteVal,
    >;
    pub type BankCfgRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::BankCfgRegwenReadVal,
        crate::regs::BankCfgRegwenWriteVal,
    >;
    pub type MpBankCfgShadowed0 = ureg::ReadWriteReg32<
        0,
        crate::regs::MpBankCfgShadowed0ReadVal,
        crate::regs::MpBankCfgShadowed0WriteVal,
    >;
    pub type OpStatus =
        ureg::ReadWriteReg32<0, crate::regs::OpStatusReadVal, crate::regs::OpStatusWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type DebugState = ureg::ReadOnlyReg32<crate::regs::DebugStateReadVal>;
    pub type ErrCode =
        ureg::ReadWriteReg32<0, crate::regs::ErrCodeReadVal, crate::regs::ErrCodeWriteVal>;
    pub type StdFaultStatus = ureg::ReadOnlyReg32<crate::regs::StdFaultStatusReadVal>;
    pub type FaultStatus =
        ureg::ReadWriteReg32<0, crate::regs::FaultStatusReadVal, crate::regs::FaultStatusWriteVal>;
    pub type ErrAddr = ureg::ReadOnlyReg32<crate::regs::ErrAddrReadVal>;
    pub type EccSingleErrCnt0 = ureg::ReadWriteReg32<
        0,
        crate::regs::EccSingleErrCnt0ReadVal,
        crate::regs::EccSingleErrCnt0WriteVal,
    >;
    pub type EccSingleErrAddr = ureg::ReadOnlyReg32<crate::regs::EccSingleErrAddrReadVal>;
    pub type PhyAlertCfg =
        ureg::ReadWriteReg32<0, crate::regs::PhyAlertCfgReadVal, crate::regs::PhyAlertCfgWriteVal>;
    pub type PhyStatus = ureg::ReadOnlyReg32<crate::regs::PhyStatusReadVal>;
    pub type Scratch = ureg::ReadWriteReg32<0, u32, u32>;
    pub type FifoLvl =
        ureg::ReadWriteReg32<0xf0f, crate::regs::FifoLvlReadVal, crate::regs::FifoLvlWriteVal>;
    pub type FifoRst =
        ureg::ReadWriteReg32<0, crate::regs::FifoRstReadVal, crate::regs::FifoRstWriteVal>;
    pub type CurrFifoLvl = ureg::ReadOnlyReg32<crate::regs::CurrFifoLvlReadVal>;
    pub type ProgFifo = ureg::WriteOnlyReg32<0, u32>;
    pub type RdFifo = ureg::ReadOnlyReg32<u32>;
}
