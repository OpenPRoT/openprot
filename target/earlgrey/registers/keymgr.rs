#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Keymgr {
    _priv: (),
}
impl Keymgr {
    pub const PTR: *mut u32 = 0x41140000 as *mut u32;
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
    #[doc = "Key manager configuration enable\n\nRead value: [`regs::CfgRegwenReadVal`]; Write value: [`regs::CfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn cfg_regwen(&self) -> ureg::RegRef<crate::meta::CfgRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager operation start\n\nRead value: [`regs::StartReadVal`]; Write value: [`regs::StartWriteVal`]"]
    #[inline(always)]
    pub fn start(&self) -> ureg::RegRef<crate::meta::Start, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager operation controls\n\nRead value: [`regs::ControlShadowedReadVal`]; Write value: [`regs::ControlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn control_shadowed(&self) -> ureg::RegRef<crate::meta::ControlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "sideload key slots clear\n\nRead value: [`regs::SideloadClearReadVal`]; Write value: [`regs::SideloadClearWriteVal`]"]
    #[inline(always)]
    pub fn sideload_clear(&self) -> ureg::RegRef<crate::meta::SideloadClear, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "regwen for reseed interval\n\nRead value: [`regs::ReseedIntervalRegwenReadVal`]; Write value: [`regs::ReseedIntervalRegwenWriteVal`]"]
    #[inline(always)]
    pub fn reseed_interval_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::ReseedIntervalRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Reseed interval for key manager entropy reseed\n\nRead value: [`regs::ReseedIntervalShadowedReadVal`]; Write value: [`regs::ReseedIntervalShadowedWriteVal`]"]
    #[inline(always)]
    pub fn reseed_interval_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::ReseedIntervalShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for SOFTWARE_BINDING\n\nRead value: [`regs::SwBindingRegwenReadVal`]; Write value: [`regs::SwBindingRegwenWriteVal`]"]
    #[inline(always)]
    pub fn sw_binding_regwen(&self) -> ureg::RegRef<crate::meta::SwBindingRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Software binding input to sealing portion of the key manager.\nThis register is lockable and shared between key manager stages.\nThis binding value is not considered secret, however its integrity is very important.\n\nThe software binding is locked by software and unlocked by hardware upon a successful advance operation.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn sealing_sw_binding(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::SealingSwBinding, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Software binding input to the attestation portion of the key manager.\nThis register is lockable and shared between key manager stages.\nThis binding value is not considered secret, however its integrity is very important.\n\nThe software binding is locked by software and unlocked by hardware upon a successful advance operation.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn attest_sw_binding(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::AttestSwBinding, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Salt value used as part of output generation\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn salt(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::Salt, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Version used as part of output generation\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key_version(&self) -> ureg::RegRef<crate::meta::KeyVersion, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MAX_CREATOR_KEY_VERSION\n\nRead value: [`regs::MaxCreatorKeyVerRegwenReadVal`]; Write value: [`regs::MaxCreatorKeyVerRegwenWriteVal`]"]
    #[inline(always)]
    pub fn max_creator_key_ver_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxCreatorKeyVerRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Max creator key version\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn max_creator_key_ver_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxCreatorKeyVerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MAX_OWNER_INT_KEY_VERSION\n\nRead value: [`regs::MaxOwnerIntKeyVerRegwenReadVal`]; Write value: [`regs::MaxOwnerIntKeyVerRegwenWriteVal`]"]
    #[inline(always)]
    pub fn max_owner_int_key_ver_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxOwnerIntKeyVerRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x98 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Max owner intermediate key version\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn max_owner_int_key_ver_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxOwnerIntKeyVerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x9c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for MAX_OWNER_KEY_VERSION\n\nRead value: [`regs::MaxOwnerKeyVerRegwenReadVal`]; Write value: [`regs::MaxOwnerKeyVerRegwenWriteVal`]"]
    #[inline(always)]
    pub fn max_owner_key_ver_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxOwnerKeyVerRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Max owner key version\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn max_owner_key_ver_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxOwnerKeyVerShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager software output.\n\nWhen a software output operation is selected, the results of the operation are placed\nhere.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn sw_share0_output(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::SwShare0Output, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xa8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager software output.\n\nWhen a software output operation is selected, the results of the operation are placed\nhere.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn sw_share1_output(
        &self,
    ) -> ureg::Array<8, ureg::RegRef<crate::meta::SwShare1Output, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xc8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager working state.\n\nThis is a readout of the current key manager working state\n\nRead value: [`regs::WorkingStateReadVal`]; Write value: [`regs::WorkingStateWriteVal`]"]
    #[inline(always)]
    pub fn working_state(&self) -> ureg::RegRef<crate::meta::WorkingState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager status.\n\nHardware sets the status based on software initiated operations.\nThis register must be explicitly cleared by software.\nSoftware clears by writing back whatever it reads.\n\nRead value: [`regs::OpStatusReadVal`]; Write value: [`regs::OpStatusWriteVal`]"]
    #[inline(always)]
    pub fn op_status(&self) -> ureg::RegRef<crate::meta::OpStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xec / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Key manager error code.\nThis register must be explicitly cleared by software.\n\nThis register represents both synchronous and asynchronous recoverable\nerrors.\n\nSynchronous errors refer to those that only happen when a keymgr operation is\ninvoked, while asynchronous refers to errors that can happen at any time.\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register represents both synchronous and asynchronous fatal faults.\n\nSynchronous faults refer to those that only happen when a keymgr operation is\ninvoked, while asynchronous refers to faults that can happen at any time.\n\n\nRead value: [`regs::FaultStatusReadVal`]; Write value: [`regs::FaultStatusWriteVal`]"]
    #[inline(always)]
    pub fn fault_status(&self) -> ureg::RegRef<crate::meta::FaultStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "The register holds some debug information that may be convenient if keymgr\nmisbehaves.\n\nRead value: [`regs::DebugReadVal`]; Write value: [`regs::DebugWriteVal`]"]
    #[inline(always)]
    pub fn debug(&self) -> ureg::RegRef<crate::meta::Debug, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf8 / core::mem::size_of::<u32>()),
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
        pub fn recov_operation_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_fault_err(self, val: bool) -> Self {
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
    pub struct CfgRegwenReadVal(u32);
    impl CfgRegwenReadVal {
        #[doc = "key manager configuration enable.\nWhen key manager operation is started (see CONTROL), registers protected by this EN are no longer\nmodifiable until the operation completes."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
    }
    impl From<u32> for CfgRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlShadowedReadVal(u32);
    impl ControlShadowedReadVal {
        #[doc = "Key manager operation selection. All values not enumerated below behave the same as disable"]
        #[inline(always)]
        pub fn operation(&self) -> super::enums::Operation {
            super::enums::Operation::try_from((self.0 >> 4) & 7).unwrap()
        }
        #[doc = "When the OPERATION field is programmed to generate output, this field selects\nthe appropriate CDI to use.\n\nThis field should be programmed for both hw / sw generation."]
        #[inline(always)]
        pub fn cdi_sel(&self) -> super::enums::CdiSel {
            super::enums::CdiSel::try_from((self.0 >> 7) & 1).unwrap()
        }
        #[doc = "When the OPERATION field is programmed to generate output, this field selects\nthe appropriate crypto cipher target.\n\nThis field should be programmed for both hw / sw generation, as this helps diverisifies the output."]
        #[inline(always)]
        pub fn dest_sel(&self) -> super::enums::DestSel {
            super::enums::DestSel::try_from((self.0 >> 12) & 3).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ControlShadowedWriteVal {
            ControlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ControlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ControlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlShadowedWriteVal(u32);
    impl ControlShadowedWriteVal {
        #[doc = "Key manager operation selection. All values not enumerated below behave the same as disable"]
        #[inline(always)]
        pub fn operation(
            self,
            f: impl FnOnce(super::enums::selector::OperationSelector) -> super::enums::Operation,
        ) -> Self {
            Self(
                (self.0 & !(7 << 4))
                    | (u32::from(f(super::enums::selector::OperationSelector())) << 4),
            )
        }
        #[doc = "When the OPERATION field is programmed to generate output, this field selects\nthe appropriate CDI to use.\n\nThis field should be programmed for both hw / sw generation."]
        #[inline(always)]
        pub fn cdi_sel(
            self,
            f: impl FnOnce(super::enums::selector::CdiSelSelector) -> super::enums::CdiSel,
        ) -> Self {
            Self(
                (self.0 & !(1 << 7))
                    | (u32::from(f(super::enums::selector::CdiSelSelector())) << 7),
            )
        }
        #[doc = "When the OPERATION field is programmed to generate output, this field selects\nthe appropriate crypto cipher target.\n\nThis field should be programmed for both hw / sw generation, as this helps diverisifies the output."]
        #[inline(always)]
        pub fn dest_sel(
            self,
            f: impl FnOnce(super::enums::selector::DestSelSelector) -> super::enums::DestSel,
        ) -> Self {
            Self(
                (self.0 & !(3 << 12))
                    | (u32::from(f(super::enums::selector::DestSelSelector())) << 12),
            )
        }
    }
    impl From<u32> for ControlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ControlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DebugReadVal(u32);
    impl DebugReadVal {
        #[doc = "Creator seed failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_creator_seed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Owner seed failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_owner_seed(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Device ID failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_dev_id(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Health state failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_health_state(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Key version failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_key_version(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Key fed to kmac failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_key(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "ROM digest failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_digest(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DebugWriteVal {
            DebugWriteVal(self.0)
        }
    }
    impl From<u32> for DebugReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DebugReadVal> for u32 {
        #[inline(always)]
        fn from(val: DebugReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DebugWriteVal(u32);
    impl DebugWriteVal {
        #[doc = "Creator seed failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_creator_seed_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "Owner seed failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_owner_seed_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "Device ID failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_dev_id_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "Health state failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_health_state_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "Key version failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_key_version_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "Key fed to kmac failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_key_clear(self) -> Self {
            Self(self.0 & !(1 << 5))
        }
        #[doc = "ROM digest failed input checks during operation"]
        #[inline(always)]
        pub fn invalid_digest_clear(self) -> Self {
            Self(self.0 & !(1 << 6))
        }
    }
    impl From<u32> for DebugWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DebugWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DebugWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrCodeReadVal(u32);
    impl ErrCodeReadVal {
        #[doc = "Invalid operation issued to key manager, synchronous error"]
        #[inline(always)]
        pub fn invalid_op(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Invalid data issued to kmac interface, synchronous error"]
        #[inline(always)]
        pub fn invalid_kmac_input(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "An error observed during shadow register updates, asynchronous error"]
        #[inline(always)]
        pub fn invalid_shadow_update(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
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
        #[doc = "Invalid operation issued to key manager, synchronous error"]
        #[inline(always)]
        pub fn invalid_op_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Invalid data issued to kmac interface, synchronous error"]
        #[inline(always)]
        pub fn invalid_kmac_input_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "An error observed during shadow register updates, asynchronous error"]
        #[inline(always)]
        pub fn invalid_shadow_update_clear(self) -> Self {
            Self(self.0 | (1 << 2))
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
        #[doc = "A non-onehot command was seen in kmac, asynchronous fault."]
        #[inline(always)]
        pub fn cmd(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The kmac transfer interface FSM is in an invalid state, asynchronous fault."]
        #[inline(always)]
        pub fn kmac_fsm(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "The kmac transfer interface encountered an unexpected done, asynchronous fault."]
        #[inline(always)]
        pub fn kmac_done(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "KMAC reported an error during keymgr usage, this should never happen - synchronous fault."]
        #[inline(always)]
        pub fn kmac_op(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "KMAC data returned as all 0's or all 1's - synchronous fault"]
        #[inline(always)]
        pub fn kmac_out(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Register file integrity error, asynchronous fault"]
        #[inline(always)]
        pub fn regfile_intg(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Shadow copy storage error, asynchronous fault"]
        #[inline(always)]
        pub fn shadow(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Control FSM integrity error, asynchronous fault"]
        #[inline(always)]
        pub fn ctrl_fsm_intg(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Control FSM cross check error, asynchronous fault"]
        #[inline(always)]
        pub fn ctrl_fsm_chk(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Control FSM counter integrity error, asynchronous fault"]
        #[inline(always)]
        pub fn ctrl_fsm_cnt(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Reseed counter integrity error, asynchronous fault"]
        #[inline(always)]
        pub fn reseed_cnt(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Sideload control FSM integrity error, asynchronous fault"]
        #[inline(always)]
        pub fn side_ctrl_fsm(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Sideload control key select error, synchronous fault"]
        #[inline(always)]
        pub fn side_ctrl_sel(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Secret key ecc error, asynchronous fault"]
        #[inline(always)]
        pub fn key_ecc(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
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
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.op_done is set."]
        #[inline(always)]
        pub fn op_done(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.op_done is set."]
        #[inline(always)]
        pub fn op_done(self, val: bool) -> Self {
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
        #[doc = "Operation complete"]
        #[inline(always)]
        pub fn op_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
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
            Self(self.0 | (1 << 0))
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
        #[doc = "Write 1 to force !!INTR_STATE.op_done to 1."]
        #[inline(always)]
        pub fn op_done(self, val: bool) -> Self {
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
    pub struct MaxCreatorKeyVerRegwenReadVal(u32);
    impl MaxCreatorKeyVerRegwenReadVal {
        #[doc = "MAX_CREATOR_KEY_VERSION configure enable."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaxCreatorKeyVerRegwenWriteVal {
            MaxCreatorKeyVerRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MaxCreatorKeyVerRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxCreatorKeyVerRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaxCreatorKeyVerRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaxCreatorKeyVerRegwenWriteVal(u32);
    impl MaxCreatorKeyVerRegwenWriteVal {
        #[doc = "MAX_CREATOR_KEY_VERSION configure enable."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MaxCreatorKeyVerRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxCreatorKeyVerRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaxCreatorKeyVerRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaxOwnerIntKeyVerRegwenReadVal(u32);
    impl MaxOwnerIntKeyVerRegwenReadVal {
        #[doc = "MAX_OWNER_INTERMEDIATE_KEY configure enable."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaxOwnerIntKeyVerRegwenWriteVal {
            MaxOwnerIntKeyVerRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MaxOwnerIntKeyVerRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxOwnerIntKeyVerRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaxOwnerIntKeyVerRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaxOwnerIntKeyVerRegwenWriteVal(u32);
    impl MaxOwnerIntKeyVerRegwenWriteVal {
        #[doc = "MAX_OWNER_INTERMEDIATE_KEY configure enable."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MaxOwnerIntKeyVerRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxOwnerIntKeyVerRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaxOwnerIntKeyVerRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaxOwnerKeyVerRegwenReadVal(u32);
    impl MaxOwnerKeyVerRegwenReadVal {
        #[doc = "MAX_OWNER_KEY configure enable."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> MaxOwnerKeyVerRegwenWriteVal {
            MaxOwnerKeyVerRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for MaxOwnerKeyVerRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxOwnerKeyVerRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: MaxOwnerKeyVerRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MaxOwnerKeyVerRegwenWriteVal(u32);
    impl MaxOwnerKeyVerRegwenWriteVal {
        #[doc = "MAX_OWNER_KEY configure enable."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for MaxOwnerKeyVerRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MaxOwnerKeyVerRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: MaxOwnerKeyVerRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OpStatusReadVal(u32);
    impl OpStatusReadVal {
        #[doc = "Operation status."]
        #[inline(always)]
        pub fn status(&self) -> super::enums::Status {
            super::enums::Status::try_from((self.0 >> 0) & 3).unwrap()
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
        #[doc = "Operation status."]
        #[inline(always)]
        pub fn status(
            self,
            f: impl FnOnce(super::enums::selector::StatusSelector) -> super::enums::Status,
        ) -> Self {
            Self(
                (self.0 & !(3 << 0))
                    | (u32::from(f(super::enums::selector::StatusSelector())) << 0),
            )
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
    pub struct ReseedIntervalRegwenReadVal(u32);
    impl ReseedIntervalRegwenReadVal {
        #[doc = "Configuration enable for reseed interval"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ReseedIntervalRegwenWriteVal {
            ReseedIntervalRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ReseedIntervalRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReseedIntervalRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ReseedIntervalRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReseedIntervalRegwenWriteVal(u32);
    impl ReseedIntervalRegwenWriteVal {
        #[doc = "Configuration enable for reseed interval"]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ReseedIntervalRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReseedIntervalRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ReseedIntervalRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReseedIntervalShadowedReadVal(u32);
    impl ReseedIntervalShadowedReadVal {
        #[doc = "Number of internal PRNG updates before a reseed is requested."]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ReseedIntervalShadowedWriteVal {
            ReseedIntervalShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for ReseedIntervalShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReseedIntervalShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: ReseedIntervalShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReseedIntervalShadowedWriteVal(u32);
    impl ReseedIntervalShadowedWriteVal {
        #[doc = "Number of internal PRNG updates before a reseed is requested."]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for ReseedIntervalShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReseedIntervalShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ReseedIntervalShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SideloadClearReadVal(u32);
    impl SideloadClearReadVal {
        #[doc = "Depending on the value programmed, a different sideload key slot is cleared.\nIf the value programmed is not one of the enumerated values below, ALL sideload\nkey slots are continuously cleared. In order to stop continuous clearing, SW should\ntoggle the clear bit again (i.e. disable continuous clearing)."]
        #[inline(always)]
        pub fn val(&self) -> super::enums::SideloadClear {
            super::enums::SideloadClear::try_from((self.0 >> 0) & 7).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SideloadClearWriteVal {
            SideloadClearWriteVal(self.0)
        }
    }
    impl From<u32> for SideloadClearReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SideloadClearReadVal> for u32 {
        #[inline(always)]
        fn from(val: SideloadClearReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SideloadClearWriteVal(u32);
    impl SideloadClearWriteVal {
        #[doc = "Depending on the value programmed, a different sideload key slot is cleared.\nIf the value programmed is not one of the enumerated values below, ALL sideload\nkey slots are continuously cleared. In order to stop continuous clearing, SW should\ntoggle the clear bit again (i.e. disable continuous clearing)."]
        #[inline(always)]
        pub fn val(
            self,
            f: impl FnOnce(super::enums::selector::SideloadClearSelector) -> super::enums::SideloadClear,
        ) -> Self {
            Self(
                (self.0 & !(7 << 0))
                    | (u32::from(f(super::enums::selector::SideloadClearSelector())) << 0),
            )
        }
    }
    impl From<u32> for SideloadClearWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SideloadClearWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SideloadClearWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StartReadVal(u32);
    impl StartReadVal {
        #[doc = "Start key manager operations"]
        #[inline(always)]
        pub fn en(&self) -> super::enums::En {
            super::enums::En::try_from((self.0 >> 0) & 1).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> StartWriteVal {
            StartWriteVal(self.0)
        }
    }
    impl From<u32> for StartReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StartReadVal> for u32 {
        #[inline(always)]
        fn from(val: StartReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StartWriteVal(u32);
    impl StartWriteVal {
        #[doc = "Start key manager operations"]
        #[inline(always)]
        pub fn en(
            self,
            f: impl FnOnce(super::enums::selector::EnSelector) -> super::enums::En,
        ) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(f(super::enums::selector::EnSelector())) << 0))
        }
    }
    impl From<u32> for StartWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StartWriteVal> for u32 {
        #[inline(always)]
        fn from(val: StartWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwBindingRegwenReadVal(u32);
    impl SwBindingRegwenReadVal {
        #[doc = "Software binding register write enable.\nThis is locked by software and unlocked by hardware upon a successful advance call.\n\nSoftware binding resets to 1, and its value cannot be altered by software until advancement to Init state."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SwBindingRegwenWriteVal {
            SwBindingRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for SwBindingRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwBindingRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwBindingRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SwBindingRegwenWriteVal(u32);
    impl SwBindingRegwenWriteVal {
        #[doc = "Software binding register write enable.\nThis is locked by software and unlocked by hardware upon a successful advance call.\n\nSoftware binding resets to 1, and its value cannot be altered by software until advancement to Init state."]
        #[inline(always)]
        pub fn en_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for SwBindingRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwBindingRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: SwBindingRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WorkingStateReadVal(u32);
    impl WorkingStateReadVal {
        #[doc = "Key manager control state"]
        #[inline(always)]
        pub fn state(&self) -> super::enums::State {
            super::enums::State::try_from((self.0 >> 0) & 7).unwrap()
        }
    }
    impl From<u32> for WorkingStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WorkingStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: WorkingStateReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum CdiSel {
        SealingCdi = 0,
        AttestationCdi = 1,
    }
    impl CdiSel {
        #[inline(always)]
        pub fn sealing_cdi(&self) -> bool {
            *self == Self::SealingCdi
        }
        #[inline(always)]
        pub fn attestation_cdi(&self) -> bool {
            *self == Self::AttestationCdi
        }
    }
    impl TryFrom<u32> for CdiSel {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<CdiSel, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, CdiSel>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<CdiSel> for u32 {
        fn from(val: CdiSel) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum DestSel {
        None = 0,
        Aes = 1,
        Kmac = 2,
        Otbn = 3,
    }
    impl DestSel {
        #[inline(always)]
        pub fn none(&self) -> bool {
            *self == Self::None
        }
        #[inline(always)]
        pub fn aes(&self) -> bool {
            *self == Self::Aes
        }
        #[inline(always)]
        pub fn kmac(&self) -> bool {
            *self == Self::Kmac
        }
        #[inline(always)]
        pub fn otbn(&self) -> bool {
            *self == Self::Otbn
        }
    }
    impl TryFrom<u32> for DestSel {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<DestSel, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, DestSel>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<DestSel> for u32 {
        fn from(val: DestSel) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum En {
        Reserved0 = 0,
        ValidState = 1,
    }
    impl En {
        #[inline(always)]
        pub fn valid_state(&self) -> bool {
            *self == Self::ValidState
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
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Operation {
        Advance = 0,
        GenerateId = 1,
        GenerateSwOutput = 2,
        GenerateHwOutput = 3,
        Disable = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl Operation {
        #[inline(always)]
        pub fn advance(&self) -> bool {
            *self == Self::Advance
        }
        #[inline(always)]
        pub fn generate_id(&self) -> bool {
            *self == Self::GenerateId
        }
        #[inline(always)]
        pub fn generate_sw_output(&self) -> bool {
            *self == Self::GenerateSwOutput
        }
        #[inline(always)]
        pub fn generate_hw_output(&self) -> bool {
            *self == Self::GenerateHwOutput
        }
        #[inline(always)]
        pub fn disable(&self) -> bool {
            *self == Self::Disable
        }
    }
    impl TryFrom<u32> for Operation {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Operation, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Operation>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Operation> for u32 {
        fn from(val: Operation) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum SideloadClear {
        None = 0,
        Aes = 1,
        Kmac = 2,
        Otbn = 3,
        Reserved4 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl SideloadClear {
        #[inline(always)]
        pub fn none(&self) -> bool {
            *self == Self::None
        }
        #[inline(always)]
        pub fn aes(&self) -> bool {
            *self == Self::Aes
        }
        #[inline(always)]
        pub fn kmac(&self) -> bool {
            *self == Self::Kmac
        }
        #[inline(always)]
        pub fn otbn(&self) -> bool {
            *self == Self::Otbn
        }
    }
    impl TryFrom<u32> for SideloadClear {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<SideloadClear, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, SideloadClear>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<SideloadClear> for u32 {
        fn from(val: SideloadClear) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum State {
        Reset = 0,
        Init = 1,
        CreatorRootKey = 2,
        OwnerIntermediateKey = 3,
        OwnerKey = 4,
        Disabled = 5,
        Invalid = 6,
        Reserved7 = 7,
    }
    impl State {
        #[inline(always)]
        pub fn reset(&self) -> bool {
            *self == Self::Reset
        }
        #[inline(always)]
        pub fn init(&self) -> bool {
            *self == Self::Init
        }
        #[inline(always)]
        pub fn creator_root_key(&self) -> bool {
            *self == Self::CreatorRootKey
        }
        #[inline(always)]
        pub fn owner_intermediate_key(&self) -> bool {
            *self == Self::OwnerIntermediateKey
        }
        #[inline(always)]
        pub fn owner_key(&self) -> bool {
            *self == Self::OwnerKey
        }
        #[inline(always)]
        pub fn disabled(&self) -> bool {
            *self == Self::Disabled
        }
        #[inline(always)]
        pub fn invalid(&self) -> bool {
            *self == Self::Invalid
        }
    }
    impl TryFrom<u32> for State {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<State, ()> {
            if val < 8 {
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
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Status {
        Idle = 0,
        Wip = 1,
        DoneSuccess = 2,
        DoneError = 3,
    }
    impl Status {
        #[inline(always)]
        pub fn idle(&self) -> bool {
            *self == Self::Idle
        }
        #[inline(always)]
        pub fn wip(&self) -> bool {
            *self == Self::Wip
        }
        #[inline(always)]
        pub fn done_success(&self) -> bool {
            *self == Self::DoneSuccess
        }
        #[inline(always)]
        pub fn done_error(&self) -> bool {
            *self == Self::DoneError
        }
    }
    impl TryFrom<u32> for Status {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Status, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Status>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Status> for u32 {
        fn from(val: Status) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct CdiSelSelector();
        impl CdiSelSelector {
            #[inline(always)]
            pub fn sealing_cdi(&self) -> super::CdiSel {
                super::CdiSel::SealingCdi
            }
            #[inline(always)]
            pub fn attestation_cdi(&self) -> super::CdiSel {
                super::CdiSel::AttestationCdi
            }
        }
        pub struct DestSelSelector();
        impl DestSelSelector {
            #[inline(always)]
            pub fn none(&self) -> super::DestSel {
                super::DestSel::None
            }
            #[inline(always)]
            pub fn aes(&self) -> super::DestSel {
                super::DestSel::Aes
            }
            #[inline(always)]
            pub fn kmac(&self) -> super::DestSel {
                super::DestSel::Kmac
            }
            #[inline(always)]
            pub fn otbn(&self) -> super::DestSel {
                super::DestSel::Otbn
            }
        }
        pub struct EnSelector();
        impl EnSelector {
            #[inline(always)]
            pub fn valid_state(&self) -> super::En {
                super::En::ValidState
            }
        }
        pub struct OperationSelector();
        impl OperationSelector {
            #[inline(always)]
            pub fn advance(&self) -> super::Operation {
                super::Operation::Advance
            }
            #[inline(always)]
            pub fn generate_id(&self) -> super::Operation {
                super::Operation::GenerateId
            }
            #[inline(always)]
            pub fn generate_sw_output(&self) -> super::Operation {
                super::Operation::GenerateSwOutput
            }
            #[inline(always)]
            pub fn generate_hw_output(&self) -> super::Operation {
                super::Operation::GenerateHwOutput
            }
            #[inline(always)]
            pub fn disable(&self) -> super::Operation {
                super::Operation::Disable
            }
        }
        pub struct SideloadClearSelector();
        impl SideloadClearSelector {
            #[inline(always)]
            pub fn none(&self) -> super::SideloadClear {
                super::SideloadClear::None
            }
            #[inline(always)]
            pub fn aes(&self) -> super::SideloadClear {
                super::SideloadClear::Aes
            }
            #[inline(always)]
            pub fn kmac(&self) -> super::SideloadClear {
                super::SideloadClear::Kmac
            }
            #[inline(always)]
            pub fn otbn(&self) -> super::SideloadClear {
                super::SideloadClear::Otbn
            }
        }
        pub struct StateSelector();
        impl StateSelector {
            #[inline(always)]
            pub fn reset(&self) -> super::State {
                super::State::Reset
            }
            #[inline(always)]
            pub fn init(&self) -> super::State {
                super::State::Init
            }
            #[inline(always)]
            pub fn creator_root_key(&self) -> super::State {
                super::State::CreatorRootKey
            }
            #[inline(always)]
            pub fn owner_intermediate_key(&self) -> super::State {
                super::State::OwnerIntermediateKey
            }
            #[inline(always)]
            pub fn owner_key(&self) -> super::State {
                super::State::OwnerKey
            }
            #[inline(always)]
            pub fn disabled(&self) -> super::State {
                super::State::Disabled
            }
            #[inline(always)]
            pub fn invalid(&self) -> super::State {
                super::State::Invalid
            }
        }
        pub struct StatusSelector();
        impl StatusSelector {
            #[inline(always)]
            pub fn idle(&self) -> super::Status {
                super::Status::Idle
            }
            #[inline(always)]
            pub fn wip(&self) -> super::Status {
                super::Status::Wip
            }
            #[inline(always)]
            pub fn done_success(&self) -> super::Status {
                super::Status::DoneSuccess
            }
            #[inline(always)]
            pub fn done_error(&self) -> super::Status {
                super::Status::DoneError
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type CfgRegwen = ureg::ReadOnlyReg32<crate::regs::CfgRegwenReadVal>;
    pub type Start = ureg::ReadWriteReg32<0, crate::regs::StartReadVal, crate::regs::StartWriteVal>;
    pub type ControlShadowed = ureg::ReadWriteReg32<
        0x10,
        crate::regs::ControlShadowedReadVal,
        crate::regs::ControlShadowedWriteVal,
    >;
    pub type SideloadClear = ureg::ReadWriteReg32<
        0,
        crate::regs::SideloadClearReadVal,
        crate::regs::SideloadClearWriteVal,
    >;
    pub type ReseedIntervalRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ReseedIntervalRegwenReadVal,
        crate::regs::ReseedIntervalRegwenWriteVal,
    >;
    pub type ReseedIntervalShadowed = ureg::ReadWriteReg32<
        0x100,
        crate::regs::ReseedIntervalShadowedReadVal,
        crate::regs::ReseedIntervalShadowedWriteVal,
    >;
    pub type SwBindingRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::SwBindingRegwenReadVal,
        crate::regs::SwBindingRegwenWriteVal,
    >;
    pub type SealingSwBinding = ureg::ReadWriteReg32<0, u32, u32>;
    pub type AttestSwBinding = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Salt = ureg::ReadWriteReg32<0, u32, u32>;
    pub type KeyVersion = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MaxCreatorKeyVerRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MaxCreatorKeyVerRegwenReadVal,
        crate::regs::MaxCreatorKeyVerRegwenWriteVal,
    >;
    pub type MaxCreatorKeyVerShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MaxOwnerIntKeyVerRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MaxOwnerIntKeyVerRegwenReadVal,
        crate::regs::MaxOwnerIntKeyVerRegwenWriteVal,
    >;
    pub type MaxOwnerIntKeyVerShadowed = ureg::ReadWriteReg32<1, u32, u32>;
    pub type MaxOwnerKeyVerRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::MaxOwnerKeyVerRegwenReadVal,
        crate::regs::MaxOwnerKeyVerRegwenWriteVal,
    >;
    pub type MaxOwnerKeyVerShadowed = ureg::ReadWriteReg32<0, u32, u32>;
    pub type SwShare0Output = ureg::ReadOnlyReg32<u32>;
    pub type SwShare1Output = ureg::ReadOnlyReg32<u32>;
    pub type WorkingState = ureg::ReadOnlyReg32<crate::regs::WorkingStateReadVal>;
    pub type OpStatus =
        ureg::ReadWriteReg32<0, crate::regs::OpStatusReadVal, crate::regs::OpStatusWriteVal>;
    pub type ErrCode =
        ureg::ReadWriteReg32<0, crate::regs::ErrCodeReadVal, crate::regs::ErrCodeWriteVal>;
    pub type FaultStatus = ureg::ReadOnlyReg32<crate::regs::FaultStatusReadVal>;
    pub type Debug = ureg::ReadWriteReg32<0, crate::regs::DebugReadVal, crate::regs::DebugWriteVal>;
}
