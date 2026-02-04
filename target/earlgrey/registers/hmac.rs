#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Hmac {
    _priv: (),
}
impl Hmac {
    pub const PTR: *mut u32 = 0x41110000 as *mut u32;
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
    #[doc = "HMAC Configuration register.\n\nThe register is updated when the engine is in Idle.\nIf the software updates the register while the engine computes the hash, the updated value is discarded.\n\nRead value: [`regs::CfgReadVal`]; Write value: [`regs::CfgWriteVal`]"]
    #[inline(always)]
    pub fn cfg(&self) -> ureg::RegRef<crate::meta::Cfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "HMAC command register\n\nRead value: [`regs::CmdReadVal`]; Write value: [`regs::CmdWriteVal`]"]
    #[inline(always)]
    pub fn cmd(&self) -> ureg::RegRef<crate::meta::Cmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "HMAC Status register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "HMAC Error Code\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Clear internal secret registers.\n\nIf CPU writes a value into the register, the value is used to clear the internal variables such as the secret key, internal state machine, or hash value.\nThe clear secret operation overwrites the internal variables with the provided 32-bit value.\nFor SHA-2 384/512 that work with 64-bit words, the 32-bit value is duplicated and concatenated to generate the 64-bit value.\nIt is recommended to use a value extracted from an entropy source.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn wipe_secret(&self) -> ureg::RegRef<crate::meta::WipeSecret, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "HMAC Secret Key\n\nHMAC using SHA-2 256/384/512 assumes any hashed secret key length up to the block size, thus capped at 1024-bit.\n!!key_length determines how many of these registers are relevant for the HMAC operation. Order of the secret key is:\nkey[1023:0] = {KEY0, KEY1, KEY2, ... , KEY31};\n\nThe registers are allowed to be updated only when the engine is in Idle state.\nIf the engine computes the hash, it discards any attempts to update the secret keys and report an error.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key(&self) -> ureg::Array<32, ureg::RegRef<crate::meta::Key, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Digest output.\n\nIf HMAC is disabled, the register shows result of SHA-2 256/384/512.\nOrder of the 512-bit digest[511:0] = {DIGEST0, DIGEST1, DIGEST2, ... , DIGEST15}.\nFor SHA-2 256 order of the 256-bit digest[255:0] = {DIGEST0, DIGEST1, DIGEST2, DIGEST3, DIGEST4, DIGEST5, DIGEST6, DIGEST7} and {DIGEST8 - DIGEST15} are irrelevant and should not be read out.\nFor SHA-2 384, {DIGEST12-DIGEST15} are truncated; they are irrelevant and should not be read out.\n\nThe digest gets cleared when `CFG.sha_en` transitions from 1 to 0.\nWhen `STATUS.hmac_idle` is 1, these registers may be written to by software.\nOutside of this window, writes can cause unpredictable behavior.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn digest(&self) -> ureg::Array<16, ureg::RegRef<crate::meta::Digest, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xa4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Received Message Length calculated by the HMAC in bits [31:0]\n\nMessage is byte granularity.\nLower 3 bits [2:0] are ignored.\n\nWhen `STATUS.hmac_idle` is 1, this register may be written by software.\nOutside of this window, writes can cause unpredictable behavior.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn msg_length_lower(&self) -> ureg::RegRef<crate::meta::MsgLengthLower, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Received Message Length calculated by the HMAC in bits [63:32]\n\nWhen `STATUS.hmac_idle` is 1, this register may be written by software.\nOutside of this window, writes can cause unpredictable behavior.\nFor SHA-2-2 256 computations, message length is 64-bit {MSG_LENGTH_UPPER, MSG_LENGTH_LOWER}.f\nFor SHA-2 384/512 message length is extended to 128-bit in line with [nist-fips-180-4] where the upper 64 bits get zero-padded: {32'b0, 32'b0, MSG_LENGTH_UPPER, MSG_LENGTH_LOWER}.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn msg_length_upper(&self) -> ureg::RegRef<crate::meta::MsgLengthUpper, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Message FIFO. Any write to this window will be appended to the FIFO.\nOnly the lower [1:0] bits of the address matter to writes within the window\n(for correctly dealing with non 32-bit writes)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn msg_fifo(&self) -> ureg::Array<1024, ureg::RegRef<crate::meta::MsgFifo, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1000 / core::mem::size_of::<u32>()),
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
    pub struct CfgReadVal(u32);
    impl CfgReadVal {
        #[doc = "HMAC datapath enable.\n\nIf this bit is 1, HMAC operates when `hash_start` toggles."]
        #[inline(always)]
        pub fn hmac_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "SHA-2 enable.\n\n If 0, the SHA engine will not initiate compression, this is used to stop operation of the SHA-2 engine until configuration has been done.\n When the SHA-2 engine is disabled the digest is cleared."]
        #[inline(always)]
        pub fn sha_en(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Endian swap.\n\nIf 0, each value will be added to the message in little-endian byte order.\nThe value is written to MSG_FIFO same to the SW writes.\nIf 1, then each individual multi-byte value, regardless of its alignment, written to !!MSG_FIFO will be added to the message in big-endian byte order.\nA message written to !!MSG_FIFO one byte at a time will not be affected by this setting.\nFrom a hardware perspective byte swaps are performed on a TL-UL word granularity."]
        #[inline(always)]
        pub fn endian_swap(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Digest register byte swap.\n\nIf 1 the value in each digest output register is converted to big-endian byte order.\nThis setting does not affect the order of the digest output registers, !!DIGEST_0 still contains the first 4 bytes of the digest."]
        #[inline(always)]
        pub fn digest_swap(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Key register byte swap.\n\nIf 1 the endianness of each KEY_* register is swapped. Default value (value 0) is big endian representation of the KEY_* CSRs."]
        #[inline(always)]
        pub fn key_swap(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Digest size configuration.\n\nThis is a 4-bit one-hot encoded field to select digest size for either HMAC or SHA-2.\nInvalid/unsupported values, i.e., values that don't correspond to SHA2_256, SHA2_384, or SHA2_512, are mapped to SHA2_None."]
        #[inline(always)]
        pub fn digest_size(&self) -> super::enums::DigestSize {
            super::enums::DigestSize::try_from((self.0 >> 5) & 0xf).unwrap()
        }
        #[doc = "Key length configuration.\n\nThis is a 6-bit one-hot encoded field to configure the key length for HMAC.\nThe HMAC supports key lengths of 128-bit, 256-bit, 384-bit, 512-bit and 1024-bit, as long as the key length is not greater than the block size: up to 1024-bit for SHA-2 384/512 and up to 512-bit for SHA-2 256.\nThe value of this register is irrelevant when only SHA-2 (not keyed HMAC) is configured.\nHowever, for HMAC mode (`hmac_en == 1`), when HMAC is triggered to start while !!KEY_LENGTH holds `Key_None` or !!KEY_LENGTH holds `Key_1024` for !!DIGEST_SIZE = `SHA2_256`, starting is blocked and an error is signalled to SW."]
        #[inline(always)]
        pub fn key_length(&self) -> super::enums::KeyLength {
            super::enums::KeyLength::try_from((self.0 >> 9) & 0x3f).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgWriteVal {
            CfgWriteVal(self.0)
        }
    }
    impl From<u32> for CfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgWriteVal(u32);
    impl CfgWriteVal {
        #[doc = "HMAC datapath enable.\n\nIf this bit is 1, HMAC operates when `hash_start` toggles."]
        #[inline(always)]
        pub fn hmac_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "SHA-2 enable.\n\n If 0, the SHA engine will not initiate compression, this is used to stop operation of the SHA-2 engine until configuration has been done.\n When the SHA-2 engine is disabled the digest is cleared."]
        #[inline(always)]
        pub fn sha_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Endian swap.\n\nIf 0, each value will be added to the message in little-endian byte order.\nThe value is written to MSG_FIFO same to the SW writes.\nIf 1, then each individual multi-byte value, regardless of its alignment, written to !!MSG_FIFO will be added to the message in big-endian byte order.\nA message written to !!MSG_FIFO one byte at a time will not be affected by this setting.\nFrom a hardware perspective byte swaps are performed on a TL-UL word granularity."]
        #[inline(always)]
        pub fn endian_swap(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Digest register byte swap.\n\nIf 1 the value in each digest output register is converted to big-endian byte order.\nThis setting does not affect the order of the digest output registers, !!DIGEST_0 still contains the first 4 bytes of the digest."]
        #[inline(always)]
        pub fn digest_swap(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Key register byte swap.\n\nIf 1 the endianness of each KEY_* register is swapped. Default value (value 0) is big endian representation of the KEY_* CSRs."]
        #[inline(always)]
        pub fn key_swap(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Digest size configuration.\n\nThis is a 4-bit one-hot encoded field to select digest size for either HMAC or SHA-2.\nInvalid/unsupported values, i.e., values that don't correspond to SHA2_256, SHA2_384, or SHA2_512, are mapped to SHA2_None."]
        #[inline(always)]
        pub fn digest_size(
            self,
            f: impl FnOnce(super::enums::selector::DigestSizeSelector) -> super::enums::DigestSize,
        ) -> Self {
            Self(
                (self.0 & !(0xf << 5))
                    | (u32::from(f(super::enums::selector::DigestSizeSelector())) << 5),
            )
        }
        #[doc = "Key length configuration.\n\nThis is a 6-bit one-hot encoded field to configure the key length for HMAC.\nThe HMAC supports key lengths of 128-bit, 256-bit, 384-bit, 512-bit and 1024-bit, as long as the key length is not greater than the block size: up to 1024-bit for SHA-2 384/512 and up to 512-bit for SHA-2 256.\nThe value of this register is irrelevant when only SHA-2 (not keyed HMAC) is configured.\nHowever, for HMAC mode (`hmac_en == 1`), when HMAC is triggered to start while !!KEY_LENGTH holds `Key_None` or !!KEY_LENGTH holds `Key_1024` for !!DIGEST_SIZE = `SHA2_256`, starting is blocked and an error is signalled to SW."]
        #[inline(always)]
        pub fn key_length(
            self,
            f: impl FnOnce(super::enums::selector::KeyLengthSelector) -> super::enums::KeyLength,
        ) -> Self {
            Self(
                (self.0 & !(0x3f << 9))
                    | (u32::from(f(super::enums::selector::KeyLengthSelector())) << 9),
            )
        }
    }
    impl From<u32> for CfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdReadVal(u32);
    impl CmdReadVal {
        #[doc = "If 1 is written into this field, SHA-2 or HMAC begins its operation.\nCPU must configure relative information first, such as the digest size, secret key and the key length."]
        #[inline(always)]
        pub fn hash_start(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1 is written to this field, SHA-2 or HMAC calculates the digest or signing based on currently received message."]
        #[inline(always)]
        pub fn hash_process(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "When 1 is written to this field, SHA-2 or HMAC will afterwards set the `hmac_done` interrupt as soon as the current block has been hashed.\nThe hash can then be read from the registers !!DIGEST_0 to !!DIGEST_15.\nTogether with the message length in !!MSG_LENGTH_LOWER and !!MSG_LENGTH_UPPER, this forms the information that has to be saved before switching context."]
        #[inline(always)]
        pub fn hash_stop(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "When 1 is written to this field, SHA-2 or HMAC will continue hashing based on the current hash in the digest registers and the message length, which both have to be restored to switch context."]
        #[inline(always)]
        pub fn hash_continue(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdWriteVal {
            CmdWriteVal(self.0)
        }
    }
    impl From<u32> for CmdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdWriteVal(u32);
    impl CmdWriteVal {
        #[doc = "If 1 is written into this field, SHA-2 or HMAC begins its operation.\nCPU must configure relative information first, such as the digest size, secret key and the key length."]
        #[inline(always)]
        pub fn hash_start_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "If 1 is written to this field, SHA-2 or HMAC calculates the digest or signing based on currently received message."]
        #[inline(always)]
        pub fn hash_process_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "When 1 is written to this field, SHA-2 or HMAC will afterwards set the `hmac_done` interrupt as soon as the current block has been hashed.\nThe hash can then be read from the registers !!DIGEST_0 to !!DIGEST_15.\nTogether with the message length in !!MSG_LENGTH_LOWER and !!MSG_LENGTH_UPPER, this forms the information that has to be saved before switching context."]
        #[inline(always)]
        pub fn hash_stop_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "When 1 is written to this field, SHA-2 or HMAC will continue hashing based on the current hash in the digest registers and the message length, which both have to be restored to switch context."]
        #[inline(always)]
        pub fn hash_continue_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
    }
    impl From<u32> for CmdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.hmac_done is set."]
        #[inline(always)]
        pub fn hmac_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.fifo_empty is set."]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.hmac_err is set."]
        #[inline(always)]
        pub fn hmac_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.hmac_done is set."]
        #[inline(always)]
        pub fn hmac_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.fifo_empty is set."]
        #[inline(always)]
        pub fn fifo_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.hmac_err is set."]
        #[inline(always)]
        pub fn hmac_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
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
        #[doc = "HMAC/SHA-2 has completed."]
        #[inline(always)]
        pub fn hmac_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The message FIFO is empty.\nThis interrupt is raised only if the message FIFO is actually writable by software, i.e., if all of the following conditions are met:\ni) The HMAC block is not running in HMAC mode and performing the second round of computing the final hash of the outer key as well as the result of the first round using the inner key.\nii) Software has not yet written the Process or Stop command to finish the hashing operation.\nFor the interrupt to be raised, the message FIFO must also have been full previously.\nOtherwise, the hardware empties the FIFO faster than software can fill it and there is no point in interrupting the software to inform it about the message FIFO being empty."]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "HMAC error has occurred. ERR_CODE register shows which error occurred."]
        #[inline(always)]
        pub fn hmac_err(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
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
        #[doc = "HMAC/SHA-2 has completed."]
        #[inline(always)]
        pub fn hmac_done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "HMAC error has occurred. ERR_CODE register shows which error occurred."]
        #[inline(always)]
        pub fn hmac_err_clear(self) -> Self {
            Self(self.0 | (1 << 2))
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
        #[doc = "Write 1 to force !!INTR_STATE.hmac_done to 1."]
        #[inline(always)]
        pub fn hmac_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.fifo_empty to 1."]
        #[inline(always)]
        pub fn fifo_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.hmac_err to 1."]
        #[inline(always)]
        pub fn hmac_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
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
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "HMAC idle status.\nWhen IDLE, the `DIGEST` and the `MSG_LENGTH_LOWER`/`MSG_LENGTH_UPPER` can be written to from SW which enables restoring context (to support context switching)."]
        #[inline(always)]
        pub fn hmac_idle(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "FIFO empty"]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "FIFO full. Data written to the FIFO whilst it is full will cause back-pressure on the interconnect"]
        #[inline(always)]
        pub fn fifo_full(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "FIFO entry count."]
        #[inline(always)]
        pub fn fifo_depth(&self) -> u32 {
            (self.0 >> 4) & 0x3f
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
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum DigestSize {
        Reserved0 = 0,
        Sha2256 = 1,
        Sha2384 = 2,
        Reserved3 = 3,
        Sha2512 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
        Sha2None = 8,
        Reserved9 = 9,
        Reserved10 = 10,
        Reserved11 = 11,
        Reserved12 = 12,
        Reserved13 = 13,
        Reserved14 = 14,
        Reserved15 = 15,
    }
    impl DigestSize {
        #[inline(always)]
        pub fn sha2_256(&self) -> bool {
            *self == Self::Sha2256
        }
        #[inline(always)]
        pub fn sha2_384(&self) -> bool {
            *self == Self::Sha2384
        }
        #[inline(always)]
        pub fn sha2_512(&self) -> bool {
            *self == Self::Sha2512
        }
        #[inline(always)]
        pub fn sha2_none(&self) -> bool {
            *self == Self::Sha2None
        }
    }
    impl TryFrom<u32> for DigestSize {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<DigestSize, ()> {
            if val < 0x10 {
                Ok(unsafe { core::mem::transmute::<u32, DigestSize>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<DigestSize> for u32 {
        fn from(val: DigestSize) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum KeyLength {
        Reserved0 = 0,
        Key128 = 1,
        Key256 = 2,
        Reserved3 = 3,
        Key384 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
        Key512 = 8,
        Reserved9 = 9,
        Reserved10 = 10,
        Reserved11 = 11,
        Reserved12 = 12,
        Reserved13 = 13,
        Reserved14 = 14,
        Reserved15 = 15,
        Key1024 = 16,
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
        KeyNone = 32,
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
    impl KeyLength {
        #[inline(always)]
        pub fn key_128(&self) -> bool {
            *self == Self::Key128
        }
        #[inline(always)]
        pub fn key_256(&self) -> bool {
            *self == Self::Key256
        }
        #[inline(always)]
        pub fn key_384(&self) -> bool {
            *self == Self::Key384
        }
        #[inline(always)]
        pub fn key_512(&self) -> bool {
            *self == Self::Key512
        }
        #[inline(always)]
        pub fn key_1024(&self) -> bool {
            *self == Self::Key1024
        }
        #[inline(always)]
        pub fn key_none(&self) -> bool {
            *self == Self::KeyNone
        }
    }
    impl TryFrom<u32> for KeyLength {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<KeyLength, ()> {
            if val < 0x40 {
                Ok(unsafe { core::mem::transmute::<u32, KeyLength>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<KeyLength> for u32 {
        fn from(val: KeyLength) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct DigestSizeSelector();
        impl DigestSizeSelector {
            #[inline(always)]
            pub fn sha2_256(&self) -> super::DigestSize {
                super::DigestSize::Sha2256
            }
            #[inline(always)]
            pub fn sha2_384(&self) -> super::DigestSize {
                super::DigestSize::Sha2384
            }
            #[inline(always)]
            pub fn sha2_512(&self) -> super::DigestSize {
                super::DigestSize::Sha2512
            }
            #[inline(always)]
            pub fn sha2_none(&self) -> super::DigestSize {
                super::DigestSize::Sha2None
            }
        }
        pub struct KeyLengthSelector();
        impl KeyLengthSelector {
            #[inline(always)]
            pub fn key_128(&self) -> super::KeyLength {
                super::KeyLength::Key128
            }
            #[inline(always)]
            pub fn key_256(&self) -> super::KeyLength {
                super::KeyLength::Key256
            }
            #[inline(always)]
            pub fn key_384(&self) -> super::KeyLength {
                super::KeyLength::Key384
            }
            #[inline(always)]
            pub fn key_512(&self) -> super::KeyLength {
                super::KeyLength::Key512
            }
            #[inline(always)]
            pub fn key_1024(&self) -> super::KeyLength {
                super::KeyLength::Key1024
            }
            #[inline(always)]
            pub fn key_none(&self) -> super::KeyLength {
                super::KeyLength::KeyNone
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
    pub type Cfg = ureg::ReadWriteReg32<0x4100, crate::regs::CfgReadVal, crate::regs::CfgWriteVal>;
    pub type Cmd = ureg::ReadWriteReg32<0, crate::regs::CmdReadVal, crate::regs::CmdWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type ErrCode = ureg::ReadOnlyReg32<u32>;
    pub type WipeSecret = ureg::WriteOnlyReg32<0, u32>;
    pub type Key = ureg::WriteOnlyReg32<0, u32>;
    pub type Digest = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MsgLengthLower = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MsgLengthUpper = ureg::ReadWriteReg32<0, u32, u32>;
    pub type MsgFifo = ureg::WriteOnlyReg32<0, u32>;
}
