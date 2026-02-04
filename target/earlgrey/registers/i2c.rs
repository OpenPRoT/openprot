#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct I2c0 {
    _priv: (),
}
impl I2c0 {
    pub const PTR: *mut u32 = 0x40080000 as *mut u32;
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
pub struct I2c1 {
    _priv: (),
}
impl I2c1 {
    pub const PTR: *mut u32 = 0x40090000 as *mut u32;
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
pub struct I2c2 {
    _priv: (),
}
impl I2c2 {
    pub const PTR: *mut u32 = 0x400a0000 as *mut u32;
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
    #[doc = "I2C Control Register\n\nRead value: [`regs::CtrlReadVal`]; Write value: [`regs::CtrlWriteVal`]"]
    #[inline(always)]
    pub fn ctrl(&self) -> ureg::RegRef<crate::meta::Ctrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C Live Status Register for Host and Target modes\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C Read Data\n\nRead value: [`regs::RdataReadVal`]; Write value: [`regs::RdataWriteVal`]"]
    #[inline(always)]
    pub fn rdata(&self) -> ureg::RegRef<crate::meta::Rdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C Host Format Data\n\nWrites to this register are used to define and drive Controller-Mode transactions.\n\nRead value: [`regs::FdataReadVal`]; Write value: [`regs::FdataWriteVal`]"]
    #[inline(always)]
    pub fn fdata(&self) -> ureg::RegRef<crate::meta::Fdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C FIFO control register\n\nRead value: [`regs::FifoCtrlReadVal`]; Write value: [`regs::FifoCtrlWriteVal`]"]
    #[inline(always)]
    pub fn fifo_ctrl(&self) -> ureg::RegRef<crate::meta::FifoCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Host mode FIFO configuration\n\nRead value: [`regs::HostFifoConfigReadVal`]; Write value: [`regs::HostFifoConfigWriteVal`]"]
    #[inline(always)]
    pub fn host_fifo_config(&self) -> ureg::RegRef<crate::meta::HostFifoConfig, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Target mode FIFO configuration\n\nRead value: [`regs::TargetFifoConfigReadVal`]; Write value: [`regs::TargetFifoConfigWriteVal`]"]
    #[inline(always)]
    pub fn target_fifo_config(&self) -> ureg::RegRef<crate::meta::TargetFifoConfig, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Host mode FIFO status register\n\nRead value: [`regs::HostFifoStatusReadVal`]; Write value: [`regs::HostFifoStatusWriteVal`]"]
    #[inline(always)]
    pub fn host_fifo_status(&self) -> ureg::RegRef<crate::meta::HostFifoStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Target mode FIFO status register\n\nRead value: [`regs::TargetFifoStatusReadVal`]; Write value: [`regs::TargetFifoStatusWriteVal`]"]
    #[inline(always)]
    pub fn target_fifo_status(&self) -> ureg::RegRef<crate::meta::TargetFifoStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C Override Control Register\n\nRead value: [`regs::OvrdReadVal`]; Write value: [`regs::OvrdWriteVal`]"]
    #[inline(always)]
    pub fn ovrd(&self) -> ureg::RegRef<crate::meta::Ovrd, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Oversampled RX values\n\nRead value: [`regs::ValReadVal`]; Write value: [`regs::ValWriteVal`]"]
    #[inline(always)]
    pub fn val(&self) -> ureg::RegRef<crate::meta::Val, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).\nAll values are expressed in units of the input clock period.\nThese must be greater than 2 in order for the change in SCL to propagate to the input of the FSM so that acknowledgements are detected correctly.\n\nRead value: [`regs::Timing0ReadVal`]; Write value: [`regs::Timing0WriteVal`]"]
    #[inline(always)]
    pub fn timing0(&self) -> ureg::RegRef<crate::meta::Timing0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).\nAll values are expressed in units of the input clock period.\n\nRead value: [`regs::Timing1ReadVal`]; Write value: [`regs::Timing1WriteVal`]"]
    #[inline(always)]
    pub fn timing1(&self) -> ureg::RegRef<crate::meta::Timing1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).\nAll values are expressed in units of the input clock period.\n\nRead value: [`regs::Timing2ReadVal`]; Write value: [`regs::Timing2WriteVal`]"]
    #[inline(always)]
    pub fn timing2(&self) -> ureg::RegRef<crate::meta::Timing2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Detailed I2C Timings (directly corresponding to table 10, in the I2C Specification).\nAll values are expressed in units of the input clock period.\n\nRead value: [`regs::Timing3ReadVal`]; Write value: [`regs::Timing3WriteVal`]"]
    #[inline(always)]
    pub fn timing3(&self) -> ureg::RegRef<crate::meta::Timing3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Detailed I2C Timings (directly corresponding to table 10, in the I2C Specification).\nAll values are expressed in units of the input clock period.\n\nRead value: [`regs::Timing4ReadVal`]; Write value: [`regs::Timing4WriteVal`]"]
    #[inline(always)]
    pub fn timing4(&self) -> ureg::RegRef<crate::meta::Timing4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C clock stretching and bus timeout control.\n\nThis timeout must be enabled by setting !!TIMEOUT_CTRL.EN to 1, and the behavior of this feature depends on the value of !!TIMEOUT_CTRL.MODE.\n\nIf the mode is \"STRETCH_TIMEOUT\", this is used in I2C controller mode to detect whether a connected target is stretching a single low time beyond the timeout value.\nConfigured as such, this timeout is more informative and doesn't do more than assert the \"stretch_timeout\" interrupt.\n\nIf the mode is \"BUS_TIMEOUT\", it is used to detect whether the clock has been held low for too long instead, inclusive of the controller's clock low time.\nThis is useful for an SMBus context, where the VAL programmed should be tTIMEOUT:MIN.\n\nRead value: [`regs::TimeoutCtrlReadVal`]; Write value: [`regs::TimeoutCtrlWriteVal`]"]
    #[inline(always)]
    pub fn timeout_ctrl(&self) -> ureg::RegRef<crate::meta::TimeoutCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C target address and mask pairs\n\nRead value: [`regs::TargetIdReadVal`]; Write value: [`regs::TargetIdWriteVal`]"]
    #[inline(always)]
    pub fn target_id(&self) -> ureg::RegRef<crate::meta::TargetId, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C target acquired data\n\nRead value: [`regs::AcqdataReadVal`]; Write value: [`regs::AcqdataWriteVal`]"]
    #[inline(always)]
    pub fn acqdata(&self) -> ureg::RegRef<crate::meta::Acqdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C target transmit data\n\nRead value: [`regs::TxdataReadVal`]; Write value: [`regs::TxdataWriteVal`]"]
    #[inline(always)]
    pub fn txdata(&self) -> ureg::RegRef<crate::meta::Txdata, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C host clock generation timeout value (in units of input clock frequency).\n\nIn an active transaction in Target-Mode, if the Controller ceases to send SCL pulses\nfor this number of cycles then the \"host_timeout\" interrupt will be asserted.\n\nIn multi-controller monitoring mode, !!HOST_TIMEOUT_CTRL is required to be nonzero to transition out of the initial busy state.\nSet this CSR to 0 to disable this behaviour.\n\nRead value: [`regs::HostTimeoutCtrlReadVal`]; Write value: [`regs::HostTimeoutCtrlWriteVal`]"]
    #[inline(always)]
    pub fn host_timeout_ctrl(&self) -> ureg::RegRef<crate::meta::HostTimeoutCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x60 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "I2C target internal stretching timeout control.\nWhen the target has stretched beyond this time it will send a NACK for incoming data bytes or release SDA for outgoing data bytes.\nThe behavior for the address byte is configurable via !!CTRL.ACK_ADDR_AFTER_TIMEOUT.\nNote that the count accumulates stretching time over the course of a transaction.\nIn other words, this is equivalent to the SMBus cumulative target clock extension time.\n\nRead value: [`regs::TargetTimeoutCtrlReadVal`]; Write value: [`regs::TargetTimeoutCtrlWriteVal`]"]
    #[inline(always)]
    pub fn target_timeout_ctrl(&self) -> ureg::RegRef<crate::meta::TargetTimeoutCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Number of times the I2C target has NACK'ed a new transaction since the last read of this register.\nReading this register clears it.\nThis is useful because when the ACQ FIFO is full the software know that a NACK has occurred, but without this register would not know how many transactions it missed.\nWhen it reaches its maximum value it will stay at that value.\n\nRead value: [`regs::TargetNackCountReadVal`]; Write value: [`regs::TargetNackCountWriteVal`]"]
    #[inline(always)]
    pub fn target_nack_count(&self) -> ureg::RegRef<crate::meta::TargetNackCount, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x68 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Controls for mid-transfer (N)ACK phase handling\n\nRead value: [`regs::TargetAckCtrlReadVal`]; Write value: [`regs::TargetAckCtrlWriteVal`]"]
    #[inline(always)]
    pub fn target_ack_ctrl(&self) -> ureg::RegRef<crate::meta::TargetAckCtrl, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "The data byte pending to be written to the ACQ FIFO.\n\nThis CSR is only valid while the Target module is stretching in the (N)ACK phase, indicated by !!STATUS.ACK_CTRL_STRETCH .\nIt is intended to be used with ACK Control Mode, so software may check the current byte.\n\nRead value: [`regs::AcqFifoNextDataReadVal`]; Write value: [`regs::AcqFifoNextDataWriteVal`]"]
    #[inline(always)]
    pub fn acq_fifo_next_data(&self) -> ureg::RegRef<crate::meta::AcqFifoNextData, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Timeout in Host-Mode for an unhandled NACK before hardware automatically ends the transaction.\n(in units of input clock frequency)\n\nIf an active Controller-Transmitter transfer receives a NACK from the Target, the !!CONTROLLER_EVENTS.NACK bit is set.\nIn turn, this causes the Controller FSM to halt awaiting software intervention, and the 'controller_halt' interrupt may assert.\nSoftware must clear the !!CONTROLLER_EVENTS.NACK bit to allow the state machine to continue, typically after clearing out the FMTFIFO to start a new transfer.\nWhile halted, the active transaction is not ended (no STOP (P) condition is created), and the block asserts SCL and leaves SDA released.\n\nThis timeout can be used to automatically produce a STOP condition, whether as a backstop for slow software responses (longer timeout) or as a convenience (short timeout).\nIf the timeout expires, the Controller FSM will issue a STOP (P) condition on the bus to end the active transaction.\nAdditionally, the !!CONTROLLER_EVENTS.UNHANDLED_NACK_TIMEOUT bit is set to alert software, and the FSM will return to the idle state and halt until the bit is cleared.\n\nThe enable bit must be set for this feature to operate.\n\nRead value: [`regs::HostNackHandlerTimeoutReadVal`]; Write value: [`regs::HostNackHandlerTimeoutWriteVal`]"]
    #[inline(always)]
    pub fn host_nack_handler_timeout(
        &self,
    ) -> ureg::RegRef<crate::meta::HostNackHandlerTimeout, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Latched events that explain why the controller halted.\n\nAny bits that are set must be written (with a 1) to clear the CONTROLLER_HALT interrupt.\n\nRead value: [`regs::ControllerEventsReadVal`]; Write value: [`regs::ControllerEventsWriteVal`]"]
    #[inline(always)]
    pub fn controller_events(&self) -> ureg::RegRef<crate::meta::ControllerEvents, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Latched events that can cause the target module to stretch the clock at the beginning of a read transfer.\n\nThese events cause TX FIFO-related stretching even when the TX FIFO has data available.\nAny bits that are set must be written (with a 1) to clear the tx_stretch interrupt.\n\nThis CSR serves as a gate to prevent the Target module from responding to a read command with unrelated, leftover data.\n\nRead value: [`regs::TargetEventsReadVal`]; Write value: [`regs::TargetEventsWriteVal`]"]
    #[inline(always)]
    pub fn target_events(&self) -> ureg::RegRef<crate::meta::TargetEvents, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AcqdataReadVal(u32);
    impl AcqdataReadVal {
        #[doc = "Address for accepted transaction or acquired byte"]
        #[inline(always)]
        pub fn abyte(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Indicates any control symbols associated with the ABYTE.\n\nFor the STOP symbol, a stretch timeout or other unexpected events will cause a NACK_STOP to appear in the ACQ FIFO.\nIf the ACQ FIFO doesn't have enough space to record a START and a STOP, the transaction will be dropped entirely on a stretch timeout.\nIn that case, the START byte will not appear (neither as START nor NACK_START), but a standalone NACK_STOP may, if there was space.\nSoftware can discard any standalone NACK_STOP that appears.\n\nSee the associated values for more information about the contents."]
        #[inline(always)]
        pub fn signal(&self) -> super::enums::Signal {
            super::enums::Signal::try_from((self.0 >> 8) & 7).unwrap()
        }
    }
    impl From<u32> for AcqdataReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AcqdataReadVal> for u32 {
        #[inline(always)]
        fn from(val: AcqdataReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AcqFifoNextDataReadVal(u32);
    impl AcqFifoNextDataReadVal {
        #[inline(always)]
        pub fn acq_fifo_next_data(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
    }
    impl From<u32> for AcqFifoNextDataReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AcqFifoNextDataReadVal> for u32 {
        #[inline(always)]
        fn from(val: AcqFifoNextDataReadVal) -> u32 {
            val.0
        }
    }
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
    pub struct ControllerEventsReadVal(u32);
    impl ControllerEventsReadVal {
        #[doc = "Received an unexpected NACK"]
        #[inline(always)]
        pub fn nack(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "A Host-Mode active transaction has been ended by the !!HOST_NACK_HANDLER_TIMEOUT mechanism."]
        #[inline(always)]
        pub fn unhandled_nack_timeout(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "A Host-Mode active transaction has terminated due to a bus timeout activated by !!TIMEOUT_CTRL."]
        #[inline(always)]
        pub fn bus_timeout(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "A Host-Mode active transaction has terminated due to lost arbitration."]
        #[inline(always)]
        pub fn arbitration_lost(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ControllerEventsWriteVal {
            ControllerEventsWriteVal(self.0)
        }
    }
    impl From<u32> for ControllerEventsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControllerEventsReadVal> for u32 {
        #[inline(always)]
        fn from(val: ControllerEventsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControllerEventsWriteVal(u32);
    impl ControllerEventsWriteVal {
        #[doc = "Received an unexpected NACK"]
        #[inline(always)]
        pub fn nack_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "A Host-Mode active transaction has been ended by the !!HOST_NACK_HANDLER_TIMEOUT mechanism."]
        #[inline(always)]
        pub fn unhandled_nack_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "A Host-Mode active transaction has terminated due to a bus timeout activated by !!TIMEOUT_CTRL."]
        #[inline(always)]
        pub fn bus_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "A Host-Mode active transaction has terminated due to lost arbitration."]
        #[inline(always)]
        pub fn arbitration_lost_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
    }
    impl From<u32> for ControllerEventsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControllerEventsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ControllerEventsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CtrlReadVal(u32);
    impl CtrlReadVal {
        #[doc = "Enable Host I2C functionality"]
        #[inline(always)]
        pub fn enablehost(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable Target I2C functionality"]
        #[inline(always)]
        pub fn enabletarget(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable I2C line loopback test\nIf line loopback is enabled, the internal design sees ACQ and RX data as \"1\""]
        #[inline(always)]
        pub fn llpbk(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable NACKing the address on a stretch timeout.\n\nThis is a Target mode feature.\nIf enabled (1), a stretch timeout will cause the device to NACK the address byte.\nIf disabled (0), a stretch timeout will cause the device to ACK the address byte.\nSMBus requires that devices always ACK their address, even for read commands.\nHowever, non-SMBus protocols may have a different approach and can choose to NACK instead.\n\nNote that both cases handle data bytes the same way.\nFor writes, the Target module will NACK all subsequent data bytes until it receives a Stop.\nFor reads, the Target module will release SDA, causing 0xff to be returned for all data bytes until it receives a Stop."]
        #[inline(always)]
        pub fn nack_addr_after_timeout(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable I2C Target ACK Control Mode.\n\nACK Control Mode works together with !!TARGET_ACK_CTRL.NBYTES to allow software to control upper-layer protocol (N)ACKing (e.g. as in SMBus).\nThis bit enables the mode when 1, and !!TARGET_ACK_CTRL.NBYTES limits how many bytes may be automatically ACK'd while the ACQ FIFO has space.\nIf it is 0, the decision to ACK or NACK is made only from stretching timeouts and !!CTRL.NACK_ADDR_AFTER_TIMEOUT."]
        #[inline(always)]
        pub fn ack_ctrl_en(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable the bus monitor in multi-controller mode.\n\nIf a 0->1 transition happens while !!CTRL.ENABLEHOST and !!CTRL.ENABLETARGET are both 0, the bus monitor will enable and begin in the \"bus busy\" state.\nTo transition to a bus free state, !!HOST_TIMEOUT_CTRL must be nonzero, so the bus monitor may count out idle cycles to confirm the freedom to transmit.\nIn addition, the bus monitor will track whether the bus is free based on the enabled timeouts and detected Stop symbols.\nFor multi-controller mode, ensure !!CTRL.MULTI_CONTROLLER_MONITOR_EN becomes 1 no later than !!CTRL.ENABLEHOST or !!CTRL.ENABLETARGET.\nThis bit can be set at the same time as either or both of the other two, though.\n\nNote that if !!CTRL.MULTI_CONTROLLER_MONITOR_EN is set after !!CTRL.ENABLEHOST or !!CTRL.ENABLETARGET, the bus monitor will begin in the \"bus free\" state instead.\nThis would violate the proper protocol for a controller to join a multi-controller environment.\nHowever, if this controller is known to be the first to join, this ordering will enable skipping the idle wait.\n\nWhen 0, the bus monitor will report that the bus is always free, so the controller FSM is never blocked from transmitting."]
        #[inline(always)]
        pub fn multi_controller_monitor_en(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If set to 1, this bit causes a read transfer addressed to this target to set the corresponding bit in !!TARGET_EVENTS.\n\nWhile !!TARGET_EVENTS.TX_PENDING is 1, subsequent read transactions will stretch the clock, even if there is data in the TX FIFO.\n\nIf enabled, this function allows software to confirm the data in the TX FIFO should be released for the current read.\nThis may be useful for cases where the TX FIFO has data that does not apply to the current transfer.\nFor example, the transaction could've targeted an alternate function via another address."]
        #[inline(always)]
        pub fn tx_stretch_ctrl_en(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
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
        #[doc = "Enable Host I2C functionality"]
        #[inline(always)]
        pub fn enablehost(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable Target I2C functionality"]
        #[inline(always)]
        pub fn enabletarget(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable I2C line loopback test\nIf line loopback is enabled, the internal design sees ACQ and RX data as \"1\""]
        #[inline(always)]
        pub fn llpbk(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable NACKing the address on a stretch timeout.\n\nThis is a Target mode feature.\nIf enabled (1), a stretch timeout will cause the device to NACK the address byte.\nIf disabled (0), a stretch timeout will cause the device to ACK the address byte.\nSMBus requires that devices always ACK their address, even for read commands.\nHowever, non-SMBus protocols may have a different approach and can choose to NACK instead.\n\nNote that both cases handle data bytes the same way.\nFor writes, the Target module will NACK all subsequent data bytes until it receives a Stop.\nFor reads, the Target module will release SDA, causing 0xff to be returned for all data bytes until it receives a Stop."]
        #[inline(always)]
        pub fn nack_addr_after_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable I2C Target ACK Control Mode.\n\nACK Control Mode works together with !!TARGET_ACK_CTRL.NBYTES to allow software to control upper-layer protocol (N)ACKing (e.g. as in SMBus).\nThis bit enables the mode when 1, and !!TARGET_ACK_CTRL.NBYTES limits how many bytes may be automatically ACK'd while the ACQ FIFO has space.\nIf it is 0, the decision to ACK or NACK is made only from stretching timeouts and !!CTRL.NACK_ADDR_AFTER_TIMEOUT."]
        #[inline(always)]
        pub fn ack_ctrl_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable the bus monitor in multi-controller mode.\n\nIf a 0->1 transition happens while !!CTRL.ENABLEHOST and !!CTRL.ENABLETARGET are both 0, the bus monitor will enable and begin in the \"bus busy\" state.\nTo transition to a bus free state, !!HOST_TIMEOUT_CTRL must be nonzero, so the bus monitor may count out idle cycles to confirm the freedom to transmit.\nIn addition, the bus monitor will track whether the bus is free based on the enabled timeouts and detected Stop symbols.\nFor multi-controller mode, ensure !!CTRL.MULTI_CONTROLLER_MONITOR_EN becomes 1 no later than !!CTRL.ENABLEHOST or !!CTRL.ENABLETARGET.\nThis bit can be set at the same time as either or both of the other two, though.\n\nNote that if !!CTRL.MULTI_CONTROLLER_MONITOR_EN is set after !!CTRL.ENABLEHOST or !!CTRL.ENABLETARGET, the bus monitor will begin in the \"bus free\" state instead.\nThis would violate the proper protocol for a controller to join a multi-controller environment.\nHowever, if this controller is known to be the first to join, this ordering will enable skipping the idle wait.\n\nWhen 0, the bus monitor will report that the bus is always free, so the controller FSM is never blocked from transmitting."]
        #[inline(always)]
        pub fn multi_controller_monitor_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If set to 1, this bit causes a read transfer addressed to this target to set the corresponding bit in !!TARGET_EVENTS.\n\nWhile !!TARGET_EVENTS.TX_PENDING is 1, subsequent read transactions will stretch the clock, even if there is data in the TX FIFO.\n\nIf enabled, this function allows software to confirm the data in the TX FIFO should be released for the current read.\nThis may be useful for cases where the TX FIFO has data that does not apply to the current transfer.\nFor example, the transaction could've targeted an alternate function via another address."]
        #[inline(always)]
        pub fn tx_stretch_ctrl_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
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
    pub struct FdataWriteVal(u32);
    impl FdataWriteVal {
        #[doc = "Format Byte.\n\nIf no flags are set, hardware will transmit this byte directly.\n\nIf READB is set, this field becomes the number of bytes hardware will automatically\nread from the bus."]
        #[inline(always)]
        pub fn fbyte(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "Issue a START condition before transmitting FBYTE."]
        #[inline(always)]
        pub fn start(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Issue a STOP condition after transmitting FBYTE."]
        #[inline(always)]
        pub fn stop(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Transfer Direction Indicator.\n\nIf unset, this write to FDATA defines a controller-transmitter operation (WRITE).\nA single byte of data (FBYTE) is written to the bus.\n\nIf set, this write to FDATA defines a controller-receiver operation (READ).\nThe value of FBYTE defines the number of bytes read from the bus. (256 if FBYTE==0)\"\nAfter this number of bytes are read, the final byte will be NACKed to end the transfer\nunless RCONT is also set."]
        #[inline(always)]
        pub fn readb(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Do not NACK the last byte read, let the read operation continue."]
        #[inline(always)]
        pub fn rcont(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "For the currrent controller-transmitter byte (WRITE), do not halt via CONTROLLER_EVENTS\nor assert the 'controller_halt' interrupt if the current byte is not ACK'd."]
        #[inline(always)]
        pub fn nakok(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
    }
    impl From<u32> for FdataWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FdataWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FdataWriteVal) -> u32 {
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
        #[doc = "FMT fifo reset. Write 1 to the register resets FMT_FIFO. Read returns 0"]
        #[inline(always)]
        pub fn fmtrst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "ACQ FIFO reset. Write 1 to the register resets it. Read returns 0"]
        #[inline(always)]
        pub fn acqrst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "TX FIFO reset. Write 1 to the register resets it. Read returns 0"]
        #[inline(always)]
        pub fn txrst(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
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
    pub struct HostFifoConfigReadVal(u32);
    impl HostFifoConfigReadVal {
        #[doc = "Threshold level for RX interrupts. Whilst the level of data in the RX FIFO\nis above this setting, the rx_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn rx_thresh(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Threshold level for FMT interrupts. Whilst the number of used entries in the\nFMT FIFO is below this setting, the fmt_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn fmt_thresh(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> HostFifoConfigWriteVal {
            HostFifoConfigWriteVal(self.0)
        }
    }
    impl From<u32> for HostFifoConfigReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostFifoConfigReadVal> for u32 {
        #[inline(always)]
        fn from(val: HostFifoConfigReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostFifoConfigWriteVal(u32);
    impl HostFifoConfigWriteVal {
        #[doc = "Threshold level for RX interrupts. Whilst the level of data in the RX FIFO\nis above this setting, the rx_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn rx_thresh(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 0)) | ((val & 0xfff) << 0))
        }
        #[doc = "Threshold level for FMT interrupts. Whilst the number of used entries in the\nFMT FIFO is below this setting, the fmt_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn fmt_thresh(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
    }
    impl From<u32> for HostFifoConfigWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostFifoConfigWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HostFifoConfigWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostFifoStatusReadVal(u32);
    impl HostFifoStatusReadVal {
        #[doc = "Current fill level of FMT fifo"]
        #[inline(always)]
        pub fn fmtlvl(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Current fill level of RX fifo"]
        #[inline(always)]
        pub fn rxlvl(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
    }
    impl From<u32> for HostFifoStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostFifoStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: HostFifoStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostNackHandlerTimeoutReadVal(u32);
    impl HostNackHandlerTimeoutReadVal {
        #[doc = "Unhandled NAK timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0x7fffffff
        }
        #[doc = "Timeout enable"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> HostNackHandlerTimeoutWriteVal {
            HostNackHandlerTimeoutWriteVal(self.0)
        }
    }
    impl From<u32> for HostNackHandlerTimeoutReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostNackHandlerTimeoutReadVal> for u32 {
        #[inline(always)]
        fn from(val: HostNackHandlerTimeoutReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostNackHandlerTimeoutWriteVal(u32);
    impl HostNackHandlerTimeoutWriteVal {
        #[doc = "Unhandled NAK timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0x7fffffff << 0)) | ((val & 0x7fffffff) << 0))
        }
        #[doc = "Timeout enable"]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for HostNackHandlerTimeoutWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostNackHandlerTimeoutWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HostNackHandlerTimeoutWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostTimeoutCtrlReadVal(u32);
    impl HostTimeoutCtrlReadVal {
        #[inline(always)]
        pub fn host_timeout_ctrl(&self) -> u32 {
            (self.0 >> 0) & 0xfffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> HostTimeoutCtrlWriteVal {
            HostTimeoutCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for HostTimeoutCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostTimeoutCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: HostTimeoutCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct HostTimeoutCtrlWriteVal(u32);
    impl HostTimeoutCtrlWriteVal {
        #[inline(always)]
        pub fn host_timeout_ctrl(self, val: u32) -> Self {
            Self((self.0 & !(0xfffff << 0)) | ((val & 0xfffff) << 0))
        }
    }
    impl From<u32> for HostTimeoutCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<HostTimeoutCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: HostTimeoutCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.fmt_threshold is set."]
        #[inline(always)]
        pub fn fmt_threshold(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_threshold is set."]
        #[inline(always)]
        pub fn rx_threshold(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.acq_threshold is set."]
        #[inline(always)]
        pub fn acq_threshold(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_overflow is set."]
        #[inline(always)]
        pub fn rx_overflow(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.controller_halt is set."]
        #[inline(always)]
        pub fn controller_halt(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.scl_interference is set."]
        #[inline(always)]
        pub fn scl_interference(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.sda_interference is set."]
        #[inline(always)]
        pub fn sda_interference(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.stretch_timeout is set."]
        #[inline(always)]
        pub fn stretch_timeout(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.sda_unstable is set."]
        #[inline(always)]
        pub fn sda_unstable(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cmd_complete is set."]
        #[inline(always)]
        pub fn cmd_complete(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_stretch is set."]
        #[inline(always)]
        pub fn tx_stretch(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_threshold is set."]
        #[inline(always)]
        pub fn tx_threshold(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.acq_stretch is set."]
        #[inline(always)]
        pub fn acq_stretch(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.unexp_stop is set."]
        #[inline(always)]
        pub fn unexp_stop(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.host_timeout is set."]
        #[inline(always)]
        pub fn host_timeout(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
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
        #[doc = "Enable interrupt when !!INTR_STATE.fmt_threshold is set."]
        #[inline(always)]
        pub fn fmt_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_threshold is set."]
        #[inline(always)]
        pub fn rx_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.acq_threshold is set."]
        #[inline(always)]
        pub fn acq_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.rx_overflow is set."]
        #[inline(always)]
        pub fn rx_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.controller_halt is set."]
        #[inline(always)]
        pub fn controller_halt(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.scl_interference is set."]
        #[inline(always)]
        pub fn scl_interference(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.sda_interference is set."]
        #[inline(always)]
        pub fn sda_interference(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.stretch_timeout is set."]
        #[inline(always)]
        pub fn stretch_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.sda_unstable is set."]
        #[inline(always)]
        pub fn sda_unstable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.cmd_complete is set."]
        #[inline(always)]
        pub fn cmd_complete(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_stretch is set."]
        #[inline(always)]
        pub fn tx_stretch(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tx_threshold is set."]
        #[inline(always)]
        pub fn tx_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.acq_stretch is set."]
        #[inline(always)]
        pub fn acq_stretch(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.unexp_stop is set."]
        #[inline(always)]
        pub fn unexp_stop(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.host_timeout is set."]
        #[inline(always)]
        pub fn host_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
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
        #[doc = "host mode interrupt: asserted whilst the FMT FIFO level is below the low threshold. This is a level status interrupt."]
        #[inline(always)]
        pub fn fmt_threshold(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "host mode interrupt: asserted whilst the RX FIFO level is above the high threshold. This is a level status interrupt."]
        #[inline(always)]
        pub fn rx_threshold(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "target mode interrupt: asserted whilst the ACQ FIFO level is above the high threshold. This is a level status interrupt."]
        #[inline(always)]
        pub fn acq_threshold(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if the RX FIFO has overflowed."]
        #[inline(always)]
        pub fn rx_overflow(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if the controller FSM is halted, such as on an unexpected NACK or lost arbitration.\nCheck !!CONTROLLER_EVENTS for the reason.\nThe interrupt will be released when the bits in !!CONTROLLER_EVENTS are cleared."]
        #[inline(always)]
        pub fn controller_halt(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if the SCL line drops early (not supported without clock synchronization)."]
        #[inline(always)]
        pub fn scl_interference(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if the SDA line goes low when host is trying to assert high"]
        #[inline(always)]
        pub fn sda_interference(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if target stretches the clock beyond the allowed timeout period"]
        #[inline(always)]
        pub fn stretch_timeout(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "host mode interrupt: raised if the target does not assert a constant value of SDA during transmission."]
        #[inline(always)]
        pub fn sda_unstable(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "host and target mode interrupt.\nIn host mode, raised if the host issues a repeated START or terminates the transaction by issuing STOP.\nIn target mode, raised if the external host issues a STOP or repeated START."]
        #[inline(always)]
        pub fn cmd_complete(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "target mode interrupt: raised if the target is stretching clocks for a read command. This is a level status interrupt."]
        #[inline(always)]
        pub fn tx_stretch(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "target mode interrupt: asserted whilst the TX FIFO level is below the low threshold. This is a level status interrupt."]
        #[inline(always)]
        pub fn tx_threshold(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "target mode interrupt: raised if the target is stretching clocks due to full ACQ FIFO or zero count in !!TARGET_ACK_CTRL.NBYTES (if enabled). This is a level status interrupt."]
        #[inline(always)]
        pub fn acq_stretch(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "target mode interrupt: raised if STOP is received without a preceding NACK during an external host read."]
        #[inline(always)]
        pub fn unexp_stop(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "target mode interrupt: raised if the host stops sending the clock during an ongoing transaction."]
        #[inline(always)]
        pub fn host_timeout(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
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
        #[doc = "host mode interrupt: raised if the RX FIFO has overflowed."]
        #[inline(always)]
        pub fn rx_overflow_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "host mode interrupt: raised if the SCL line drops early (not supported without clock synchronization)."]
        #[inline(always)]
        pub fn scl_interference_clear(self) -> Self {
            Self(self.0 | (1 << 5))
        }
        #[doc = "host mode interrupt: raised if the SDA line goes low when host is trying to assert high"]
        #[inline(always)]
        pub fn sda_interference_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "host mode interrupt: raised if target stretches the clock beyond the allowed timeout period"]
        #[inline(always)]
        pub fn stretch_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
        #[doc = "host mode interrupt: raised if the target does not assert a constant value of SDA during transmission."]
        #[inline(always)]
        pub fn sda_unstable_clear(self) -> Self {
            Self(self.0 | (1 << 8))
        }
        #[doc = "host and target mode interrupt.\nIn host mode, raised if the host issues a repeated START or terminates the transaction by issuing STOP.\nIn target mode, raised if the external host issues a STOP or repeated START."]
        #[inline(always)]
        pub fn cmd_complete_clear(self) -> Self {
            Self(self.0 | (1 << 9))
        }
        #[doc = "target mode interrupt: raised if STOP is received without a preceding NACK during an external host read."]
        #[inline(always)]
        pub fn unexp_stop_clear(self) -> Self {
            Self(self.0 | (1 << 13))
        }
        #[doc = "target mode interrupt: raised if the host stops sending the clock during an ongoing transaction."]
        #[inline(always)]
        pub fn host_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 14))
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
        #[doc = "Write 1 to force !!INTR_STATE.fmt_threshold to 1."]
        #[inline(always)]
        pub fn fmt_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_threshold to 1."]
        #[inline(always)]
        pub fn rx_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.acq_threshold to 1."]
        #[inline(always)]
        pub fn acq_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.rx_overflow to 1."]
        #[inline(always)]
        pub fn rx_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to force !!INTR_STATE.controller_halt to 1."]
        #[inline(always)]
        pub fn controller_halt(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to force !!INTR_STATE.scl_interference to 1."]
        #[inline(always)]
        pub fn scl_interference(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Write 1 to force !!INTR_STATE.sda_interference to 1."]
        #[inline(always)]
        pub fn sda_interference(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Write 1 to force !!INTR_STATE.stretch_timeout to 1."]
        #[inline(always)]
        pub fn stretch_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Write 1 to force !!INTR_STATE.sda_unstable to 1."]
        #[inline(always)]
        pub fn sda_unstable(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Write 1 to force !!INTR_STATE.cmd_complete to 1."]
        #[inline(always)]
        pub fn cmd_complete(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tx_stretch to 1."]
        #[inline(always)]
        pub fn tx_stretch(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tx_threshold to 1."]
        #[inline(always)]
        pub fn tx_threshold(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Write 1 to force !!INTR_STATE.acq_stretch to 1."]
        #[inline(always)]
        pub fn acq_stretch(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Write 1 to force !!INTR_STATE.unexp_stop to 1."]
        #[inline(always)]
        pub fn unexp_stop(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Write 1 to force !!INTR_STATE.host_timeout to 1."]
        #[inline(always)]
        pub fn host_timeout(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
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
        #[doc = "Override the SDA and SCL TX signals."]
        #[inline(always)]
        pub fn txovrden(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Value for SCL Override. Set to 0 to drive TX Low, and set to 1 for high-Z"]
        #[inline(always)]
        pub fn sclval(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Value for SDA Override. Set to 0 to drive TX Low, and set to 1 for high-Z"]
        #[inline(always)]
        pub fn sdaval(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
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
        #[doc = "Override the SDA and SCL TX signals."]
        #[inline(always)]
        pub fn txovrden(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Value for SCL Override. Set to 0 to drive TX Low, and set to 1 for high-Z"]
        #[inline(always)]
        pub fn sclval(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Value for SDA Override. Set to 0 to drive TX Low, and set to 1 for high-Z"]
        #[inline(always)]
        pub fn sdaval(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
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
        #[doc = "Host mode FMT FIFO is full"]
        #[inline(always)]
        pub fn fmtfull(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Host mode RX FIFO is full"]
        #[inline(always)]
        pub fn rxfull(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Host mode FMT FIFO is empty"]
        #[inline(always)]
        pub fn fmtempty(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Host functionality is idle. No Host transaction is in progress"]
        #[inline(always)]
        pub fn hostidle(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Target functionality is idle. No Target transaction is in progress"]
        #[inline(always)]
        pub fn targetidle(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Host mode RX FIFO is empty"]
        #[inline(always)]
        pub fn rxempty(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Target mode TX FIFO is full"]
        #[inline(always)]
        pub fn txfull(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Target mode receive FIFO is full"]
        #[inline(always)]
        pub fn acqfull(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Target mode TX FIFO is empty"]
        #[inline(always)]
        pub fn txempty(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Target mode receive FIFO is empty"]
        #[inline(always)]
        pub fn acqempty(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Target mode stretching at (N)ACK phase due to zero count in !!TARGET_ACK_CTRL.NBYTES"]
        #[inline(always)]
        pub fn ack_ctrl_stretch(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
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
    pub struct TargetAckCtrlReadVal(u32);
    impl TargetAckCtrlReadVal {
        #[doc = "Remaining number of bytes the Target module may ACK automatically.\n\nIf !!CTRL.ACK_CTRL_EN is set to 1, the Target module will stretch the clock at the (N)ACK phase of a byte if this CSR is 0, awaiting software's instructions.\n\nAt the beginning of each Write transfer, this byte count is reset to 0.\nWrites to this CSR also are only accepted while the Target module is stretching the clock.\nThe Target module will always ACK its address if the ACQ FIFO has space.\nFor data bytes afterwards, it will stop at the (N)ACK phase and stretch the clock when this CSR is 0.\nFor each data byte that is ACK'd in a transaction, the byte count will decrease by 1.\n\nNote that a full ACQ FIFO can still cause the Target module to halt at the beginning of a new byte.\nThe ACK Control Mode provides an additional synchronization point, during the (N)ACK phase instead of after.\nFor both cases, !!TARGET_TIMEOUT_CTRL applies, and stretching past the timeout will produce an automatic NACK.\n\nThis mode can be used to implement the mid-transfer (N)ACK responses required by various SMBus protocols."]
        #[inline(always)]
        pub fn nbytes(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TargetAckCtrlWriteVal {
            TargetAckCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for TargetAckCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetAckCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetAckCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetAckCtrlWriteVal(u32);
    impl TargetAckCtrlWriteVal {
        #[doc = "Remaining number of bytes the Target module may ACK automatically.\n\nIf !!CTRL.ACK_CTRL_EN is set to 1, the Target module will stretch the clock at the (N)ACK phase of a byte if this CSR is 0, awaiting software's instructions.\n\nAt the beginning of each Write transfer, this byte count is reset to 0.\nWrites to this CSR also are only accepted while the Target module is stretching the clock.\nThe Target module will always ACK its address if the ACQ FIFO has space.\nFor data bytes afterwards, it will stop at the (N)ACK phase and stretch the clock when this CSR is 0.\nFor each data byte that is ACK'd in a transaction, the byte count will decrease by 1.\n\nNote that a full ACQ FIFO can still cause the Target module to halt at the beginning of a new byte.\nThe ACK Control Mode provides an additional synchronization point, during the (N)ACK phase instead of after.\nFor both cases, !!TARGET_TIMEOUT_CTRL applies, and stretching past the timeout will produce an automatic NACK.\n\nThis mode can be used to implement the mid-transfer (N)ACK responses required by various SMBus protocols."]
        #[inline(always)]
        pub fn nbytes(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "When the Target module stretches on the (N)ACK phase of a Write due to !!TARGET_ACK_CTRL.NBYTES being 0, writing a 1 here will cause it to send a NACK.\n\nIf software chooses to NACK, note that the NACKing behavior is the same as if a stretch timeout occurred.\nThe rest of the transaction will be NACK'd, including subsequent transfers.\nFor the address byte, the (N)ACK phase of subsequent transfers will follow the behavior specified by !!CTRL.NACK_ADDR_AFTER_TIMEOUT.\n\nAutomatically clears to 0."]
        #[inline(always)]
        pub fn nack(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for TargetAckCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetAckCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TargetAckCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetEventsReadVal(u32);
    impl TargetEventsReadVal {
        #[doc = "A new Target-Mode read transfer has arrived that addressed this target.\n\nThis bit is used by software to confirm the release of the contents in the TX FIFO.\nIf the contents do not apply, software should first reset the TX FIFO, then load it with the correct data, then clear this bit.\n\nOptionally enabled by !!CTRL.TX_STRETCH_CTRL_EN."]
        #[inline(always)]
        pub fn tx_pending(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "A Target-Mode read transfer has terminated due to a bus timeout activated by !!TIMEOUT_CTRL."]
        #[inline(always)]
        pub fn bus_timeout(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "A Target-Mode read transfer has terminated due to lost arbitration."]
        #[inline(always)]
        pub fn arbitration_lost(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TargetEventsWriteVal {
            TargetEventsWriteVal(self.0)
        }
    }
    impl From<u32> for TargetEventsReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetEventsReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetEventsReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetEventsWriteVal(u32);
    impl TargetEventsWriteVal {
        #[doc = "A new Target-Mode read transfer has arrived that addressed this target.\n\nThis bit is used by software to confirm the release of the contents in the TX FIFO.\nIf the contents do not apply, software should first reset the TX FIFO, then load it with the correct data, then clear this bit.\n\nOptionally enabled by !!CTRL.TX_STRETCH_CTRL_EN."]
        #[inline(always)]
        pub fn tx_pending_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "A Target-Mode read transfer has terminated due to a bus timeout activated by !!TIMEOUT_CTRL."]
        #[inline(always)]
        pub fn bus_timeout_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "A Target-Mode read transfer has terminated due to lost arbitration."]
        #[inline(always)]
        pub fn arbitration_lost_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
    }
    impl From<u32> for TargetEventsWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetEventsWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TargetEventsWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetFifoConfigReadVal(u32);
    impl TargetFifoConfigReadVal {
        #[doc = "Threshold level for TX interrupts. Whilst the number of used entries in the\nTX FIFO is below this setting, the tx_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn tx_thresh(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Threshold level for ACQ interrupts. Whilst the level of data in the ACQ FIFO\nis above this setting, the acq_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn acq_thresh(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TargetFifoConfigWriteVal {
            TargetFifoConfigWriteVal(self.0)
        }
    }
    impl From<u32> for TargetFifoConfigReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetFifoConfigReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetFifoConfigReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetFifoConfigWriteVal(u32);
    impl TargetFifoConfigWriteVal {
        #[doc = "Threshold level for TX interrupts. Whilst the number of used entries in the\nTX FIFO is below this setting, the tx_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn tx_thresh(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 0)) | ((val & 0xfff) << 0))
        }
        #[doc = "Threshold level for ACQ interrupts. Whilst the level of data in the ACQ FIFO\nis above this setting, the acq_threshold interrupt will be asserted."]
        #[inline(always)]
        pub fn acq_thresh(self, val: u32) -> Self {
            Self((self.0 & !(0xfff << 16)) | ((val & 0xfff) << 16))
        }
    }
    impl From<u32> for TargetFifoConfigWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetFifoConfigWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TargetFifoConfigWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetFifoStatusReadVal(u32);
    impl TargetFifoStatusReadVal {
        #[doc = "Current fill level of TX fifo"]
        #[inline(always)]
        pub fn txlvl(&self) -> u32 {
            (self.0 >> 0) & 0xfff
        }
        #[doc = "Current fill level of ACQ fifo"]
        #[inline(always)]
        pub fn acqlvl(&self) -> u32 {
            (self.0 >> 16) & 0xfff
        }
    }
    impl From<u32> for TargetFifoStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetFifoStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetFifoStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetIdReadVal(u32);
    impl TargetIdReadVal {
        #[doc = "I2C target address number 0"]
        #[inline(always)]
        pub fn address0(&self) -> u32 {
            (self.0 >> 0) & 0x7f
        }
        #[doc = "I2C target mask number 0.\nAt least one bit in MASK0 must be set to 1 for ADDRESS0 to be used."]
        #[inline(always)]
        pub fn mask0(&self) -> u32 {
            (self.0 >> 7) & 0x7f
        }
        #[doc = "I2C target address number 1"]
        #[inline(always)]
        pub fn address1(&self) -> u32 {
            (self.0 >> 14) & 0x7f
        }
        #[doc = "I2C target mask number 1.\nAt least one bit in MASK1 must be set to 1 for ADDRESS1 to be used."]
        #[inline(always)]
        pub fn mask1(&self) -> u32 {
            (self.0 >> 21) & 0x7f
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TargetIdWriteVal {
            TargetIdWriteVal(self.0)
        }
    }
    impl From<u32> for TargetIdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetIdReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetIdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetIdWriteVal(u32);
    impl TargetIdWriteVal {
        #[doc = "I2C target address number 0"]
        #[inline(always)]
        pub fn address0(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 0)) | ((val & 0x7f) << 0))
        }
        #[doc = "I2C target mask number 0.\nAt least one bit in MASK0 must be set to 1 for ADDRESS0 to be used."]
        #[inline(always)]
        pub fn mask0(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 7)) | ((val & 0x7f) << 7))
        }
        #[doc = "I2C target address number 1"]
        #[inline(always)]
        pub fn address1(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 14)) | ((val & 0x7f) << 14))
        }
        #[doc = "I2C target mask number 1.\nAt least one bit in MASK1 must be set to 1 for ADDRESS1 to be used."]
        #[inline(always)]
        pub fn mask1(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 21)) | ((val & 0x7f) << 21))
        }
    }
    impl From<u32> for TargetIdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetIdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TargetIdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetNackCountReadVal(u32);
    impl TargetNackCountReadVal {
        #[inline(always)]
        pub fn target_nack_count(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
    }
    impl From<u32> for TargetNackCountReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetNackCountReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetNackCountReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetTimeoutCtrlReadVal(u32);
    impl TargetTimeoutCtrlReadVal {
        #[doc = "Clock stretching timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0x7fffffff
        }
        #[doc = "Enable timeout feature and send NACK once the timeout has been reached"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TargetTimeoutCtrlWriteVal {
            TargetTimeoutCtrlWriteVal(self.0)
        }
    }
    impl From<u32> for TargetTimeoutCtrlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetTimeoutCtrlReadVal> for u32 {
        #[inline(always)]
        fn from(val: TargetTimeoutCtrlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TargetTimeoutCtrlWriteVal(u32);
    impl TargetTimeoutCtrlWriteVal {
        #[doc = "Clock stretching timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0x7fffffff << 0)) | ((val & 0x7fffffff) << 0))
        }
        #[doc = "Enable timeout feature and send NACK once the timeout has been reached"]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for TargetTimeoutCtrlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TargetTimeoutCtrlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TargetTimeoutCtrlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TimeoutCtrlReadVal(u32);
    impl TimeoutCtrlReadVal {
        #[doc = "Clock stretching timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(&self) -> u32 {
            (self.0 >> 0) & 0x3fffffff
        }
        #[doc = "Selects the timeout mode, between a stretch timeout and a bus timeout.\n\nBetween the two modes, the primary difference is how much of the clock low period is counted.\nFor a stretch timeout, only the time that another device holds the clock low will be counted.\nFor a bus timeout, the entire clock low time is counted, consistent with the SMBus tTIMEOUT type.\n\n!!TIMEOUT_CTRL.EN must be 1 for either of these features to be enabled."]
        #[inline(always)]
        pub fn mode(&self) -> super::enums::Mode {
            super::enums::Mode::try_from((self.0 >> 30) & 1).unwrap()
        }
        #[doc = "Enable stretch timeout or bus timeout feature"]
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
        #[doc = "Clock stretching timeout value (in units of input clock frequency)"]
        #[inline(always)]
        pub fn val(self, val: u32) -> Self {
            Self((self.0 & !(0x3fffffff << 0)) | ((val & 0x3fffffff) << 0))
        }
        #[doc = "Selects the timeout mode, between a stretch timeout and a bus timeout.\n\nBetween the two modes, the primary difference is how much of the clock low period is counted.\nFor a stretch timeout, only the time that another device holds the clock low will be counted.\nFor a bus timeout, the entire clock low time is counted, consistent with the SMBus tTIMEOUT type.\n\n!!TIMEOUT_CTRL.EN must be 1 for either of these features to be enabled."]
        #[inline(always)]
        pub fn mode(
            self,
            f: impl FnOnce(super::enums::selector::ModeSelector) -> super::enums::Mode,
        ) -> Self {
            Self(
                (self.0 & !(1 << 30))
                    | (u32::from(f(super::enums::selector::ModeSelector())) << 30),
            )
        }
        #[doc = "Enable stretch timeout or bus timeout feature"]
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
    pub struct Timing0ReadVal(u32);
    impl Timing0ReadVal {
        #[doc = "The actual time to hold SCL high in a given pulse.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn thigh(&self) -> u32 {
            (self.0 >> 0) & 0x1fff
        }
        #[doc = "The actual time to hold SCL low between any two SCL pulses.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tlow(&self) -> u32 {
            (self.0 >> 16) & 0x1fff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Timing0WriteVal {
            Timing0WriteVal(self.0)
        }
    }
    impl From<u32> for Timing0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Timing0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing0WriteVal(u32);
    impl Timing0WriteVal {
        #[doc = "The actual time to hold SCL high in a given pulse.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn thigh(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 0)) | ((val & 0x1fff) << 0))
        }
        #[doc = "The actual time to hold SCL low between any two SCL pulses.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tlow(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 16)) | ((val & 0x1fff) << 16))
        }
    }
    impl From<u32> for Timing0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Timing0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing1ReadVal(u32);
    impl Timing1ReadVal {
        #[doc = "The nominal rise time to anticipate for the bus (depends on capacitance).\nThis field is sized to have a range of at least Standard Mode's 1000 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_r(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = "The nominal fall time to anticipate for the bus (influences SDA hold times).\nThis field is sized to have a range of at least Standard Mode's 300 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_f(&self) -> u32 {
            (self.0 >> 16) & 0x1ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Timing1WriteVal {
            Timing1WriteVal(self.0)
        }
    }
    impl From<u32> for Timing1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Timing1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing1WriteVal(u32);
    impl Timing1WriteVal {
        #[doc = "The nominal rise time to anticipate for the bus (depends on capacitance).\nThis field is sized to have a range of at least Standard Mode's 1000 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_r(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[doc = "The nominal fall time to anticipate for the bus (influences SDA hold times).\nThis field is sized to have a range of at least Standard Mode's 300 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_f(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 16)) | ((val & 0x1ff) << 16))
        }
    }
    impl From<u32> for Timing1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Timing1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing2ReadVal(u32);
    impl Timing2ReadVal {
        #[doc = "Actual setup time for repeated start signals.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_sta(&self) -> u32 {
            (self.0 >> 0) & 0x1fff
        }
        #[doc = "Actual hold time for start signals.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn thd_sta(&self) -> u32 {
            (self.0 >> 16) & 0x1fff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Timing2WriteVal {
            Timing2WriteVal(self.0)
        }
    }
    impl From<u32> for Timing2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Timing2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing2WriteVal(u32);
    impl Timing2WriteVal {
        #[doc = "Actual setup time for repeated start signals.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_sta(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 0)) | ((val & 0x1fff) << 0))
        }
        #[doc = "Actual hold time for start signals.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn thd_sta(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 16)) | ((val & 0x1fff) << 16))
        }
    }
    impl From<u32> for Timing2WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing2WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Timing2WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing3ReadVal(u32);
    impl Timing3ReadVal {
        #[doc = "Actual setup time for data (or ack) bits.\nThis field is sized to have a range of at least Standard Mode's 250 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_dat(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = "Actual hold time for data (or ack) bits.\n(Note, where required, the parameters TVD_DAT is taken to be THD_DAT+T_F)\nThis field is sized to have a range that accommodates Standard Mode's 3.45 us max for TVD_DAT with a core clock at 1 GHz.\nHowever, this field is generally expected to represent a time substantially shorter than that.\nIt should be long enough to cover the maximum round-trip latency from output pins, through pads and voltage transitions on the board, and back to the input pins, but it should not be substantially greater."]
        #[inline(always)]
        pub fn thd_dat(&self) -> u32 {
            (self.0 >> 16) & 0x1fff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Timing3WriteVal {
            Timing3WriteVal(self.0)
        }
    }
    impl From<u32> for Timing3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Timing3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing3WriteVal(u32);
    impl Timing3WriteVal {
        #[doc = "Actual setup time for data (or ack) bits.\nThis field is sized to have a range of at least Standard Mode's 250 ns max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_dat(self, val: u32) -> Self {
            Self((self.0 & !(0x1ff << 0)) | ((val & 0x1ff) << 0))
        }
        #[doc = "Actual hold time for data (or ack) bits.\n(Note, where required, the parameters TVD_DAT is taken to be THD_DAT+T_F)\nThis field is sized to have a range that accommodates Standard Mode's 3.45 us max for TVD_DAT with a core clock at 1 GHz.\nHowever, this field is generally expected to represent a time substantially shorter than that.\nIt should be long enough to cover the maximum round-trip latency from output pins, through pads and voltage transitions on the board, and back to the input pins, but it should not be substantially greater."]
        #[inline(always)]
        pub fn thd_dat(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 16)) | ((val & 0x1fff) << 16))
        }
    }
    impl From<u32> for Timing3WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing3WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Timing3WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing4ReadVal(u32);
    impl Timing4ReadVal {
        #[doc = "Actual setup time for stop signals.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_sto(&self) -> u32 {
            (self.0 >> 0) & 0x1fff
        }
        #[doc = "Actual time between each STOP signal and the following START signal.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_buf(&self) -> u32 {
            (self.0 >> 16) & 0x1fff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Timing4WriteVal {
            Timing4WriteVal(self.0)
        }
    }
    impl From<u32> for Timing4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Timing4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Timing4WriteVal(u32);
    impl Timing4WriteVal {
        #[doc = "Actual setup time for stop signals.\nThis field is sized to have a range of at least Standard Mode's 4.0 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn tsu_sto(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 0)) | ((val & 0x1fff) << 0))
        }
        #[doc = "Actual time between each STOP signal and the following START signal.\nThis field is sized to have a range of at least Standard Mode's 4.7 us max with a core clock at 1 GHz."]
        #[inline(always)]
        pub fn t_buf(self, val: u32) -> Self {
            Self((self.0 & !(0x1fff << 16)) | ((val & 0x1fff) << 16))
        }
    }
    impl From<u32> for Timing4WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Timing4WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Timing4WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TxdataWriteVal(u32);
    impl TxdataWriteVal {
        #[inline(always)]
        pub fn txdata(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for TxdataWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TxdataWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TxdataWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ValReadVal(u32);
    impl ValReadVal {
        #[doc = "Last 16 oversampled values of SCL. Most recent bit is bit 0, oldest 15."]
        #[inline(always)]
        pub fn scl_rx(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Last 16 oversampled values of SDA. Most recent bit is bit 16, oldest 31."]
        #[inline(always)]
        pub fn sda_rx(&self) -> u32 {
            (self.0 >> 16) & 0xffff
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
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Mode {
        StretchTimeout = 0,
        BusTimeout = 1,
    }
    impl Mode {
        #[inline(always)]
        pub fn stretch_timeout(&self) -> bool {
            *self == Self::StretchTimeout
        }
        #[inline(always)]
        pub fn bus_timeout(&self) -> bool {
            *self == Self::BusTimeout
        }
    }
    impl TryFrom<u32> for Mode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Mode, ()> {
            if val < 2 {
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
    pub enum Signal {
        None = 0,
        Start = 1,
        Stop = 2,
        Restart = 3,
        Nack = 4,
        NackStart = 5,
        NackStop = 6,
        Reserved7 = 7,
    }
    impl Signal {
        #[inline(always)]
        pub fn none(&self) -> bool {
            *self == Self::None
        }
        #[inline(always)]
        pub fn start(&self) -> bool {
            *self == Self::Start
        }
        #[inline(always)]
        pub fn stop(&self) -> bool {
            *self == Self::Stop
        }
        #[inline(always)]
        pub fn restart(&self) -> bool {
            *self == Self::Restart
        }
        #[inline(always)]
        pub fn nack(&self) -> bool {
            *self == Self::Nack
        }
        #[inline(always)]
        pub fn nack_start(&self) -> bool {
            *self == Self::NackStart
        }
        #[inline(always)]
        pub fn nack_stop(&self) -> bool {
            *self == Self::NackStop
        }
    }
    impl TryFrom<u32> for Signal {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Signal, ()> {
            if val < 8 {
                Ok(unsafe { core::mem::transmute::<u32, Signal>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Signal> for u32 {
        fn from(val: Signal) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct ModeSelector();
        impl ModeSelector {
            #[inline(always)]
            pub fn stretch_timeout(&self) -> super::Mode {
                super::Mode::StretchTimeout
            }
            #[inline(always)]
            pub fn bus_timeout(&self) -> super::Mode {
                super::Mode::BusTimeout
            }
        }
        pub struct SignalSelector();
        impl SignalSelector {
            #[inline(always)]
            pub fn none(&self) -> super::Signal {
                super::Signal::None
            }
            #[inline(always)]
            pub fn start(&self) -> super::Signal {
                super::Signal::Start
            }
            #[inline(always)]
            pub fn stop(&self) -> super::Signal {
                super::Signal::Stop
            }
            #[inline(always)]
            pub fn restart(&self) -> super::Signal {
                super::Signal::Restart
            }
            #[inline(always)]
            pub fn nack(&self) -> super::Signal {
                super::Signal::Nack
            }
            #[inline(always)]
            pub fn nack_start(&self) -> super::Signal {
                super::Signal::NackStart
            }
            #[inline(always)]
            pub fn nack_stop(&self) -> super::Signal {
                super::Signal::NackStop
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
    pub type Ctrl = ureg::ReadWriteReg32<0, crate::regs::CtrlReadVal, crate::regs::CtrlWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type Rdata = ureg::ReadOnlyReg32<crate::regs::RdataReadVal>;
    pub type Fdata = ureg::WriteOnlyReg32<0, crate::regs::FdataWriteVal>;
    pub type FifoCtrl = ureg::WriteOnlyReg32<0, crate::regs::FifoCtrlWriteVal>;
    pub type HostFifoConfig = ureg::ReadWriteReg32<
        0,
        crate::regs::HostFifoConfigReadVal,
        crate::regs::HostFifoConfigWriteVal,
    >;
    pub type TargetFifoConfig = ureg::ReadWriteReg32<
        0,
        crate::regs::TargetFifoConfigReadVal,
        crate::regs::TargetFifoConfigWriteVal,
    >;
    pub type HostFifoStatus = ureg::ReadOnlyReg32<crate::regs::HostFifoStatusReadVal>;
    pub type TargetFifoStatus = ureg::ReadOnlyReg32<crate::regs::TargetFifoStatusReadVal>;
    pub type Ovrd = ureg::ReadWriteReg32<0, crate::regs::OvrdReadVal, crate::regs::OvrdWriteVal>;
    pub type Val = ureg::ReadOnlyReg32<crate::regs::ValReadVal>;
    pub type Timing0 =
        ureg::ReadWriteReg32<0, crate::regs::Timing0ReadVal, crate::regs::Timing0WriteVal>;
    pub type Timing1 =
        ureg::ReadWriteReg32<0, crate::regs::Timing1ReadVal, crate::regs::Timing1WriteVal>;
    pub type Timing2 =
        ureg::ReadWriteReg32<0, crate::regs::Timing2ReadVal, crate::regs::Timing2WriteVal>;
    pub type Timing3 =
        ureg::ReadWriteReg32<0, crate::regs::Timing3ReadVal, crate::regs::Timing3WriteVal>;
    pub type Timing4 =
        ureg::ReadWriteReg32<0, crate::regs::Timing4ReadVal, crate::regs::Timing4WriteVal>;
    pub type TimeoutCtrl =
        ureg::ReadWriteReg32<0, crate::regs::TimeoutCtrlReadVal, crate::regs::TimeoutCtrlWriteVal>;
    pub type TargetId =
        ureg::ReadWriteReg32<0, crate::regs::TargetIdReadVal, crate::regs::TargetIdWriteVal>;
    pub type Acqdata = ureg::ReadOnlyReg32<crate::regs::AcqdataReadVal>;
    pub type Txdata = ureg::WriteOnlyReg32<0, crate::regs::TxdataWriteVal>;
    pub type HostTimeoutCtrl = ureg::ReadWriteReg32<
        0,
        crate::regs::HostTimeoutCtrlReadVal,
        crate::regs::HostTimeoutCtrlWriteVal,
    >;
    pub type TargetTimeoutCtrl = ureg::ReadWriteReg32<
        0,
        crate::regs::TargetTimeoutCtrlReadVal,
        crate::regs::TargetTimeoutCtrlWriteVal,
    >;
    pub type TargetNackCount = ureg::ReadOnlyReg32<crate::regs::TargetNackCountReadVal>;
    pub type TargetAckCtrl = ureg::ReadWriteReg32<
        0,
        crate::regs::TargetAckCtrlReadVal,
        crate::regs::TargetAckCtrlWriteVal,
    >;
    pub type AcqFifoNextData = ureg::ReadOnlyReg32<crate::regs::AcqFifoNextDataReadVal>;
    pub type HostNackHandlerTimeout = ureg::ReadWriteReg32<
        0,
        crate::regs::HostNackHandlerTimeoutReadVal,
        crate::regs::HostNackHandlerTimeoutWriteVal,
    >;
    pub type ControllerEvents = ureg::ReadWriteReg32<
        0,
        crate::regs::ControllerEventsReadVal,
        crate::regs::ControllerEventsWriteVal,
    >;
    pub type TargetEvents = ureg::ReadWriteReg32<
        0,
        crate::regs::TargetEventsReadVal,
        crate::regs::TargetEventsWriteVal,
    >;
}
