#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Aes {
    _priv: (),
}
impl Aes {
    pub const PTR: *mut u32 = 0x41100000 as *mut u32;
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
    #[doc = "Initial Key Registers Share 0.\n\nThe actual initial key corresponds to Initial Key Registers Share 0 XORed with Initial Key Registers Share 1.\nLoaded into the internal Full Key register upon starting encryption/decryption of the next block.\nAll key registers (Share 0 and Share 1) must be written at least once when the key is changed, regardless of key length (write random data for unused bits).\nThe order in which the registers are updated does not matter.\nCan only be updated when the AES unit is idle.\nIf the AES unit is non-idle, writes to these registers are ignored.\nUpon reset, these registers are cleared with pseudo-random data.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key_share0(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::KeyShare0, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Initial Key Registers Share 1.\n\nThe actual initial key corresponds to Initial Key Registers Share 0 XORed with Initial Key Registers Share 1.\nLoaded into the internal Full Key register upon starting encryption/decryption of the next block.\nAll key registers (Share 0 and Share 1) must be written at least once when the key is changed, regardless of key length (write random data for unused bits).\nThe order in which the registers are updated does not matter.\nCan only be updated when the AES unit is idle.\nIf the AES unit is non-idle, writes to these registers are ignored.\nUpon reset, these registers are cleared with pseudo-random data.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key_share1(&self) -> ureg::Array<8, ureg::RegRef<crate::meta::KeyShare1, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Initialization Vector Registers.\n\nThe initialization vector (IV) or initial counter value must be written to these registers when starting a new message in CBC or CTR mode (see Control Register), respectively.\nIn CBC and CTR modes, the AES unit does not start encryption/decryption with a partially updated IV.\nEach register has to be written at least once.\nThe order in which the registers are written does not matter.\nIf the AES unit is non-idle, writes to these registers are ignored.\nWhenever starting a new message, the corresponding IV value must be provided by the processor.\nOnce started, the AES unit automatically updates the contents of these registers.\nIn ECB mode, the IV registers are not used and do not need to be configured.\nUpon reset, these registers are cleared with pseudo-random data.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn iv(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::Iv, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Input Data Registers.\n\nIf MANUAL_OPERATION=0 (see Control Register), the AES unit automatically starts encryption/decryption after all Input Data registers have been written.\nEach register has to be written at least once.\nThe order in which the registers are written does not matter.\nLoaded into the internal State register upon starting encryption/decryption of the next block.\nAfter that, the processor can update the Input Data registers (See INPUT_READY field of Status Register).\nUpon reset, these registers are cleared with pseudo-random data.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn data_in(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::DataIn, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Output Data Register.\n\nHolds the output data produced by the AES unit during the last encryption/decryption operation.\nIf MANUAL_OPERATION=0 (see Control Register), the AES unit is stalled when the previous output data has not yet been read and is about to be overwritten.\nEach register has to be read at least once.\nThe order in which the registers are read does not matter.\nUpon reset, these registers are cleared with pseudo-random data.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn data_out(&self) -> ureg::Array<4, ureg::RegRef<crate::meta::DataOut, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control Register.\n\nCan only be updated when the AES unit is idle.\nIf the AES unit is non-idle, writes to this register are ignored.\nThis register is shadowed, meaning two subsequent write operations are required to change its content.\nIf the two write operations try to set a different value, a recoverable alert is triggered (See Status Register).\nA read operation clears the internal phase tracking: The next write operation is always considered a first write operation of an update sequence.\nAny write operation to this register will clear the status tracking required for automatic mode (See MANUAL_OPERATION field).\nA write to the Control Register is considered the start of a new message.\nHence, software needs to provide new key, IV and input data afterwards.\n\nRead value: [`regs::CtrlShadowedReadVal`]; Write value: [`regs::CtrlShadowedWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_shadowed(&self) -> ureg::RegRef<crate::meta::CtrlShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Auxiliary Control Register.\n\nThis register is shadowed, meaning two subsequent write operations are required to change its content.\nIf the two write operations try to set a different value, a recoverable alert is triggered (See Status Register).\nA read operation clears the internal phase tracking: The next write operation is always considered a first write operation of an update sequence.\n\nRead value: [`regs::CtrlAuxShadowedReadVal`]; Write value: [`regs::CtrlAuxShadowedWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_aux_shadowed(&self) -> ureg::RegRef<crate::meta::CtrlAuxShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Lock bit for Auxiliary Control Register.\n\nRead value: [`regs::CtrlAuxRegwenReadVal`]; Write value: [`regs::CtrlAuxRegwenWriteVal`]"]
    #[inline(always)]
    pub fn ctrl_aux_regwen(&self) -> ureg::RegRef<crate::meta::CtrlAuxRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Trigger Register.\n\nEach bit is individually cleared to zero when executing the corresponding trigger.\nWhile executing any of the triggered operations, the AES unit will set the IDLE bit in the Status Register to zero.\nThe processor must check the Status Register before triggering further actions.\nFor example, writes to Initial Key and IV Registers are ignored while the AES unit is busy.\nWrites to the Input Data Registers are not ignored but the data will be cleared if a KEY_IV_DATA_IN_CLEAR operation is pending.\n\nRead value: [`regs::TriggerReadVal`]; Write value: [`regs::TriggerWriteVal`]"]
    #[inline(always)]
    pub fn trigger(&self) -> ureg::RegRef<crate::meta::Trigger, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status Register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
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
        pub fn recov_ctrl_update_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to trigger one alert event of this kind."]
        #[inline(always)]
        pub fn fatal_fault(self, val: bool) -> Self {
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
    pub struct CtrlAuxRegwenReadVal(u32);
    impl CtrlAuxRegwenReadVal {
        #[doc = "Auxiliary Control Register configuration enable bit.\nIf this is cleared to 0, the Auxiliary Control Register cannot be written anymore."]
        #[inline(always)]
        pub fn ctrl_aux_regwen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlAuxRegwenWriteVal {
            CtrlAuxRegwenWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlAuxRegwenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlAuxRegwenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlAuxRegwenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlAuxRegwenWriteVal(u32);
    impl CtrlAuxRegwenWriteVal {
        #[doc = "Auxiliary Control Register configuration enable bit.\nIf this is cleared to 0, the Auxiliary Control Register cannot be written anymore."]
        #[inline(always)]
        pub fn ctrl_aux_regwen_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
    }
    impl From<u32> for CtrlAuxRegwenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlAuxRegwenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlAuxRegwenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlAuxShadowedReadVal(u32);
    impl CtrlAuxShadowedReadVal {
        #[doc = "Controls whether providing a new key triggers the reseeding of internal pseudo-random number generators used for clearing and masking (1) or not (0)."]
        #[inline(always)]
        pub fn key_touch_forces_reseed(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Allow the internal masking PRNG to advance (0) or force its internal state (1) leading to constant masks.\nSetting all masks to constant value can be useful when performing SCA.\nTo completely disable the masking, the second key share (KEY_SHARE1_0 - KEY_SHARE1_7) must be zero as well.\nIn addition, a special seed needs to be loaded into the masking PRNG using the EDN interface.\nOnly applicable if both the Masking parameter and the SecAllowForcingMasks parameter are set to one."]
        #[inline(always)]
        pub fn force_masks(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlAuxShadowedWriteVal {
            CtrlAuxShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlAuxShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlAuxShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlAuxShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlAuxShadowedWriteVal(u32);
    impl CtrlAuxShadowedWriteVal {
        #[doc = "Controls whether providing a new key triggers the reseeding of internal pseudo-random number generators used for clearing and masking (1) or not (0)."]
        #[inline(always)]
        pub fn key_touch_forces_reseed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Allow the internal masking PRNG to advance (0) or force its internal state (1) leading to constant masks.\nSetting all masks to constant value can be useful when performing SCA.\nTo completely disable the masking, the second key share (KEY_SHARE1_0 - KEY_SHARE1_7) must be zero as well.\nIn addition, a special seed needs to be loaded into the masking PRNG using the EDN interface.\nOnly applicable if both the Masking parameter and the SecAllowForcingMasks parameter are set to one."]
        #[inline(always)]
        pub fn force_masks(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for CtrlAuxShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlAuxShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlAuxShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlShadowedReadVal(u32);
    impl CtrlShadowedReadVal {
        #[doc = "2-bit one-hot field to select the operation of AES unit.\nInvalid input values, i.e., values with multiple bits set and value 2'b00, are mapped to AES_ENC (2'b01)."]
        #[inline(always)]
        pub fn operation(&self) -> super::enums::Operation {
            super::enums::Operation::try_from((self.0 >> 0) & 3).unwrap()
        }
        #[doc = "6-bit one-hot field to select AES block cipher mode.\nInvalid input values, i.e., values with multiple bits set and value 6'b00_0000, are mapped to AES_NONE (6'b10_0000)."]
        #[inline(always)]
        pub fn mode(&self) -> super::enums::Mode {
            super::enums::Mode::try_from((self.0 >> 2) & 0x3f).unwrap()
        }
        #[doc = "3-bit one-hot field to select AES key length.\nInvalid input values, i.e., values with multiple bits set, value 3'b000, and value 3'b010 in case 192-bit keys are not supported (because disabled at compile time) are mapped to AES_256 (3'b100)."]
        #[inline(always)]
        pub fn key_len(&self) -> super::enums::KeyLen {
            super::enums::KeyLen::try_from((self.0 >> 8) & 7).unwrap()
        }
        #[doc = "Controls whether the AES unit uses the key provided by the key manager via key sideload interface (1) or the key provided by software via Initial Key Registers KEY_SHARE1_0 - KEY_SHARE1_7 (0)."]
        #[inline(always)]
        pub fn sideload(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "3-bit one-hot field to control the reseeding rate of the internal pseudo-random number generator (PRNG) used for masking.\nInvalid input values, i.e., values with multiple bits set and value 3'b000 are mapped to the highest reseeding rate PER_1 (3'b001)."]
        #[inline(always)]
        pub fn prng_reseed_rate(&self) -> super::enums::PrngReseedRate {
            super::enums::PrngReseedRate::try_from((self.0 >> 12) & 7).unwrap()
        }
        #[doc = "Controls whether the AES unit is operated in normal/automatic mode (0) or fully manual mode (1).\nIn automatic mode (0), the AES unit automatically i) starts to encrypt/decrypt when it receives new input data, and ii) stalls during the last encryption/decryption cycle if the previous output data has not yet been read.\nThis is the most efficient mode to operate in.\nNote that the corresponding status tracking is automatically cleared upon a write to the Control Register.\nIn manual mode (1), the AES unit i) only starts to encrypt/decrypt after receiving a start trigger (see Trigger Register), and ii) overwrites previous output data irrespective of whether it has been read out or not.\nThis mode is useful if software needs full control over the AES unit."]
        #[inline(always)]
        pub fn manual_operation(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CtrlShadowedWriteVal {
            CtrlShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for CtrlShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlShadowedWriteVal(u32);
    impl CtrlShadowedWriteVal {
        #[doc = "2-bit one-hot field to select the operation of AES unit.\nInvalid input values, i.e., values with multiple bits set and value 2'b00, are mapped to AES_ENC (2'b01)."]
        #[inline(always)]
        pub fn operation(
            self,
            f: impl FnOnce(super::enums::selector::OperationSelector) -> super::enums::Operation,
        ) -> Self {
            Self(
                (self.0 & !(3 << 0))
                    | (u32::from(f(super::enums::selector::OperationSelector())) << 0),
            )
        }
        #[doc = "6-bit one-hot field to select AES block cipher mode.\nInvalid input values, i.e., values with multiple bits set and value 6'b00_0000, are mapped to AES_NONE (6'b10_0000)."]
        #[inline(always)]
        pub fn mode(
            self,
            f: impl FnOnce(super::enums::selector::ModeSelector) -> super::enums::Mode,
        ) -> Self {
            Self(
                (self.0 & !(0x3f << 2))
                    | (u32::from(f(super::enums::selector::ModeSelector())) << 2),
            )
        }
        #[doc = "3-bit one-hot field to select AES key length.\nInvalid input values, i.e., values with multiple bits set, value 3'b000, and value 3'b010 in case 192-bit keys are not supported (because disabled at compile time) are mapped to AES_256 (3'b100)."]
        #[inline(always)]
        pub fn key_len(
            self,
            f: impl FnOnce(super::enums::selector::KeyLenSelector) -> super::enums::KeyLen,
        ) -> Self {
            Self(
                (self.0 & !(7 << 8))
                    | (u32::from(f(super::enums::selector::KeyLenSelector())) << 8),
            )
        }
        #[doc = "Controls whether the AES unit uses the key provided by the key manager via key sideload interface (1) or the key provided by software via Initial Key Registers KEY_SHARE1_0 - KEY_SHARE1_7 (0)."]
        #[inline(always)]
        pub fn sideload(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "3-bit one-hot field to control the reseeding rate of the internal pseudo-random number generator (PRNG) used for masking.\nInvalid input values, i.e., values with multiple bits set and value 3'b000 are mapped to the highest reseeding rate PER_1 (3'b001)."]
        #[inline(always)]
        pub fn prng_reseed_rate(
            self,
            f: impl FnOnce(
                super::enums::selector::PrngReseedRateSelector,
            ) -> super::enums::PrngReseedRate,
        ) -> Self {
            Self(
                (self.0 & !(7 << 12))
                    | (u32::from(f(super::enums::selector::PrngReseedRateSelector())) << 12),
            )
        }
        #[doc = "Controls whether the AES unit is operated in normal/automatic mode (0) or fully manual mode (1).\nIn automatic mode (0), the AES unit automatically i) starts to encrypt/decrypt when it receives new input data, and ii) stalls during the last encryption/decryption cycle if the previous output data has not yet been read.\nThis is the most efficient mode to operate in.\nNote that the corresponding status tracking is automatically cleared upon a write to the Control Register.\nIn manual mode (1), the AES unit i) only starts to encrypt/decrypt after receiving a start trigger (see Trigger Register), and ii) overwrites previous output data irrespective of whether it has been read out or not.\nThis mode is useful if software needs full control over the AES unit."]
        #[inline(always)]
        pub fn manual_operation(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
    }
    impl From<u32> for CtrlShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CtrlShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CtrlShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "The AES unit is idle (1) or busy (0).\nThis flag is `0` if one of the following operations is currently running: i) encryption/decryption, ii) register clearing or iii) PRNG reseeding.\nThis flag is also `0` if an encryption/decryption is running but the AES unit is stalled."]
        #[inline(always)]
        pub fn idle(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The AES unit is not stalled (0) or stalled (1) because there is previous\noutput data that must be read by the processor before the AES unit can\noverwrite this data.\nThis flag is not meaningful if MANUAL_OPERATION=1 (see Control Register)."]
        #[inline(always)]
        pub fn stall(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "All previous output data has been fully read by the processor (0) or at least one previous output data block has been lost (1).\nIt has been overwritten by the AES unit before the processor could fully read it.\nOnce set to `1`, this flag remains set until AES operation is restarted by re-writing the Control Register.\nThe primary use of this flag is for design verification.\nThis flag is not meaningful if MANUAL_OPERATION=0 (see Control Register)."]
        #[inline(always)]
        pub fn output_lost(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "The AES unit has no valid output (0) or has valid output data (1)."]
        #[inline(always)]
        pub fn output_valid(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "The AES unit is ready (1) or not ready (0) to receive new data input via the DATA_IN registers.\nIf the present values in the DATA_IN registers have not yet been loaded into the\nmodule this flag is `0` (not ready)."]
        #[inline(always)]
        pub fn input_ready(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "An update error has not occurred (0) or has occurred (1) in the shadowed Control Register.\nAES operation needs to be restarted by re-writing the Control Register."]
        #[inline(always)]
        pub fn alert_recov_ctrl_update_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "No fatal fault has occurred inside the AES unit (0).\nA fatal fault has occurred and the AES unit needs to be reset (1).\nExamples for fatal faults include\ni) storage errors in the Control Register,\nii) if any internal FSM enters an invalid state,\niii) if any sparsely encoded signal takes on an invalid value,\niv) errors in the internal round counter,\nv) escalations triggered by the life cycle controller, and\nvi) fatal integrity failures on the TL-UL bus."]
        #[inline(always)]
        pub fn alert_fatal_fault(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
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
    pub struct TriggerWriteVal(u32);
    impl TriggerWriteVal {
        #[doc = "Keep AES unit paused (0) or trigger the encryption/decryption of one data block (1).\nThis trigger is cleared to `0` if MANUAL_OPERATION=0 or if MODE=AES_NONE (see Control Register)."]
        #[inline(always)]
        pub fn start(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Keep current values in Initial Key, internal Full Key and Decryption Key registers, IV registers and Input Data registers (0) or clear all those registers with pseudo-random data (1)."]
        #[inline(always)]
        pub fn key_iv_data_in_clear(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Keep current values in Output Data registers (0) or clear those registers with pseudo-random data (1)."]
        #[inline(always)]
        pub fn data_out_clear(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Keep continuing with the current states of the internal pseudo-random number generators used for register clearing and masking (0) or perform a reseed of the internal states from the connected entropy source (1).\nIf the KEY_TOUCH_FORCES_RESEED bit in the Auxiliary Control Register is set to one, this trigger will automatically get set after providing a new initial key."]
        #[inline(always)]
        pub fn prng_reseed(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for TriggerWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TriggerWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TriggerWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum KeyLen {
        Reserved0 = 0,
        Aes128 = 1,
        Aes192 = 2,
        Reserved3 = 3,
        Aes256 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl KeyLen {
        #[inline(always)]
        pub fn aes_128(&self) -> bool {
            *self == Self::Aes128
        }
        #[inline(always)]
        pub fn aes_192(&self) -> bool {
            *self == Self::Aes192
        }
        #[inline(always)]
        pub fn aes_256(&self) -> bool {
            *self == Self::Aes256
        }
    }
    impl TryFrom<u32> for KeyLen {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<KeyLen, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, KeyLen>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<KeyLen> for u32 {
        fn from(val: KeyLen) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Mode {
        Reserved0 = 0,
        AesEcb = 1,
        AesCbc = 2,
        Reserved3 = 3,
        AesCfb = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
        AesOfb = 8,
        Reserved9 = 9,
        Reserved10 = 10,
        Reserved11 = 11,
        Reserved12 = 12,
        Reserved13 = 13,
        Reserved14 = 14,
        Reserved15 = 15,
        AesCtr = 16,
        Reserved17 = 17,
        Reserved18 = 18,
        Reserved19 = 19,
        Reserved20 = 20,
        Reserved21 = 21,
        Reserved22 = 22,
        Reserved23 = 23,
        Reserved24 = 24,
        Reserved25 = 25,
        Reserved26 = 26,
        Reserved27 = 27,
        Reserved28 = 28,
        Reserved29 = 29,
        Reserved30 = 30,
        Reserved31 = 31,
        AesNone = 32,
        Reserved33 = 33,
        Reserved34 = 34,
        Reserved35 = 35,
        Reserved36 = 36,
        Reserved37 = 37,
        Reserved38 = 38,
        Reserved39 = 39,
        Reserved40 = 40,
        Reserved41 = 41,
        Reserved42 = 42,
        Reserved43 = 43,
        Reserved44 = 44,
        Reserved45 = 45,
        Reserved46 = 46,
        Reserved47 = 47,
        Reserved48 = 48,
        Reserved49 = 49,
        Reserved50 = 50,
        Reserved51 = 51,
        Reserved52 = 52,
        Reserved53 = 53,
        Reserved54 = 54,
        Reserved55 = 55,
        Reserved56 = 56,
        Reserved57 = 57,
        Reserved58 = 58,
        Reserved59 = 59,
        Reserved60 = 60,
        Reserved61 = 61,
        Reserved62 = 62,
        Reserved63 = 63,
    }
    impl Mode {
        #[inline(always)]
        pub fn aes_ecb(&self) -> bool {
            *self == Self::AesEcb
        }
        #[inline(always)]
        pub fn aes_cbc(&self) -> bool {
            *self == Self::AesCbc
        }
        #[inline(always)]
        pub fn aes_cfb(&self) -> bool {
            *self == Self::AesCfb
        }
        #[inline(always)]
        pub fn aes_ofb(&self) -> bool {
            *self == Self::AesOfb
        }
        #[inline(always)]
        pub fn aes_ctr(&self) -> bool {
            *self == Self::AesCtr
        }
        #[inline(always)]
        pub fn aes_none(&self) -> bool {
            *self == Self::AesNone
        }
    }
    impl TryFrom<u32> for Mode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Mode, ()> {
            if val < 0x40 {
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
    pub enum Operation {
        Reserved0 = 0,
        AesEnc = 1,
        AesDec = 2,
        Reserved3 = 3,
    }
    impl Operation {
        #[inline(always)]
        pub fn aes_enc(&self) -> bool {
            *self == Self::AesEnc
        }
        #[inline(always)]
        pub fn aes_dec(&self) -> bool {
            *self == Self::AesDec
        }
    }
    impl TryFrom<u32> for Operation {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Operation, ()> {
            if val < 4 {
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
    pub enum PrngReseedRate {
        Reserved0 = 0,
        Per1 = 1,
        Per64 = 2,
        Reserved3 = 3,
        Per8k = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl PrngReseedRate {
        #[inline(always)]
        pub fn per_1(&self) -> bool {
            *self == Self::Per1
        }
        #[inline(always)]
        pub fn per_64(&self) -> bool {
            *self == Self::Per64
        }
        #[inline(always)]
        pub fn per_8_k(&self) -> bool {
            *self == Self::Per8k
        }
    }
    impl TryFrom<u32> for PrngReseedRate {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<PrngReseedRate, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, PrngReseedRate>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<PrngReseedRate> for u32 {
        fn from(val: PrngReseedRate) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct KeyLenSelector();
        impl KeyLenSelector {
            #[inline(always)]
            pub fn aes_128(&self) -> super::KeyLen {
                super::KeyLen::Aes128
            }
            #[inline(always)]
            pub fn aes_192(&self) -> super::KeyLen {
                super::KeyLen::Aes192
            }
            #[inline(always)]
            pub fn aes_256(&self) -> super::KeyLen {
                super::KeyLen::Aes256
            }
        }
        pub struct ModeSelector();
        impl ModeSelector {
            #[inline(always)]
            pub fn aes_ecb(&self) -> super::Mode {
                super::Mode::AesEcb
            }
            #[inline(always)]
            pub fn aes_cbc(&self) -> super::Mode {
                super::Mode::AesCbc
            }
            #[inline(always)]
            pub fn aes_cfb(&self) -> super::Mode {
                super::Mode::AesCfb
            }
            #[inline(always)]
            pub fn aes_ofb(&self) -> super::Mode {
                super::Mode::AesOfb
            }
            #[inline(always)]
            pub fn aes_ctr(&self) -> super::Mode {
                super::Mode::AesCtr
            }
            #[inline(always)]
            pub fn aes_none(&self) -> super::Mode {
                super::Mode::AesNone
            }
        }
        pub struct OperationSelector();
        impl OperationSelector {
            #[inline(always)]
            pub fn aes_enc(&self) -> super::Operation {
                super::Operation::AesEnc
            }
            #[inline(always)]
            pub fn aes_dec(&self) -> super::Operation {
                super::Operation::AesDec
            }
        }
        pub struct PrngReseedRateSelector();
        impl PrngReseedRateSelector {
            #[inline(always)]
            pub fn per_1(&self) -> super::PrngReseedRate {
                super::PrngReseedRate::Per1
            }
            #[inline(always)]
            pub fn per_64(&self) -> super::PrngReseedRate {
                super::PrngReseedRate::Per64
            }
            #[inline(always)]
            pub fn per_8_k(&self) -> super::PrngReseedRate {
                super::PrngReseedRate::Per8k
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type KeyShare0 = ureg::WriteOnlyReg32<0, u32>;
    pub type KeyShare1 = ureg::WriteOnlyReg32<0, u32>;
    pub type Iv = ureg::ReadWriteReg32<0, u32, u32>;
    pub type DataIn = ureg::WriteOnlyReg32<0, u32>;
    pub type DataOut = ureg::ReadOnlyReg32<u32>;
    pub type CtrlShadowed = ureg::ReadWriteReg32<
        0x1181,
        crate::regs::CtrlShadowedReadVal,
        crate::regs::CtrlShadowedWriteVal,
    >;
    pub type CtrlAuxShadowed = ureg::ReadWriteReg32<
        1,
        crate::regs::CtrlAuxShadowedReadVal,
        crate::regs::CtrlAuxShadowedWriteVal,
    >;
    pub type CtrlAuxRegwen = ureg::ReadWriteReg32<
        1,
        crate::regs::CtrlAuxRegwenReadVal,
        crate::regs::CtrlAuxRegwenWriteVal,
    >;
    pub type Trigger = ureg::WriteOnlyReg32<0xe, crate::regs::TriggerWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
}
