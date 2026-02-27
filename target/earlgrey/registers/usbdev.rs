#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Usbdev {
    _priv: (),
}
impl Usbdev {
    pub const PTR: *mut u32 = 0x40320000 as *mut u32;
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
    #[doc = "USB Control\n\nRead value: [`regs::UsbctrlReadVal`]; Write value: [`regs::UsbctrlWriteVal`]"]
    #[inline(always)]
    pub fn usbctrl(&self) -> ureg::RegRef<crate::meta::Usbctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable an endpoint to respond to transactions in the downstream direction.\nNote that as the default endpoint, endpoint 0 must be enabled in both the IN and OUT directions before enabling the USB interface to connect.\n\nRead value: [`regs::EpOutEnable0ReadVal`]; Write value: [`regs::EpOutEnable0WriteVal`]"]
    #[inline(always)]
    pub fn ep_out_enable0(&self) -> ureg::RegRef<crate::meta::EpOutEnable0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Enable an endpoint to respond to transactions in the upstream direction.\nNote that as the default endpoint, endpoint 0 must be enabled in both the IN and OUT directions before enabling the USB interface to connect.\n\nRead value: [`regs::EpInEnable0ReadVal`]; Write value: [`regs::EpInEnable0WriteVal`]"]
    #[inline(always)]
    pub fn ep_in_enable0(&self) -> ureg::RegRef<crate::meta::EpInEnable0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB Status\n\nRead value: [`regs::UsbstatReadVal`]; Write value: [`regs::UsbstatWriteVal`]"]
    #[inline(always)]
    pub fn usbstat(&self) -> ureg::RegRef<crate::meta::Usbstat, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Available OUT Buffer FIFO\n\nRead value: [`regs::AvoutbufferReadVal`]; Write value: [`regs::AvoutbufferWriteVal`]"]
    #[inline(always)]
    pub fn avoutbuffer(&self) -> ureg::RegRef<crate::meta::Avoutbuffer, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Available SETUP Buffer FIFO\n\nRead value: [`regs::AvsetupbufferReadVal`]; Write value: [`regs::AvsetupbufferWriteVal`]"]
    #[inline(always)]
    pub fn avsetupbuffer(&self) -> ureg::RegRef<crate::meta::Avsetupbuffer, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Received Buffer FIFO\n\nRead value: [`regs::RxfifoReadVal`]; Write value: [`regs::RxfifoWriteVal`]"]
    #[inline(always)]
    pub fn rxfifo(&self) -> ureg::RegRef<crate::meta::Rxfifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Receive SETUP transaction enable\n\nRead value: [`regs::RxenableSetup0ReadVal`]; Write value: [`regs::RxenableSetup0WriteVal`]"]
    #[inline(always)]
    pub fn rxenable_setup0(&self) -> ureg::RegRef<crate::meta::RxenableSetup0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Receive OUT transaction enable\n\nRead value: [`regs::RxenableOut0ReadVal`]; Write value: [`regs::RxenableOut0WriteVal`]"]
    #[inline(always)]
    pub fn rxenable_out0(&self) -> ureg::RegRef<crate::meta::RxenableOut0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Set NAK after OUT transactions\n\nRead value: [`regs::SetNakOut0ReadVal`]; Write value: [`regs::SetNakOut0WriteVal`]"]
    #[inline(always)]
    pub fn set_nak_out0(&self) -> ureg::RegRef<crate::meta::SetNakOut0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "IN Transaction Sent\n\nRead value: [`regs::InSent0ReadVal`]; Write value: [`regs::InSent0WriteVal`]"]
    #[inline(always)]
    pub fn in_sent0(&self) -> ureg::RegRef<crate::meta::InSent0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "OUT Endpoint STALL control\n\nRead value: [`regs::OutStall0ReadVal`]; Write value: [`regs::OutStall0WriteVal`]"]
    #[inline(always)]
    pub fn out_stall0(&self) -> ureg::RegRef<crate::meta::OutStall0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "IN Endpoint STALL control\n\nRead value: [`regs::InStall0ReadVal`]; Write value: [`regs::InStall0WriteVal`]"]
    #[inline(always)]
    pub fn in_stall0(&self) -> ureg::RegRef<crate::meta::InStall0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configure IN Transaction\n\nRead value: [`regs::ConfiginReadVal`]; Write value: [`regs::ConfiginWriteVal`]"]
    #[inline(always)]
    pub fn configin(&self) -> ureg::Array<12, ureg::RegRef<crate::meta::Configin, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "OUT Endpoint isochronous setting\n\nRead value: [`regs::OutIso0ReadVal`]; Write value: [`regs::OutIso0WriteVal`]"]
    #[inline(always)]
    pub fn out_iso0(&self) -> ureg::RegRef<crate::meta::OutIso0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "IN Endpoint isochronous setting\n\nRead value: [`regs::InIso0ReadVal`]; Write value: [`regs::InIso0WriteVal`]"]
    #[inline(always)]
    pub fn in_iso0(&self) -> ureg::RegRef<crate::meta::InIso0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "OUT Endpoints Data Toggles\n\nRead value: [`regs::OutDataToggleReadVal`]; Write value: [`regs::OutDataToggleWriteVal`]"]
    #[inline(always)]
    pub fn out_data_toggle(&self) -> ureg::RegRef<crate::meta::OutDataToggle, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "IN Endpoints Data Toggles\n\nRead value: [`regs::InDataToggleReadVal`]; Write value: [`regs::InDataToggleWriteVal`]"]
    #[inline(always)]
    pub fn in_data_toggle(&self) -> ureg::RegRef<crate::meta::InDataToggle, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB PHY pins sense.\nThis register can be used to read out the state of the USB device inputs and outputs from software.\nThis is designed to be used for debugging purposes or during chip testing.\n\nRead value: [`regs::PhyPinsSenseReadVal`]; Write value: [`regs::PhyPinsSenseWriteVal`]"]
    #[inline(always)]
    pub fn phy_pins_sense(&self) -> ureg::RegRef<crate::meta::PhyPinsSense, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB PHY pins drive.\nThis register can be used to control the USB device inputs and outputs from software.\nThis is designed to be used for debugging purposes or during chip testing.\n\nRead value: [`regs::PhyPinsDriveReadVal`]; Write value: [`regs::PhyPinsDriveWriteVal`]"]
    #[inline(always)]
    pub fn phy_pins_drive(&self) -> ureg::RegRef<crate::meta::PhyPinsDrive, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x88 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB PHY Configuration\n\nRead value: [`regs::PhyConfigReadVal`]; Write value: [`regs::PhyConfigWriteVal`]"]
    #[inline(always)]
    pub fn phy_config(&self) -> ureg::RegRef<crate::meta::PhyConfig, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB wake module control for suspend / resume\n\nRead value: [`regs::WakeControlReadVal`]; Write value: [`regs::WakeControlWriteVal`]"]
    #[inline(always)]
    pub fn wake_control(&self) -> ureg::RegRef<crate::meta::WakeControl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "USB wake module events and debug\n\nRead value: [`regs::WakeEventsReadVal`]; Write value: [`regs::WakeEventsWriteVal`]"]
    #[inline(always)]
    pub fn wake_events(&self) -> ureg::RegRef<crate::meta::WakeEvents, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "FIFO control register\n\nRead value: [`regs::FifoCtrlReadVal`]; Write value: [`regs::FifoCtrlWriteVal`]"]
    #[inline(always)]
    pub fn fifo_ctrl(&self) -> ureg::RegRef<crate::meta::FifoCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x98 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Counter for OUT side USB events.\n\nRead value: [`regs::CountOutReadVal`]; Write value: [`regs::CountOutWriteVal`]"]
    #[inline(always)]
    pub fn count_out(&self) -> ureg::RegRef<crate::meta::CountOut, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x9c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Counter for IN side USB events.\n\nRead value: [`regs::CountInReadVal`]; Write value: [`regs::CountInWriteVal`]"]
    #[inline(always)]
    pub fn count_in(&self) -> ureg::RegRef<crate::meta::CountIn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Count of IN transactions for which no packet data was available.\n\nThis secondary register allows some partitioning of endpoints among the two\ncounters, for more targeted measurement, eg. endpoints may be grouped according to\nthe expected bandwidth usage, or Isochronous vs. non-Isochronous transfers.\n\nRead value: [`regs::CountNodataInReadVal`]; Write value: [`regs::CountNodataInWriteVal`]"]
    #[inline(always)]
    pub fn count_nodata_in(&self) -> ureg::RegRef<crate::meta::CountNodataIn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Count of error conditions detected on token packets from the host.\n\nRead value: [`regs::CountErrorsReadVal`]; Write value: [`regs::CountErrorsWriteVal`]"]
    #[inline(always)]
    pub fn count_errors(&self) -> ureg::RegRef<crate::meta::CountErrors, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "2 KiB packet buffer. Divided into thirty two 64-byte buffers.\n\nThe packet buffer is used for sending and receiving packets.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn buffer(&self) -> ureg::Array<512, ureg::RegRef<crate::meta::Buffer, &TMmio>> {
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
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.pkt_received is set."]
        #[inline(always)]
        pub fn pkt_received(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.pkt_sent is set."]
        #[inline(always)]
        pub fn pkt_sent(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.disconnected is set."]
        #[inline(always)]
        pub fn disconnected(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.host_lost is set."]
        #[inline(always)]
        pub fn host_lost(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_reset is set."]
        #[inline(always)]
        pub fn link_reset(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_suspend is set."]
        #[inline(always)]
        pub fn link_suspend(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_resume is set."]
        #[inline(always)]
        pub fn link_resume(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_out_empty is set."]
        #[inline(always)]
        pub fn av_out_empty(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_full is set."]
        #[inline(always)]
        pub fn rx_full(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_overflow is set."]
        #[inline(always)]
        pub fn av_overflow(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_in_err is set."]
        #[inline(always)]
        pub fn link_in_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_crc_err is set."]
        #[inline(always)]
        pub fn rx_crc_err(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_pid_err is set."]
        #[inline(always)]
        pub fn rx_pid_err(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_bitstuff_err is set."]
        #[inline(always)]
        pub fn rx_bitstuff_err(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.frame is set."]
        #[inline(always)]
        pub fn frame(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.powered is set."]
        #[inline(always)]
        pub fn powered(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_out_err is set."]
        #[inline(always)]
        pub fn link_out_err(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_setup_empty is set."]
        #[inline(always)]
        pub fn av_setup_empty(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.pkt_received is set."]
        #[inline(always)]
        pub fn pkt_received(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.pkt_sent is set."]
        #[inline(always)]
        pub fn pkt_sent(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.disconnected is set."]
        #[inline(always)]
        pub fn disconnected(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.host_lost is set."]
        #[inline(always)]
        pub fn host_lost(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_reset is set."]
        #[inline(always)]
        pub fn link_reset(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_suspend is set."]
        #[inline(always)]
        pub fn link_suspend(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_resume is set."]
        #[inline(always)]
        pub fn link_resume(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_out_empty is set."]
        #[inline(always)]
        pub fn av_out_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_full is set."]
        #[inline(always)]
        pub fn rx_full(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_overflow is set."]
        #[inline(always)]
        pub fn av_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_in_err is set."]
        #[inline(always)]
        pub fn link_in_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_crc_err is set."]
        #[inline(always)]
        pub fn rx_crc_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_pid_err is set."]
        #[inline(always)]
        pub fn rx_pid_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_bitstuff_err is set."]
        #[inline(always)]
        pub fn rx_bitstuff_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.frame is set."]
        #[inline(always)]
        pub fn frame(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.powered is set."]
        #[inline(always)]
        pub fn powered(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.link_out_err is set."]
        #[inline(always)]
        pub fn link_out_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.av_setup_empty is set."]
        #[inline(always)]
        pub fn av_setup_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
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
        #[doc = "Raised if a packet was received using an OUT or SETUP transaction.\nThis interrupt is directly tied to whether the RX FIFO is empty, so it should be cleared only after handling the FIFO entry."]
        #[inline(always)]
        pub fn pkt_received(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Raised if a packet was sent as part of an IN transaction.\nThis interrupt is directly tied to whether a sent packet has not been acknowledged in the !!in_sent register.\nIt should be cleared only after clearing all bits in the !!in_sent register."]
        #[inline(always)]
        pub fn pkt_sent(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Raised if VBUS is lost, thus the link is disconnected."]
        #[inline(always)]
        pub fn disconnected(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Raised if link is active but SOF was not received from host for 4.096 ms. The SOF should be every 1 ms."]
        #[inline(always)]
        pub fn host_lost(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Raised if the link is at SE0 longer than 3 us indicating a link reset (host asserts for min 10 ms, device can react after 2.5 us)."]
        #[inline(always)]
        pub fn link_reset(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Raised if the line has signaled J for longer than 3ms and is therefore in suspend state."]
        #[inline(always)]
        pub fn link_suspend(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Raised when the link becomes active again after being suspended."]
        #[inline(always)]
        pub fn link_resume(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Raised when the Available OUT Buffer FIFO is empty and the device interface is enabled.\nThis interrupt is directly tied to the FIFO status, so the Available OUT Buffer FIFO must be provided with a free buffer before the interrupt can be cleared."]
        #[inline(always)]
        pub fn av_out_empty(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Raised when the RX FIFO is full and the device interface is enabled.\nThis interrupt is directly tied to the FIFO status, so the RX FIFO must have an entry removed before the interrupt is cleared.\nIf the condition is not cleared, the interrupt can re-assert."]
        #[inline(always)]
        pub fn rx_full(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Raised if a write was done to either the Available OUT Buffer FIFO or the Available SETUP Buffer FIFO when the FIFO was full."]
        #[inline(always)]
        pub fn av_overflow(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Raised if a packet to an IN endpoint started to be received but was\nthen dropped due to an error. After transmitting the IN payload,\nthe USB device expects a valid ACK handshake packet. This error is\nraised if either the packet or CRC is invalid, leading to a NAK instead,\nor if a different token was received."]
        #[inline(always)]
        pub fn link_in_err(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Raised if a CRC error occurred on a received packet."]
        #[inline(always)]
        pub fn rx_crc_err(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Raised if an invalid Packet IDentifier (PID) was received."]
        #[inline(always)]
        pub fn rx_pid_err(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Raised if an invalid bitstuffing was received."]
        #[inline(always)]
        pub fn rx_bitstuff_err(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Raised when the USB frame number is updated with a valid SOF."]
        #[inline(always)]
        pub fn frame(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Raised if VBUS is applied."]
        #[inline(always)]
        pub fn powered(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Raised if a packet to an OUT endpoint started to be received but was then dropped due to an error.\nThis error is raised if the data toggle, token, packet and/or CRC are invalid, or if the appropriate Available OUT Buffer FIFO is empty and/or the Received Buffer FIFO is full when a packet should have been received."]
        #[inline(always)]
        pub fn link_out_err(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Raised when the Available SETUP Buffer FIFO is empty and the device interface is enabled.\nThis interrupt is directly tied to the FIFO status, so the Available SETUP Buffer FIFO must be provided with a free buffer before the interrupt can be cleared."]
        #[inline(always)]
        pub fn av_setup_empty(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
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
        #[doc = "Raised if VBUS is lost, thus the link is disconnected."]
        #[inline(always)]
        pub fn disconnected_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Raised if link is active but SOF was not received from host for 4.096 ms. The SOF should be every 1 ms."]
        #[inline(always)]
        pub fn host_lost_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "Raised if the link is at SE0 longer than 3 us indicating a link reset (host asserts for min 10 ms, device can react after 2.5 us)."]
        #[inline(always)]
        pub fn link_reset_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "Raised if the line has signaled J for longer than 3ms and is therefore in suspend state."]
        #[inline(always)]
        pub fn link_suspend_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "Raised when the link becomes active again after being suspended."]
        #[inline(always)]
        pub fn link_resume_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "Raised if a write was done to either the Available OUT Buffer FIFO or the Available SETUP Buffer FIFO when the FIFO was full."]
        #[inline(always)]
        pub fn av_overflow_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "Raised if a packet to an IN endpoint started to be received but was\nthen dropped due to an error. After transmitting the IN payload,\nthe USB device expects a valid ACK handshake packet. This error is\nraised if either the packet or CRC is invalid, leading to a NAK instead,\nor if a different token was received."]
        #[inline(always)]
        pub fn link_in_err_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
        #[doc = "Raised if a CRC error occurred on a received packet."]
        #[inline(always)]
        pub fn rx_crc_err_clear(self) -> Self {
            Self(self.0 | (1 << 11))
        }
        #[doc = "Raised if an invalid Packet IDentifier (PID) was received."]
        #[inline(always)]
        pub fn rx_pid_err_clear(self) -> Self {
            Self(self.0 | (1 << 12))
        }
        #[doc = "Raised if an invalid bitstuffing was received."]
        #[inline(always)]
        pub fn rx_bitstuff_err_clear(self) -> Self {
            Self(self.0 | (1 << 13))
        }
        #[doc = "Raised when the USB frame number is updated with a valid SOF."]
        #[inline(always)]
        pub fn frame_clear(self) -> Self {
            Self(self.0 | (1 << 14))
        }
        #[doc = "Raised if VBUS is applied."]
        #[inline(always)]
        pub fn powered_clear(self) -> Self {
            Self(self.0 | (1 << 15))
        }
        #[doc = "Raised if a packet to an OUT endpoint started to be received but was then dropped due to an error.\nThis error is raised if the data toggle, token, packet and/or CRC are invalid, or if the appropriate Available OUT Buffer FIFO is empty and/or the Received Buffer FIFO is full when a packet should have been received."]
        #[inline(always)]
        pub fn link_out_err_clear(self) -> Self {
            Self(self.0 | (1 << 16))
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
        #[doc = "Write 1 to force !!INTR_STATE.pkt_received to 1."]
        #[inline(always)]
        pub fn pkt_received(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.pkt_sent to 1."]
        #[inline(always)]
        pub fn pkt_sent(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.disconnected to 1."]
        #[inline(always)]
        pub fn disconnected(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.host_lost to 1."]
        #[inline(always)]
        pub fn host_lost(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to force !!INTR_STATE.link_reset to 1."]
        #[inline(always)]
        pub fn link_reset(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to force !!INTR_STATE.link_suspend to 1."]
        #[inline(always)]
        pub fn link_suspend(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Write 1 to force !!INTR_STATE.link_resume to 1."]
        #[inline(always)]
        pub fn link_resume(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Write 1 to force !!INTR_STATE.av_out_empty to 1."]
        #[inline(always)]
        pub fn av_out_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_full to 1."]
        #[inline(always)]
        pub fn rx_full(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Write 1 to force !!INTR_STATE.av_overflow to 1."]
        #[inline(always)]
        pub fn av_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Write 1 to force !!INTR_STATE.link_in_err to 1."]
        #[inline(always)]
        pub fn link_in_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_crc_err to 1."]
        #[inline(always)]
        pub fn rx_crc_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_pid_err to 1."]
        #[inline(always)]
        pub fn rx_pid_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_bitstuff_err to 1."]
        #[inline(always)]
        pub fn rx_bitstuff_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Write 1 to force !!INTR_STATE.frame to 1."]
        #[inline(always)]
        pub fn frame(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Write 1 to force !!INTR_STATE.powered to 1."]
        #[inline(always)]
        pub fn powered(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Write 1 to force !!INTR_STATE.link_out_err to 1."]
        #[inline(always)]
        pub fn link_out_err(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Write 1 to force !!INTR_STATE.av_setup_empty to 1."]
        #[inline(always)]
        pub fn av_setup_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
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
    pub struct AvoutbufferWriteVal(u32);
    impl AvoutbufferWriteVal {
        #[doc = "This field contains the buffer ID being passed to the USB receive engine.\n\nIf the Available OUT Buffer FIFO is full, any write operations are discarded."]
        #[inline(always)]
        pub fn buffer(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 0)) | ((val & 0x1f) << 0))
        }
    }
    impl From<u32> for AvoutbufferWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AvoutbufferWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AvoutbufferWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AvsetupbufferWriteVal(u32);
    impl AvsetupbufferWriteVal {
        #[doc = "This field contains the buffer ID being passed to the USB receive engine.\n\nIf the Available SETUP Buffer FIFO is full, any write operations are discarded."]
        #[inline(always)]
        pub fn buffer(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 0)) | ((val & 0x1f) << 0))
        }
    }
    impl From<u32> for AvsetupbufferWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AvsetupbufferWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AvsetupbufferWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ConfiginReadVal(u32);
    impl ConfiginReadVal {
        #[doc = "The buffer ID containing the data to send when an IN transaction is received on the endpoint."]
        #[inline(always)]
        pub fn buffer(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = "The number of bytes to send from the buffer.\n\nIf this is 0 then a CRC only packet is sent.\n\nIf this is greater than 64 then 64 bytes are sent."]
        #[inline(always)]
        pub fn size(&self) -> u32 {
            (self.0 >> 8) & 0x7f
        }
        #[doc = "This bit indicates that the buffer is in the process of being collected by the\nhost. It becomes set upon the first attempt by the host to collect a buffer from\nthis endpoint when the rdy bit was set.\n\nIt is cleared when the packet has been collected successfully or the pending\ntransaction has been canceled by the hardware through detection of a\nlink reset or receipt of a SETUP packet."]
        #[inline(always)]
        pub fn sending(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "This bit indicates a pending transaction was canceled by the hardware.\n\nThe bit is set when the rdy bit is cleared by hardware because of a\nSETUP packet being received or a link reset being detected.\n\nThe bit remains set until cleared by being written with a 1."]
        #[inline(always)]
        pub fn pend(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "This bit should be set to indicate the buffer is ready for sending.\nIt will be cleared when the ACK is received indicating the host has accepted the data.\n\nThis bit will also be cleared if an enabled SETUP transaction is received on the endpoint.\nThis allows use of the IN channel for transfer of SETUP information.\nThe original buffer must be resubmitted after the SETUP sequence is complete.\nA link reset also clears the bit.\nIn either of the cases where the hardware cancels the transaction it will also set the pend bit."]
        #[inline(always)]
        pub fn rdy(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ConfiginWriteVal {
            ConfiginWriteVal(self.0)
        }
    }
    impl From<u32> for ConfiginReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ConfiginReadVal> for u32 {
        #[inline(always)]
        fn from(val: ConfiginReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ConfiginWriteVal(u32);
    impl ConfiginWriteVal {
        #[doc = "The buffer ID containing the data to send when an IN transaction is received on the endpoint."]
        #[inline(always)]
        pub fn buffer(self, val: u32) -> Self {
            Self((self.0 & !(0x1f << 0)) | ((val & 0x1f) << 0))
        }
        #[doc = "The number of bytes to send from the buffer.\n\nIf this is 0 then a CRC only packet is sent.\n\nIf this is greater than 64 then 64 bytes are sent."]
        #[inline(always)]
        pub fn size(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 8)) | ((val & 0x7f) << 8))
        }
        #[doc = "This bit indicates that the buffer is in the process of being collected by the\nhost. It becomes set upon the first attempt by the host to collect a buffer from\nthis endpoint when the rdy bit was set.\n\nIt is cleared when the packet has been collected successfully or the pending\ntransaction has been canceled by the hardware through detection of a\nlink reset or receipt of a SETUP packet."]
        #[inline(always)]
        pub fn sending_clear(self) -> Self {
            Self(self.0 | (1 << 29))
        }
        #[doc = "This bit indicates a pending transaction was canceled by the hardware.\n\nThe bit is set when the rdy bit is cleared by hardware because of a\nSETUP packet being received or a link reset being detected.\n\nThe bit remains set until cleared by being written with a 1."]
        #[inline(always)]
        pub fn pend_clear(self) -> Self {
            Self(self.0 | (1 << 30))
        }
        #[doc = "This bit should be set to indicate the buffer is ready for sending.\nIt will be cleared when the ACK is received indicating the host has accepted the data.\n\nThis bit will also be cleared if an enabled SETUP transaction is received on the endpoint.\nThis allows use of the IN channel for transfer of SETUP information.\nThe original buffer must be resubmitted after the SETUP sequence is complete.\nA link reset also clears the bit.\nIn either of the cases where the hardware cancels the transaction it will also set the pend bit."]
        #[inline(always)]
        pub fn rdy(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for ConfiginWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ConfiginWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ConfiginWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountErrorsReadVal(u32);
    impl CountErrorsReadVal {
        #[doc = "Number of events counted."]
        #[inline(always)]
        pub fn count(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Number of Invalid PIDs detected on packets from the host. Invalid PIDs may\nindicate very unreliable communication and/or a substantial frequency mismatch\nbetween the host and the device."]
        #[inline(always)]
        pub fn pid_invalid(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Number of SETUP/OUT packets that were ignored, dropped or NAKed because a\nBit Stuffing error was detected."]
        #[inline(always)]
        pub fn bitstuff(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Count SETUP/OUT DATA packets that were ignored, dropped or NAKed because a\nCRC16 error was detected."]
        #[inline(always)]
        pub fn crc16(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Count CRC5 errors detected on token packets sent by the host. CRC5 errors on\ntoken packets received from the host indicate very unreliable communication and\npossibly a substantial frequency mismatch between the host and the device."]
        #[inline(always)]
        pub fn crc5(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CountErrorsWriteVal {
            CountErrorsWriteVal(self.0)
        }
    }
    impl From<u32> for CountErrorsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountErrorsReadVal> for u32 {
        #[inline(always)]
        fn from(val: CountErrorsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountErrorsWriteVal(u32);
    impl CountErrorsWriteVal {
        #[doc = "Number of Invalid PIDs detected on packets from the host. Invalid PIDs may\nindicate very unreliable communication and/or a substantial frequency mismatch\nbetween the host and the device."]
        #[inline(always)]
        pub fn pid_invalid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Number of SETUP/OUT packets that were ignored, dropped or NAKed because a\nBit Stuffing error was detected."]
        #[inline(always)]
        pub fn bitstuff(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Count SETUP/OUT DATA packets that were ignored, dropped or NAKed because a\nCRC16 error was detected."]
        #[inline(always)]
        pub fn crc16(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Count CRC5 errors detected on token packets sent by the host. CRC5 errors on\ntoken packets received from the host indicate very unreliable communication and\npossibly a substantial frequency mismatch between the host and the device."]
        #[inline(always)]
        pub fn crc5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Write 1 to reset the counter."]
        #[inline(always)]
        pub fn rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CountErrorsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountErrorsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CountErrorsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountInReadVal(u32);
    impl CountInReadVal {
        #[doc = "Number of events counted."]
        #[inline(always)]
        pub fn count(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Count the IN transactions that were attempted when there was no packet available\nin the corresponding 'configin' register(s). This is not necessarily an error\ncondition, and the counter primarily offers some visibility into when the IN\ntraffic is underusing the available bus bandwidth.\nIt is of particular utility to Isochronous IN endpoints."]
        #[inline(always)]
        pub fn nodata(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Count the IN transactions rejected by the host responding with a NAK handshake."]
        #[inline(always)]
        pub fn nak(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Count the IN transactions for which the USB host did not respond with a handshake,\nand the transactions timed out. This indicates that the host did not receive it\nand decode it as a valid packet, suggesting that communication is unreliable.\n\nIsochronous IN transactions are excluded from this count because there is no\nhandshake response to Isochronous packet transfers."]
        #[inline(always)]
        pub fn timeout(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Set of endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CountInWriteVal {
            CountInWriteVal(self.0)
        }
    }
    impl From<u32> for CountInReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountInReadVal> for u32 {
        #[inline(always)]
        fn from(val: CountInReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountInWriteVal(u32);
    impl CountInWriteVal {
        #[doc = "Count the IN transactions that were attempted when there was no packet available\nin the corresponding 'configin' register(s). This is not necessarily an error\ncondition, and the counter primarily offers some visibility into when the IN\ntraffic is underusing the available bus bandwidth.\nIt is of particular utility to Isochronous IN endpoints."]
        #[inline(always)]
        pub fn nodata(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Count the IN transactions rejected by the host responding with a NAK handshake."]
        #[inline(always)]
        pub fn nak(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Count the IN transactions for which the USB host did not respond with a handshake,\nand the transactions timed out. This indicates that the host did not receive it\nand decode it as a valid packet, suggesting that communication is unreliable.\n\nIsochronous IN transactions are excluded from this count because there is no\nhandshake response to Isochronous packet transfers."]
        #[inline(always)]
        pub fn timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Set of endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
        #[doc = "Write 1 to reset the counter."]
        #[inline(always)]
        pub fn rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CountInWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountInWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CountInWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountNodataInReadVal(u32);
    impl CountNodataInReadVal {
        #[doc = "Number of IN transactions that were attempted when there was no packet available\nin the corresponding 'configin' register(s)."]
        #[inline(always)]
        pub fn count(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Set of endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CountNodataInWriteVal {
            CountNodataInWriteVal(self.0)
        }
    }
    impl From<u32> for CountNodataInReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountNodataInReadVal> for u32 {
        #[inline(always)]
        fn from(val: CountNodataInReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountNodataInWriteVal(u32);
    impl CountNodataInWriteVal {
        #[doc = "Set of endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
        #[doc = "Write 1 to reset the counter."]
        #[inline(always)]
        pub fn rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CountNodataInWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountNodataInWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CountNodataInWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountOutReadVal(u32);
    impl CountOutReadVal {
        #[doc = "Number of events counted."]
        #[inline(always)]
        pub fn count(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Count the OUT transactions for which the USB device acknowledged the OUT packet\nand dropped it internally, which is the correct response to a packet transmitted\nwith an incorrect Data Toggle.\n\nThe expectation is that this packet is a retry of the previous packet transmission\nand that the handshake response was not received intact by the USB host, indicating\nunreliable communications.\n\nOther causes of Data Toggle synchronization failure may result in data loss."]
        #[inline(always)]
        pub fn datatog_out(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Count the SETUP/OUT packets ignored, dropped or NAKed because the RX FIFO was full.\nSETUP packets have been ignored, Isochronous OUT packets have been dropped, and\nnon-Isochronous OUT packets have been NAKed."]
        #[inline(always)]
        pub fn drop_rx(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Count the OUT packets that could not be accepted because there was no buffer in the\nAv OUT FIFO. Non-Isochronous OUT packets have been NAKed.\nIsochronous OUT packets were ignored."]
        #[inline(always)]
        pub fn drop_avout(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Count the SETUP packets that were ignored because there was no buffer in the\nAv SETUP FIFO."]
        #[inline(always)]
        pub fn ign_avsetup(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Set of OUT endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CountOutWriteVal {
            CountOutWriteVal(self.0)
        }
    }
    impl From<u32> for CountOutReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountOutReadVal> for u32 {
        #[inline(always)]
        fn from(val: CountOutReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CountOutWriteVal(u32);
    impl CountOutWriteVal {
        #[doc = "Count the OUT transactions for which the USB device acknowledged the OUT packet\nand dropped it internally, which is the correct response to a packet transmitted\nwith an incorrect Data Toggle.\n\nThe expectation is that this packet is a retry of the previous packet transmission\nand that the handshake response was not received intact by the USB host, indicating\nunreliable communications.\n\nOther causes of Data Toggle synchronization failure may result in data loss."]
        #[inline(always)]
        pub fn datatog_out(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Count the SETUP/OUT packets ignored, dropped or NAKed because the RX FIFO was full.\nSETUP packets have been ignored, Isochronous OUT packets have been dropped, and\nnon-Isochronous OUT packets have been NAKed."]
        #[inline(always)]
        pub fn drop_rx(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Count the OUT packets that could not be accepted because there was no buffer in the\nAv OUT FIFO. Non-Isochronous OUT packets have been NAKed.\nIsochronous OUT packets were ignored."]
        #[inline(always)]
        pub fn drop_avout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Count the SETUP packets that were ignored because there was no buffer in the\nAv SETUP FIFO."]
        #[inline(always)]
        pub fn ign_avsetup(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Set of OUT endpoints for which this counter is enabled."]
        #[inline(always)]
        pub fn endpoints(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
        #[doc = "Write 1 to reset the counter."]
        #[inline(always)]
        pub fn rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CountOutWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CountOutWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CountOutWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EpInEnable0ReadVal(u32);
    impl EpInEnable0ReadVal {
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EpInEnable0WriteVal {
            EpInEnable0WriteVal(self.0)
        }
    }
    impl From<u32> for EpInEnable0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EpInEnable0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: EpInEnable0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EpInEnable0WriteVal(u32);
    impl EpInEnable0WriteVal {
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "This bit must be set to enable upstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear then any IN packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for EpInEnable0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EpInEnable0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: EpInEnable0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EpOutEnable0ReadVal(u32);
    impl EpOutEnable0ReadVal {
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> EpOutEnable0WriteVal {
            EpOutEnable0WriteVal(self.0)
        }
    }
    impl From<u32> for EpOutEnable0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EpOutEnable0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: EpOutEnable0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct EpOutEnable0WriteVal(u32);
    impl EpOutEnable0WriteVal {
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "This bit must be set to enable downstream transactions to be received on the endpoint and elicit responses.\nIf the bit is clear, any SETUP or OUT packets sent to the endpoint will be ignored."]
        #[inline(always)]
        pub fn enable11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for EpOutEnable0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<EpOutEnable0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: EpOutEnable0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FifoCtrlWriteVal(u32);
    impl FifoCtrlWriteVal {
        #[doc = "Software reset of the Available OUT Buffer FIFO. This must be used only when the USB device\nis not connected to the USB."]
        #[inline(always)]
        pub fn avout_rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Software reset of the Available SETUP Buffer FIFO. This must be used only when the USB device\nis not connected to the USB."]
        #[inline(always)]
        pub fn avsetup_rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Software reset the of Rx Buffer FIFO. This must be used only when the USB device is not\nconnected to the USB."]
        #[inline(always)]
        pub fn rx_rst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
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
    pub struct InDataToggleReadVal(u32);
    impl InDataToggleReadVal {
        #[doc = "Reading returns the current state of the IN endpoint Data Toggle flags.\nWriting sets the Data Toggle flag for each endpoint if the corresponding mask bit\nin the upper half of this register is set."]
        #[inline(always)]
        pub fn status(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Reads as zero.\nWhen writing, a set bit will cause the Data Toggle flag of the corresponding\nIN endpoint to be updated. A clear bit will leave the flag for the corresponding\nendpoint unchanged."]
        #[inline(always)]
        pub fn mask(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InDataToggleWriteVal {
            InDataToggleWriteVal(self.0)
        }
    }
    impl From<u32> for InDataToggleReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InDataToggleReadVal> for u32 {
        #[inline(always)]
        fn from(val: InDataToggleReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InDataToggleWriteVal(u32);
    impl InDataToggleWriteVal {
        #[doc = "Reading returns the current state of the IN endpoint Data Toggle flags.\nWriting sets the Data Toggle flag for each endpoint if the corresponding mask bit\nin the upper half of this register is set."]
        #[inline(always)]
        pub fn status(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 0)) | ((val & 0xfff) << 0))
        }
        #[doc = "Reads as zero.\nWhen writing, a set bit will cause the Data Toggle flag of the corresponding\nIN endpoint to be updated. A clear bit will leave the flag for the corresponding\nendpoint unchanged."]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
    }
    impl From<u32> for InDataToggleWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InDataToggleWriteVal> for u32 {
        #[inline(always)]
        fn from(val: InDataToggleWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InIso0ReadVal(u32);
    impl InIso0ReadVal {
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InIso0WriteVal {
            InIso0WriteVal(self.0)
        }
    }
    impl From<u32> for InIso0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InIso0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: InIso0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InIso0WriteVal(u32);
    impl InIso0WriteVal {
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be expected for an IN transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for InIso0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InIso0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: InIso0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InSent0ReadVal(u32);
    impl InSent0ReadVal {
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InSent0WriteVal {
            InSent0WriteVal(self.0)
        }
    }
    impl From<u32> for InSent0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InSent0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: InSent0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InSent0WriteVal(u32);
    impl InSent0WriteVal {
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent0_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent1_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent2_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent3_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent4_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent5_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent6_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent7_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent8_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent9_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent10_clear(self) -> Self {
            Self(self.0 | (1 << 10))
        }
        #[doc = "This bit will be set when the ACK is received from\nthe host to indicate successful packet delivery\nas part of an IN transaction."]
        #[inline(always)]
        pub fn sent11_clear(self) -> Self {
            Self(self.0 | (1 << 11))
        }
    }
    impl From<u32> for InSent0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InSent0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: InSent0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InStall0ReadVal(u32);
    impl InStall0ReadVal {
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InStall0WriteVal {
            InStall0WriteVal(self.0)
        }
    }
    impl From<u32> for InStall0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InStall0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: InStall0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InStall0WriteVal(u32);
    impl InStall0WriteVal {
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If this bit is set then an IN transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for InStall0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InStall0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: InStall0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutDataToggleReadVal(u32);
    impl OutDataToggleReadVal {
        #[doc = "Reading returns the current state of the OUT endpoint Data Toggle flags.\nWriting sets the Data Toggle flag for each endpoint if the corresponding mask bit\nin the upper half of this register is set."]
        #[inline(always)]
        pub fn status(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Reads as zero.\nWhen writing, a set bit will cause the Data Toggle flag of the corresponding\nOUT endpoint to be updated. A clear bit will leave the flag for the corresponding\nendpoint unchanged."]
        #[inline(always)]
        pub fn mask(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OutDataToggleWriteVal {
            OutDataToggleWriteVal(self.0)
        }
    }
    impl From<u32> for OutDataToggleReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutDataToggleReadVal> for u32 {
        #[inline(always)]
        fn from(val: OutDataToggleReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutDataToggleWriteVal(u32);
    impl OutDataToggleWriteVal {
        #[doc = "Reading returns the current state of the OUT endpoint Data Toggle flags.\nWriting sets the Data Toggle flag for each endpoint if the corresponding mask bit\nin the upper half of this register is set."]
        #[inline(always)]
        pub fn status(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 0)) | ((val & 0xfff) << 0))
        }
        #[doc = "Reads as zero.\nWhen writing, a set bit will cause the Data Toggle flag of the corresponding\nOUT endpoint to be updated. A clear bit will leave the flag for the corresponding\nendpoint unchanged."]
        #[inline(always)]
        pub fn mask(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
    }
    impl From<u32> for OutDataToggleWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutDataToggleWriteVal> for u32 {
        #[inline(always)]
        fn from(val: OutDataToggleWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutIso0ReadVal(u32);
    impl OutIso0ReadVal {
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OutIso0WriteVal {
            OutIso0WriteVal(self.0)
        }
    }
    impl From<u32> for OutIso0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutIso0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: OutIso0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutIso0WriteVal(u32);
    impl OutIso0WriteVal {
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If this bit is set then the endpoint will be treated as an isochronous endpoint.\nNo handshake packet will be sent for an OUT transaction.\nNote that if a rxenable_setup is set for this endpoint's number, this bit will not take effect.\nControl endpoint configuration trumps isochronous endpoint configuration."]
        #[inline(always)]
        pub fn iso11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for OutIso0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutIso0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: OutIso0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutStall0ReadVal(u32);
    impl OutStall0ReadVal {
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> OutStall0WriteVal {
            OutStall0WriteVal(self.0)
        }
    }
    impl From<u32> for OutStall0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutStall0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: OutStall0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct OutStall0WriteVal(u32);
    impl OutStall0WriteVal {
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If this bit is set then an OUT transaction to this endpoint will elicit a STALL handshake, when a non-isochronous endpoint is enabled.\nIf the configuration has both STALL and NAK enabled, the STALL handshake will take priority.\n\nNote that SETUP transactions are always either accepted or ignored.\nFor endpoints that accept SETUP transactions, a SETUP packet will clear the STALL flag on endpoints for both the IN and OUT directions."]
        #[inline(always)]
        pub fn endpoint11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for OutStall0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<OutStall0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: OutStall0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyConfigReadVal(u32);
    impl PhyConfigReadVal {
        #[doc = "Detect received K and J symbols from the usb_rx_d signal, which must be driven from an external differential receiver.\nIf 1, make use of the usb_rx_d input.\nIf 0, the usb_rx_d input is ignored and the usb_rx_dp and usb_rx_dn pair are used to detect K and J (useful for some environments, but will be unlikely to pass full USB compliance).\nRegardless of the state of this field usb_rx_dp and usb_rx_dn are always used to detect SE0.\nThis bit also feeds the rx_enable pin, activating the receiver when the device is not suspended."]
        #[inline(always)]
        pub fn use_diff_rcvr(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, select the d and se0 TX interface.\nIf 0, select the dp and dn TX interface.\nThis directly controls the output pin of the same name.\nIt is intended to be used to enable the use of a variety of external transceivers, to select an encoding that matches the transceiver."]
        #[inline(always)]
        pub fn tx_use_d_se0(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Recognize a single SE0 bit as an end of packet, otherwise two successive bits are required."]
        #[inline(always)]
        pub fn eop_single_bit(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Flip the D+/D- pins.\nParticularly useful if D+/D- are mapped to SBU1/SBU2 pins of USB-C.\n"]
        #[inline(always)]
        pub fn pinflip(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "0: Enable reference signal generation for clock synchronization, 1: disable it by forcing the associated signals to zero.\n"]
        #[inline(always)]
        pub fn usb_ref_disable(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Disable (0) or enable (1) oscillator test mode.\nIf enabled, the device constantly transmits a J/K pattern, which is useful for testing the USB clock.\nNote that while in oscillator test mode, the device no longer receives SOFs and consequently does not generate the reference signal for clock synchronization.\nThe clock might drift off.\n"]
        #[inline(always)]
        pub fn tx_osc_test_mode(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PhyConfigWriteVal {
            PhyConfigWriteVal(self.0)
        }
    }
    impl From<u32> for PhyConfigReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyConfigReadVal> for u32 {
        #[inline(always)]
        fn from(val: PhyConfigReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyConfigWriteVal(u32);
    impl PhyConfigWriteVal {
        #[doc = "Detect received K and J symbols from the usb_rx_d signal, which must be driven from an external differential receiver.\nIf 1, make use of the usb_rx_d input.\nIf 0, the usb_rx_d input is ignored and the usb_rx_dp and usb_rx_dn pair are used to detect K and J (useful for some environments, but will be unlikely to pass full USB compliance).\nRegardless of the state of this field usb_rx_dp and usb_rx_dn are always used to detect SE0.\nThis bit also feeds the rx_enable pin, activating the receiver when the device is not suspended."]
        #[inline(always)]
        pub fn use_diff_rcvr(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, select the d and se0 TX interface.\nIf 0, select the dp and dn TX interface.\nThis directly controls the output pin of the same name.\nIt is intended to be used to enable the use of a variety of external transceivers, to select an encoding that matches the transceiver."]
        #[inline(always)]
        pub fn tx_use_d_se0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Recognize a single SE0 bit as an end of packet, otherwise two successive bits are required."]
        #[inline(always)]
        pub fn eop_single_bit(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Flip the D+/D- pins.\nParticularly useful if D+/D- are mapped to SBU1/SBU2 pins of USB-C.\n"]
        #[inline(always)]
        pub fn pinflip(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "0: Enable reference signal generation for clock synchronization, 1: disable it by forcing the associated signals to zero.\n"]
        #[inline(always)]
        pub fn usb_ref_disable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Disable (0) or enable (1) oscillator test mode.\nIf enabled, the device constantly transmits a J/K pattern, which is useful for testing the USB clock.\nNote that while in oscillator test mode, the device no longer receives SOFs and consequently does not generate the reference signal for clock synchronization.\nThe clock might drift off.\n"]
        #[inline(always)]
        pub fn tx_osc_test_mode(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for PhyConfigWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyConfigWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PhyConfigWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyPinsDriveReadVal(u32);
    impl PhyPinsDriveReadVal {
        #[doc = "USB transmit D+ output, used with dn_o."]
        #[inline(always)]
        pub fn dp_o(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "USB transmit D- output, used with dp_o."]
        #[inline(always)]
        pub fn dn_o(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "USB transmit data output, encoding K and J when se0_o is 0."]
        #[inline(always)]
        pub fn d_o(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "USB single-ended zero output."]
        #[inline(always)]
        pub fn se0_o(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "USB OE output."]
        #[inline(always)]
        pub fn oe_o(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable differential receiver."]
        #[inline(always)]
        pub fn rx_enable_o(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "USB D+ pullup enable output."]
        #[inline(always)]
        pub fn dp_pullup_en_o(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "USB D- pullup enable output."]
        #[inline(always)]
        pub fn dn_pullup_en_o(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "0: Outputs are controlled by the hardware block.\n1: Outputs are controlled with this register."]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> PhyPinsDriveWriteVal {
            PhyPinsDriveWriteVal(self.0)
        }
    }
    impl From<u32> for PhyPinsDriveReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyPinsDriveReadVal> for u32 {
        #[inline(always)]
        fn from(val: PhyPinsDriveReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyPinsDriveWriteVal(u32);
    impl PhyPinsDriveWriteVal {
        #[doc = "USB transmit D+ output, used with dn_o."]
        #[inline(always)]
        pub fn dp_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "USB transmit D- output, used with dp_o."]
        #[inline(always)]
        pub fn dn_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "USB transmit data output, encoding K and J when se0_o is 0."]
        #[inline(always)]
        pub fn d_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "USB single-ended zero output."]
        #[inline(always)]
        pub fn se0_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "USB OE output."]
        #[inline(always)]
        pub fn oe_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable differential receiver."]
        #[inline(always)]
        pub fn rx_enable_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "USB D+ pullup enable output."]
        #[inline(always)]
        pub fn dp_pullup_en_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "USB D- pullup enable output."]
        #[inline(always)]
        pub fn dn_pullup_en_o(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "0: Outputs are controlled by the hardware block.\n1: Outputs are controlled with this register."]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
    }
    impl From<u32> for PhyPinsDriveWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyPinsDriveWriteVal> for u32 {
        #[inline(always)]
        fn from(val: PhyPinsDriveWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct PhyPinsSenseReadVal(u32);
    impl PhyPinsSenseReadVal {
        #[doc = "USB D+ input."]
        #[inline(always)]
        pub fn rx_dp_i(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "USB D- input."]
        #[inline(always)]
        pub fn rx_dn_i(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "USB data input from an external differential receiver, if available."]
        #[inline(always)]
        pub fn rx_d_i(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "USB transmit D+ output (readback)."]
        #[inline(always)]
        pub fn tx_dp_o(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "USB transmit D- output (readback)."]
        #[inline(always)]
        pub fn tx_dn_o(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "USB transmit data value (readback)."]
        #[inline(always)]
        pub fn tx_d_o(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "USB single-ended zero output (readback)."]
        #[inline(always)]
        pub fn tx_se0_o(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "USB OE output (readback)."]
        #[inline(always)]
        pub fn tx_oe_o(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "USB power sense signal."]
        #[inline(always)]
        pub fn pwr_sense(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
    }
    impl From<u32> for PhyPinsSenseReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<PhyPinsSenseReadVal> for u32 {
        #[inline(always)]
        fn from(val: PhyPinsSenseReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RxenableOut0ReadVal(u32);
    impl RxenableOut0ReadVal {
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RxenableOut0WriteVal {
            RxenableOut0WriteVal(self.0)
        }
    }
    impl From<u32> for RxenableOut0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RxenableOut0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: RxenableOut0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RxenableOut0WriteVal(u32);
    impl RxenableOut0WriteVal {
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "This bit must be set to enable OUT transactions to be received on the endpoint.\nIf the bit is clear then an OUT request will be responded to with a NAK, if the endpoint is enabled.\nIf set_nak_out for this endpoint is set, hardware will clear this bit whenever an OUT transaction is received on this endpoint.\nSoftware must set this bit again to receive the next OUT transaction.\nUntil that happens, hardware will continue to NAK any OUT transaction to this endpoint."]
        #[inline(always)]
        pub fn out11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for RxenableOut0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RxenableOut0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: RxenableOut0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RxenableSetup0ReadVal(u32);
    impl RxenableSetup0ReadVal {
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> RxenableSetup0WriteVal {
            RxenableSetup0WriteVal(self.0)
        }
    }
    impl From<u32> for RxenableSetup0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RxenableSetup0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: RxenableSetup0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RxenableSetup0WriteVal(u32);
    impl RxenableSetup0WriteVal {
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "This bit must be set to enable SETUP transactions to be\nreceived on the endpoint. If the bit is clear then a\nSETUP packet will be ignored. The bit should be set for\ncontrol endpoints (and only control endpoints)."]
        #[inline(always)]
        pub fn setup11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for RxenableSetup0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RxenableSetup0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: RxenableSetup0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct RxfifoReadVal(u32);
    impl RxfifoReadVal {
        #[doc = "This field contains the buffer ID that data was received into.\nOn read the buffer ID is popped from the Received Buffer FIFO and returned to software."]
        #[inline(always)]
        pub fn buffer(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = "This field contains the data length in bytes of the packet written to the buffer."]
        #[inline(always)]
        pub fn size(&self) -> u32 {
            (self.0 >> 8) & 0x7f
        }
        #[doc = "This bit indicates if the received transaction is of type SETUP (1) or OUT (0)."]
        #[inline(always)]
        pub fn setup(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "This field contains the endpoint ID to which the packet was directed."]
        #[inline(always)]
        pub fn ep(&self) -> u32 {
            (self.0 >> 20) & 0xf
        }
    }
    impl From<u32> for RxfifoReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<RxfifoReadVal> for u32 {
        #[inline(always)]
        fn from(val: RxfifoReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SetNakOut0ReadVal(u32);
    impl SetNakOut0ReadVal {
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> SetNakOut0WriteVal {
            SetNakOut0WriteVal(self.0)
        }
    }
    impl From<u32> for SetNakOut0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SetNakOut0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: SetNakOut0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct SetNakOut0WriteVal(u32);
    impl SetNakOut0WriteVal {
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "When this bit is set, hardware will clear this endpoint's rxenable_out bit whenever an OUT transaction is received on this endpoint.\nThis bit should not be changed while the endpoint is active."]
        #[inline(always)]
        pub fn enable11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
    }
    impl From<u32> for SetNakOut0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<SetNakOut0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: SetNakOut0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbctrlReadVal(u32);
    impl UsbctrlReadVal {
        #[doc = "Set to connect the USB interface (i.e. assert the pullup)."]
        #[inline(always)]
        pub fn enable(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Device address set by host (this should be copied from\nthe Set Device ID SETUP packet).\n\nThis will be zeroed by the hardware when the link resets."]
        #[inline(always)]
        pub fn device_address(&self) -> u32 {
            (self.0 >> 16) & 0x7f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> UsbctrlWriteVal {
            UsbctrlWriteVal(self.0)
        }
    }
    impl From<u32> for UsbctrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbctrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: UsbctrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbctrlWriteVal(u32);
    impl UsbctrlWriteVal {
        #[doc = "Set to connect the USB interface (i.e. assert the pullup)."]
        #[inline(always)]
        pub fn enable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write a 1 to this bit to instruct usbdev to jump to the LinkResuming state.\nThe write will only have an effect when the device is in the LinkPowered state.\nIts intention is to handle a resume-from-suspend event after the IP has been powered down."]
        #[inline(always)]
        pub fn resume_link_active(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Device address set by host (this should be copied from\nthe Set Device ID SETUP packet).\n\nThis will be zeroed by the hardware when the link resets."]
        #[inline(always)]
        pub fn device_address(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 16)) | ((val & 0x7f) << 16))
        }
    }
    impl From<u32> for UsbctrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbctrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: UsbctrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UsbstatReadVal(u32);
    impl UsbstatReadVal {
        #[doc = "Frame index received from host. On an active link, this will increment every milisecond."]
        #[inline(always)]
        pub fn frame(&self) -> u32 {
            (self.0 >> 0) & 0x7ff
        }
        #[doc = "Start of frame not received from host for 4.096 ms and the line is active."]
        #[inline(always)]
        pub fn host_lost(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "State of USB link, decoded from line."]
        #[inline(always)]
        pub fn link_state(&self) -> super::enums::LinkState {
            super::enums::LinkState::try_from((self.0 >> 12) & 7).unwrap()
        }
        #[doc = "Reflects the state of the sense pin.\n1 indicates that the host is providing VBUS.\nNote that this bit always shows the state of the actual pin and does not take account of the override control."]
        #[inline(always)]
        pub fn sense(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Number of buffers in the Available OUT Buffer FIFO.\n\nThese buffers are available for receiving OUT DATA packets."]
        #[inline(always)]
        pub fn av_out_depth(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "Number of buffers in the Available SETUP Buffer FIFO.\n\nThese buffers are available for receiving SETUP DATA packets."]
        #[inline(always)]
        pub fn av_setup_depth(&self) -> u32 {
            (self.0 >> 20) & 7
        }
        #[doc = "Available OUT Buffer FIFO is full."]
        #[inline(always)]
        pub fn av_out_full(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Number of buffers in the Received Buffer FIFO.\n\nThese buffers have packets that have been received and\nshould be popped from the FIFO and processed."]
        #[inline(always)]
        pub fn rx_depth(&self) -> u32 {
            (self.0 >> 24) & 0xf
        }
        #[doc = "Available SETUP Buffer FIFO is full."]
        #[inline(always)]
        pub fn av_setup_full(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Received Buffer FIFO is empty."]
        #[inline(always)]
        pub fn rx_empty(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for UsbstatReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UsbstatReadVal> for u32 {
        #[inline(always)]
        fn from(val: UsbstatReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeControlWriteVal(u32);
    impl WakeControlWriteVal {
        #[doc = "Suspend request to the wake detection module.\n\nTrigger the wake detection module to begin monitoring for wake-from-suspend events.\nWhen written with a 1, the wake detection module will activate.\nActivation may not happen immediately, and its status can be verified by checking wake_events.module_active."]
        #[inline(always)]
        pub fn suspend_req(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Wake acknowledgement.\n\nSignal to the wake detection module that it may release control of the pull-ups back to the main block and return to an inactive state.\nThe release back to normal state may not happen immediately.\nThe status can be confirmed via wake_events.module_active.\n\nNote that this bit can also be used without powering down, such as when usbdev detects resume signaling before transitions to low power states have begun."]
        #[inline(always)]
        pub fn wake_ack(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
    }
    impl From<u32> for WakeControlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeControlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: WakeControlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct WakeEventsReadVal(u32);
    impl WakeEventsReadVal {
        #[doc = "USB aon wake module is active, monitoring events and controlling the pull-ups."]
        #[inline(always)]
        pub fn module_active(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "USB aon wake module detected VBUS was interrupted while monitoring events."]
        #[inline(always)]
        pub fn disconnected(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "USB aon wake module detected a bus reset while monitoring events."]
        #[inline(always)]
        pub fn bus_reset(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "USB aon wake module detected a non-idle bus while monitoring events."]
        #[inline(always)]
        pub fn bus_not_idle(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
    }
    impl From<u32> for WakeEventsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<WakeEventsReadVal> for u32 {
        #[inline(always)]
        fn from(val: WakeEventsReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum LinkState {
        Disconnected = 0,
        Powered = 1,
        PoweredSuspended = 2,
        Active = 3,
        Suspended = 4,
        ActiveNosof = 5,
        Resuming = 6,
        Reserved7 = 7,
    }
    impl LinkState {
        #[inline(always)]
        pub fn disconnected(&self) -> bool {
            *self == Self::Disconnected
        }
        #[inline(always)]
        pub fn powered(&self) -> bool {
            *self == Self::Powered
        }
        #[inline(always)]
        pub fn powered_suspended(&self) -> bool {
            *self == Self::PoweredSuspended
        }
        #[inline(always)]
        pub fn active(&self) -> bool {
            *self == Self::Active
        }
        #[inline(always)]
        pub fn suspended(&self) -> bool {
            *self == Self::Suspended
        }
        #[inline(always)]
        pub fn active_nosof(&self) -> bool {
            *self == Self::ActiveNosof
        }
        #[inline(always)]
        pub fn resuming(&self) -> bool {
            *self == Self::Resuming
        }
    }
    impl TryFrom<u32> for LinkState {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<LinkState, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, LinkState>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<LinkState> for u32 {
        fn from(val: LinkState) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct LinkStateSelector();
        impl LinkStateSelector {
            #[inline(always)]
            pub fn disconnected(&self) -> super::LinkState {
                super::LinkState::Disconnected
            }
            #[inline(always)]
            pub fn powered(&self) -> super::LinkState {
                super::LinkState::Powered
            }
            #[inline(always)]
            pub fn powered_suspended(&self) -> super::LinkState {
                super::LinkState::PoweredSuspended
            }
            #[inline(always)]
            pub fn active(&self) -> super::LinkState {
                super::LinkState::Active
            }
            #[inline(always)]
            pub fn suspended(&self) -> super::LinkState {
                super::LinkState::Suspended
            }
            #[inline(always)]
            pub fn active_nosof(&self) -> super::LinkState {
                super::LinkState::ActiveNosof
            }
            #[inline(always)]
            pub fn resuming(&self) -> super::LinkState {
                super::LinkState::Resuming
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
    pub type Usbctrl =
        ureg::ReadWriteReg32<0, crate::regs::UsbctrlReadVal, crate::regs::UsbctrlWriteVal>;
    pub type EpOutEnable0 = ureg::ReadWriteReg32<
        0,
        crate::regs::EpOutEnable0ReadVal,
        crate::regs::EpOutEnable0WriteVal,
    >;
    pub type EpInEnable0 =
        ureg::ReadWriteReg32<0, crate::regs::EpInEnable0ReadVal, crate::regs::EpInEnable0WriteVal>;
    pub type Usbstat = ureg::ReadOnlyReg32<crate::regs::UsbstatReadVal>;
    pub type Avoutbuffer = ureg::WriteOnlyReg32<0, crate::regs::AvoutbufferWriteVal>;
    pub type Avsetupbuffer = ureg::WriteOnlyReg32<0, crate::regs::AvsetupbufferWriteVal>;
    pub type Rxfifo = ureg::ReadOnlyReg32<crate::regs::RxfifoReadVal>;
    pub type RxenableSetup0 = ureg::ReadWriteReg32<
        0,
        crate::regs::RxenableSetup0ReadVal,
        crate::regs::RxenableSetup0WriteVal,
    >;
    pub type RxenableOut0 = ureg::ReadWriteReg32<
        0,
        crate::regs::RxenableOut0ReadVal,
        crate::regs::RxenableOut0WriteVal,
    >;
    pub type SetNakOut0 =
        ureg::ReadWriteReg32<0, crate::regs::SetNakOut0ReadVal, crate::regs::SetNakOut0WriteVal>;
    pub type InSent0 =
        ureg::ReadWriteReg32<0, crate::regs::InSent0ReadVal, crate::regs::InSent0WriteVal>;
    pub type OutStall0 =
        ureg::ReadWriteReg32<0, crate::regs::OutStall0ReadVal, crate::regs::OutStall0WriteVal>;
    pub type InStall0 =
        ureg::ReadWriteReg32<0, crate::regs::InStall0ReadVal, crate::regs::InStall0WriteVal>;
    pub type Configin =
        ureg::ReadWriteReg32<0, crate::regs::ConfiginReadVal, crate::regs::ConfiginWriteVal>;
    pub type OutIso0 =
        ureg::ReadWriteReg32<0, crate::regs::OutIso0ReadVal, crate::regs::OutIso0WriteVal>;
    pub type InIso0 =
        ureg::ReadWriteReg32<0, crate::regs::InIso0ReadVal, crate::regs::InIso0WriteVal>;
    pub type OutDataToggle = ureg::ReadWriteReg32<
        0,
        crate::regs::OutDataToggleReadVal,
        crate::regs::OutDataToggleWriteVal,
    >;
    pub type InDataToggle = ureg::ReadWriteReg32<
        0,
        crate::regs::InDataToggleReadVal,
        crate::regs::InDataToggleWriteVal,
    >;
    pub type PhyPinsSense = ureg::ReadOnlyReg32<crate::regs::PhyPinsSenseReadVal>;
    pub type PhyPinsDrive = ureg::ReadWriteReg32<
        0,
        crate::regs::PhyPinsDriveReadVal,
        crate::regs::PhyPinsDriveWriteVal,
    >;
    pub type PhyConfig =
        ureg::ReadWriteReg32<4, crate::regs::PhyConfigReadVal, crate::regs::PhyConfigWriteVal>;
    pub type WakeControl = ureg::WriteOnlyReg32<0, crate::regs::WakeControlWriteVal>;
    pub type WakeEvents = ureg::ReadOnlyReg32<crate::regs::WakeEventsReadVal>;
    pub type FifoCtrl = ureg::WriteOnlyReg32<0, crate::regs::FifoCtrlWriteVal>;
    pub type CountOut =
        ureg::ReadWriteReg32<0, crate::regs::CountOutReadVal, crate::regs::CountOutWriteVal>;
    pub type CountIn =
        ureg::ReadWriteReg32<0, crate::regs::CountInReadVal, crate::regs::CountInWriteVal>;
    pub type CountNodataIn = ureg::ReadWriteReg32<
        0,
        crate::regs::CountNodataInReadVal,
        crate::regs::CountNodataInWriteVal,
    >;
    pub type CountErrors =
        ureg::ReadWriteReg32<0, crate::regs::CountErrorsReadVal, crate::regs::CountErrorsWriteVal>;
    pub type Buffer = ureg::ReadWriteReg32<0, u32, u32>;
}
