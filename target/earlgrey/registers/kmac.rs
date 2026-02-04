#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Kmac {
    _priv: (),
}
impl Kmac {
    pub const PTR: *mut u32 = 0x41120000 as *mut u32;
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
    #[doc = "Controls the configurability of !!CFG_SHADOWED register.\n\nThis register ensures the contents of !!CFG_SHADOWED register cannot be\nchanged by the software while the KMAC/SHA3 is in operation mode.\n\nRead value: [`regs::CfgRegwenReadVal`]; Write value: [`regs::CfgRegwenWriteVal`]"]
    #[inline(always)]
    pub fn cfg_regwen(&self) -> ureg::RegRef<crate::meta::CfgRegwen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC Configuration register.\n\nThis register is  updated when the hashing engine is in Idle.\nIf the software updates the register while the engine computes, the\nupdated value will be discarded.\n\nRead value: [`regs::CfgShadowedReadVal`]; Write value: [`regs::CfgShadowedWriteVal`]"]
    #[inline(always)]
    pub fn cfg_shadowed(&self) -> ureg::RegRef<crate::meta::CfgShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC/ SHA3 command register.\n\nThis register is to control the KMAC to start accepting message,\nto process the message, and to manually run additional keccak\nrounds at the end. Only at certain stage, the CMD affects to the\ncontrol logic. It follows the sequence of\n\n`start` --> `process` --> {`run` if needed --> } `done`\n\nRead value: [`regs::CmdReadVal`]; Write value: [`regs::CmdWriteVal`]"]
    #[inline(always)]
    pub fn cmd(&self) -> ureg::RegRef<crate::meta::Cmd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC/SHA3 Status register.\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Entropy Timer Periods.\n\nRead value: [`regs::EntropyPeriodReadVal`]; Write value: [`regs::EntropyPeriodWriteVal`]"]
    #[inline(always)]
    pub fn entropy_period(&self) -> ureg::RegRef<crate::meta::EntropyPeriod, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Entropy Refresh Counter\n\nKMAC entropy can be refreshed after the given threshold KMAC operations\nrun. If the KMAC hash counter !!ENTROPY_REFRESH_HASH_CNT hits (GTE) the\nconfigured threshold !!ENTROPY_REFRESH_THRESHOLD_SHADOWED, the entropy\nmodule in the KMAC IP requests new seed to EDN and reset the KMAC\nhash counter.\n\nIf the threshold is 0, the refresh by the counter does not work. And the\ncounter is only reset by the CMD.hash_cnt_clr CSR bit.\n\nRead value: [`regs::EntropyRefreshHashCntReadVal`]; Write value: [`regs::EntropyRefreshHashCntWriteVal`]"]
    #[inline(always)]
    pub fn entropy_refresh_hash_cnt(
        &self,
    ) -> ureg::RegRef<crate::meta::EntropyRefreshHashCnt, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Entropy Refresh Threshold\n\nKMAC entropy can be refreshed after the given threshold KMAC operations\nrun. If the KMAC hash counter !!ENTROPY_REFRESH_HASH_CNT hits (GTE) the\nconfigured threshold !!ENTROPY_REFRESH_THRESHOLD_SHADOWED, the entropy\nmodule in the KMAC IP requests new seed to EDN and reset the KMAC\nhash counter.\n\nIf the threshold is 0, the refresh by the counter does not work. And the\ncounter is only reset by the CMD.hash_cnt_clr CSR bit.\n\nRead value: [`regs::EntropyRefreshThresholdShadowedReadVal`]; Write value: [`regs::EntropyRefreshThresholdShadowedWriteVal`]"]
    #[inline(always)]
    pub fn entropy_refresh_threshold_shadowed(
        &self,
    ) -> ureg::RegRef<crate::meta::EntropyRefreshThresholdShadowed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Entropy Seed\n\nEntropy seed register for the integrated entropy generator.\n\nIf !!CFG_SHADOWED.entropy_mode is set to sw_mode, software first needs to set !!CFG_SHADOWED.entropy_ready.\nThen, software needs to write the !!ENTROPY_SEED register 9 times.\nUpon each write, the written value is loaded into the corresponding state chunk of the entropy generator.\n\nAfter writing the !!ENTROPY_SEED register 9 times, the entropy generator will start its operation.\nAfter this point, writing this register has no longer any effect.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn entropy_seed(&self) -> ureg::RegRef<crate::meta::EntropySeed, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC Secret Key\n\nKMAC secret key can be up to 512 bit.\nOrder of the secret key is:\nkey[512:0] = {KEY15, KEY14, ... , KEY0};\n\nThe registers are allowed to be updated when the engine is in Idle state.\nIf the engine computes the hash, it discards any attempts to update the secret keys\nand report an error.\n\nCurrent KMAC supports up to 512 bit secret key. It is the sw\nresponsibility to keep upper bits of the secret key to 0.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key_share0(&self) -> ureg::Array<16, ureg::RegRef<crate::meta::KeyShare0, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC Secret Key, 2nd share.\n\nKMAC secret key can be up to 512 bit.\nOrder of the secret key is:\nkey[512:0] = {KEY15, KEY14, ... , KEY0};\n\nThe registers are allowed to be updated when the engine is in Idle state.\nIf the engine computes the hash, it discards any attempts to update the secret keys\nand report an error.\n\nCurrent KMAC supports up to 512 bit secret key. It is the sw\nresponsibility to keep upper bits of the secret key to 0.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn key_share1(&self) -> ureg::Array<16, ureg::RegRef<crate::meta::KeyShare1, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Secret Key length in bit.\n\nThis value is used to make encoded secret key in KMAC.\nKMAC supports certain lengths of the secret key. Currently it\nsupports 128b, 192b, 256b, 384b, and 512b secret keys.\n\nRead value: [`regs::KeyLenReadVal`]; Write value: [`regs::KeyLenWriteVal`]"]
    #[inline(always)]
    pub fn key_len(&self) -> ureg::RegRef<crate::meta::KeyLen, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xb0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "cSHAKE Prefix register.\n\nPrefix including Function Name N and Customization String S.\nThe SHA3 assumes this register value is encoded as:\n`encode_string(N) || encode_string(S) || 0`. It means that the\nsoftware can freely decide the length of N or S based on the\ngiven Prefix register size 320bit. 320bit is determined to have\n32-bit of N and up to 256-bit of S + encode of their length.\n\nIt is SW responsibility to fill the register with encoded value\nthat is described at Section 2.3.2 String Encoding in NIST SP\n800-185 specification.\n\nOrder of Prefix is:\nprefix[end:0] := {PREFIX(N-1), ..., PREFIX(1), PREFIX(0) }\n\nThe registers are allowed to be updated when the engine is in Idle state.\nIf the engine computes the hash, it discards any attempts to update the secret keys\nand report an error.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn prefix(&self) -> ureg::Array<11, ureg::RegRef<crate::meta::Prefix, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0xb4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "KMAC/SHA3 Error Code\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn err_code(&self) -> ureg::RegRef<crate::meta::ErrCode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Keccak State (1600 bit) memory.\n\nThe software can get the processed digest by reading this memory\nregion. Unlike MSG_FIFO, STATE memory space sees the addr[9:0].\nIf Masking feature is enabled, the software reads two shares from\nthis memory space.\n\n0x400 - 0x4C7: State share\n0x500 - 0x5C7: Mask share of the state, 0 if EnMasking = 0\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn state(&self) -> ureg::Array<128, ureg::RegRef<crate::meta::State, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x400 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Message FIFO.\n\nAny write operation to this window will be appended to MSG_FIFO. SW can\nsimply write bytes/words to any address within this address range.\nOrdering and packing of the incoming bytes/words are handled\ninternally. Therefore, the least significant 12 bits of the address\nare ignored.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn msg_fifo(&self) -> ureg::Array<512, ureg::RegRef<crate::meta::MsgFifo, &TMmio>> {
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
        #[doc = "Configuration enable."]
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
    pub struct CfgShadowedReadVal(u32);
    impl CfgShadowedReadVal {
        #[doc = "KMAC datapath enable.\n\nIf this bit is 1, the incoming message is processed in KMAC\nwith the secret key."]
        #[inline(always)]
        pub fn kmac_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Hashing Strength\n\n3 bit field to select the security strength of SHA3 hashing\nengine. If mode field is set to SHAKE or cSHAKE, only 128 and\n256 strength can be selected. Other value will result error\nwhen hashing starts."]
        #[inline(always)]
        pub fn kstrength(&self) -> super::enums::Kstrength {
            super::enums::Kstrength::try_from((self.0 >> 1) & 7).unwrap()
        }
        #[doc = "Keccak hashing mode.\n\nThis module supports SHA3 main hashing algorithm and the part\nof its derived functions, SHAKE and cSHAKE with limitations.\nThis field is to select the mode."]
        #[inline(always)]
        pub fn mode(&self) -> super::enums::Mode {
            super::enums::Mode::try_from((self.0 >> 4) & 3).unwrap()
        }
        #[doc = "Message Endianness.\n\nIf 1 then each individual multi-byte value, regardless of its\nalignment, written to !!MSG_FIFO will be added to the message\nin big-endian byte order.\nIf 0, each value will be added to the message in little-endian\nbyte order.\nA message written to !!MSG_FIFO one byte at a time will not be\naffected by this setting.\nFrom a hardware perspective byte swaps are performed on a TL-UL\nword granularity."]
        #[inline(always)]
        pub fn msg_endianness(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "State Endianness.\n\nIf 1 then each individual word in the !!STATE output register\nis converted to big-endian byte order.\nThe order of the words in relation to one another is not\nchanged.\nThis setting does not affect how the state is interpreted\nduring computation."]
        #[inline(always)]
        pub fn state_endianness(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Sideloaded Key.\n\nIf 1, KMAC uses KeyMgr sideloaded key for SW initiated KMAC\noperation. KMAC uses the sideloaded key regardless of this\nconfiguration when KeyMgr initiates the KMAC operation for\nKey Derivation Function (KDF)."]
        #[inline(always)]
        pub fn sideload(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Entropy Mode\n\nUsing this field, software can configure mode of operation of the internal pseudo-random number generator (PRNG).\nFor the hardware to actually switch to an entropy mode other than the default idle_mode, software further needs to set the !!CFG_SHADOWED.entropy_ready bit.\nAfter that point, the hardware cannot be made to return to idle_mode unless the module is reset."]
        #[inline(always)]
        pub fn entropy_mode(&self) -> super::enums::EntropyMode {
            super::enums::EntropyMode::try_from((self.0 >> 16) & 3).unwrap()
        }
        #[doc = "Entropy Fast process mode.\n\nIf 1, entropy logic uses garbage data while not processing the KMAC\nkey block. It will re-use previous entropy value and will not\nexpand the entropy when it is consumed. Only it refreshes the\nentropy while processing the secret key block. This process should\nnot be used if SCA resistance is required because it may cause side\nchannel leakage."]
        #[inline(always)]
        pub fn entropy_fast_process(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Message Masking with PRNG.\n\nIf 1, KMAC applies PRNG to the input messages to the Keccak module\nwhen KMAC mode is on."]
        #[inline(always)]
        pub fn msg_mask(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Entropy Ready status.\n\nSoftware sets this field to allow the entropy generator in KMAC to\nfetch the entropy and run."]
        #[inline(always)]
        pub fn entropy_ready(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Enable Unsupported Mode and Strength configs.\n\nSW may set this field for KMAC to move forward with unsupported\nKeccak Mode and Strength configurations, such as cSHAKE512.\n\nIf not set, KMAC won't propagate the SW command (CmdStart) to the\nrest of the blocks (AppIntf, KMAC Core, SHA3)."]
        #[inline(always)]
        pub fn en_unsupported_modestrength(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgShadowedWriteVal {
            CfgShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for CfgShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgShadowedWriteVal(u32);
    impl CfgShadowedWriteVal {
        #[doc = "KMAC datapath enable.\n\nIf this bit is 1, the incoming message is processed in KMAC\nwith the secret key."]
        #[inline(always)]
        pub fn kmac_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Hashing Strength\n\n3 bit field to select the security strength of SHA3 hashing\nengine. If mode field is set to SHAKE or cSHAKE, only 128 and\n256 strength can be selected. Other value will result error\nwhen hashing starts."]
        #[inline(always)]
        pub fn kstrength(
            self,
            f: impl FnOnce(super::enums::selector::KstrengthSelector) -> super::enums::Kstrength,
        ) -> Self {
            Self(
                (self.0 & !(7 << 1))
                    | (u32::from(f(super::enums::selector::KstrengthSelector())) << 1),
            )
        }
        #[doc = "Keccak hashing mode.\n\nThis module supports SHA3 main hashing algorithm and the part\nof its derived functions, SHAKE and cSHAKE with limitations.\nThis field is to select the mode."]
        #[inline(always)]
        pub fn mode(
            self,
            f: impl FnOnce(super::enums::selector::ModeSelector) -> super::enums::Mode,
        ) -> Self {
            Self((self.0 & !(3 << 4)) | (u32::from(f(super::enums::selector::ModeSelector())) << 4))
        }
        #[doc = "Message Endianness.\n\nIf 1 then each individual multi-byte value, regardless of its\nalignment, written to !!MSG_FIFO will be added to the message\nin big-endian byte order.\nIf 0, each value will be added to the message in little-endian\nbyte order.\nA message written to !!MSG_FIFO one byte at a time will not be\naffected by this setting.\nFrom a hardware perspective byte swaps are performed on a TL-UL\nword granularity."]
        #[inline(always)]
        pub fn msg_endianness(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "State Endianness.\n\nIf 1 then each individual word in the !!STATE output register\nis converted to big-endian byte order.\nThe order of the words in relation to one another is not\nchanged.\nThis setting does not affect how the state is interpreted\nduring computation."]
        #[inline(always)]
        pub fn state_endianness(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Sideloaded Key.\n\nIf 1, KMAC uses KeyMgr sideloaded key for SW initiated KMAC\noperation. KMAC uses the sideloaded key regardless of this\nconfiguration when KeyMgr initiates the KMAC operation for\nKey Derivation Function (KDF)."]
        #[inline(always)]
        pub fn sideload(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Entropy Mode\n\nUsing this field, software can configure mode of operation of the internal pseudo-random number generator (PRNG).\nFor the hardware to actually switch to an entropy mode other than the default idle_mode, software further needs to set the !!CFG_SHADOWED.entropy_ready bit.\nAfter that point, the hardware cannot be made to return to idle_mode unless the module is reset."]
        #[inline(always)]
        pub fn entropy_mode(
            self,
            f: impl FnOnce(super::enums::selector::EntropyModeSelector) -> super::enums::EntropyMode,
        ) -> Self {
            Self(
                (self.0 & !(3 << 16))
                    | (u32::from(f(super::enums::selector::EntropyModeSelector())) << 16),
            )
        }
        #[doc = "Entropy Fast process mode.\n\nIf 1, entropy logic uses garbage data while not processing the KMAC\nkey block. It will re-use previous entropy value and will not\nexpand the entropy when it is consumed. Only it refreshes the\nentropy while processing the secret key block. This process should\nnot be used if SCA resistance is required because it may cause side\nchannel leakage."]
        #[inline(always)]
        pub fn entropy_fast_process(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Message Masking with PRNG.\n\nIf 1, KMAC applies PRNG to the input messages to the Keccak module\nwhen KMAC mode is on."]
        #[inline(always)]
        pub fn msg_mask(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Entropy Ready status.\n\nSoftware sets this field to allow the entropy generator in KMAC to\nfetch the entropy and run."]
        #[inline(always)]
        pub fn entropy_ready(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Enable Unsupported Mode and Strength configs.\n\nSW may set this field for KMAC to move forward with unsupported\nKeccak Mode and Strength configurations, such as cSHAKE512.\n\nIf not set, KMAC won't propagate the SW command (CmdStart) to the\nrest of the blocks (AppIntf, KMAC Core, SHA3)."]
        #[inline(always)]
        pub fn en_unsupported_modestrength(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
    }
    impl From<u32> for CfgShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdReadVal(u32);
    impl CmdReadVal {
        #[doc = "Issue a command to the KMAC/SHA3 IP. The command is sparse\nencoded. To prevent sw from writing multiple commands at once,\nthe field is defined as enum."]
        #[inline(always)]
        pub fn cmd(&self) -> super::enums::Cmd {
            super::enums::Cmd::try_from((self.0 >> 0) & 0x3f).unwrap()
        }
        #[doc = "SW triggered Entropy Request\n\nIf writes 1 to this field"]
        #[inline(always)]
        pub fn entropy_req(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If writes 1, it clears the hash (KMAC) counter in the entropy module"]
        #[inline(always)]
        pub fn hash_cnt_clr(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "When error occurs and one of the state machine stays at\n Error handling state, SW may process the error based on\n ERR_CODE, then let FSM back to the reset state"]
        #[inline(always)]
        pub fn err_processed(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
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
        #[doc = "SW triggered Entropy Request\n\nIf writes 1 to this field"]
        #[inline(always)]
        pub fn entropy_req_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "If writes 1, it clears the hash (KMAC) counter in the entropy module"]
        #[inline(always)]
        pub fn hash_cnt_clr_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "When error occurs and one of the state machine stays at\n Error handling state, SW may process the error based on\n ERR_CODE, then let FSM back to the reset state"]
        #[inline(always)]
        pub fn err_processed_clear(self) -> Self {
            Self(self.0 | (1 << 10))
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
    pub struct EntropyPeriodReadVal(u32);
    impl EntropyPeriodReadVal {
        #[doc = "EDN Wait timer prescaler.\n\nEDN Wait timer has 16 bit value. The timer value is increased when the timer pulse is generated. Timer pulse is raises when the number of the clock cycles hit this prescaler value.\n\nThe exact period of the timer pulse is unknown as the KMAC input clock may contain jitters."]
        #[inline(always)]
        pub fn prescaler(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = "EDN request wait timer.\n\nThe entropy module in KMAC waits up to this field in the timer pulse\nafter it sends request to EDN module. If the timer expires, the\nentropy module moves to an error state and notifies to the system.\n\nIf there is a pending EDN request during wait timer update, then this update is delayed until the EDN request is complete.\n\nIf 0, the entropy module waits the EDN response always. If EDN does\nnot respond in this configuration, the software shall reset the IP."]
        #[inline(always)]
        pub fn wait_timer(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EntropyPeriodWriteVal {
            EntropyPeriodWriteVal(self.0)
        }
    }
    impl From<u32> for EntropyPeriodReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EntropyPeriodReadVal> for u32 {
        #[inline(always)]
        fn from(val: EntropyPeriodReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EntropyPeriodWriteVal(u32);
    impl EntropyPeriodWriteVal {
        #[doc = "EDN Wait timer prescaler.\n\nEDN Wait timer has 16 bit value. The timer value is increased when the timer pulse is generated. Timer pulse is raises when the number of the clock cycles hit this prescaler value.\n\nThe exact period of the timer pulse is unknown as the KMAC input clock may contain jitters."]
        #[inline(always)]
        pub fn prescaler(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[doc = "EDN request wait timer.\n\nThe entropy module in KMAC waits up to this field in the timer pulse\nafter it sends request to EDN module. If the timer expires, the\nentropy module moves to an error state and notifies to the system.\n\nIf there is a pending EDN request during wait timer update, then this update is delayed until the EDN request is complete.\n\nIf 0, the entropy module waits the EDN response always. If EDN does\nnot respond in this configuration, the software shall reset the IP."]
        #[inline(always)]
        pub fn wait_timer(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for EntropyPeriodWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EntropyPeriodWriteVal> for u32 {
        #[inline(always)]
        fn from(val: EntropyPeriodWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EntropyRefreshHashCntReadVal(u32);
    impl EntropyRefreshHashCntReadVal {
        #[doc = "Hash (KMAC) counter"]
        #[inline(always)]
        pub fn hash_cnt(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
    }
    impl From<u32> for EntropyRefreshHashCntReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EntropyRefreshHashCntReadVal> for u32 {
        #[inline(always)]
        fn from(val: EntropyRefreshHashCntReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EntropyRefreshThresholdShadowedReadVal(u32);
    impl EntropyRefreshThresholdShadowedReadVal {
        #[doc = "Hash Threshold"]
        #[inline(always)]
        pub fn threshold(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EntropyRefreshThresholdShadowedWriteVal {
            EntropyRefreshThresholdShadowedWriteVal(self.0)
        }
    }
    impl From<u32> for EntropyRefreshThresholdShadowedReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EntropyRefreshThresholdShadowedReadVal> for u32 {
        #[inline(always)]
        fn from(val: EntropyRefreshThresholdShadowedReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EntropyRefreshThresholdShadowedWriteVal(u32);
    impl EntropyRefreshThresholdShadowedWriteVal {
        #[doc = "Hash Threshold"]
        #[inline(always)]
        pub fn threshold(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
    }
    impl From<u32> for EntropyRefreshThresholdShadowedWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EntropyRefreshThresholdShadowedWriteVal> for u32 {
        #[inline(always)]
        fn from(val: EntropyRefreshThresholdShadowedWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.kmac_done is set."]
        #[inline(always)]
        pub fn kmac_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.fifo_empty is set."]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.kmac_err is set."]
        #[inline(always)]
        pub fn kmac_err(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.kmac_done is set."]
        #[inline(always)]
        pub fn kmac_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.fifo_empty is set."]
        #[inline(always)]
        pub fn fifo_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.kmac_err is set."]
        #[inline(always)]
        pub fn kmac_err(self, val: bool) -> Self {
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
        #[doc = "KMAC/SHA3 absorbing has been completed"]
        #[inline(always)]
        pub fn kmac_done(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "The message FIFO is empty.\nThis interrupt is raised only if the message FIFO is actually writable by software, i.e., if all of the following conditions are met:\ni) The KMAC block is not exercised by a hardware application interface.\nii) The SHA3 block is in the Absorb state.\niii) Software has not yet written the Process command to finish the absorption process.\nFor the interrupt to be raised, the message FIFO must also have been full previously.\nOtherwise, the hardware empties the FIFO faster than software can fill it and there is no point in interrupting the software to inform it about the message FIFO being empty."]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "KMAC/SHA3 error occurred. ERR_CODE register shows the details"]
        #[inline(always)]
        pub fn kmac_err(&self) -> bool {
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
        #[doc = "KMAC/SHA3 absorbing has been completed"]
        #[inline(always)]
        pub fn kmac_done_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "KMAC/SHA3 error occurred. ERR_CODE register shows the details"]
        #[inline(always)]
        pub fn kmac_err_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.kmac_done to 1."]
        #[inline(always)]
        pub fn kmac_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.fifo_empty to 1."]
        #[inline(always)]
        pub fn fifo_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.kmac_err to 1."]
        #[inline(always)]
        pub fn kmac_err(self, val: bool) -> Self {
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
    pub struct KeyLenWriteVal(u32);
    impl KeyLenWriteVal {
        #[doc = "Key length choice"]
        #[inline(always)]
        pub fn len(
            self,
            f: impl FnOnce(super::enums::selector::LenSelector) -> super::enums::Len,
        ) -> Self {
            Self((self.0 & !(7 << 0)) | (u32::from(f(super::enums::selector::LenSelector())) << 0))
        }
    }
    impl From<u32> for KeyLenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<KeyLenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: KeyLenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "If 1, SHA3 hashing engine is in idle state."]
        #[inline(always)]
        pub fn sha3_idle(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, SHA3 is receiving message stream and processing it"]
        #[inline(always)]
        pub fn sha3_absorb(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, SHA3 completes sponge absorbing stage.\nIn this stage, SW can manually run the hashing engine."]
        #[inline(always)]
        pub fn sha3_squeeze(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Count of occupied entries in the message FIFO."]
        #[inline(always)]
        pub fn fifo_depth(&self) -> u32 {
            (self.0 >> 8) & 0x1f
        }
        #[doc = "Message FIFO Empty indicator.\n\nThe FIFO's `Pass` parameter is set to `1'b 1`. So, by default, if\nthe SHA engine is ready, the write data to FIFO just passes\nthrough.\n\nIn this case, `fifo_depth` remains **0**. `fifo_empty`, however,\nlowers the value to **0** for a cycle, then goes back to the empty\nstate, **1**.\n\nSee the \"Message FIFO\" section in the spec for the reason."]
        #[inline(always)]
        pub fn fifo_empty(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Message FIFO Full indicator"]
        #[inline(always)]
        pub fn fifo_full(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "No fatal fault has occurred inside the KMAC unit (0).\nA fatal fault has occured and the KMAC unit needs to be reset (1),\nExamples for such faults include\ni) TL-UL bus integrity fault\nii) storage errors in the shadow registers\niii) errors in the message, round, or key counter\niv) any internal FSM entering an invalid state\nv) an error in the redundant lfsr"]
        #[inline(always)]
        pub fn alert_fatal_fault(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "An update error has not occurred (0) or has occured (1) in the shadowed Control Register.\nKMAC operation needs to be restarted by re-writing the Control Register."]
        #[inline(always)]
        pub fn alert_recov_ctrl_update_err(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
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
    pub enum Cmd {
        Reserved0 = 0,
        Reserved1 = 1,
        Reserved2 = 2,
        Reserved3 = 3,
        Reserved4 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
        Reserved8 = 8,
        Reserved9 = 9,
        Reserved10 = 10,
        Reserved11 = 11,
        Reserved12 = 12,
        Reserved13 = 13,
        Reserved14 = 14,
        Reserved15 = 15,
        Reserved16 = 16,
        Reserved17 = 17,
        Reserved18 = 18,
        Reserved19 = 19,
        Reserved20 = 20,
        Reserved21 = 21,
        Done = 22,
        Reserved23 = 23,
        Reserved24 = 24,
        Reserved25 = 25,
        Reserved26 = 26,
        Reserved27 = 27,
        Reserved28 = 28,
        Start = 29,
        Reserved30 = 30,
        Reserved31 = 31,
        Reserved32 = 32,
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
        Process = 46,
        Reserved47 = 47,
        Reserved48 = 48,
        Run = 49,
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
    impl Cmd {
        #[inline(always)]
        pub fn done(&self) -> bool {
            *self == Self::Done
        }
        #[inline(always)]
        pub fn start(&self) -> bool {
            *self == Self::Start
        }
        #[inline(always)]
        pub fn process(&self) -> bool {
            *self == Self::Process
        }
        #[inline(always)]
        pub fn run(&self) -> bool {
            *self == Self::Run
        }
    }
    impl TryFrom<u32> for Cmd {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Cmd, ()> {
            if val < 0x40 {
                Ok(unsafe { core::mem::transmute::<u32, Cmd>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Cmd> for u32 {
        fn from(val: Cmd) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum EntropyMode {
        IdleMode = 0,
        EdnMode = 1,
        SwMode = 2,
        Reserved3 = 3,
    }
    impl EntropyMode {
        #[inline(always)]
        pub fn idle_mode(&self) -> bool {
            *self == Self::IdleMode
        }
        #[inline(always)]
        pub fn edn_mode(&self) -> bool {
            *self == Self::EdnMode
        }
        #[inline(always)]
        pub fn sw_mode(&self) -> bool {
            *self == Self::SwMode
        }
    }
    impl TryFrom<u32> for EntropyMode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<EntropyMode, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, EntropyMode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<EntropyMode> for u32 {
        fn from(val: EntropyMode) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Kstrength {
        L128 = 0,
        L224 = 1,
        L256 = 2,
        L384 = 3,
        L512 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl Kstrength {
        #[inline(always)]
        pub fn l128(&self) -> bool {
            *self == Self::L128
        }
        #[inline(always)]
        pub fn l224(&self) -> bool {
            *self == Self::L224
        }
        #[inline(always)]
        pub fn l256(&self) -> bool {
            *self == Self::L256
        }
        #[inline(always)]
        pub fn l384(&self) -> bool {
            *self == Self::L384
        }
        #[inline(always)]
        pub fn l512(&self) -> bool {
            *self == Self::L512
        }
    }
    impl TryFrom<u32> for Kstrength {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Kstrength, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Kstrength>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Kstrength> for u32 {
        fn from(val: Kstrength) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Len {
        Key128 = 0,
        Key192 = 1,
        Key256 = 2,
        Key384 = 3,
        Key512 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl Len {
        #[inline(always)]
        pub fn key128(&self) -> bool {
            *self == Self::Key128
        }
        #[inline(always)]
        pub fn key192(&self) -> bool {
            *self == Self::Key192
        }
        #[inline(always)]
        pub fn key256(&self) -> bool {
            *self == Self::Key256
        }
        #[inline(always)]
        pub fn key384(&self) -> bool {
            *self == Self::Key384
        }
        #[inline(always)]
        pub fn key512(&self) -> bool {
            *self == Self::Key512
        }
    }
    impl TryFrom<u32> for Len {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Len, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Len>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Len> for u32 {
        fn from(val: Len) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Mode {
        Sha3 = 0,
        Reserved1 = 1,
        Shake = 2,
        Cshake = 3,
    }
    impl Mode {
        #[inline(always)]
        pub fn sha3(&self) -> bool {
            *self == Self::Sha3
        }
        #[inline(always)]
        pub fn shake(&self) -> bool {
            *self == Self::Shake
        }
        #[inline(always)]
        pub fn c_shake(&self) -> bool {
            *self == Self::Cshake
        }
    }
    impl TryFrom<u32> for Mode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Mode, ()> {
            if val < 4 {
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
    pub mod selector {
        pub struct CmdSelector();
        impl CmdSelector {
            #[inline(always)]
            pub fn start(&self) -> super::Cmd {
                super::Cmd::Start
            }
            #[inline(always)]
            pub fn process(&self) -> super::Cmd {
                super::Cmd::Process
            }
            #[inline(always)]
            pub fn run(&self) -> super::Cmd {
                super::Cmd::Run
            }
            #[inline(always)]
            pub fn done(&self) -> super::Cmd {
                super::Cmd::Done
            }
        }
        pub struct EntropyModeSelector();
        impl EntropyModeSelector {
            #[inline(always)]
            pub fn idle_mode(&self) -> super::EntropyMode {
                super::EntropyMode::IdleMode
            }
            #[inline(always)]
            pub fn edn_mode(&self) -> super::EntropyMode {
                super::EntropyMode::EdnMode
            }
            #[inline(always)]
            pub fn sw_mode(&self) -> super::EntropyMode {
                super::EntropyMode::SwMode
            }
        }
        pub struct KstrengthSelector();
        impl KstrengthSelector {
            #[inline(always)]
            pub fn l128(&self) -> super::Kstrength {
                super::Kstrength::L128
            }
            #[inline(always)]
            pub fn l224(&self) -> super::Kstrength {
                super::Kstrength::L224
            }
            #[inline(always)]
            pub fn l256(&self) -> super::Kstrength {
                super::Kstrength::L256
            }
            #[inline(always)]
            pub fn l384(&self) -> super::Kstrength {
                super::Kstrength::L384
            }
            #[inline(always)]
            pub fn l512(&self) -> super::Kstrength {
                super::Kstrength::L512
            }
        }
        pub struct LenSelector();
        impl LenSelector {
            #[inline(always)]
            pub fn key128(&self) -> super::Len {
                super::Len::Key128
            }
            #[inline(always)]
            pub fn key192(&self) -> super::Len {
                super::Len::Key192
            }
            #[inline(always)]
            pub fn key256(&self) -> super::Len {
                super::Len::Key256
            }
            #[inline(always)]
            pub fn key384(&self) -> super::Len {
                super::Len::Key384
            }
            #[inline(always)]
            pub fn key512(&self) -> super::Len {
                super::Len::Key512
            }
        }
        pub struct ModeSelector();
        impl ModeSelector {
            #[inline(always)]
            pub fn sha3(&self) -> super::Mode {
                super::Mode::Sha3
            }
            #[inline(always)]
            pub fn shake(&self) -> super::Mode {
                super::Mode::Shake
            }
            #[inline(always)]
            pub fn c_shake(&self) -> super::Mode {
                super::Mode::Cshake
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
    pub type CfgShadowed =
        ureg::ReadWriteReg32<0, crate::regs::CfgShadowedReadVal, crate::regs::CfgShadowedWriteVal>;
    pub type Cmd = ureg::ReadWriteReg32<0, crate::regs::CmdReadVal, crate::regs::CmdWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type EntropyPeriod = ureg::ReadWriteReg32<
        0,
        crate::regs::EntropyPeriodReadVal,
        crate::regs::EntropyPeriodWriteVal,
    >;
    pub type EntropyRefreshHashCnt = ureg::ReadOnlyReg32<crate::regs::EntropyRefreshHashCntReadVal>;
    pub type EntropyRefreshThresholdShadowed = ureg::ReadWriteReg32<
        0,
        crate::regs::EntropyRefreshThresholdShadowedReadVal,
        crate::regs::EntropyRefreshThresholdShadowedWriteVal,
    >;
    pub type EntropySeed = ureg::WriteOnlyReg32<0, u32>;
    pub type KeyShare0 = ureg::WriteOnlyReg32<0, u32>;
    pub type KeyShare1 = ureg::WriteOnlyReg32<0, u32>;
    pub type KeyLen = ureg::WriteOnlyReg32<0, crate::regs::KeyLenWriteVal>;
    pub type Prefix = ureg::ReadWriteReg32<0, u32, u32>;
    pub type ErrCode = ureg::ReadOnlyReg32<u32>;
    pub type State = ureg::ReadOnlyReg32<u32>;
    pub type MsgFifo = ureg::WriteOnlyReg32<0, u32>;
}
