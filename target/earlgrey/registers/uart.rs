#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Uart0 {
    _priv: (),
}
impl Uart0 {
    pub const PTR: *mut u32 = 0x40000000 as *mut u32;
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Caller must ensure that all concurrent use of this"]
    #[doc = r" peripheral in the firmware is done so in a compatible"]
    #[doc = r" way. The simplest way to enforce this is to only call"]
    #[doc = r" this function once."]
    #[inline(always)]
    pub const unsafe fn new() -> Self {
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
pub struct Uart1 {
    _priv: (),
}
impl Uart1 {
    pub const PTR: *mut u32 = 0x40010000 as *mut u32;
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
pub struct Uart2 {
    _priv: (),
}
impl Uart2 {
    pub const PTR: *mut u32 = 0x40020000 as *mut u32;
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
pub struct Uart3 {
    _priv: (),
}
impl Uart3 {
    pub const PTR: *mut u32 = 0x40030000 as *mut u32;
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
    #[doc = "UART control register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART live status register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART read data\n\nRead value: [`regs::RdataReadVal`]; Write value: [`regs::RdataWriteVal`]"]
    #[inline(always)]
    pub fn rdata(&self) -> ureg::RegRef<crate::meta::Rdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART write data\n\nRead value: [`regs::WdataReadVal`]; Write value: [`regs::WdataWriteVal`]"]
    #[inline(always)]
    pub fn wdata(&self) -> ureg::RegRef<crate::meta::Wdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART FIFO control register\n\nRead value: [`regs::FifoCtrlReadVal`]; Write value: [`regs::FifoCtrlWriteVal`]"]
    #[inline(always)]
    pub fn fifo_ctrl(&self) -> ureg::RegRef<crate::meta::FifoCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART FIFO status register\n\nRead value: [`regs::FifoStatusReadVal`]; Write value: [`regs::FifoStatusWriteVal`]"]
    #[inline(always)]
    pub fn fifo_status(&self) -> ureg::RegRef<crate::meta::FifoStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TX pin override control. Gives direct SW control over TX pin state\n\nRead value: [`regs::OvrdReadVal`]; Write value: [`regs::OvrdWriteVal`]"]
    #[inline(always)]
    pub fn ovrd(&self) -> ureg::RegRef<crate::meta::Ovrd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART oversampled values\n\nRead value: [`regs::ValReadVal`]; Write value: [`regs::ValWriteVal`]"]
    #[inline(always)]
    pub fn val(&self) -> ureg::RegRef<crate::meta::Val, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "UART RX timeout control\n\nRead value: [`regs::TimeoutCtrlReadVal`]; Write value: [`regs::TimeoutCtrlWriteVal`]"]
    #[inline(always)]
    pub fn timeout_ctrl(&self) -> ureg::RegRef<crate::meta::TimeoutCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
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
    pub struct CtrlReadVal(u32);
    impl CtrlReadVal {
        #[doc = "TX enable"]
        #[inline(always)]
        pub fn tx(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "RX enable"]
        #[inline(always)]
        pub fn rx(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "RX noise filter enable.\nIf the noise filter is enabled, RX line goes through the 3-tap\nrepetition code. It ignores single IP clock period noise."]
        #[inline(always)]
        pub fn nf(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "System loopback enable.\n\nIf this bit is turned on, any outgoing bits to TX are received through RX.\nSee Block Diagram. Note that the TX line goes 1 if System loopback is enabled."]
        #[inline(always)]
        pub fn slpbk(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Line loopback enable.\n\nIf this bit is turned on, incoming bits are forwarded to TX for testing purpose.\nSee Block Diagram. Note that the internal design sees RX value as 1 always if line\nloopback is enabled."]
        #[inline(always)]
        pub fn llpbk(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If true, parity is enabled in both RX and TX directions."]
        #[inline(always)]
        pub fn parity_en(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If PARITY_EN is true, this determines the type, 1 for odd parity, 0 for even."]
        #[inline(always)]
        pub fn parity_odd(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Trigger level for RX break detection. Sets the number of character\ntimes the line must be low to detect a break."]
        #[inline(always)]
        pub fn rxblvl(&self) -> super::enums::Rxblvl {
            super::enums::Rxblvl::try_from((self.0 >> 8) & 3).unwrap()
        }
        #[doc = "BAUD clock rate control."]
        #[inline(always)]
        pub fn nco(&self) -> u32 {
            (self.0 >> 16) & 0xffff
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
        #[doc = "TX enable"]
        #[inline(always)]
        pub fn tx(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "RX enable"]
        #[inline(always)]
        pub fn rx(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "RX noise filter enable.\nIf the noise filter is enabled, RX line goes through the 3-tap\nrepetition code. It ignores single IP clock period noise."]
        #[inline(always)]
        pub fn nf(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "System loopback enable.\n\nIf this bit is turned on, any outgoing bits to TX are received through RX.\nSee Block Diagram. Note that the TX line goes 1 if System loopback is enabled."]
        #[inline(always)]
        pub fn slpbk(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Line loopback enable.\n\nIf this bit is turned on, incoming bits are forwarded to TX for testing purpose.\nSee Block Diagram. Note that the internal design sees RX value as 1 always if line\nloopback is enabled."]
        #[inline(always)]
        pub fn llpbk(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If true, parity is enabled in both RX and TX directions."]
        #[inline(always)]
        pub fn parity_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If PARITY_EN is true, this determines the type, 1 for odd parity, 0 for even."]
        #[inline(always)]
        pub fn parity_odd(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Trigger level for RX break detection. Sets the number of character\ntimes the line must be low to detect a break."]
        #[inline(always)]
        pub fn rxblvl(
            self,
            f: impl FnOnce(super::enums::selector::RxblvlSelector) -> super::enums::Rxblvl,
        ) -> Self {
            Self(
                (self.0 & !(3 << 8))
                    | (u32::from(f(super::enums::selector::RxblvlSelector())) << 8),
            )
        }
        #[doc = "BAUD clock rate control."]
        #[inline(always)]
        pub fn nco(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
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
    pub struct FifoCtrlReadVal(u32);
    impl FifoCtrlReadVal {
        #[doc = "Trigger level for RX interrupts. If the FIFO depth is greater than or equal to\nthe setting, it raises rx_watermark interrupt."]
        #[inline(always)]
        pub fn rxilvl(&self) -> super::enums::Rxilvl {
            super::enums::Rxilvl::try_from((self.0 >> 2) & 7).unwrap()
        }
        #[doc = "Trigger level for TX interrupts. If the FIFO depth is less than the setting, it\nraises tx_watermark interrupt."]
        #[inline(always)]
        pub fn txilvl(&self) -> super::enums::Txilvl {
            super::enums::Txilvl::try_from((self.0 >> 5) & 7).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FifoCtrlWriteVal {
            FifoCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for FifoCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: FifoCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoCtrlWriteVal(u32);
    impl FifoCtrlWriteVal {
        #[doc = "RX fifo reset. Write 1 to the register resets RX_FIFO. Read returns 0"]
        #[inline(always)]
        pub fn rxrst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "TX fifo reset. Write 1 to the register resets TX_FIFO. Read returns 0"]
        #[inline(always)]
        pub fn txrst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Trigger level for RX interrupts. If the FIFO depth is greater than or equal to\nthe setting, it raises rx_watermark interrupt."]
        #[inline(always)]
        pub fn rxilvl(
            self,
            f: impl FnOnce(super::enums::selector::RxilvlSelector) -> super::enums::Rxilvl,
        ) -> Self {
            Self(
                (self.0 & !(7 << 2))
                    | (u32::from(f(super::enums::selector::RxilvlSelector())) << 2),
            )
        }
        #[doc = "Trigger level for TX interrupts. If the FIFO depth is less than the setting, it\nraises tx_watermark interrupt."]
        #[inline(always)]
        pub fn txilvl(
            self,
            f: impl FnOnce(super::enums::selector::TxilvlSelector) -> super::enums::Txilvl,
        ) -> Self {
            Self(
                (self.0 & !(7 << 5))
                    | (u32::from(f(super::enums::selector::TxilvlSelector())) << 5),
            )
        }
    }
    impl From<u32> for FifoCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FifoCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoStatusReadVal(u32);
    impl FifoStatusReadVal {
        #[doc = "Current fill level of TX fifo"]
        #[inline(always)]
        pub fn txlvl(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Current fill level of RX fifo"]
        #[inline(always)]
        pub fn rxlvl(&self) -> u32 {
            (self.0 >> 16) & 0xff
        }
    }
    impl From<u32> for FifoStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FifoStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: FifoStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.tx_watermark is set."]
        #[inline(always)]
        pub fn tx_watermark(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_watermark is set."]
        #[inline(always)]
        pub fn rx_watermark(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_done is set."]
        #[inline(always)]
        pub fn tx_done(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_overflow is set."]
        #[inline(always)]
        pub fn rx_overflow(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_frame_err is set."]
        #[inline(always)]
        pub fn rx_frame_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_break_err is set."]
        #[inline(always)]
        pub fn rx_break_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_timeout is set."]
        #[inline(always)]
        pub fn rx_timeout(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_parity_err is set."]
        #[inline(always)]
        pub fn rx_parity_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_empty is set."]
        #[inline(always)]
        pub fn tx_empty(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.tx_watermark is set."]
        #[inline(always)]
        pub fn tx_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_watermark is set."]
        #[inline(always)]
        pub fn rx_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_done is set."]
        #[inline(always)]
        pub fn tx_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_overflow is set."]
        #[inline(always)]
        pub fn rx_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_frame_err is set."]
        #[inline(always)]
        pub fn rx_frame_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_break_err is set."]
        #[inline(always)]
        pub fn rx_break_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_timeout is set."]
        #[inline(always)]
        pub fn rx_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_parity_err is set."]
        #[inline(always)]
        pub fn rx_parity_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_empty is set."]
        #[inline(always)]
        pub fn tx_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
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
        #[doc = "raised if the transmit FIFO is past the high-water mark."]
        #[inline(always)]
        pub fn tx_watermark(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "raised if the receive FIFO is past the high-water mark."]
        #[inline(always)]
        pub fn rx_watermark(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "raised if the transmit FIFO has emptied and no transmit is ongoing."]
        #[inline(always)]
        pub fn tx_done(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "raised if the receive FIFO has overflowed."]
        #[inline(always)]
        pub fn rx_overflow(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "raised if a framing error has been detected on receive."]
        #[inline(always)]
        pub fn rx_frame_err(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "raised if break condition has been detected on receive."]
        #[inline(always)]
        pub fn rx_break_err(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "raised if RX FIFO has characters remaining in the FIFO without being\nretrieved for the programmed time period."]
        #[inline(always)]
        pub fn rx_timeout(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "raised if the receiver has detected a parity error."]
        #[inline(always)]
        pub fn rx_parity_err(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "raised if the transmit FIFO is empty."]
        #[inline(always)]
        pub fn tx_empty(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
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
        #[doc = "raised if the transmit FIFO has emptied and no transmit is ongoing."]
        #[inline(always)]
        pub fn tx_done_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "raised if the receive FIFO has overflowed."]
        #[inline(always)]
        pub fn rx_overflow_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "raised if a framing error has been detected on receive."]
        #[inline(always)]
        pub fn rx_frame_err_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "raised if break condition has been detected on receive."]
        #[inline(always)]
        pub fn rx_break_err_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "raised if RX FIFO has characters remaining in the FIFO without being\nretrieved for the programmed time period."]
        #[inline(always)]
        pub fn rx_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "raised if the receiver has detected a parity error."]
        #[inline(always)]
        pub fn rx_parity_err_clear(self) -> Self {
            Self(self.0 | (1 << 7))
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
        #[doc = "Write 1 to force !!INTR_STATE.tx_watermark to 1."]
        #[inline(always)]
        pub fn tx_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_watermark to 1."]
        #[inline(always)]
        pub fn rx_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tx_done to 1."]
        #[inline(always)]
        pub fn tx_done(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_overflow to 1."]
        #[inline(always)]
        pub fn rx_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_frame_err to 1."]
        #[inline(always)]
        pub fn rx_frame_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_break_err to 1."]
        #[inline(always)]
        pub fn rx_break_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_timeout to 1."]
        #[inline(always)]
        pub fn rx_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_parity_err to 1."]
        #[inline(always)]
        pub fn rx_parity_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tx_empty to 1."]
        #[inline(always)]
        pub fn tx_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
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
    pub struct OvrdReadVal(u32);
    impl OvrdReadVal {
        #[doc = "Enable TX pin override control"]
        #[inline(always)]
        pub fn txen(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Write to set the value of the TX pin"]
        #[inline(always)]
        pub fn txval(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OvrdWriteVal {
            OvrdWriteVal(self.0)
        }
    }
    impl From<u32> for OvrdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OvrdReadVal> for u32 {
        #[inline(always)]
        fn from(val: OvrdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OvrdWriteVal(u32);
    impl OvrdWriteVal {
        #[doc = "Enable TX pin override control"]
        #[inline(always)]
        pub fn txen(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write to set the value of the TX pin"]
        #[inline(always)]
        pub fn txval(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for OvrdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OvrdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: OvrdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RdataReadVal(u32);
    impl RdataReadVal {
        #[inline(always)]
        pub fn rdata(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
    }
    impl From<u32> for RdataReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RdataReadVal> for u32 {
        #[inline(always)]
        fn from(val: RdataReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "TX buffer is full"]
        #[inline(always)]
        pub fn txfull(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "RX buffer is full"]
        #[inline(always)]
        pub fn rxfull(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "TX FIFO is empty"]
        #[inline(always)]
        pub fn txempty(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "TX FIFO is empty and all bits have been transmitted"]
        #[inline(always)]
        pub fn txidle(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "RX is idle"]
        #[inline(always)]
        pub fn rxidle(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "RX FIFO is empty"]
        #[inline(always)]
        pub fn rxempty(&self) -> bool {
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
    pub struct TimeoutCtrlReadVal(u32);
    impl TimeoutCtrlReadVal {
        #[doc = "RX timeout value in UART bit times"]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0xffffff
        }
        #[doc = "Enable RX timeout feature"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TimeoutCtrlWriteVal {
            TimeoutCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for TimeoutCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TimeoutCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: TimeoutCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TimeoutCtrlWriteVal(u32);
    impl TimeoutCtrlWriteVal {
        #[doc = "RX timeout value in UART bit times"]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0xffffff << 0)) | ((val & 0xffffff) << 0))
        }
        #[doc = "Enable RX timeout feature"]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for TimeoutCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TimeoutCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TimeoutCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ValReadVal(u32);
    impl ValReadVal {
        #[doc = "Last 16 oversampled values of RX. Most recent bit is bit 0, oldest 15."]
        #[inline(always)]
        pub fn rx(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
    }
    impl From<u32> for ValReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ValReadVal> for u32 {
        #[inline(always)]
        fn from(val: ValReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WdataWriteVal(u32);
    impl WdataWriteVal {
        #[inline(always)]
        pub fn wdata(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for WdataWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WdataWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WdataWriteVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Rxblvl {
        Break2 = 0,
        Break4 = 1,
        Break8 = 2,
        Break16 = 3,
    }
    impl Rxblvl {
        #[inline(always)]
        pub fn break2(&self) -> bool {
            *self == Self::Break2
        }
        #[inline(always)]
        pub fn break4(&self) -> bool {
            *self == Self::Break4
        }
        #[inline(always)]
        pub fn break8(&self) -> bool {
            *self == Self::Break8
        }
        #[inline(always)]
        pub fn break16(&self) -> bool {
            *self == Self::Break16
        }
    }
    impl TryFrom<u32> for Rxblvl {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Rxblvl, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Rxblvl>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Rxblvl> for u32 {
        fn from(val: Rxblvl) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Rxilvl {
        Rxlvl1 = 0,
        Rxlvl2 = 1,
        Rxlvl4 = 2,
        Rxlvl8 = 3,
        Rxlvl16 = 4,
        Rxlvl32 = 5,
        Rxlvl62 = 6,
        Reserved7 = 7,
    }
    impl Rxilvl {
        #[inline(always)]
        pub fn rxlvl1(&self) -> bool {
            *self == Self::Rxlvl1
        }
        #[inline(always)]
        pub fn rxlvl2(&self) -> bool {
            *self == Self::Rxlvl2
        }
        #[inline(always)]
        pub fn rxlvl4(&self) -> bool {
            *self == Self::Rxlvl4
        }
        #[inline(always)]
        pub fn rxlvl8(&self) -> bool {
            *self == Self::Rxlvl8
        }
        #[inline(always)]
        pub fn rxlvl16(&self) -> bool {
            *self == Self::Rxlvl16
        }
        #[inline(always)]
        pub fn rxlvl32(&self) -> bool {
            *self == Self::Rxlvl32
        }
        #[inline(always)]
        pub fn rxlvl62(&self) -> bool {
            *self == Self::Rxlvl62
        }
    }
    impl TryFrom<u32> for Rxilvl {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Rxilvl, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Rxilvl>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Rxilvl> for u32 {
        fn from(val: Rxilvl) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Txilvl {
        Txlvl1 = 0,
        Txlvl2 = 1,
        Txlvl4 = 2,
        Txlvl8 = 3,
        Txlvl16 = 4,
        Reserved5 = 5,
        Reserved6 = 6,
        Reserved7 = 7,
    }
    impl Txilvl {
        #[inline(always)]
        pub fn txlvl1(&self) -> bool {
            *self == Self::Txlvl1
        }
        #[inline(always)]
        pub fn txlvl2(&self) -> bool {
            *self == Self::Txlvl2
        }
        #[inline(always)]
        pub fn txlvl4(&self) -> bool {
            *self == Self::Txlvl4
        }
        #[inline(always)]
        pub fn txlvl8(&self) -> bool {
            *self == Self::Txlvl8
        }
        #[inline(always)]
        pub fn txlvl16(&self) -> bool {
            *self == Self::Txlvl16
        }
    }
    impl TryFrom<u32> for Txilvl {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Txilvl, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Txilvl>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Txilvl> for u32 {
        fn from(val: Txilvl) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct RxblvlSelector();
        impl RxblvlSelector {
            #[inline(always)]
            pub fn break2(&self) -> super::Rxblvl {
                super::Rxblvl::Break2
            }
            #[inline(always)]
            pub fn break4(&self) -> super::Rxblvl {
                super::Rxblvl::Break4
            }
            #[inline(always)]
            pub fn break8(&self) -> super::Rxblvl {
                super::Rxblvl::Break8
            }
            #[inline(always)]
            pub fn break16(&self) -> super::Rxblvl {
                super::Rxblvl::Break16
            }
        }
        pub struct RxilvlSelector();
        impl RxilvlSelector {
            #[inline(always)]
            pub fn rxlvl1(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl1
            }
            #[inline(always)]
            pub fn rxlvl2(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl2
            }
            #[inline(always)]
            pub fn rxlvl4(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl4
            }
            #[inline(always)]
            pub fn rxlvl8(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl8
            }
            #[inline(always)]
            pub fn rxlvl16(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl16
            }
            #[inline(always)]
            pub fn rxlvl32(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl32
            }
            #[inline(always)]
            pub fn rxlvl62(&self) -> super::Rxilvl {
                super::Rxilvl::Rxlvl62
            }
        }
        pub struct TxilvlSelector();
        impl TxilvlSelector {
            #[inline(always)]
            pub fn txlvl1(&self) -> super::Txilvl {
                super::Txilvl::Txlvl1
            }
            #[inline(always)]
            pub fn txlvl2(&self) -> super::Txilvl {
                super::Txilvl::Txlvl2
            }
            #[inline(always)]
            pub fn txlvl4(&self) -> super::Txilvl {
                super::Txilvl::Txlvl4
            }
            #[inline(always)]
            pub fn txlvl8(&self) -> super::Txilvl {
                super::Txilvl::Txlvl8
            }
            #[inline(always)]
            pub fn txlvl16(&self) -> super::Txilvl {
                super::Txilvl::Txlvl16
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<0x101, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Ctrl = ureg::ReadWriteReg32<0, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type Rdata = ureg::ReadOnlyReg32<crate::regs::RdataReadVal>;
    pub type Wdata = ureg::WriteOnlyReg32<0, crate::regs::WdataWriteVal>;
    pub type FifoCtrl =
        ureg::ReadWriteReg32<0, crate::regs::FifoCtrlReadVal, crate::regs::FifoCtrlWriteVal>;
    pub type FifoStatus = ureg::ReadOnlyReg32<crate::regs::FifoStatusReadVal>;
    pub type Ovrd = ureg::ReadWriteReg32<0, crate::regs::OvrdReadVal, crate::regs::OvrdWriteVal>;
    pub type Val = ureg::ReadOnlyReg32<crate::regs::ValReadVal>;
    pub type TimeoutCtrl =
        ureg::ReadWriteReg32<0, crate::regs::TimeoutCtrlReadVal, crate::regs::TimeoutCtrlWriteVal>;
}
