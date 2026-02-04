#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct OtpCtrl {
    _priv: (),
}
impl OtpCtrl {
    pub const PTR: *mut u32 = 0x40130000 as *mut u32;
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
    #[doc = "OTP status register.\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register holds information about error conditions that occurred in the agents\ninteracting with the OTP macro via the internal bus. The error codes should be checked\nif the partitions, DAI or LCI flag an error in the !!STATUS register, or when an\n!!INTR_STATE.otp_error has been triggered. Note that all errors trigger an otp_error\ninterrupt, and in addition some errors may trigger either an fatal_macro_error or an\nfatal_check_error alert.\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::Array<13, ureg::RegRef<crate::meta::ErrCode, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for all direct access interface registers.\n\nRead value: [`regs::DirectAccessRegwenReadVal`]; Write value: [`regs::DirectAccessRegwenWriteVal`]"]
    #[inline(always)]
    pub fn direct_access_regwen(&self) -> ureg::RegRef<crate::meta::DirectAccessRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command register for direct accesses.\n\nRead value: [`regs::DirectAccessCmdReadVal`]; Write value: [`regs::DirectAccessCmdWriteVal`]"]
    #[inline(always)]
    pub fn direct_access_cmd(&self) -> ureg::RegRef<crate::meta::DirectAccessCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Address register for direct accesses.\n\nRead value: [`regs::DirectAccessAddressReadVal`]; Write value: [`regs::DirectAccessAddressWriteVal`]"]
    #[inline(always)]
    pub fn direct_access_address(&self) -> ureg::RegRef<crate::meta::DirectAccessAddress, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Write data for direct accesses.\nHardware automatically determines the access granule (32bit or 64bit) based on which\npartition is being written to.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn direct_access_wdata(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::DirectAccessWdata, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Read data for direct accesses.\nHardware automatically determines the access granule (32bit or 64bit) based on which\npartition is read from.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn direct_access_rdata(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::DirectAccessRdata, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for !!CHECK_TRIGGER.\n\nRead value: [`regs::CheckTriggerRegwenReadVal`]; Write value: [`regs::CheckTriggerRegwenWriteVal`]"]
    #[inline(always)]
    pub fn check_trigger_regwen(&self) -> ureg::RegRef<crate::meta::CheckTriggerRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command register for direct accesses.\n\nRead value: [`regs::CheckTriggerReadVal`]; Write value: [`regs::CheckTriggerWriteVal`]"]
    #[inline(always)]
    pub fn check_trigger(&self) -> ureg::RegRef<crate::meta::CheckTrigger, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x68 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for !!INTEGRITY_CHECK_PERIOD and !!CONSISTENCY_CHECK_PERIOD.\n\nRead value: [`regs::CheckRegwenReadVal`]; Write value: [`regs::CheckRegwenWriteVal`]"]
    #[inline(always)]
    pub fn check_regwen(&self) -> ureg::RegRef<crate::meta::CheckRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timeout value for the integrity and consistency checks.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn check_timeout(&self) -> ureg::RegRef<crate::meta::CheckTimeout, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This value specifies the maximum period that can be generated pseudo-randomly.\nOnly applies to the HW_CFG* and SECRET* partitions once they are locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn integrity_check_period(
        &self,
    ) -> ureg::RegRef<crate::meta::IntegrityCheckPeriod, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This value specifies the maximum period that can be generated pseudo-randomly.\nThis applies to the LIFE_CYCLE partition and the HW_CFG* and SECRET* partitions once they are locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn consistency_check_period(
        &self,
    ) -> ureg::RegRef<crate::meta::ConsistencyCheckPeriod, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Runtime read lock for the VENDOR_TEST partition.\n\nRead value: [`regs::VendorTestReadLockReadVal`]; Write value: [`regs::VendorTestReadLockWriteVal`]"]
    #[inline(always)]
    pub fn vendor_test_read_lock(&self) -> ureg::RegRef<crate::meta::VendorTestReadLock, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Runtime read lock for the CREATOR_SW_CFG partition.\n\nRead value: [`regs::CreatorSwCfgReadLockReadVal`]; Write value: [`regs::CreatorSwCfgReadLockWriteVal`]"]
    #[inline(always)]
    pub fn creator_sw_cfg_read_lock(
        &self,
    ) -> ureg::RegRef<crate::meta::CreatorSwCfgReadLock, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Runtime read lock for the OWNER_SW_CFG partition.\n\nRead value: [`regs::OwnerSwCfgReadLockReadVal`]; Write value: [`regs::OwnerSwCfgReadLockWriteVal`]"]
    #[inline(always)]
    pub fn owner_sw_cfg_read_lock(&self) -> ureg::RegRef<crate::meta::OwnerSwCfgReadLock, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Runtime read lock for the ROT_CREATOR_AUTH_CODESIGN partition.\n\nRead value: [`regs::RotCreatorAuthCodesignReadLockReadVal`]; Write value: [`regs::RotCreatorAuthCodesignReadLockWriteVal`]"]
    #[inline(always)]
    pub fn rot_creator_auth_codesign_read_lock(
        &self,
    ) -> ureg::RegRef<crate::meta::RotCreatorAuthCodesignReadLock, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x88 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Runtime read lock for the ROT_CREATOR_AUTH_STATE partition.\n\nRead value: [`regs::RotCreatorAuthStateReadLockReadVal`]; Write value: [`regs::RotCreatorAuthStateReadLockWriteVal`]"]
    #[inline(always)]
    pub fn rot_creator_auth_state_read_lock(
        &self,
    ) -> ureg::RegRef<crate::meta::RotCreatorAuthStateReadLock, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the VENDOR_TEST partition.\nThe integrity digest is 0 by default. Software must write this\ndigest value via the direct access interface in order to lock the partition.\nAfter a reset, write access to the VENDOR_TEST partition is locked and\nthe digest becomes visible in this CSR.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn vendor_test_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::VendorTestDigest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the CREATOR_SW_CFG partition.\nThe integrity digest is 0 by default. Software must write this\ndigest value via the direct access interface in order to lock the partition.\nAfter a reset, write access to the CREATOR_SW_CFG partition is locked and\nthe digest becomes visible in this CSR.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn creator_sw_cfg_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::CreatorSwCfgDigest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x98 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the OWNER_SW_CFG partition.\nThe integrity digest is 0 by default. Software must write this\ndigest value via the direct access interface in order to lock the partition.\nAfter a reset, write access to the OWNER_SW_CFG partition is locked and\nthe digest becomes visible in this CSR.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn owner_sw_cfg_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::OwnerSwCfgDigest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xa0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the ROT_CREATOR_AUTH_CODESIGN partition.\nThe integrity digest is 0 by default. Software must write this\ndigest value via the direct access interface in order to lock the partition.\nAfter a reset, write access to the ROT_CREATOR_AUTH_CODESIGN partition is locked and\nthe digest becomes visible in this CSR.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rot_creator_auth_codesign_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::RotCreatorAuthCodesignDigest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xa8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the ROT_CREATOR_AUTH_STATE partition.\nThe integrity digest is 0 by default. Software must write this\ndigest value via the direct access interface in order to lock the partition.\nAfter a reset, write access to the ROT_CREATOR_AUTH_STATE partition is locked and\nthe digest becomes visible in this CSR.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rot_creator_auth_state_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::RotCreatorAuthStateDigest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xb0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the HW_CFG0 partition.\nThe integrity digest is 0 by default. The digest calculation can be triggered via the !!DIRECT_ACCESS_CMD.\nAfter a reset, the digest then becomes visible in this CSR, and the corresponding partition becomes write-locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn hw_cfg0_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::HwCfg0Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xb8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the HW_CFG1 partition.\nThe integrity digest is 0 by default. The digest calculation can be triggered via the !!DIRECT_ACCESS_CMD.\nAfter a reset, the digest then becomes visible in this CSR, and the corresponding partition becomes write-locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn hw_cfg1_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::HwCfg1Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xc0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the SECRET0 partition.\nThe integrity digest is 0 by default. The digest calculation can be triggered via the !!DIRECT_ACCESS_CMD.\nAfter a reset, the digest then becomes visible in this CSR, and the corresponding partition becomes write-locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn secret0_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Secret0Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xc8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the SECRET1 partition.\nThe integrity digest is 0 by default. The digest calculation can be triggered via the !!DIRECT_ACCESS_CMD.\nAfter a reset, the digest then becomes visible in this CSR, and the corresponding partition becomes write-locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn secret1_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Secret1Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xd0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Integrity digest for the SECRET2 partition.\nThe integrity digest is 0 by default. The digest calculation can be triggered via the !!DIRECT_ACCESS_CMD.\nAfter a reset, the digest then becomes visible in this CSR, and the corresponding partition becomes write-locked.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn secret2_digest(
        &self,
    ) -> ureg::Array<2, ureg::RegRef<crate::meta::Secret2Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xd8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Any read to this window directly maps to the corresponding offset in the creator and owner software\nconfig partitions, and triggers an OTP readout of the bytes requested. Note that the transaction\nwill block until OTP readout has completed.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn sw_cfg_window(
        &self,
    ) -> ureg::Array<512, ureg::RegRef<crate::meta::SwCfgWindow, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x800 / core::mem::size_of::<u32>()),
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
        pub fn fatal_macro_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_check_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_bus_integ_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_prim_otp_alert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn recov_prim_otp_alert(self, val: bool) -> Self {
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
    pub struct CheckRegwenReadVal(u32);
    impl CheckRegwenReadVal {
        #[doc = "When cleared to 0, !!INTEGRITY_CHECK_PERIOD and !!CONSISTENCY_CHECK_PERIOD registers cannot be written anymore.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn check_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CheckRegwenWriteVal {
            CheckRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CheckRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CheckRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CheckRegwenWriteVal(u32);
    impl CheckRegwenWriteVal {
        #[doc = "When cleared to 0, !!INTEGRITY_CHECK_PERIOD and !!CONSISTENCY_CHECK_PERIOD registers cannot be written anymore.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn check_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CheckRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CheckRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CheckTriggerReadVal(u32);
    impl CheckTriggerReadVal {
        #[doc = "Writing 1 to this bit triggers an integrity check. SW should monitor !!STATUS.CHECK_PENDING\nand wait until the check has been completed. If there are any errors, those will be flagged\nin the !!STATUS and !!ERR_CODE registers, and via the interrupts and alerts."]
        #[inline(always)]
        pub fn integrity(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Writing 1 to this bit triggers a consistency check. SW should monitor !!STATUS.CHECK_PENDING\nand wait until the check has been completed. If there are any errors, those will be flagged\nin the !!STATUS and !!ERR_CODE registers, and via interrupts and alerts."]
        #[inline(always)]
        pub fn consistency(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CheckTriggerWriteVal {
            CheckTriggerWriteVal(self.0)
        }
    }
    impl From<u32> for CheckTriggerReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckTriggerReadVal> for u32 {
        #[inline(always)]
        fn from(val: CheckTriggerReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CheckTriggerWriteVal(u32);
    impl CheckTriggerWriteVal {
        #[doc = "Writing 1 to this bit triggers an integrity check. SW should monitor !!STATUS.CHECK_PENDING\nand wait until the check has been completed. If there are any errors, those will be flagged\nin the !!STATUS and !!ERR_CODE registers, and via the interrupts and alerts."]
        #[inline(always)]
        pub fn integrity_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Writing 1 to this bit triggers a consistency check. SW should monitor !!STATUS.CHECK_PENDING\nand wait until the check has been completed. If there are any errors, those will be flagged\nin the !!STATUS and !!ERR_CODE registers, and via interrupts and alerts."]
        #[inline(always)]
        pub fn consistency_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
    }
    impl From<u32> for CheckTriggerWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckTriggerWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CheckTriggerWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CheckTriggerRegwenReadVal(u32);
    impl CheckTriggerRegwenReadVal {
        #[doc = "When cleared to 0, the !!CHECK_TRIGGER register cannot be written anymore.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn check_trigger_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CheckTriggerRegwenWriteVal {
            CheckTriggerRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CheckTriggerRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckTriggerRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CheckTriggerRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CheckTriggerRegwenWriteVal(u32);
    impl CheckTriggerRegwenWriteVal {
        #[doc = "When cleared to 0, the !!CHECK_TRIGGER register cannot be written anymore.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn check_trigger_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CheckTriggerRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CheckTriggerRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CheckTriggerRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CreatorSwCfgReadLockReadVal(u32);
    impl CreatorSwCfgReadLockReadVal {
        #[doc = "When cleared to 0, read access to the CREATOR_SW_CFG partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn creator_sw_cfg_read_lock(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CreatorSwCfgReadLockWriteVal {
            CreatorSwCfgReadLockWriteVal(self.0)
        }
    }
    impl From<u32> for CreatorSwCfgReadLockReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CreatorSwCfgReadLockReadVal> for u32 {
        #[inline(always)]
        fn from(val: CreatorSwCfgReadLockReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CreatorSwCfgReadLockWriteVal(u32);
    impl CreatorSwCfgReadLockWriteVal {
        #[doc = "When cleared to 0, read access to the CREATOR_SW_CFG partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn creator_sw_cfg_read_lock_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CreatorSwCfgReadLockWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CreatorSwCfgReadLockWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CreatorSwCfgReadLockWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessAddressReadVal(u32);
    impl DirectAccessAddressReadVal {
        #[doc = "This is the address for the OTP word to be read or written through\nthe direct access interface. Note that the address is aligned to the access size\ninternally, hence bits 1:0 are ignored for 32bit accesses, and bits 2:0 are ignored\nfor 64bit accesses.\n\nFor the digest calculation command, set this register to the partition base offset."]
        #[inline(always)]
        pub fn direct_access_address(&self) -> u32 {
            (self.0 >> 0) & 0x7ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DirectAccessAddressWriteVal {
            DirectAccessAddressWriteVal(self.0)
        }
    }
    impl From<u32> for DirectAccessAddressReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessAddressReadVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessAddressReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessAddressWriteVal(u32);
    impl DirectAccessAddressWriteVal {
        #[doc = "This is the address for the OTP word to be read or written through\nthe direct access interface. Note that the address is aligned to the access size\ninternally, hence bits 1:0 are ignored for 32bit accesses, and bits 2:0 are ignored\nfor 64bit accesses.\n\nFor the digest calculation command, set this register to the partition base offset."]
        #[inline(always)]
        pub fn direct_access_address(self, val: u32) -> Self {
            Self((self.0 & !(0x7ff << 0)) | ((val & 0x7ff) << 0))
        }
    }
    impl From<u32> for DirectAccessAddressWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessAddressWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessAddressWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessCmdReadVal(u32);
    impl DirectAccessCmdReadVal {
        #[doc = "Initiates a readout sequence that reads the location specified\nby !!DIRECT_ACCESS_ADDRESS. The command places the data read into\n!!DIRECT_ACCESS_RDATA_0 and !!DIRECT_ACCESS_RDATA_1 (for 64bit partitions)."]
        #[inline(always)]
        pub fn rd(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Initiates a programming sequence that writes the data in !!DIRECT_ACCESS_WDATA_0\nand !!DIRECT_ACCESS_WDATA_1 (for 64bit partitions) to the location specified by\n!!DIRECT_ACCESS_ADDRESS."]
        #[inline(always)]
        pub fn wr(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Initiates the digest calculation and locking sequence for the partition specified by\n!!DIRECT_ACCESS_ADDRESS."]
        #[inline(always)]
        pub fn digest(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DirectAccessCmdWriteVal {
            DirectAccessCmdWriteVal(self.0)
        }
    }
    impl From<u32> for DirectAccessCmdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessCmdReadVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessCmdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessCmdWriteVal(u32);
    impl DirectAccessCmdWriteVal {
        #[doc = "Initiates a readout sequence that reads the location specified\nby !!DIRECT_ACCESS_ADDRESS. The command places the data read into\n!!DIRECT_ACCESS_RDATA_0 and !!DIRECT_ACCESS_RDATA_1 (for 64bit partitions)."]
        #[inline(always)]
        pub fn rd_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Initiates a programming sequence that writes the data in !!DIRECT_ACCESS_WDATA_0\nand !!DIRECT_ACCESS_WDATA_1 (for 64bit partitions) to the location specified by\n!!DIRECT_ACCESS_ADDRESS."]
        #[inline(always)]
        pub fn wr_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Initiates the digest calculation and locking sequence for the partition specified by\n!!DIRECT_ACCESS_ADDRESS."]
        #[inline(always)]
        pub fn digest_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
    }
    impl From<u32> for DirectAccessCmdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessCmdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessCmdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessRegwenReadVal(u32);
    impl DirectAccessRegwenReadVal {
        #[doc = "This bit controls whether the DAI registers can be written.\nWrite 0 to it in order to clear the bit.\n\nNote that the hardware also modulates this bit and sets it to 0 temporarily\nduring an OTP operation such that the corresponding address and data registers\ncannot be modified while an operation is pending. The !!DAI_IDLE status bit\nwill also be set to 0 in such a case."]
        #[inline(always)]
        pub fn direct_access_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> DirectAccessRegwenWriteVal {
            DirectAccessRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for DirectAccessRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct DirectAccessRegwenWriteVal(u32);
    impl DirectAccessRegwenWriteVal {
        #[doc = "This bit controls whether the DAI registers can be written.\nWrite 0 to it in order to clear the bit.\n\nNote that the hardware also modulates this bit and sets it to 0 temporarily\nduring an OTP operation such that the corresponding address and data registers\ncannot be modified while an operation is pending. The !!DAI_IDLE status bit\nwill also be set to 0 in such a case."]
        #[inline(always)]
        pub fn direct_access_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for DirectAccessRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<DirectAccessRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: DirectAccessRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrCodeReadVal(u32);
    impl ErrCodeReadVal {
        #[inline(always)]
        pub fn err_code(&self) -> super::enums::ErrCode {
            super::enums::ErrCode::try_from((self.0 >> 0) & 7).unwrap()
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
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.otp_operation_done is set."]
        #[inline(always)]
        pub fn otp_operation_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.otp_error is set."]
        #[inline(always)]
        pub fn otp_error(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.otp_operation_done is set."]
        #[inline(always)]
        pub fn otp_operation_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.otp_error is set."]
        #[inline(always)]
        pub fn otp_error(self, val: bool) -> Self {
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
        #[doc = "A direct access command or digest calculation operation has completed."]
        #[inline(always)]
        pub fn otp_operation_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "An error has occurred in the OTP controller. Check the !!ERR_CODE register to get more information."]
        #[inline(always)]
        pub fn otp_error(&self) -> bool {
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
        #[doc = "A direct access command or digest calculation operation has completed."]
        #[inline(always)]
        pub fn otp_operation_done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "An error has occurred in the OTP controller. Check the !!ERR_CODE register to get more information."]
        #[inline(always)]
        pub fn otp_error_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.otp_operation_done to 1."]
        #[inline(always)]
        pub fn otp_operation_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.otp_error to 1."]
        #[inline(always)]
        pub fn otp_error(self, val: bool) -> Self {
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
    pub struct OwnerSwCfgReadLockReadVal(u32);
    impl OwnerSwCfgReadLockReadVal {
        #[doc = "When cleared to 0, read access to the OWNER_SW_CFG partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn owner_sw_cfg_read_lock(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OwnerSwCfgReadLockWriteVal {
            OwnerSwCfgReadLockWriteVal(self.0)
        }
    }
    impl From<u32> for OwnerSwCfgReadLockReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OwnerSwCfgReadLockReadVal> for u32 {
        #[inline(always)]
        fn from(val: OwnerSwCfgReadLockReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OwnerSwCfgReadLockWriteVal(u32);
    impl OwnerSwCfgReadLockWriteVal {
        #[doc = "When cleared to 0, read access to the OWNER_SW_CFG partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn owner_sw_cfg_read_lock_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for OwnerSwCfgReadLockWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OwnerSwCfgReadLockWriteVal> for u32 {
        #[inline(always)]
        fn from(val: OwnerSwCfgReadLockWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RotCreatorAuthCodesignReadLockReadVal(u32);
    impl RotCreatorAuthCodesignReadLockReadVal {
        #[doc = "When cleared to 0, read access to the ROT_CREATOR_AUTH_CODESIGN partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn rot_creator_auth_codesign_read_lock(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RotCreatorAuthCodesignReadLockWriteVal {
            RotCreatorAuthCodesignReadLockWriteVal(self.0)
        }
    }
    impl From<u32> for RotCreatorAuthCodesignReadLockReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RotCreatorAuthCodesignReadLockReadVal> for u32 {
        #[inline(always)]
        fn from(val: RotCreatorAuthCodesignReadLockReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RotCreatorAuthCodesignReadLockWriteVal(u32);
    impl RotCreatorAuthCodesignReadLockWriteVal {
        #[doc = "When cleared to 0, read access to the ROT_CREATOR_AUTH_CODESIGN partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn rot_creator_auth_codesign_read_lock_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for RotCreatorAuthCodesignReadLockWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RotCreatorAuthCodesignReadLockWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RotCreatorAuthCodesignReadLockWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RotCreatorAuthStateReadLockReadVal(u32);
    impl RotCreatorAuthStateReadLockReadVal {
        #[doc = "When cleared to 0, read access to the ROT_CREATOR_AUTH_STATE partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn rot_creator_auth_state_read_lock(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RotCreatorAuthStateReadLockWriteVal {
            RotCreatorAuthStateReadLockWriteVal(self.0)
        }
    }
    impl From<u32> for RotCreatorAuthStateReadLockReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RotCreatorAuthStateReadLockReadVal> for u32 {
        #[inline(always)]
        fn from(val: RotCreatorAuthStateReadLockReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RotCreatorAuthStateReadLockWriteVal(u32);
    impl RotCreatorAuthStateReadLockWriteVal {
        #[doc = "When cleared to 0, read access to the ROT_CREATOR_AUTH_STATE partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn rot_creator_auth_state_read_lock_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for RotCreatorAuthStateReadLockWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RotCreatorAuthStateReadLockWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RotCreatorAuthStateReadLockWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn vendor_test_error(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn creator_sw_cfg_error(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn owner_sw_cfg_error(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn rot_creator_auth_codesign_error(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn rot_creator_auth_state_error(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn hw_cfg0_error(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn hw_cfg1_error(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn secret0_error(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn secret1_error(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn secret2_error(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in this partition.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn life_cycle_error(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in the DAI.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn dai_error(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Set to 1 if an error occurred in the LCI.\nIf set to 1, SW should check the !!ERR_CODE register at the corresponding index."]
        #[inline(always)]
        pub fn lci_error(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Set to 1 if an integrity or consistency check times out.\nThis raises an fatal_check_error alert and is an unrecoverable error condition."]
        #[inline(always)]
        pub fn timeout_error(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Set to 1 if the LFSR timer FSM has reached an invalid state.\nThis raises an fatal_check_error alert and is an unrecoverable error condition."]
        #[inline(always)]
        pub fn lfsr_fsm_error(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Set to 1 if the scrambling datapath FSM has reached an invalid state.\nThis raises an fatal_check_error alert and is an unrecoverable error condition."]
        #[inline(always)]
        pub fn scrambling_fsm_error(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Set to 1 if the key derivation FSM has reached an invalid state.\nThis raises an fatal_check_error alert and is an unrecoverable error condition."]
        #[inline(always)]
        pub fn key_deriv_fsm_error(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "This bit is set to 1 if a fatal bus integrity fault is detected.\nThis error triggers a fatal_bus_integ_error alert."]
        #[inline(always)]
        pub fn bus_integ_error(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Set to 1 if the DAI is idle and ready to accept commands."]
        #[inline(always)]
        pub fn dai_idle(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Set to 1 if an integrity or consistency check triggered by the LFSR timer or via !!CHECK_TRIGGER is pending."]
        #[inline(always)]
        pub fn check_pending(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
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
    pub struct VendorTestReadLockReadVal(u32);
    impl VendorTestReadLockReadVal {
        #[doc = "When cleared to 0, read access to the VENDOR_TEST partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn vendor_test_read_lock(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> VendorTestReadLockWriteVal {
            VendorTestReadLockWriteVal(self.0)
        }
    }
    impl From<u32> for VendorTestReadLockReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<VendorTestReadLockReadVal> for u32 {
        #[inline(always)]
        fn from(val: VendorTestReadLockReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct VendorTestReadLockWriteVal(u32);
    impl VendorTestReadLockWriteVal {
        #[doc = "When cleared to 0, read access to the VENDOR_TEST partition is locked.\nWrite 0 to clear this bit."]
        #[inline(always)]
        pub fn vendor_test_read_lock_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for VendorTestReadLockWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<VendorTestReadLockWriteVal> for u32 {
        #[inline(always)]
        fn from(val: VendorTestReadLockWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum ErrCode {
        NoError = 0,
        MacroError = 1,
        MacroEccCorrError = 2,
        MacroEccUncorrError = 3,
        MacroWriteBlankError = 4,
        AccessError = 5,
        CheckFailError = 6,
        FsmStateError = 7,
    }
    impl ErrCode {
        #[inline(always)]
        pub fn no_error(&self) -> bool {
            *self == Self::NoError
        }
        #[inline(always)]
        pub fn macro_error(&self) -> bool {
            *self == Self::MacroError
        }
        #[inline(always)]
        pub fn macro_ecc_corr_error(&self) -> bool {
            *self == Self::MacroEccCorrError
        }
        #[inline(always)]
        pub fn macro_ecc_uncorr_error(&self) -> bool {
            *self == Self::MacroEccUncorrError
        }
        #[inline(always)]
        pub fn macro_write_blank_error(&self) -> bool {
            *self == Self::MacroWriteBlankError
        }
        #[inline(always)]
        pub fn access_error(&self) -> bool {
            *self == Self::AccessError
        }
        #[inline(always)]
        pub fn check_fail_error(&self) -> bool {
            *self == Self::CheckFailError
        }
        #[inline(always)]
        pub fn fsm_state_error(&self) -> bool {
            *self == Self::FsmStateError
        }
    }
    impl TryFrom<u32> for ErrCode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<ErrCode, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, ErrCode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<ErrCode> for u32 {
        fn from(val: ErrCode) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct ErrCodeSelector();
        impl ErrCodeSelector {
            #[inline(always)]
            pub fn no_error(&self) -> super::ErrCode {
                super::ErrCode::NoError
            }
            #[inline(always)]
            pub fn macro_error(&self) -> super::ErrCode {
                super::ErrCode::MacroError
            }
            #[inline(always)]
            pub fn macro_ecc_corr_error(&self) -> super::ErrCode {
                super::ErrCode::MacroEccCorrError
            }
            #[inline(always)]
            pub fn macro_ecc_uncorr_error(&self) -> super::ErrCode {
                super::ErrCode::MacroEccUncorrError
            }
            #[inline(always)]
            pub fn macro_write_blank_error(&self) -> super::ErrCode {
                super::ErrCode::MacroWriteBlankError
            }
            #[inline(always)]
            pub fn access_error(&self) -> super::ErrCode {
                super::ErrCode::AccessError
            }
            #[inline(always)]
            pub fn check_fail_error(&self) -> super::ErrCode {
                super::ErrCode::CheckFailError
            }
            #[inline(always)]
            pub fn fsm_state_error(&self) -> super::ErrCode {
                super::ErrCode::FsmStateError
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
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ErrCode = ureg::ReadOnlyReg32<crate::regs::ErrCodeReadVal>;
    pub type DirectAccessRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::DirectAccessRegwenReadVal,
        crate::regs::DirectAccessRegwenWriteVal,
    >;
    pub type DirectAccessCmd = ureg::ReadWriteReg32<
        0,
        crate::regs::DirectAccessCmdReadVal,
        crate::regs::DirectAccessCmdWriteVal,
    >;
    pub type DirectAccessAddress = ureg::ReadWriteReg32<
        0,
        crate::regs::DirectAccessAddressReadVal,
        crate::regs::DirectAccessAddressWriteVal,
    >;
    pub type DirectAccessWdata = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DirectAccessRdata = ureg::ReadOnlyReg32<u32>;
    pub type CheckTriggerRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::CheckTriggerRegwenReadVal,
        crate::regs::CheckTriggerRegwenWriteVal,
    >;
    pub type CheckTrigger = ureg::ReadWriteReg32<
        0,
        crate::regs::CheckTriggerReadVal,
        crate::regs::CheckTriggerWriteVal,
    >;
    pub type CheckRegwen =
        ureg::ReadWriteReg32<1, crate::regs::CheckRegwenReadVal, crate::regs::CheckRegwenWriteVal>;
    pub type CheckTimeout = ureg::ReadWriteReg32<0, u32, u32>;
    pub type IntegrityCheckPeriod = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ConsistencyCheckPeriod = ureg::ReadWriteReg32<0, u32, u32>;
    pub type VendorTestReadLock = ureg::ReadWriteReg32<
        1,
        crate::regs::VendorTestReadLockReadVal,
        crate::regs::VendorTestReadLockWriteVal,
    >;
    pub type CreatorSwCfgReadLock = ureg::ReadWriteReg32<
        1,
        crate::regs::CreatorSwCfgReadLockReadVal,
        crate::regs::CreatorSwCfgReadLockWriteVal,
    >;
    pub type OwnerSwCfgReadLock = ureg::ReadWriteReg32<
        1,
        crate::regs::OwnerSwCfgReadLockReadVal,
        crate::regs::OwnerSwCfgReadLockWriteVal,
    >;
    pub type RotCreatorAuthCodesignReadLock = ureg::ReadWriteReg32<
        1,
        crate::regs::RotCreatorAuthCodesignReadLockReadVal,
        crate::regs::RotCreatorAuthCodesignReadLockWriteVal,
    >;
    pub type RotCreatorAuthStateReadLock = ureg::ReadWriteReg32<
        1,
        crate::regs::RotCreatorAuthStateReadLockReadVal,
        crate::regs::RotCreatorAuthStateReadLockWriteVal,
    >;
    pub type VendorTestDigest = ureg::ReadOnlyReg32<u32>;
    pub type CreatorSwCfgDigest = ureg::ReadOnlyReg32<u32>;
    pub type OwnerSwCfgDigest = ureg::ReadOnlyReg32<u32>;
    pub type RotCreatorAuthCodesignDigest = ureg::ReadOnlyReg32<u32>;
    pub type RotCreatorAuthStateDigest = ureg::ReadOnlyReg32<u32>;
    pub type HwCfg0Digest = ureg::ReadOnlyReg32<u32>;
    pub type HwCfg1Digest = ureg::ReadOnlyReg32<u32>;
    pub type Secret0Digest = ureg::ReadOnlyReg32<u32>;
    pub type Secret1Digest = ureg::ReadOnlyReg32<u32>;
    pub type Secret2Digest = ureg::ReadOnlyReg32<u32>;
    pub type SwCfgWindow = ureg::ReadOnlyReg32<u32>;
}
