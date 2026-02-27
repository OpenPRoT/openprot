#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Csrng {
    _priv: (),
}
impl Csrng {
    pub const PTR: *mut u32 = 0x41150000 as *mut u32;
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
    #[doc = "Register write enable for all control registers\n\nRead value: [`regs::RegwenReadVal`]; Write value: [`regs::RegwenWriteVal`]"]
    #[inline(always)]
    pub fn regwen(&self) -> ureg::RegRef<crate::meta::Regwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command request register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn cmd_req(&self) -> ureg::RegRef<crate::meta::CmdReq, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "CSRNG maximum number of generate requests allowed between reseeds register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn reseed_interval(&self) -> ureg::RegRef<crate::meta::ReseedInterval, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Reseed counter.\n\nThe per-instance reseed counter indicates the number of Generate requests that have been completed since new entropy input has been obtained with an Instantiate or a Reseed command.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn reseed_counter(
        &self,
    ) -> ureg::Array<3, ureg::RegRef<crate::meta::ReseedCounter, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Application interface command status register\n\nRead value: [`regs::SwCmdStsReadVal`]; Write value: [`regs::SwCmdStsWriteVal`]"]
    #[inline(always)]
    pub fn sw_cmd_sts(&self) -> ureg::RegRef<crate::meta::SwCmdSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Generate bits returned valid register\n\nRead value: [`regs::GenbitsVldReadVal`]; Write value: [`regs::GenbitsVldWriteVal`]"]
    #[inline(always)]
    pub fn genbits_vld(&self) -> ureg::RegRef<crate::meta::GenbitsVld, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Generate bits returned register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn genbits(&self) -> ureg::RegRef<crate::meta::Genbits, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Internal state read enable register\n\nRead value: [`regs::IntStateReadEnableReadVal`]; Write value: [`regs::IntStateReadEnableWriteVal`]"]
    #[inline(always)]
    pub fn int_state_read_enable(&self) -> ureg::RegRef<crate::meta::IntStateReadEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Internal state read enable REGWEN register\n\nRead value: [`regs::IntStateReadEnableRegwenReadVal`]; Write value: [`regs::IntStateReadEnableRegwenWriteVal`]"]
    #[inline(always)]
    pub fn int_state_read_enable_regwen(
        &self,
    ) -> ureg::RegRef<crate::meta::IntStateReadEnableRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Internal state number register\n\nRead value: [`regs::IntStateNumReadVal`]; Write value: [`regs::IntStateNumWriteVal`]"]
    #[inline(always)]
    pub fn int_state_num(&self) -> ureg::RegRef<crate::meta::IntStateNum, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Internal state read access register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn int_state_val(&self) -> ureg::RegRef<crate::meta::IntStateVal, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "FIPS/CC compliance flag forcing register\n\nRead value: [`regs::FipsForceReadVal`]; Write value: [`regs::FipsForceWriteVal`]"]
    #[inline(always)]
    pub fn fips_force(&self) -> ureg::RegRef<crate::meta::FipsForce, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Hardware instance exception status register\n\nRead value: [`regs::HwExcStsReadVal`]; Write value: [`regs::HwExcStsWriteVal`]"]
    #[inline(always)]
    pub fn hw_exc_sts(&self) -> ureg::RegRef<crate::meta::HwExcSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Recoverable alert status register\n\nRead value: [`regs::RecovAlertStsReadVal`]; Write value: [`regs::RecovAlertStsWriteVal`]"]
    #[inline(always)]
    pub fn recov_alert_sts(&self) -> ureg::RegRef<crate::meta::RecovAlertSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Hardware detection of error conditions status register\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Test error conditions register\n\nRead value: [`regs::ErrCodeTestReadVal`]; Write value: [`regs::ErrCodeTestWriteVal`]"]
    #[inline(always)]
    pub fn err_code_test(&self) -> ureg::RegRef<crate::meta::ErrCodeTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Main state machine state debug register\n\nRead value: [`regs::MainSmStateReadVal`]; Write value: [`regs::MainSmStateWriteVal`]"]
    #[inline(always)]
    pub fn main_sm_state(&self) -> ureg::RegRef<crate::meta::MainSmState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
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
        pub fn recov_alert(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_alert(self, val: bool) -> Self {
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
    pub struct CtrlReadVal(u32);
    impl CtrlReadVal {
        #[doc = "Setting this field to kMultiBitBool4True will enable the CSRNG module. The modules\nof the entropy complex may only be enabled and disabled in a specific order, see\nProgrammers Guide for details."]
        #[inline(always)]
        pub fn enable(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable reading from the !!GENBITS register.\nThis application interface for software (register based) will be enabled\nonly if the otp_en_csrng_sw_app_read input vector is set to the enable encoding."]
        #[inline(always)]
        pub fn sw_app_enable(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable reading from the !!INT_STATE_VAL register.\nReading the internal state of the enable instances will be enabled\nonly if the otp_en_csrng_sw_app_read input vector is set to the enable encoding.\nAlso, the !!INT_STATE_READ_ENABLE bit of the selected instance needs to be set to true for this to work."]
        #[inline(always)]
        pub fn read_int_state(&self) -> u32 {
            (self.0 >> 8) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True enables forcing the FIPS/CC compliance flag to true via the !!FIPS_FORCE register."]
        #[inline(always)]
        pub fn fips_force_enable(&self) -> u32 {
            (self.0 >> 12) & 0xf
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
        #[doc = "Setting this field to kMultiBitBool4True will enable the CSRNG module. The modules\nof the entropy complex may only be enabled and disabled in a specific order, see\nProgrammers Guide for details."]
        #[inline(always)]
        pub fn enable(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable reading from the !!GENBITS register.\nThis application interface for software (register based) will be enabled\nonly if the otp_en_csrng_sw_app_read input vector is set to the enable encoding."]
        #[inline(always)]
        pub fn sw_app_enable(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable reading from the !!INT_STATE_VAL register.\nReading the internal state of the enable instances will be enabled\nonly if the otp_en_csrng_sw_app_read input vector is set to the enable encoding.\nAlso, the !!INT_STATE_READ_ENABLE bit of the selected instance needs to be set to true for this to work."]
        #[inline(always)]
        pub fn read_int_state(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 8)) | ((val & 0xf) << 8))
        }
        #[doc = "Setting this field to kMultiBitBool4True enables forcing the FIPS/CC compliance flag to true via the !!FIPS_FORCE register."]
        #[inline(always)]
        pub fn fips_force_enable(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 12)) | ((val & 0xf) << 12))
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
    pub struct ErrCodeReadVal(u32);
    impl ErrCodeReadVal {
        #[doc = "This bit will be set to one when an error has been detected for the\ncommand stage command FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_cmd_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ncommand stage genbits FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_genbits_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ncmdreq FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_cmdreq_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nrcstage FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_rcstage_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nkeyvrc FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_keyvrc_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nupdreq FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_updreq_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nbencreq FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_bencreq_err(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nbencack FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_bencack_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\npdata FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_pdata_err(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nfinal FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_final_err(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ngbencack FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_gbencack_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ngrcstage FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_grcstage_err(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nggenreq FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_ggenreq_err(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ngadstage FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_gadstage_err(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nggenbits FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_ggenbits_err(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\nblkenc FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_blkenc_err(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\ncommand stage state machine. This error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn cmd_stage_sm_err(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nmain state machine. This error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn main_sm_err(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nctr_drbg gen state machine. This error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn drbg_gen_sm_err(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nctr_drbg update block encode state machine. This error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn drbg_updbe_sm_err(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nctr_drbg update out block state machine. This error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn drbg_updob_sm_err(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "This bit will be set to one when an AES fatal error has been detected.\nThis error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn aes_cipher_sm_err(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "This bit will be set to one when a mismatch in any of the hardened counters\nhas been detected.\nThis error will signal a fatal alert, and also\nan interrupt if enabled.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn cmd_gen_cnt_err(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 15 of this\nthis register) are asserted as a result of an error pulse generated from\nany full FIFO that has been recieved a write pulse.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn fifo_write_err(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 15 of this\nthis register) are asserted as a result of an error pulse generated from\nany empty FIFO that has recieved a read pulse.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn fifo_read_err(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 15 of this\nthis register) are asserted as a result of an error pulse generated from\nany FIFO where both the empty and full status bits are set.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn fifo_state_err(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
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
    pub struct ErrCodeTestReadVal(u32);
    impl ErrCodeTestReadVal {
        #[doc = "Setting this field will set the bit number for which an error\nwill be forced in the hardware. This bit number is that same one\nfound in the !!ERR_CODE register. The action of writing this\nregister will force an error pulse. The sole purpose of this\nregister is to test that any error properly propagates to either\nan interrupt or an alert."]
        #[inline(always)]
        pub fn err_code_test(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrCodeTestWriteVal {
            ErrCodeTestWriteVal(self.0)
        }
    }
    impl From<u32> for ErrCodeTestReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrCodeTestReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrCodeTestReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrCodeTestWriteVal(u32);
    impl ErrCodeTestWriteVal {
        #[doc = "Setting this field will set the bit number for which an error\nwill be forced in the hardware. This bit number is that same one\nfound in the !!ERR_CODE register. The action of writing this\nregister will force an error pulse. The sole purpose of this\nregister is to test that any error properly propagates to either\nan interrupt or an alert."]
        #[inline(always)]
        pub fn err_code_test(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 0)) | ((val & 0x1f) << 0))
        }
    }
    impl From<u32> for ErrCodeTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrCodeTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrCodeTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FipsForceReadVal(u32);
    impl FipsForceReadVal {
        #[doc = "Force the FIPS/CC compliance flag of individual instances to true.\nThis allows CSRNG to set the output FIPS/CC compliance flag to true despite running in fully deterministic mode (flag0 being true).\nThis can be useful e.g. for known-answer testing through entropy consumers accepting FIPS/CC compliant entropy only, or when firmware is used to derive FIPS/CC compliant entropy seeds.\nAfter setting a particular bit to 1, the FIPS/CC compliance flag of the corresponding instance will be forced to true upon the next Instantiate or Reseed command.\n\nNote that for this to work, !!CTRL.FIPS_FORCE_ENABLE needs to be set to kMultiBitBool4True."]
        #[inline(always)]
        pub fn fips_force(&self) -> u32 {
            (self.0 >> 0) & 7
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FipsForceWriteVal {
            FipsForceWriteVal(self.0)
        }
    }
    impl From<u32> for FipsForceReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FipsForceReadVal> for u32 {
        #[inline(always)]
        fn from(val: FipsForceReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FipsForceWriteVal(u32);
    impl FipsForceWriteVal {
        #[doc = "Force the FIPS/CC compliance flag of individual instances to true.\nThis allows CSRNG to set the output FIPS/CC compliance flag to true despite running in fully deterministic mode (flag0 being true).\nThis can be useful e.g. for known-answer testing through entropy consumers accepting FIPS/CC compliant entropy only, or when firmware is used to derive FIPS/CC compliant entropy seeds.\nAfter setting a particular bit to 1, the FIPS/CC compliance flag of the corresponding instance will be forced to true upon the next Instantiate or Reseed command.\n\nNote that for this to work, !!CTRL.FIPS_FORCE_ENABLE needs to be set to kMultiBitBool4True."]
        #[inline(always)]
        pub fn fips_force(self, val: u32) -> Self {
            Self((self.0 & !(7 << 0)) | ((val & 7) << 0))
        }
    }
    impl From<u32> for FipsForceWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FipsForceWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FipsForceWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct GenbitsVldReadVal(u32);
    impl GenbitsVldReadVal {
        #[doc = "This bit is set when genbits are available on this application interface after a generate command has been issued."]
        #[inline(always)]
        pub fn genbits_vld(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit is set when genbits are FIPS/CC compliant."]
        #[inline(always)]
        pub fn genbits_fips(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
    }
    impl From<u32> for GenbitsVldReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<GenbitsVldReadVal> for u32 {
        #[inline(always)]
        fn from(val: GenbitsVldReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwExcStsReadVal(u32);
    impl HwExcStsReadVal {
        #[doc = "Reading this register indicates whether one of the CSRNG HW instances has\nencountered an exception.  Each bit corresponds to a particular hardware\ninstance, with bit 0 corresponding to instance HW0, bit 1 corresponding\nto instance HW1, and so forth. (To monitor the status of requests made\nto the SW instance, check the !!SW_CMD_STS register). Writing a zero to this register\nresets the status bits."]
        #[inline(always)]
        pub fn hw_exc_sts(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> HwExcStsWriteVal {
            HwExcStsWriteVal(self.0)
        }
    }
    impl From<u32> for HwExcStsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwExcStsReadVal> for u32 {
        #[inline(always)]
        fn from(val: HwExcStsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HwExcStsWriteVal(u32);
    impl HwExcStsWriteVal {
        #[doc = "Reading this register indicates whether one of the CSRNG HW instances has\nencountered an exception.  Each bit corresponds to a particular hardware\ninstance, with bit 0 corresponding to instance HW0, bit 1 corresponding\nto instance HW1, and so forth. (To monitor the status of requests made\nto the SW instance, check the !!SW_CMD_STS register). Writing a zero to this register\nresets the status bits."]
        #[inline(always)]
        pub fn hw_exc_sts(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
    }
    impl From<u32> for HwExcStsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwExcStsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HwExcStsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.cs_cmd_req_done is set."]
        #[inline(always)]
        pub fn cs_cmd_req_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_entropy_req is set."]
        #[inline(always)]
        pub fn cs_entropy_req(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_hw_inst_exc is set."]
        #[inline(always)]
        pub fn cs_hw_inst_exc(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_fatal_err is set."]
        #[inline(always)]
        pub fn cs_fatal_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.cs_cmd_req_done is set."]
        #[inline(always)]
        pub fn cs_cmd_req_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_entropy_req is set."]
        #[inline(always)]
        pub fn cs_entropy_req(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_hw_inst_exc is set."]
        #[inline(always)]
        pub fn cs_hw_inst_exc(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cs_fatal_err is set."]
        #[inline(always)]
        pub fn cs_fatal_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
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
        #[doc = "Asserted when a command request is completed."]
        #[inline(always)]
        pub fn cs_cmd_req_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Asserted when a request for entropy has been made."]
        #[inline(always)]
        pub fn cs_entropy_req(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Asserted when a hardware-attached CSRNG instance encounters a command exception"]
        #[inline(always)]
        pub fn cs_hw_inst_exc(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Asserted when a FIFO error or a fatal alert occurs. Check the !!ERR_CODE register to get more information."]
        #[inline(always)]
        pub fn cs_fatal_err(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
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
        #[doc = "Asserted when a command request is completed."]
        #[inline(always)]
        pub fn cs_cmd_req_done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Asserted when a request for entropy has been made."]
        #[inline(always)]
        pub fn cs_entropy_req_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Asserted when a hardware-attached CSRNG instance encounters a command exception"]
        #[inline(always)]
        pub fn cs_hw_inst_exc_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Asserted when a FIFO error or a fatal alert occurs. Check the !!ERR_CODE register to get more information."]
        #[inline(always)]
        pub fn cs_fatal_err_clear(self) -> Self {
            Self(self.0 | (1 << 3))
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
        #[doc = "Write 1 to force !!INTR_STATE.cs_cmd_req_done to 1."]
        #[inline(always)]
        pub fn cs_cmd_req_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.cs_entropy_req to 1."]
        #[inline(always)]
        pub fn cs_entropy_req(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.cs_hw_inst_exc to 1."]
        #[inline(always)]
        pub fn cs_hw_inst_exc(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.cs_fatal_err to 1."]
        #[inline(always)]
        pub fn cs_fatal_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
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
    pub struct IntStateNumReadVal(u32);
    impl IntStateNumReadVal {
        #[doc = "Setting this field will set the number for which internal state can be\nselected for a read access. Up to 16 internal state values can be chosen\nfrom this register. The actual number of valid internal state fields\nis set by parameter NHwApps plus 1 software app. For those selections that point\nto reserved locations (greater than NHwApps plus 1), the returned value\nwill be zero. Writing this register will also reset the internal read\npointer for the !!INT_STATE_VAL register.\nNote: This register should be read back after being written to ensure\nthat the !!INT_STATE_VAL read back is accurate."]
        #[inline(always)]
        pub fn int_state_num(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntStateNumWriteVal {
            IntStateNumWriteVal(self.0)
        }
    }
    impl From<u32> for IntStateNumReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateNumReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateNumReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntStateNumWriteVal(u32);
    impl IntStateNumWriteVal {
        #[doc = "Setting this field will set the number for which internal state can be\nselected for a read access. Up to 16 internal state values can be chosen\nfrom this register. The actual number of valid internal state fields\nis set by parameter NHwApps plus 1 software app. For those selections that point\nto reserved locations (greater than NHwApps plus 1), the returned value\nwill be zero. Writing this register will also reset the internal read\npointer for the !!INT_STATE_VAL register.\nNote: This register should be read back after being written to ensure\nthat the !!INT_STATE_VAL read back is accurate."]
        #[inline(always)]
        pub fn int_state_num(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
    }
    impl From<u32> for IntStateNumWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateNumWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateNumWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntStateReadEnableReadVal(u32);
    impl IntStateReadEnableReadVal {
        #[doc = "Per-instance internal state read enable.\nDefines whether the internal state of the corresponding instance is readable via !!INT_STATE_VAL.\nNote that for !!INT_STATE_VAL to provide read access to the internal state, also !!CTRL.READ_INT_STATE needs to be set to `kMultiBitBool4True`.\nIn addition, the otp_en_csrng_sw_app_read input needs to be set to `kMultiBitBool8True`."]
        #[inline(always)]
        pub fn int_state_read_enable(&self) -> u32 {
            (self.0 >> 0) & 7
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntStateReadEnableWriteVal {
            IntStateReadEnableWriteVal(self.0)
        }
    }
    impl From<u32> for IntStateReadEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateReadEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateReadEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntStateReadEnableWriteVal(u32);
    impl IntStateReadEnableWriteVal {
        #[doc = "Per-instance internal state read enable.\nDefines whether the internal state of the corresponding instance is readable via !!INT_STATE_VAL.\nNote that for !!INT_STATE_VAL to provide read access to the internal state, also !!CTRL.READ_INT_STATE needs to be set to `kMultiBitBool4True`.\nIn addition, the otp_en_csrng_sw_app_read input needs to be set to `kMultiBitBool8True`."]
        #[inline(always)]
        pub fn int_state_read_enable(self, val: u32) -> Self {
            Self((self.0 & !(7 << 0)) | ((val & 7) << 0))
        }
    }
    impl From<u32> for IntStateReadEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateReadEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateReadEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntStateReadEnableRegwenReadVal(u32);
    impl IntStateReadEnableRegwenReadVal {
        #[doc = "INT_STATE_READ_ENABLE register configuration enable bit.\nIf this is cleared to 0, the INT_STATE_READ_ENABLE register cannot be written anymore."]
        #[inline(always)]
        pub fn int_state_read_enable_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntStateReadEnableRegwenWriteVal {
            IntStateReadEnableRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for IntStateReadEnableRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateReadEnableRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateReadEnableRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntStateReadEnableRegwenWriteVal(u32);
    impl IntStateReadEnableRegwenWriteVal {
        #[doc = "INT_STATE_READ_ENABLE register configuration enable bit.\nIf this is cleared to 0, the INT_STATE_READ_ENABLE register cannot be written anymore."]
        #[inline(always)]
        pub fn int_state_read_enable_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for IntStateReadEnableRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntStateReadEnableRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntStateReadEnableRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct MainSmStateReadVal(u32);
    impl MainSmStateReadVal {
        #[doc = "This is the state of the CSRNG main state machine.\nSee the RTL file `csrng_main_sm` for the meaning of the values."]
        #[inline(always)]
        pub fn main_sm_state(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
    }
    impl From<u32> for MainSmStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<MainSmStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: MainSmStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovAlertStsReadVal(u32);
    impl RecovAlertStsReadVal {
        #[doc = "This bit is set when the ENABLE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn enable_field_alert(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit is set when the SW_APP_ENABLE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn sw_app_enable_field_alert(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit is set when the READ_INT_STATE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn read_int_state_field_alert(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit is set when the FIPS_FORCE_ENABLE field in the !!CTRL register is set to a value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn fips_force_enable_field_alert(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit is set when the FLAG0 field in the Application Command is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn acmd_flag0_field_alert(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit is set when the software application port genbits bus value is equal\nto the prior valid value on the bus, indicating a possible attack.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cs_bus_cmp_alert(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This bit is set when an unsupported/illegal CSRNG command is received by the\nmain state machine.\nThe invalid command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_invalid_acmd_alert(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "This bit is set when an out of order command is received by the main state machine.\nThis happens when an instantiate command is sent for a state that was already\ninstantiated or when any command other than instantiate is sent for a state that\nwasn't instantiated yet.\nThe invalid command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_invalid_cmd_seq_alert(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "This bit is set when the maximum number of generate requests between reseeds is\nexceeded.\nThe invalid generate command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_reseed_cnt_alert(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RecovAlertStsWriteVal {
            RecovAlertStsWriteVal(self.0)
        }
    }
    impl From<u32> for RecovAlertStsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovAlertStsReadVal> for u32 {
        #[inline(always)]
        fn from(val: RecovAlertStsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RecovAlertStsWriteVal(u32);
    impl RecovAlertStsWriteVal {
        #[doc = "This bit is set when the ENABLE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn enable_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "This bit is set when the SW_APP_ENABLE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn sw_app_enable_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "This bit is set when the READ_INT_STATE field in the !!CTRL register is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn read_int_state_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "This bit is set when the FIPS_FORCE_ENABLE field in the !!CTRL register is set to a value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn fips_force_enable_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "This bit is set when the FLAG0 field in the Application Command is set to\na value other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn acmd_flag0_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 4))
        }
        #[doc = "This bit is set when the software application port genbits bus value is equal\nto the prior valid value on the bus, indicating a possible attack.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cs_bus_cmp_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 12))
        }
        #[doc = "This bit is set when an unsupported/illegal CSRNG command is received by the\nmain state machine.\nThe invalid command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_invalid_acmd_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 13))
        }
        #[doc = "This bit is set when an out of order command is received by the main state machine.\nThis happens when an instantiate command is sent for a state that was already\ninstantiated or when any command other than instantiate is sent for a state that\nwasn't instantiated yet.\nThe invalid command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_invalid_cmd_seq_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 14))
        }
        #[doc = "This bit is set when the maximum number of generate requests between reseeds is\nexceeded.\nThe invalid generate command is ignored and CSRNG continues to operate.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_stage_reseed_cnt_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 15))
        }
    }
    impl From<u32> for RecovAlertStsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RecovAlertStsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: RecovAlertStsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RegwenReadVal(u32);
    impl RegwenReadVal {
        #[doc = "When true, all writeable registers can be modified.\nWhen false, they become read-only."]
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
        #[doc = "When true, all writeable registers can be modified.\nWhen false, they become read-only."]
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
    #[derive(Clone, Copy)]
    pub struct SwCmdStsReadVal(u32);
    impl SwCmdStsReadVal {
        #[doc = "This bit indicates when the command interface is ready to accept commands.\nBefore starting to write a new command to !!SW_CMD_REQ, this field needs to be polled.\n0b0: CSRNG is not ready to accept commands or the last command hasn't been acked yet.\n0b1: CSRNG is ready to accept the next command."]
        #[inline(always)]
        pub fn cmd_rdy(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This one bit field indicates when a SW command has been acknowledged by the CSRNG.\nIt is set to low each time a new command is written to !!CMD_REQ.\nThe field is set to high once a SW command request has been acknowledged by the CSRNG.\n0b0: The last SW command has not been acknowledged yet.\n0b1: The last SW command has been acknowledged.\nIn case of a generate command the acknowledgement goes high after all of the requested entropy is consumed."]
        #[inline(always)]
        pub fn cmd_ack(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This field represents the status code returned with the application command ack.\nIt is updated each time a command ack is asserted on the internal application\ninterface for software use.\nTo check whether a command was successful, wait for !!INTR_STATE.CS_CMD_REQ_DONE or\n!!SW_CMD_STS.CMD_ACK to be high and then check the value of this field."]
        #[inline(always)]
        pub fn cmd_sts(&self) -> super::enums::CmdSts {
            super::enums::CmdSts::try_from((self.0 >> 3) & 7).unwrap()
        }
    }
    impl From<u32> for SwCmdStsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SwCmdStsReadVal> for u32 {
        #[inline(always)]
        fn from(val: SwCmdStsReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum CmdSts {
        Success = 0,
        InvalidAcmd = 1,
        InvalidGenCmd = 2,
        InvalidCmdSeq = 3,
        ReseedCntExceeded = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl CmdSts {
        #[inline(always)]
        pub fn success(&self) -> bool {
            *self == Self::Success
        }
        #[inline(always)]
        pub fn invalid_acmd(&self) -> bool {
            *self == Self::InvalidAcmd
        }
        #[inline(always)]
        pub fn invalid_gen_cmd(&self) -> bool {
            *self == Self::InvalidGenCmd
        }
        #[inline(always)]
        pub fn invalid_cmd_seq(&self) -> bool {
            *self == Self::InvalidCmdSeq
        }
        #[inline(always)]
        pub fn reseed_cnt_exceeded(&self) -> bool {
            *self == Self::ReseedCntExceeded
        }
    }
    impl TryFrom<u32> for CmdSts {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<CmdSts, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, CmdSts>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<CmdSts> for u32 {
        fn from(val: CmdSts) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct CmdStsSelector();
        impl CmdStsSelector {
            #[inline(always)]
            pub fn success(&self) -> super::CmdSts {
                super::CmdSts::Success
            }
            #[inline(always)]
            pub fn invalid_acmd(&self) -> super::CmdSts {
                super::CmdSts::InvalidAcmd
            }
            #[inline(always)]
            pub fn invalid_gen_cmd(&self) -> super::CmdSts {
                super::CmdSts::InvalidGenCmd
            }
            #[inline(always)]
            pub fn invalid_cmd_seq(&self) -> super::CmdSts {
                super::CmdSts::InvalidCmdSeq
            }
            #[inline(always)]
            pub fn reseed_cnt_exceeded(&self) -> super::CmdSts {
                super::CmdSts::ReseedCntExceeded
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
    pub type Regwen =
        ureg::ReadWriteReg32<1, crate::regs::RegwenReadVal, crate::regs::RegwenWriteVal>;
    pub type Ctrl =
        ureg::ReadWriteReg32<0x9999, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type CmdReq = ureg::WriteOnlyReg32<0, u32>;
    pub type ReseedInterval = ureg::ReadWriteReg32<0xffffffff, u32, u32>;
    pub type ReseedCounter = ureg::ReadOnlyReg32<u32>;
    pub type SwCmdSts = ureg::ReadOnlyReg32<crate::regs::SwCmdStsReadVal>;
    pub type GenbitsVld = ureg::ReadOnlyReg32<crate::regs::GenbitsVldReadVal>;
    pub type Genbits = ureg::ReadOnlyReg32<u32>;
    pub type IntStateReadEnable = ureg::ReadWriteReg32<
        7,
        crate::regs::IntStateReadEnableReadVal,
        crate::regs::IntStateReadEnableWriteVal,
    >;
    pub type IntStateReadEnableRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::IntStateReadEnableRegwenReadVal,
        crate::regs::IntStateReadEnableRegwenWriteVal,
    >;
    pub type IntStateNum =
        ureg::ReadWriteReg32<0, crate::regs::IntStateNumReadVal, crate::regs::IntStateNumWriteVal>;
    pub type IntStateVal = ureg::ReadOnlyReg32<u32>;
    pub type FipsForce =
        ureg::ReadWriteReg32<0, crate::regs::FipsForceReadVal, crate::regs::FipsForceWriteVal>;
    pub type HwExcSts =
        ureg::ReadWriteReg32<0, crate::regs::HwExcStsReadVal, crate::regs::HwExcStsWriteVal>;
    pub type RecovAlertSts = ureg::ReadWriteReg32<
        0,
        crate::regs::RecovAlertStsReadVal,
        crate::regs::RecovAlertStsWriteVal,
    >;
    pub type ErrCode = ureg::ReadOnlyReg32<crate::regs::ErrCodeReadVal>;
    pub type ErrCodeTest =
        ureg::ReadWriteReg32<0, crate::regs::ErrCodeTestReadVal, crate::regs::ErrCodeTestWriteVal>;
    pub type MainSmState = ureg::ReadOnlyReg32<crate::regs::MainSmStateReadVal>;
}
