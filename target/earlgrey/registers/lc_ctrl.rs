#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct LcCtrl {
    _priv: (),
}
impl LcCtrl {
    pub const PTR: *mut u32 = 0x40140000 as *mut u32;
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
    #[doc = "life cycle status register. Note that all errors are terminal and require a reset cycle.\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for the hardware mutex register.\n\nRead value: [`regs::ClaimTransitionIfRegwenReadVal`]; Write value: [`regs::ClaimTransitionIfRegwenWriteVal`]"]
    #[inline(always)]
    pub fn claim_transition_if_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::ClaimTransitionIfRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Hardware mutex to claim exclusive access to the transition interface.\n\nRead value: [`regs::ClaimTransitionIfReadVal`]; Write value: [`regs::ClaimTransitionIfWriteVal`]"]
    #[inline(always)]
    pub fn claim_transition_if(&self) -> ureg::RegRef<crate::meta::ClaimTransitionIf, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Register write enable for all transition interface registers.\n\nRead value: [`regs::TransitionRegwenReadVal`]; Write value: [`regs::TransitionRegwenWriteVal`]"]
    #[inline(always)]
    pub fn transition_regwen(&self) -> ureg::RegRef<crate::meta::TransitionRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command register for state transition requests.\n\nRead value: [`regs::TransitionCmdReadVal`]; Write value: [`regs::TransitionCmdWriteVal`]"]
    #[inline(always)]
    pub fn transition_cmd(&self) -> ureg::RegRef<crate::meta::TransitionCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control register for state transition requests.\n\nRead value: [`regs::TransitionCtrlReadVal`]; Write value: [`regs::TransitionCtrlWriteVal`]"]
    #[inline(always)]
    pub fn transition_ctrl(&self) -> ureg::RegRef<crate::meta::TransitionCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "128bit token for conditional transitions.\nMake sure to set this to 0 for unconditional transitions.\nNote that this register is shared with the life cycle TAP interface.\nIn order to have exclusive access to this register, SW must first claim the associated\nhardware mutex via !!CLAIM_TRANSITION_IF.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn transition_token(
        &self,
    ) -> ureg::Array<4, ureg::RegRef<crate::meta::TransitionToken, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register exposes the decoded life cycle state.\n\nRead value: [`regs::TransitionTargetReadVal`]; Write value: [`regs::TransitionTargetWriteVal`]"]
    #[inline(always)]
    pub fn transition_target(&self) -> ureg::RegRef<crate::meta::TransitionTarget, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Test/vendor-specific settings for the OTP macro wrapper.\nThese values are only active during RAW, TEST_* and RMA life cycle states.\nIn all other states, these values will be gated to zero before sending\nthem to the OTP macro wrapper - even if this register is programmed to a non-zero value.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn otp_vendor_test_ctrl(&self) -> ureg::RegRef<crate::meta::OtpVendorTestCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Test/vendor-specific settings for the OTP macro wrapper.\nThese values are only active during RAW, TEST_* and RMA life cycle states.\nIn all other states, these values will read as zero.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn otp_vendor_test_status(&self) -> ureg::RegRef<crate::meta::OtpVendorTestStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register exposes the decoded life cycle state.\n\nRead value: [`regs::LcStateReadVal`]; Write value: [`regs::LcStateWriteVal`]"]
    #[inline(always)]
    pub fn lc_state(&self) -> ureg::RegRef<crate::meta::LcState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register exposes the state of the decoded life cycle transition counter.\n\nRead value: [`regs::LcTransitionCntReadVal`]; Write value: [`regs::LcTransitionCntWriteVal`]"]
    #[inline(always)]
    pub fn lc_transition_cnt(&self) -> ureg::RegRef<crate::meta::LcTransitionCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register exposes the id state of the device.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn lc_id_state(&self) -> ureg::RegRef<crate::meta::LcIdState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register holds the SILICON_CREATOR_ID and the PRODUCT_ID.\n\nRead value: [`regs::HwRevision0ReadVal`]; Write value: [`regs::HwRevision0WriteVal`]"]
    #[inline(always)]
    pub fn hw_revision0(&self) -> ureg::RegRef<crate::meta::HwRevision0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This register holds the REVISION_ID.\n\nRead value: [`regs::HwRevision1ReadVal`]; Write value: [`regs::HwRevision1WriteVal`]"]
    #[inline(always)]
    pub fn hw_revision1(&self) -> ureg::RegRef<crate::meta::HwRevision1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This is the 256bit DEVICE_ID value that is stored in the HW_CFG0 partition in OTP.\nIf this register reads all-one, the HW_CFG0 partition has not been initialized yet or is in error state.\nIf this register reads all-zero, this is indicative that the value has not been programmed to OTP yet.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn device_id(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::DeviceId, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "This is a 256bit field used for keeping track of the manufacturing state.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn manuf_state(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::ManufState, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
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
        pub fn fatal_prog_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_state_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_bus_integ_error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
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
    pub struct ClaimTransitionIfReadVal(u32);
    impl ClaimTransitionIfReadVal {
        #[doc = "In order to have exclusive access to the transition interface, SW must first claim the associated\nhardware mutex by writing kMultiBitBool8True to this register.\nIf the register reads back kMultiBitBool8True, the mutex claim has been successful, and !!TRANSITION_REGWEN\nwill be set automatically to 1 by HW.\nWrite 0 to this register in order to release the HW mutex."]
        #[inline(always)]
        pub fn mutex(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClaimTransitionIfWriteVal {
            ClaimTransitionIfWriteVal(self.0)
        }
    }
    impl From<u32> for ClaimTransitionIfReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClaimTransitionIfReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClaimTransitionIfReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClaimTransitionIfWriteVal(u32);
    impl ClaimTransitionIfWriteVal {
        #[doc = "In order to have exclusive access to the transition interface, SW must first claim the associated\nhardware mutex by writing kMultiBitBool8True to this register.\nIf the register reads back kMultiBitBool8True, the mutex claim has been successful, and !!TRANSITION_REGWEN\nwill be set automatically to 1 by HW.\nWrite 0 to this register in order to release the HW mutex."]
        #[inline(always)]
        pub fn mutex(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for ClaimTransitionIfWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClaimTransitionIfWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClaimTransitionIfWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClaimTransitionIfRegwenReadVal(u32);
    impl ClaimTransitionIfRegwenReadVal {
        #[doc = "This bit is managed by software and is set to 1 by default.\nWhen cleared to 0, the !!CLAIM_TRANSITION_IF mutex register cannot be written to anymore. Write 0 to clear this bit."]
        #[inline(always)]
        pub fn claim_transition_if_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ClaimTransitionIfRegwenWriteVal {
            ClaimTransitionIfRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for ClaimTransitionIfRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClaimTransitionIfRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: ClaimTransitionIfRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ClaimTransitionIfRegwenWriteVal(u32);
    impl ClaimTransitionIfRegwenWriteVal {
        #[doc = "This bit is managed by software and is set to 1 by default.\nWhen cleared to 0, the !!CLAIM_TRANSITION_IF mutex register cannot be written to anymore. Write 0 to clear this bit."]
        #[inline(always)]
        pub fn claim_transition_if_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for ClaimTransitionIfRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ClaimTransitionIfRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ClaimTransitionIfRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwRevision0ReadVal(u32);
    impl HwRevision0ReadVal {
        #[doc = "Used to identify a class of devices.\nAssigned by the Silicon Creator.\nZero is an invalid value.\nThe encoding must follow the following range constraints:\n\n0x0000: invalid value\n0x0001 - 0x3FFF: reserved for discrete chip products\n0x4000 - 0x7FFF: reserved for integrated IP products\n0x8000 - 0xFFFF: reserved for future use"]
        #[inline(always)]
        pub fn product_id(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "ID of the silicon creator.\nAssigned by the OpenTitan project.\nZero is an invalid value.\nThe encoding must follow the following range constraints:\n\n0x0000: invalid value\n0x0001 - 0x3FFF: reserved for use in the open-source OpenTitan project\n0x4000 - 0x7FFF: reserved for real integrations of OpenTitan\n0x8000 - 0xFFFF: reserved for future use"]
        #[inline(always)]
        pub fn silicon_creator_id(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
    }
    impl From<u32> for HwRevision0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwRevision0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: HwRevision0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwRevision1ReadVal(u32);
    impl HwRevision1ReadVal {
        #[doc = "Product revision ID. Assigned by the Silicon Creator.\nThe encoding is not specified other than that different tapeouts must be assigned different revision numbers.\nI.e., each base or metal layer respin must be reflected so that software can rely on it to modify firmware and driver behavior.\nZero is an invalid value."]
        #[inline(always)]
        pub fn revision_id(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Reserved bits.\nSet to zero."]
        #[inline(always)]
        pub fn reserved(&self) -> u32 {
            (self.0 >> 8) & 0xffffff
        }
    }
    impl From<u32> for HwRevision1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwRevision1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: HwRevision1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LcStateReadVal(u32);
    impl LcStateReadVal {
        #[doc = "This field exposes the decoded life cycle state in a redundant enum format.\nThe 5bit state enum is repeated 6x so that it fills the entire 32bit register.\nThe encoding is straightforward replication: [val, val, val, val, val, val]."]
        #[inline(always)]
        pub fn state(&self) -> u32 {
            (self.0 >> 0) & 0x3fffffff
        }
    }
    impl From<u32> for LcStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LcStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: LcStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct LcTransitionCntReadVal(u32);
    impl LcTransitionCntReadVal {
        #[doc = "Number of total life cycle state transition attempts.\nThe life cycle controller allows up to 24 transition attempts.\nIf this counter is equal to 24, the !!LC_STATE is considered\nto be invalid and will read as SCRAP.\n\nIf the counter state is invalid, or the life cycle controller is in the post-transition state,\nthe counter will have the value 31 (i.e., all counter bits will be set)."]
        #[inline(always)]
        pub fn cnt(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
    }
    impl From<u32> for LcTransitionCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<LcTransitionCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: LcTransitionCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "This bit is set to 1 if the life cycle controller has successfully initialized and the\nstate exposed in !!LC_STATE and !!LC_TRANSITION_CNT is valid."]
        #[inline(always)]
        pub fn initialized(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the life cycle controller has successfully initialized and is\nready to accept a life cycle transition command."]
        #[inline(always)]
        pub fn ready(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the clock manager has successfully switched to the external clock due to\n!!EXT_CLOCK_EN being set to 1."]
        #[inline(always)]
        pub fn ext_clock_switched(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the last life cycle transition request was successful.\nNote that each transition attempt increments the !!LC_TRANSITION_CNT and\nmoves the life cycle state into POST_TRANSITION."]
        #[inline(always)]
        pub fn transition_successful(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the !!LC_TRANSITION_CNT has reached its maximum.\nIf this is the case, no more state transitions can be performed.\nNote that each transition attempt increments the !!LC_TRANSITION_CNT and\nmoves the life cycle state into POST_TRANSITION."]
        #[inline(always)]
        pub fn transition_count_error(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the last transition command requested an invalid state transition\n(e.g. DEV -> RAW). Note that each transition attempt increments the !!LC_TRANSITION_CNT and\nmoves the life cycle state into POST_TRANSITION."]
        #[inline(always)]
        pub fn transition_error(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the token supplied for a conditional transition was invalid.\nNote that each transition attempt increments the !!LC_TRANSITION_CNT and\nmoves the life cycle state into POST_TRANSITION."]
        #[inline(always)]
        pub fn token_error(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit is set to 1 if flash failed to correctly respond to an RMA request.\nNote that each transition attempt increments the !!LC_TRANSITION_CNT and\nmoves the life cycle state into POST_TRANSITION."]
        #[inline(always)]
        pub fn flash_rma_error(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit is set to 1 if an error occurred during an OTP programming operation.\nThis error will move the life cycle state automatically to POST_TRANSITION and raise a\nfatal_prog_error alert."]
        #[inline(always)]
        pub fn otp_error(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit is set to 1 if either the controller FSM state or the life cycle state is invalid or\nhas been corrupted as part of a tampering attempt. This error will move the life cycle state\nautomatically to INVALID and raise a fatal_state_error alert."]
        #[inline(always)]
        pub fn state_error(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit is set to 1 if a fatal bus integrity fault is detected.\nThis error triggers a fatal_bus_integ_error alert."]
        #[inline(always)]
        pub fn bus_integ_error(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit is set to 1 if the life cycle partition in OTP is in error state.\nThis bit is intended for production testing during the RAW life cycle state,\nwhere the OTP control and status registers are not accessible.\nThis error does not trigger an alert in the life cycle controller."]
        #[inline(always)]
        pub fn otp_partition_error(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
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
    pub struct TransitionCmdReadVal(u32);
    impl TransitionCmdReadVal {
        #[doc = "Writing a 1 to this register initiates the life cycle state transition to the state\nspecified in !!TRANSITION_TARGET.\nNote that not all transitions are possible, and certain conditional transitions require\nan additional !!TRANSITION_TOKEN_0.\nIn order to have exclusive access to this register, SW must first claim the associated\nhardware mutex via !!CLAIM_TRANSITION_IF."]
        #[inline(always)]
        pub fn start(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TransitionCmdWriteVal {
            TransitionCmdWriteVal(self.0)
        }
    }
    impl From<u32> for TransitionCmdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionCmdReadVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionCmdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionCmdWriteVal(u32);
    impl TransitionCmdWriteVal {
        #[doc = "Writing a 1 to this register initiates the life cycle state transition to the state\nspecified in !!TRANSITION_TARGET.\nNote that not all transitions are possible, and certain conditional transitions require\nan additional !!TRANSITION_TOKEN_0.\nIn order to have exclusive access to this register, SW must first claim the associated\nhardware mutex via !!CLAIM_TRANSITION_IF."]
        #[inline(always)]
        pub fn start_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
    }
    impl From<u32> for TransitionCmdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionCmdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionCmdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionCtrlReadVal(u32);
    impl TransitionCtrlReadVal {
        #[doc = "When set to 1, the OTP clock will be switched to an externally supplied clock right away when the\ndevice is in a non-PROD life cycle state. The clock mux will remain switched until the next system reset."]
        #[inline(always)]
        pub fn ext_clock_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "When set to 1, LC_CTRL performs a volatile lifecycle transition from RAW -> TEST_UNLOCKED0.\nNo state update will be written to OTP, and no reset will be needed after the transition has succeeded.\nNote that the token to be provided has to be the hashed unlock token, since in this case the token is NOT passed through KMAC before performing the comparison.\n\nAfter a successful VOLATILE_RAW_UNLOCK transition from RAW -> TEST_UNLOCKED0, the LC_CTRL FSM will go back to the IdleSt and set the STATUS.TRANSITION_SUCCESSFUL bit.\nThe LC_CTRL accepts further transition commands in this state.\n\nIMPORTANT NOTE: this feature is intended for test chips only in order to mitigate the risks of a malfunctioning\nOTP macro. Production devices will permanently disable this feature at compile time via the SecVolatileRawUnlockEn parameter.\n\nSoftware can check whether VOLATILE_RAW_UNLOCK is available by writing 1 and reading back\nthe register value. If the register reads back as 1 the mechanism is available, and if it reads back 0 it is not."]
        #[inline(always)]
        pub fn volatile_raw_unlock(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TransitionCtrlWriteVal {
            TransitionCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for TransitionCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionCtrlWriteVal(u32);
    impl TransitionCtrlWriteVal {
        #[doc = "When set to 1, the OTP clock will be switched to an externally supplied clock right away when the\ndevice is in a non-PROD life cycle state. The clock mux will remain switched until the next system reset."]
        #[inline(always)]
        pub fn ext_clock_en_set(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "When set to 1, LC_CTRL performs a volatile lifecycle transition from RAW -> TEST_UNLOCKED0.\nNo state update will be written to OTP, and no reset will be needed after the transition has succeeded.\nNote that the token to be provided has to be the hashed unlock token, since in this case the token is NOT passed through KMAC before performing the comparison.\n\nAfter a successful VOLATILE_RAW_UNLOCK transition from RAW -> TEST_UNLOCKED0, the LC_CTRL FSM will go back to the IdleSt and set the STATUS.TRANSITION_SUCCESSFUL bit.\nThe LC_CTRL accepts further transition commands in this state.\n\nIMPORTANT NOTE: this feature is intended for test chips only in order to mitigate the risks of a malfunctioning\nOTP macro. Production devices will permanently disable this feature at compile time via the SecVolatileRawUnlockEn parameter.\n\nSoftware can check whether VOLATILE_RAW_UNLOCK is available by writing 1 and reading back\nthe register value. If the register reads back as 1 the mechanism is available, and if it reads back 0 it is not."]
        #[inline(always)]
        pub fn volatile_raw_unlock(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for TransitionCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionRegwenReadVal(u32);
    impl TransitionRegwenReadVal {
        #[doc = "This bit is hardware-managed and only readable by software.\nBy default, this bit is set to 0 by hardware.\nOnce SW has claimed the !!CLAIM_TRANSITION_IF mutex, this bit will be set to 1.\nNote that the life cycle controller sets this bit temporarily to 0 while executing a life cycle state\ntransition."]
        #[inline(always)]
        pub fn transition_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
    }
    impl From<u32> for TransitionRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionTargetReadVal(u32);
    impl TransitionTargetReadVal {
        #[doc = "This field encodes the target life cycle state in a redundant enum format.\nThe 5bit state enum is repeated 6x so that it fills the entire 32bit register.\nThe encoding is straightforward replication: [val, val, val, val, val, val].\n\nNote that this register is shared with the life cycle TAP interface.\nIn order to have exclusive access to this register, SW must first claim the associated\nhardware mutex via !!CLAIM_TRANSITION_IF."]
        #[inline(always)]
        pub fn state(&self) -> u32 {
            (self.0 >> 0) & 0x3fffffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TransitionTargetWriteVal {
            TransitionTargetWriteVal(self.0)
        }
    }
    impl From<u32> for TransitionTargetReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionTargetReadVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionTargetReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TransitionTargetWriteVal(u32);
    impl TransitionTargetWriteVal {
        #[doc = "This field encodes the target life cycle state in a redundant enum format.\nThe 5bit state enum is repeated 6x so that it fills the entire 32bit register.\nThe encoding is straightforward replication: [val, val, val, val, val, val].\n\nNote that this register is shared with the life cycle TAP interface.\nIn order to have exclusive access to this register, SW must first claim the associated\nhardware mutex via !!CLAIM_TRANSITION_IF."]
        #[inline(always)]
        pub fn state(self, val: u32) -> Self {
            Self((self.0 & !(0x3fffffff << 0)) | ((val & 0x3fffffff) << 0))
        }
    }
    impl From<u32> for TransitionTargetWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TransitionTargetWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TransitionTargetWriteVal) -> u32 {
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
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ClaimTransitionIfRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::ClaimTransitionIfRegwenReadVal,
        crate::regs::ClaimTransitionIfRegwenWriteVal,
    >;
    pub type ClaimTransitionIf = ureg::ReadWriteReg32<
        0x69,
        crate::regs::ClaimTransitionIfReadVal,
        crate::regs::ClaimTransitionIfWriteVal,
    >;
    pub type TransitionRegwen = ureg::ReadOnlyReg32<crate::regs::TransitionRegwenReadVal>;
    pub type TransitionCmd = ureg::ReadWriteReg32<
        0,
        crate::regs::TransitionCmdReadVal,
        crate::regs::TransitionCmdWriteVal,
    >;
    pub type TransitionCtrl = ureg::ReadWriteReg32<
        0,
        crate::regs::TransitionCtrlReadVal,
        crate::regs::TransitionCtrlWriteVal,
    >;
    pub type TransitionToken = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TransitionTarget = ureg::ReadWriteReg32<
        0,
        crate::regs::TransitionTargetReadVal,
        crate::regs::TransitionTargetWriteVal,
    >;
    pub type OtpVendorTestCtrl = ureg::ReadWriteReg32<0, u32, u32>;
    pub type OtpVendorTestStatus = ureg::ReadOnlyReg32<u32>;
    pub type LcState = ureg::ReadOnlyReg32<crate::regs::LcStateReadVal>;
    pub type LcTransitionCnt = ureg::ReadOnlyReg32<crate::regs::LcTransitionCntReadVal>;
    pub type LcIdState = ureg::ReadOnlyReg32<u32>;
    pub type HwRevision0 = ureg::ReadOnlyReg32<crate::regs::HwRevision0ReadVal>;
    pub type HwRevision1 = ureg::ReadOnlyReg32<crate::regs::HwRevision1ReadVal>;
    pub type DeviceId = ureg::ReadOnlyReg32<u32>;
    pub type ManufState = ureg::ReadOnlyReg32<u32>;
}
