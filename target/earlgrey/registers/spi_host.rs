#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SpiHost0 {
    _priv: (),
}
impl SpiHost0 {
    pub const PTR: *mut u32 = 0x40300000 as *mut u32;
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
pub struct SpiHost1 {
    _priv: (),
}
impl SpiHost1 {
    pub const PTR: *mut u32 = 0x40310000 as *mut u32;
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
    #[doc = "Control register\n\nRead value: [`regs::ControlReadVal`]; Write value: [`regs::ControlWriteVal`]"]
    #[inline(always)]
    pub fn control(&self) -> ureg::RegRef<crate::meta::Control, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Status register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration options register.\n\n   Contains options for controlling each peripheral. One register per\n   cs_n line\n\nRead value: [`regs::ConfigoptsReadVal`]; Write value: [`regs::ConfigoptsWriteVal`]"]
    #[inline(always)]
    pub fn configopts(&self) -> ureg::RegRef<crate::meta::Configopts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Chip-Select ID\n\n   Controls which device to target with the next command.  This register\n   is passed to the core whenever !!COMMAND is written.  The core then\n   asserts cio_csb_o[!!CSID] during the execution of the command.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn csid(&self) -> ureg::RegRef<crate::meta::Csid, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Register\n\n   Parameters specific to each command segment.  Unlike the !!CONFIGOPTS multi-register,\n   there is only one command register for controlling all attached SPI devices\n\nRead value: [`regs::CommandReadVal`]; Write value: [`regs::CommandWriteVal`]"]
    #[inline(always)]
    pub fn command(&self) -> ureg::RegRef<crate::meta::Command, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI Receive Data.\n\n   Reads from this window pull data from the RXFIFO.\n\n   The serial order of bit transmission\n   is chosen to match SPI flash devices. Individual bytes\n   are always transmitted with the most significant bit first.\n   Only four-byte reads are supported. If ByteOrder = 0,\n   the first byte received is packed in the MSB of !!RXDATA.\n   For some processor architectures, this could lead to shuffling\n   of flash data as compared to how it is written in memory.\n   In which case, choosing ByteOrder = 1 can reverse the\n   byte-order of each data read, causing the first byte\n   received to be packed into the LSB of !!RXDATA. (Though within\n   each byte the most significant bit is always pulled\n   from the bus first.)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rxdata(&self) -> ureg::RegRef<crate::meta::Rxdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI Transmit Data.\n\n   Data written to this window is placed into the TXFIFO.\n   Byte-enables are supported for writes.\n\n   The serial order of bit transmission\n   is chosen to match SPI flash devices. Individual bytes\n   are always transmitted with the most significant bit first.\n   Multi-byte writes are also supported, and if ByteOrder = 0,\n   the bits of !!TXDATA are transmitted strictly in order of\n   decreasing signficance (i.e. most signicant bit first).\n   For some processor architectures, this could lead to shuffling\n   of flash data as compared to how it is written in memory.\n   In which case, choosing ByteOrder = 1 can reverse the\n   byte-order of multi-byte data writes.  (Though within\n   each byte the most significant bit is always sent first.)\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn txdata(&self) -> ureg::RegRef<crate::meta::Txdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls which classes of errors raise an interrupt.\n\nRead value: [`regs::ErrorEnableReadVal`]; Write value: [`regs::ErrorEnableWriteVal`]"]
    #[inline(always)]
    pub fn error_enable(&self) -> ureg::RegRef<crate::meta::ErrorEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Indicates that any errors that have occurred.\n   When an error\n   occurs, the corresponding bit must be cleared here before\n   issuing any further commands.\n\nRead value: [`regs::ErrorStatusReadVal`]; Write value: [`regs::ErrorStatusWriteVal`]"]
    #[inline(always)]
    pub fn error_status(&self) -> ureg::RegRef<crate::meta::ErrorStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls which classes of SPI events raise an interrupt.\n\nRead value: [`regs::EventEnableReadVal`]; Write value: [`regs::EventEnableWriteVal`]"]
    #[inline(always)]
    pub fn event_enable(&self) -> ureg::RegRef<crate::meta::EventEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
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
    pub struct CommandWriteVal(u32);
    impl CommandWriteVal {
        #[doc = "Segment Length.\n\n   For read or write segments, this field controls the\n   number of 1-byte bursts to transmit and or receive in\n   this command segment.  The number of cyles required\n   to send or received a byte will depend on !!COMMAND.SPEED.\n   For dummy segments, (!!COMMAND.DIRECTION == 0), this register\n   controls the number of dummy cycles to issue.\n   The number of bytes (or dummy cycles) in the segment will be\n   equal to !!COMMAND.LEN + 1."]
        #[inline(always)]
        pub fn len(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "**C**hip **S**elect **A**ctive **A**fter **T**ransaction.\n   If !!COMMAND.CSAAT = 0, the chip select line is raised immediately\n   at the end of the command segment.\n   If !!COMMAND.CSAAT = 1, the chip select line is left low at the\n   end of the current transaction segment.\n   This allows the creation of longer, more complete SPI transactions,\n   consisting of several separate segments for issuing instructions,\n   pausing for dummy cycles, and transmitting or receiving data from\n   the device."]
        #[inline(always)]
        pub fn csaat(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "The speed for this command segment: \"0\" = Standard SPI. \"1\" = Dual SPI.\n   \"2\"=Quad SPI,  \"3\": RESERVED."]
        #[inline(always)]
        pub fn speed(self, val: u32) -> Self {
            Self((self.0 & !(3 << 10)) | ((val & 3) << 10))
        }
        #[doc = "The direction for the following command: \"0\" = Dummy cycles\n   (no TX/RX). \"1\" = Rx only, \"2\" = Tx only, \"3\" = Bidirectional\n   Tx/Rx (Standard SPI mode only)."]
        #[inline(always)]
        pub fn direction(self, val: u32) -> Self {
            Self((self.0 & !(3 << 12)) | ((val & 3) << 12))
        }
    }
    impl From<u32> for CommandWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CommandWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CommandWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ConfigoptsReadVal(u32);
    impl ConfigoptsReadVal {
        #[doc = "Core clock divider.  Slows down subsequent SPI transactions by a\n   factor of (CLKDIV+1) relative to the core clock frequency.  The\n   period of sck, T(sck) then becomes `2*(CLK_DIV+1)*T(core)`"]
        #[inline(always)]
        pub fn clkdiv(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Minimum idle time between commands. Indicates the minimum\n   number of sck half-cycles to hold cs_n high between commands.\n   Setting this register to zero creates a minimally-wide CS_N-high\n   pulse of one-half sck cycle."]
        #[inline(always)]
        pub fn csnidle(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "CS_N Trailing Time.  Indicates the number of half sck cycles,\n   CSNTRAIL+1, to leave between last edge of sck and the rising\n   edge of cs_n. Setting this register to zero corresponds\n   to the minimum delay of one-half sck cycle."]
        #[inline(always)]
        pub fn csntrail(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
        #[doc = "CS_N Leading Time.  Indicates the number of half sck cycles,\n   CSNLEAD+1, to leave between the falling edge of cs_n and\n   the first edge of sck.  Setting this register to zero\n   corresponds to the minimum delay of one-half sck cycle"]
        #[inline(always)]
        pub fn csnlead(&self) -> u32 {
            (self.0 >> 24) & 0xf
        }
        #[doc = "Full cycle.  Modifies the CPHA sampling behaviour to allow\n   for longer device logic setup times.  Rather than sampling the SD\n   bus a half cycle after shifting out data, the data is sampled\n   a full cycle after shifting data out.  This means that if\n   CPHA = 0, data is shifted out on the trailing edge, and\n   sampled a full cycle later.  If CPHA = 1, data is shifted and\n   sampled with the trailing edge, also separated by a\n   full cycle."]
        #[inline(always)]
        pub fn fullcyc(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "The phase of the sck clock signal relative to the data. When\n   CPHA = 0, the data changes on the trailing edge of sck\n   and is typically sampled on the leading edge.  Conversely\n   if CPHA = 1 high, data lines change on the leading edge of\n   sck and are typically sampled on the trailing edge.\n   CPHA should be chosen to match the phase of the selected\n   device.  The sampling behavior is modified by the\n   !!CONFIGOPTS.FULLCYC bit."]
        #[inline(always)]
        pub fn cpha(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "The polarity of the sck clock signal.  When CPOL is 0,\n   sck is low when idle, and emits high pulses.   When CPOL\n   is 1, sck is high when idle, and emits a series of low\n   pulses."]
        #[inline(always)]
        pub fn cpol(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ConfigoptsWriteVal {
            ConfigoptsWriteVal(self.0)
        }
    }
    impl From<u32> for ConfigoptsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ConfigoptsReadVal> for u32 {
        #[inline(always)]
        fn from(val: ConfigoptsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ConfigoptsWriteVal(u32);
    impl ConfigoptsWriteVal {
        #[doc = "Core clock divider.  Slows down subsequent SPI transactions by a\n   factor of (CLKDIV+1) relative to the core clock frequency.  The\n   period of sck, T(sck) then becomes `2*(CLK_DIV+1)*T(core)`"]
        #[inline(always)]
        pub fn clkdiv(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Minimum idle time between commands. Indicates the minimum\n   number of sck half-cycles to hold cs_n high between commands.\n   Setting this register to zero creates a minimally-wide CS_N-high\n   pulse of one-half sck cycle."]
        #[inline(always)]
        pub fn csnidle(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 16)) | ((val & 0xf) << 16))
        }
        #[doc = "CS_N Trailing Time.  Indicates the number of half sck cycles,\n   CSNTRAIL+1, to leave between last edge of sck and the rising\n   edge of cs_n. Setting this register to zero corresponds\n   to the minimum delay of one-half sck cycle."]
        #[inline(always)]
        pub fn csntrail(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 20)) | ((val & 0xf) << 20))
        }
        #[doc = "CS_N Leading Time.  Indicates the number of half sck cycles,\n   CSNLEAD+1, to leave between the falling edge of cs_n and\n   the first edge of sck.  Setting this register to zero\n   corresponds to the minimum delay of one-half sck cycle"]
        #[inline(always)]
        pub fn csnlead(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 24)) | ((val & 0xf) << 24))
        }
        #[doc = "Full cycle.  Modifies the CPHA sampling behaviour to allow\n   for longer device logic setup times.  Rather than sampling the SD\n   bus a half cycle after shifting out data, the data is sampled\n   a full cycle after shifting data out.  This means that if\n   CPHA = 0, data is shifted out on the trailing edge, and\n   sampled a full cycle later.  If CPHA = 1, data is shifted and\n   sampled with the trailing edge, also separated by a\n   full cycle."]
        #[inline(always)]
        pub fn fullcyc(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "The phase of the sck clock signal relative to the data. When\n   CPHA = 0, the data changes on the trailing edge of sck\n   and is typically sampled on the leading edge.  Conversely\n   if CPHA = 1 high, data lines change on the leading edge of\n   sck and are typically sampled on the trailing edge.\n   CPHA should be chosen to match the phase of the selected\n   device.  The sampling behavior is modified by the\n   !!CONFIGOPTS.FULLCYC bit."]
        #[inline(always)]
        pub fn cpha(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "The polarity of the sck clock signal.  When CPOL is 0,\n   sck is low when idle, and emits high pulses.   When CPOL\n   is 1, sck is high when idle, and emits a series of low\n   pulses."]
        #[inline(always)]
        pub fn cpol(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for ConfigoptsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ConfigoptsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ConfigoptsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlReadVal(u32);
    impl ControlReadVal {
        #[doc = "If !!EVENT_ENABLE.RXWM is set, the IP will send\n   an interrupt when the depth of the RX FIFO reaches\n   RX_WATERMARK words (32b each)."]
        #[inline(always)]
        pub fn rx_watermark(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If !!EVENT_ENABLE.TXWM is set, the IP will send\n   an interrupt when the depth of the TX FIFO drops below\n   TX_WATERMARK words (32b each)."]
        #[inline(always)]
        pub fn tx_watermark(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = "Enable the SPI host output buffers for the sck, csb, and sd lines.  This allows\n   the SPI_HOST IP to connect to the same bus as other SPI controllers without\n   interference."]
        #[inline(always)]
        pub fn output_en(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Clears the entire IP to the reset state when set to 1, including\n   the FIFOs, the CDC's, the core state machine and the shift register.\n   In the current implementation, the CDC FIFOs are drained not reset.\n   Therefore software must confirm that both FIFO's empty before releasing\n   the IP from reset."]
        #[inline(always)]
        pub fn sw_rst(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Enables the SPI host.  On reset, this field is 0, meaning\n   that no transactions can proceed."]
        #[inline(always)]
        pub fn spien(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
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
        #[doc = "If !!EVENT_ENABLE.RXWM is set, the IP will send\n   an interrupt when the depth of the RX FIFO reaches\n   RX_WATERMARK words (32b each)."]
        #[inline(always)]
        pub fn rx_watermark(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "If !!EVENT_ENABLE.TXWM is set, the IP will send\n   an interrupt when the depth of the TX FIFO drops below\n   TX_WATERMARK words (32b each)."]
        #[inline(always)]
        pub fn tx_watermark(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 8)) | ((val & 0xff) << 8))
        }
        #[doc = "Enable the SPI host output buffers for the sck, csb, and sd lines.  This allows\n   the SPI_HOST IP to connect to the same bus as other SPI controllers without\n   interference."]
        #[inline(always)]
        pub fn output_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Clears the entire IP to the reset state when set to 1, including\n   the FIFOs, the CDC's, the core state machine and the shift register.\n   In the current implementation, the CDC FIFOs are drained not reset.\n   Therefore software must confirm that both FIFO's empty before releasing\n   the IP from reset."]
        #[inline(always)]
        pub fn sw_rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Enables the SPI host.  On reset, this field is 0, meaning\n   that no transactions can proceed."]
        #[inline(always)]
        pub fn spien(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
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
    pub struct ErrorEnableReadVal(u32);
    impl ErrorEnableReadVal {
        #[doc = "Command Error: If this bit is set, the block sends an error\n   interrupt whenever a command is issued while busy (i.e. a 1 is\n   when !!STATUS.READY is not asserted.)"]
        #[inline(always)]
        pub fn cmdbusy(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Overflow Errors: If this bit is set, the block sends an\n   error interrupt whenever the TX FIFO overflows."]
        #[inline(always)]
        pub fn overflow(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Underflow Errors: If this bit is set, the block sends an\n   error interrupt whenever there is a read from !!RXDATA\n   but the RX FIFO is empty."]
        #[inline(always)]
        pub fn underflow(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Invalid Command Errors: If this bit is set, the block sends an\n   error interrupt whenever a command is sent with invalid values for\n   !!COMMAND.SPEED or !!COMMAND.DIRECTION."]
        #[inline(always)]
        pub fn cmdinval(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Invalid CSID: If this bit is set, the block sends an error interrupt whenever\n   a command is submitted, but CSID exceeds NumCS."]
        #[inline(always)]
        pub fn csidinval(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrorEnableWriteVal {
            ErrorEnableWriteVal(self.0)
        }
    }
    impl From<u32> for ErrorEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrorEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrorEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrorEnableWriteVal(u32);
    impl ErrorEnableWriteVal {
        #[doc = "Command Error: If this bit is set, the block sends an error\n   interrupt whenever a command is issued while busy (i.e. a 1 is\n   when !!STATUS.READY is not asserted.)"]
        #[inline(always)]
        pub fn cmdbusy(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Overflow Errors: If this bit is set, the block sends an\n   error interrupt whenever the TX FIFO overflows."]
        #[inline(always)]
        pub fn overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Underflow Errors: If this bit is set, the block sends an\n   error interrupt whenever there is a read from !!RXDATA\n   but the RX FIFO is empty."]
        #[inline(always)]
        pub fn underflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Invalid Command Errors: If this bit is set, the block sends an\n   error interrupt whenever a command is sent with invalid values for\n   !!COMMAND.SPEED or !!COMMAND.DIRECTION."]
        #[inline(always)]
        pub fn cmdinval(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Invalid CSID: If this bit is set, the block sends an error interrupt whenever\n   a command is submitted, but CSID exceeds NumCS."]
        #[inline(always)]
        pub fn csidinval(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
    }
    impl From<u32> for ErrorEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrorEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrorEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrorStatusReadVal(u32);
    impl ErrorStatusReadVal {
        #[doc = "Indicates a write to !!COMMAND when !!STATUS.READY = 0.\n   "]
        #[inline(always)]
        pub fn cmdbusy(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Indicates that firmware has overflowed the TX FIFO"]
        #[inline(always)]
        pub fn overflow(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Indicates that firmware has attempted to read from\n   !!RXDATA when the RX FIFO is empty."]
        #[inline(always)]
        pub fn underflow(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Indicates an invalid command segment, meaning either an invalid value of\n   !!COMMAND.SPEED or a request for bidirectional data transfer at dual or quad\n   speed"]
        #[inline(always)]
        pub fn cmdinval(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Indicates a command was attempted with an invalid value for !!CSID."]
        #[inline(always)]
        pub fn csidinval(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Indicates that TLUL attempted to write to TXDATA with no bytes enabled. Such\n   'zero byte' writes are not supported."]
        #[inline(always)]
        pub fn accessinval(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ErrorStatusWriteVal {
            ErrorStatusWriteVal(self.0)
        }
    }
    impl From<u32> for ErrorStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrorStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: ErrorStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ErrorStatusWriteVal(u32);
    impl ErrorStatusWriteVal {
        #[doc = "Indicates a write to !!COMMAND when !!STATUS.READY = 0.\n   "]
        #[inline(always)]
        pub fn cmdbusy_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Indicates that firmware has overflowed the TX FIFO"]
        #[inline(always)]
        pub fn overflow_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Indicates that firmware has attempted to read from\n   !!RXDATA when the RX FIFO is empty."]
        #[inline(always)]
        pub fn underflow_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Indicates an invalid command segment, meaning either an invalid value of\n   !!COMMAND.SPEED or a request for bidirectional data transfer at dual or quad\n   speed"]
        #[inline(always)]
        pub fn cmdinval_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "Indicates a command was attempted with an invalid value for !!CSID."]
        #[inline(always)]
        pub fn csidinval_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "Indicates that TLUL attempted to write to TXDATA with no bytes enabled. Such\n   'zero byte' writes are not supported."]
        #[inline(always)]
        pub fn accessinval_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
    }
    impl From<u32> for ErrorStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ErrorStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ErrorStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EventEnableReadVal(u32);
    impl EventEnableReadVal {
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.RXFULL\n   goes high"]
        #[inline(always)]
        pub fn rxfull(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.TXEMPTY\n   goes high"]
        #[inline(always)]
        pub fn txempty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Assert to send a spi_event interrupt whenever the number of 32-bit words in\n   the RX FIFO is greater than !!CONTROL.RX_WATERMARK. To prevent the\n   reassertion of this interrupt, read more data from the RX FIFO, or\n   increase !!CONTROL.RX_WATERMARK."]
        #[inline(always)]
        pub fn rxwm(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Assert to send a spi_event interrupt whenever the number of 32-bit words in\n   the TX FIFO is less than !!CONTROL.TX_WATERMARK.  To prevent the\n   reassertion of this interrupt add more data to the TX FIFO, or\n   reduce !!CONTROL.TX_WATERMARK."]
        #[inline(always)]
        pub fn txwm(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.READY\n   goes high"]
        #[inline(always)]
        pub fn ready(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.ACTIVE\n   goes low"]
        #[inline(always)]
        pub fn idle(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EventEnableWriteVal {
            EventEnableWriteVal(self.0)
        }
    }
    impl From<u32> for EventEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EventEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: EventEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EventEnableWriteVal(u32);
    impl EventEnableWriteVal {
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.RXFULL\n   goes high"]
        #[inline(always)]
        pub fn rxfull(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.TXEMPTY\n   goes high"]
        #[inline(always)]
        pub fn txempty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Assert to send a spi_event interrupt whenever the number of 32-bit words in\n   the RX FIFO is greater than !!CONTROL.RX_WATERMARK. To prevent the\n   reassertion of this interrupt, read more data from the RX FIFO, or\n   increase !!CONTROL.RX_WATERMARK."]
        #[inline(always)]
        pub fn rxwm(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Assert to send a spi_event interrupt whenever the number of 32-bit words in\n   the TX FIFO is less than !!CONTROL.TX_WATERMARK.  To prevent the\n   reassertion of this interrupt add more data to the TX FIFO, or\n   reduce !!CONTROL.TX_WATERMARK."]
        #[inline(always)]
        pub fn txwm(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.READY\n   goes high"]
        #[inline(always)]
        pub fn ready(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Assert to send a spi_event interrupt whenever !!STATUS.ACTIVE\n   goes low"]
        #[inline(always)]
        pub fn idle(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
    }
    impl From<u32> for EventEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EventEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: EventEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.error is set."]
        #[inline(always)]
        pub fn error(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.spi_event is set."]
        #[inline(always)]
        pub fn spi_event(&self) -> bool {
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
        #[doc = "Enable interrupt when !!INTR_STATE.error is set."]
        #[inline(always)]
        pub fn error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.spi_event is set."]
        #[inline(always)]
        pub fn spi_event(self, val: bool) -> Self {
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
        #[doc = "Error-related interrupts, see !!ERROR_ENABLE register for more\n   information."]
        #[inline(always)]
        pub fn error(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Event-related interrupts, see !!EVENT_ENABLE register for more\n   information."]
        #[inline(always)]
        pub fn spi_event(&self) -> bool {
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
        #[doc = "Error-related interrupts, see !!ERROR_ENABLE register for more\n   information."]
        #[inline(always)]
        pub fn error_clear(self) -> Self {
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
        #[doc = "Write 1 to force !!INTR_STATE.error to 1."]
        #[inline(always)]
        pub fn error(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.spi_event to 1."]
        #[inline(always)]
        pub fn spi_event(self, val: bool) -> Self {
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
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "Transmit queue depth.\n   Indicates how many unsent 32-bit words are currently in the TX FIFO.\n   When active, this result may be an overestimate due to synchronization delays."]
        #[inline(always)]
        pub fn txqd(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Receive queue depth. Indicates how many unread 32-bit words are\n   currently in the RX FIFO.  When active, this result may an\n   underestimate due to synchronization delays."]
        #[inline(always)]
        pub fn rxqd(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = "Command queue depth. Indicates how many unread 32-bit words are\n   currently in the command segment queue."]
        #[inline(always)]
        pub fn cmdqd(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "If high, the number of 32-bits in the RX FIFO now exceeds the\n   !!CONTROL.RX_WATERMARK entries (32b each)."]
        #[inline(always)]
        pub fn rxwm(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "The value of the ByteOrder parameter, provided so that firmware\n   can confirm proper IP configuration."]
        #[inline(always)]
        pub fn byteorder(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If high, signifies that an ongoing transaction has stalled\n   due to lack of available space in the RX FIFO"]
        #[inline(always)]
        pub fn rxstall(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "When high, indicates that the receive fifo is empty.\n   Any reads from RX FIFO will cause an error interrupt."]
        #[inline(always)]
        pub fn rxempty(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "When high, indicates that the receive fifo is full.  Any\n   ongoing transactions will stall until firmware reads some\n   data from !!RXDATA."]
        #[inline(always)]
        pub fn rxfull(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If high, the amount of data in the TX FIFO has fallen below the\n   level of !!CONTROL.TX_WATERMARK words (32b each)."]
        #[inline(always)]
        pub fn txwm(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If high, signifies that an ongoing transaction has stalled\n   due to lack of data in the TX FIFO"]
        #[inline(always)]
        pub fn txstall(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "When high, indicates that the transmit data fifo is empty."]
        #[inline(always)]
        pub fn txempty(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "When high, indicates that the transmit data fifo is full.\n   Any further writes to !!TXDATA will create an error interrupt."]
        #[inline(always)]
        pub fn txfull(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "When high, indicates the SPI host is processing a previously\n   issued command."]
        #[inline(always)]
        pub fn active(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "When high, indicates the SPI host is ready to receive\n   commands. Writing to COMMAND when READY is low is\n   an error, and will trigger an interrupt."]
        #[inline(always)]
        pub fn ready(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
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
    pub type Control =
        ureg::ReadWriteReg32<0x7f, crate::regs::ControlReadVal, crate::regs::ControlWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type Configopts =
        ureg::ReadWriteReg32<0, crate::regs::ConfigoptsReadVal, crate::regs::ConfigoptsWriteVal>;
    pub type Csid = ureg::ReadWriteReg32<0, u32, u32>;
    pub type Command = ureg::WriteOnlyReg32<0, crate::regs::CommandWriteVal>;
    pub type Rxdata = ureg::ReadOnlyReg32<u32>;
    pub type Txdata = ureg::WriteOnlyReg32<0, u32>;
    pub type ErrorEnable = ureg::ReadWriteReg32<
        0x1f,
        crate::regs::ErrorEnableReadVal,
        crate::regs::ErrorEnableWriteVal,
    >;
    pub type ErrorStatus =
        ureg::ReadWriteReg32<0, crate::regs::ErrorStatusReadVal, crate::regs::ErrorStatusWriteVal>;
    pub type EventEnable =
        ureg::ReadWriteReg32<0, crate::regs::EventEnableReadVal, crate::regs::EventEnableWriteVal>;
}
