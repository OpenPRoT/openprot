#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Edn0 {
    _priv: (),
}
impl Edn0 {
    pub const PTR: *mut u32 = 0x41170000 as *mut u32;
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
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Edn1 {
    _priv: (),
}
impl Edn1 {
    pub const PTR: *mut u32 = 0x41180000 as *mut u32;
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
    #[doc = "EDN control register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN boot instantiate command register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn boot_ins_cmd(&self) -> ureg::RegRef<crate::meta::BootInsCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN boot generate command register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn boot_gen_cmd(&self) -> ureg::RegRef<crate::meta::BootGenCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN csrng app command request register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn sw_cmd_req(&self) -> ureg::RegRef<crate::meta::SwCmdReq, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN software command status register\n\nRead value: [`regs::SwCmdStsReadVal`]; Write value: [`regs::SwCmdStsWriteVal`]"]
    #[inline(always)]
    pub fn sw_cmd_sts(&self) -> ureg::RegRef<crate::meta::SwCmdSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN hardware command status register\n\nRead value: [`regs::HwCmdStsReadVal`]; Write value: [`regs::HwCmdStsWriteVal`]"]
    #[inline(always)]
    pub fn hw_cmd_sts(&self) -> ureg::RegRef<crate::meta::HwCmdSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN csrng reseed command register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn reseed_cmd(&self) -> ureg::RegRef<crate::meta::ReseedCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN csrng generate command register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn generate_cmd(&self) -> ureg::RegRef<crate::meta::GenerateCmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "EDN maximum number of requests between reseeds register\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn max_num_reqs_between_reseeds(
        &self,
    ) -> ureg::RegRef<crate::meta::MaxNumReqsBetweenReseeds, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Recoverable alert status register\n\nRead value: [`regs::RecovAlertStsReadVal`]; Write value: [`regs::RecovAlertStsWriteVal`]"]
    #[inline(always)]
    pub fn recov_alert_sts(&self) -> ureg::RegRef<crate::meta::RecovAlertSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Hardware detection of fatal error conditions status register\n\nRead value: [`regs::ErrCodeReadVal`]; Write value: [`regs::ErrCodeWriteVal`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Test error conditions register\n\nRead value: [`regs::ErrCodeTestReadVal`]; Write value: [`regs::ErrCodeTestWriteVal`]"]
    #[inline(always)]
    pub fn err_code_test(&self) -> ureg::RegRef<crate::meta::ErrCodeTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Main state machine state observation register\n\nRead value: [`regs::MainSmStateReadVal`]; Write value: [`regs::MainSmStateWriteVal`]"]
    #[inline(always)]
    pub fn main_sm_state(&self) -> ureg::RegRef<crate::meta::MainSmState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
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
        #[doc = "Setting this field to kMultiBitBool4True enables the EDN module. The modules of the\nentropy complex may only be enabled and disabled in a specific order, see\nProgrammers Guide for details."]
        #[inline(always)]
        pub fn edn_enable(&self) -> u32 {
            (self.0 >> 0) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable the feature where the EDN block\nwill automatically send a boot-time request to the CSRNG application interface.\nThe purpose of this feature is to request entropy as fast as possible after reset,\nand during chip boot-time."]
        #[inline(always)]
        pub fn boot_req_mode(&self) -> u32 {
            (self.0 >> 4) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable the EDN block to automatically\nsend another request to CSRNG application interface. It is assumed that a CSRNG\ninstantiate command will be issued using the !!SW_CMD_REQ register interface.\nWhen this command has an command ack returned from CSRNG, a\nnew generate command will be send out again without software intervention. It is\nexpected that the generate command will be sent repeatedly so that a continuous\nsupply of entropy can be delivered to the endpoints. Reseed commands will be sent\non a programmable basic between generate commands.\nThe !!GENERATE_CMD, !!RESEED_CMD, and !!MAX_NUM_REQS_BETWEEN_RESEEDS registers must\nset up before the !!SW_CMD_REQ register command is issued."]
        #[inline(always)]
        pub fn auto_req_mode(&self) -> u32 {
            (self.0 >> 8) & 0xf
        }
        #[doc = "Setting this field to kMultiBitBool4True clears the two command FIFOs: the\nRESEED_CMD FIFO and the GENERATE_CMD FIFO. This field must be\nset to the reset state by software before any further commands can be issued to\nthese FIFOs."]
        #[inline(always)]
        pub fn cmd_fifo_rst(&self) -> u32 {
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
        #[doc = "Setting this field to kMultiBitBool4True enables the EDN module. The modules of the\nentropy complex may only be enabled and disabled in a specific order, see\nProgrammers Guide for details."]
        #[inline(always)]
        pub fn edn_enable(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 0)) | ((val & 0xf) << 0))
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable the feature where the EDN block\nwill automatically send a boot-time request to the CSRNG application interface.\nThe purpose of this feature is to request entropy as fast as possible after reset,\nand during chip boot-time."]
        #[inline(always)]
        pub fn boot_req_mode(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 4)) | ((val & 0xf) << 4))
        }
        #[doc = "Setting this field to kMultiBitBool4True will enable the EDN block to automatically\nsend another request to CSRNG application interface. It is assumed that a CSRNG\ninstantiate command will be issued using the !!SW_CMD_REQ register interface.\nWhen this command has an command ack returned from CSRNG, a\nnew generate command will be send out again without software intervention. It is\nexpected that the generate command will be sent repeatedly so that a continuous\nsupply of entropy can be delivered to the endpoints. Reseed commands will be sent\non a programmable basic between generate commands.\nThe !!GENERATE_CMD, !!RESEED_CMD, and !!MAX_NUM_REQS_BETWEEN_RESEEDS registers must\nset up before the !!SW_CMD_REQ register command is issued."]
        #[inline(always)]
        pub fn auto_req_mode(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 8)) | ((val & 0xf) << 8))
        }
        #[doc = "Setting this field to kMultiBitBool4True clears the two command FIFOs: the\nRESEED_CMD FIFO and the GENERATE_CMD FIFO. This field must be\nset to the reset state by software before any further commands can be issued to\nthese FIFOs."]
        #[inline(always)]
        pub fn cmd_fifo_rst(self, val: u32) -> Self {
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
        #[doc = "This bit will be set to one when an error has been detected for the\nreseed command FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nWhen this bit is set, a fatal error condition will result."]
        #[inline(always)]
        pub fn sfifo_rescmd_err(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit will be set to one when an error has been detected for the\ngenerate command FIFO. The type of error is reflected in the type status\nbits (bits 28 through 30 of this register).\nWhen this bit is set, a fatal error condition will result.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn sfifo_gencmd_err(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nEDN ack stage state machine. This error will signal a fatal alert.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn edn_ack_sm_err(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "This bit will be set to one when an illegal state has been detected for the\nEDN main stage state machine. This error will signal a fatal alert.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn edn_main_sm_err(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "This bit will be set to one when a hardened counter has detected an error\ncondition. This error will signal a fatal alert.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn edn_cntr_err(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 1 of this register) are asserted as a result of an error pulse generated from any full FIFO that has received a write pulse.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn fifo_write_err(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 1 of this register) are asserted as a result of an error pulse generated from any empty FIFO that has received a read pulse.\nThis bit will stay set until the next reset."]
        #[inline(always)]
        pub fn fifo_read_err(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "This bit will be set to one when any of the source bits (bits 0 through 1 of this register) are asserted as a result of an error pulse generated from any FIFO where both the empty and full status bits are set or in case of error conditions inside the hardened counters.\nThis bit will stay set until the next reset."]
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
    pub struct HwCmdStsReadVal(u32);
    impl HwCmdStsReadVal {
        #[doc = "This one bit field indicates whether the EDN is in the hardware controlled boot mode.\n0b0: The EDN is not in boot mode.\n0b1: The EDN is in boot mode."]
        #[inline(always)]
        pub fn boot_mode(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This one bit field indicates whether the EDN is in the hardware controlled part of auto mode.\nThe instantiate command is issued via SW interface and is thus not part of the hardware controlled part of auto mode.\n0b0: The EDN is not in the hardware controlled part of auto mode.\n0b1: The EDN is in the hardware controlled part of auto mode."]
        #[inline(always)]
        pub fn auto_mode(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This field contains the application command type of the hardware controlled command issued last.\nThe application command selects one of five operations to perform.\nA description of the application command types can be found [here](../../csrng/doc/theory_of_operation.md#command-description)."]
        #[inline(always)]
        pub fn cmd_type(&self) -> u32 {
            (self.0 >> 2) & 0xf
        }
        #[doc = "This one bit field indicates when a HW command has been acknowledged by the CSRNG.\nIt is set to low each time a new command is sent to the CSRNG.\nThe field is set to high once a HW command request has been acknowledged by the CSRNG.\n0b0: The last HW command has not been acknowledged yet.\n0b1: The last HW command has been acknowledged."]
        #[inline(always)]
        pub fn cmd_ack(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This field represents the status code returned with the CSRNG application command ack.\nIt is updated each time a HW command is acknowledged by CSRNG.\nA description of the command status types can be found [here](../../csrng/doc/registers.md#sw_cmd_sts--cmd_sts)."]
        #[inline(always)]
        pub fn cmd_sts(&self) -> u32 {
            (self.0 >> 7) & 7
        }
    }
    impl From<u32> for HwCmdStsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HwCmdStsReadVal> for u32 {
        #[inline(always)]
        fn from(val: HwCmdStsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.edn_cmd_req_done is set."]
        #[inline(always)]
        pub fn edn_cmd_req_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.edn_fatal_err is set."]
        #[inline(always)]
        pub fn edn_fatal_err(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.edn_cmd_req_done is set."]
        #[inline(always)]
        pub fn edn_cmd_req_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.edn_fatal_err is set."]
        #[inline(always)]
        pub fn edn_fatal_err(self, val: bool) -> Self {
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
        #[doc = "Asserted when a software CSRNG request has completed."]
        #[inline(always)]
        pub fn edn_cmd_req_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Asserted when a FIFO error occurs."]
        #[inline(always)]
        pub fn edn_fatal_err(&self) -> bool {
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
        #[doc = "Asserted when a software CSRNG request has completed."]
        #[inline(always)]
        pub fn edn_cmd_req_done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Asserted when a FIFO error occurs."]
        #[inline(always)]
        pub fn edn_fatal_err_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.edn_cmd_req_done to 1."]
        #[inline(always)]
        pub fn edn_cmd_req_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.edn_fatal_err to 1."]
        #[inline(always)]
        pub fn edn_fatal_err(self, val: bool) -> Self {
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
    pub struct MainSmStateReadVal(u32);
    impl MainSmStateReadVal {
        #[doc = "This is the state of the EDN main state machine.\nSee the RTL file `edn_main_sm` for the meaning of the values."]
        #[inline(always)]
        pub fn main_sm_state(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
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
        #[doc = "This bit is set when the EDN_ENABLE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn edn_enable_field_alert(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit is set when the BOOT_REQ_MODE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn boot_req_mode_field_alert(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit is set when the !!CTRL.AUTO_REQ_MODE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn auto_req_mode_field_alert(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit is set when the CMD_FIFO_RST field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_fifo_rst_field_alert(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit is set when the interal entropy bus value is equal to the prior\nvalid value on the bus, indicating a possible attack.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn edn_bus_cmp_alert(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "This bit is set when the CSRNG returns an acknowledgement where the status\nsignal is high.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn csrng_ack_err(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
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
        #[doc = "This bit is set when the EDN_ENABLE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn edn_enable_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "This bit is set when the BOOT_REQ_MODE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn boot_req_mode_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "This bit is set when the !!CTRL.AUTO_REQ_MODE field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn auto_req_mode_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 2))
        }
        #[doc = "This bit is set when the CMD_FIFO_RST field is set to an illegal value,\nsomething other than kMultiBitBool4True or kMultiBitBool4False.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn cmd_fifo_rst_field_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 3))
        }
        #[doc = "This bit is set when the interal entropy bus value is equal to the prior\nvalid value on the bus, indicating a possible attack.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn edn_bus_cmp_alert_clear(self) -> Self {
            Self(self.0 & !(1 << 12))
        }
        #[doc = "This bit is set when the CSRNG returns an acknowledgement where the status\nsignal is high.\nWriting a zero resets this status bit."]
        #[inline(always)]
        pub fn csrng_ack_err_clear(self) -> Self {
            Self(self.0 & !(1 << 13))
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
        #[doc = "When true, the CTRL can be written by software.\nWhen false, this field read-only. Defaults true, write zero to clear.\nNote that this needs to be cleared after initial configuration at boot in order to\nlock in the listed register settings."]
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
        #[doc = "When true, the CTRL can be written by software.\nWhen false, this field read-only. Defaults true, write zero to clear.\nNote that this needs to be cleared after initial configuration at boot in order to\nlock in the listed register settings."]
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
        #[doc = "This bit indicates when !!SW_CMD_REQ is ready to accept the next word.\nThis bit has to be polled before each word of a command is written to !!SW_CMD_REQ.\n0b0: The EDN is not ready to accept the next word yet.\n0b1: The EDN is ready to accept the next word."]
        #[inline(always)]
        pub fn cmd_reg_rdy(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit indicates when the EDN is ready to accept the next command.\nBefore starting to write a new command to !!SW_CMD_REQ, this field needs to be polled.\n0b0: The EDN is not ready to accept commands or the last command hasn't been acked yet.\n0b1: The EDN is ready to accept the next command."]
        #[inline(always)]
        pub fn cmd_rdy(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This one bit field indicates when a SW command has been acknowledged by the CSRNG.\nIt is set to low each time a new command is written to !!SW_CMD_REQ.\nThe field is set to high once a SW command request has been acknowledged by the CSRNG.\n0b0: The last SW command has not been acknowledged yet.\n0b1: The last SW command has been acknowledged."]
        #[inline(always)]
        pub fn cmd_ack(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This field represents the status code returned with the CSRNG application command ack.\nIt is updated each time a SW command is acknowledged by CSRNG.\nTo check whether a command was successful, wait for !!INTR_STATE.EDN_CMD_REQ_DONE or\n!!SW_CMD_STS.CMD_ACK to be high and then check the value of this field.\nA description of the command status types can be found [here](../../csrng/doc/registers.md#sw_cmd_sts--cmd_sts)."]
        #[inline(always)]
        pub fn cmd_sts(&self) -> u32 {
            (self.0 >> 3) & 7
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
    pub type Regwen =
        ureg::ReadWriteReg32<1, crate::regs::RegwenReadVal, crate::regs::RegwenWriteVal>;
    pub type Ctrl =
        ureg::ReadWriteReg32<0x9999, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type BootInsCmd = ureg::ReadWriteReg32<0x901, u32, u32>;
    pub type BootGenCmd = ureg::ReadWriteReg32<0xfff003, u32, u32>;
    pub type SwCmdReq = ureg::WriteOnlyReg32<0, u32>;
    pub type SwCmdSts = ureg::ReadOnlyReg32<crate::regs::SwCmdStsReadVal>;
    pub type HwCmdSts = ureg::ReadOnlyReg32<crate::regs::HwCmdStsReadVal>;
    pub type ReseedCmd = ureg::WriteOnlyReg32<0, u32>;
    pub type GenerateCmd = ureg::WriteOnlyReg32<0, u32>;
    pub type MaxNumReqsBetweenReseeds = ureg::ReadWriteReg32<0, u32, u32>;
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
