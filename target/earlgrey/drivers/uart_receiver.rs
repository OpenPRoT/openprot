#![no_std]
use uart;

pub struct UartReceiver {
    regs: uart::RegisterBlock<ureg::RealMmioMut<'static>>,
}

impl UartReceiver {
    pub unsafe fn new(ptr: *mut u32) -> Self {
        Self {
            regs: unsafe { uart::RegisterBlock::new(ptr) },
        }
    }

    pub fn enable_receiver(&mut self) {
        self.regs.ctrl().modify(|ctrl| ctrl.rx(true));
    }

    pub fn enable_interrupt(&mut self) {
        self.regs.intr_enable().modify(|en| en.rx_watermark(true));
        self.regs.fifo_ctrl()
            .modify(|fifo| fifo.rxilvl(|lvl| lvl.rxlvl1()));
    }

    pub fn receive(&mut self) -> Option<u8> {
        if !self.regs.status().read().rxempty() {
            let value = u32::from(self.regs.rdata().read());
            Some(value as u8)
        } else {
            None
        }
    }
}
