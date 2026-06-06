// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(test), no_std)]

use uart::RegisterBlock;
use usart_api::backend::{BackendError, IrqMask, LineStatus, Parity, UsartBackend, UsartConfig};

pub trait UartTx {
    fn tx_fifo_full(&self) -> bool;
    fn write_byte(&mut self, byte: u8);
}

pub struct UartDriver {
    uart: RegisterBlock<ureg::RealMmioMut<'static>>,
}

impl UartDriver {
    /// Creates a new `UartDriver` for the UART peripheral at the given base address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` points to a valid UART peripheral register block,
    /// and that they have exclusive access to it.
    pub unsafe fn new(ptr: *mut u32) -> Self {
        Self {
            uart: unsafe { RegisterBlock::new(ptr) },
        }
    }

    fn rx_fifo_empty(&self) -> bool {
        self.uart.status().read().rxempty()
    }

    fn read_byte(&mut self) -> u8 {
        self.uart.rdata().read().rdata() as u8
    }
}

impl UartTx for UartDriver {
    fn tx_fifo_full(&self) -> bool {
        self.uart.status().read().txfull()
    }

    fn write_byte(&mut self, byte: u8) {
        self.uart.wdata().write(|w| w.wdata(byte as u32));
    }
}

impl UsartBackend for UartDriver {
    fn configure(&mut self, config: UsartConfig) -> Result<(), BackendError> {
        if config.stop_bits != 1 {
            return Err(BackendError::InvalidConfiguration);
        }
        let parity_en = config.parity != Parity::None;
        let parity_odd = matches!(config.parity, Parity::Odd);

        if config.baud_rate != 0 {
            let nco = (((config.baud_rate as u64) << 20)
                / (earlgrey_clock_domain::PERIPHERAL_CLOCK_HZ)) as u32;
            if nco > 0xffff {
                return Err(BackendError::InvalidConfiguration);
            }
            self.uart.ctrl().write(|w| {
                w.tx(true)
                    .rx(true)
                    .parity_en(parity_en)
                    .parity_odd(parity_odd)
                    .nco(nco)
            });
        } else {
            self.uart.ctrl().modify(|w| {
                w.tx(true)
                    .rx(true)
                    .parity_en(parity_en)
                    .parity_odd(parity_odd)
            });
        }

        // Reset FIFOs and set RX trigger level to 1 byte
        self.uart.fifo_ctrl().write(|w| {
            w.txrst(true)
                .rxrst(true)
                .with_rxilvl(uart::enums::Rxilvl::Rxlvl1)
        });

        // Configure RX timeout (8 bit times)
        self.uart.timeout_ctrl().write(|w| w.val(8).en(true));

        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, BackendError> {
        if data.is_empty() {
            return Ok(0);
        }
        if self.tx_fifo_full() {
            return Err(BackendError::WouldBlock);
        }
        let mut written = 0;
        for &byte in data {
            if self.tx_fifo_full() {
                break;
            }
            self.write_byte(byte);
            written += 1;
        }
        Ok(written)
    }

    fn read(&mut self, out: &mut [u8]) -> Result<usize, BackendError> {
        let mut read_bytes = 0;
        for byte in out.iter_mut() {
            while self.rx_fifo_empty() {
                core::hint::spin_loop();
            }
            *byte = self.read_byte();
            read_bytes += 1;
        }
        Ok(read_bytes)
    }

    fn try_read(&mut self, out: &mut [u8]) -> Result<usize, BackendError> {
        if self.rx_fifo_empty() {
            return Err(BackendError::WouldBlock);
        }

        // Clear errors and timeout in intr_state before reading
        self.uart.intr_state().write(|w| {
            w.rx_overflow_clear()
                .rx_frame_err_clear()
                .rx_parity_err_clear()
                .rx_break_err_clear()
                .rx_timeout_clear()
        });

        let mut read_bytes = 0;
        for byte in out.iter_mut() {
            if self.rx_fifo_empty() {
                break;
            }
            *byte = self.read_byte();
            read_bytes += 1;
        }
        Ok(read_bytes)
    }

    fn line_status(&self) -> Result<LineStatus, BackendError> {
        let status = self.uart.status().read();
        let intr = self.uart.intr_state().read();

        let mut bits = 0u8;
        if !status.rxempty() {
            bits |= 0x01; // DataReady
        }
        if status.txempty() {
            bits |= 0x40 | 0x20; // TransmitterEmpty | TransmitterHoldingRegisterEmpty
        }
        if intr.rx_overflow() {
            bits |= 0x02; // OverrunError
        }
        if intr.rx_parity_err() {
            bits |= 0x04; // ParityError
        }
        if intr.rx_frame_err() {
            bits |= 0x08; // FramingError
        }
        if intr.rx_break_err() {
            bits |= 0x10; // BreakInterrupt
        }

        Ok(LineStatus(bits))
    }

    fn enable_interrupts(&mut self, mask: IrqMask) -> Result<(), BackendError> {
        // Clear pending interrupts first to avoid stale triggers
        self.uart.intr_state().write(|w| {
            let mut w = w;
            if mask.contains(IrqMask::RX_DATA_AVAILABLE) {
                w = w.rx_timeout_clear();
            }
            if mask.contains(IrqMask::TX_IDLE) {
                w = w.tx_done_clear();
            }
            w
        });

        self.uart.intr_enable().modify(|w| {
            let mut w = w;
            if mask.contains(IrqMask::RX_DATA_AVAILABLE) {
                w = w.rx_watermark(true).rx_timeout(true);
            }
            if mask.contains(IrqMask::TX_IDLE) {
                w = w.tx_done(true);
            }
            w
        });
        Ok(())
    }

    fn disable_interrupts(&mut self, mask: IrqMask) -> Result<(), BackendError> {
        self.uart.intr_enable().modify(|w| {
            let mut w = w;
            if mask.contains(IrqMask::RX_DATA_AVAILABLE) {
                w = w.rx_watermark(false).rx_timeout(false);
            }
            if mask.contains(IrqMask::TX_IDLE) {
                w = w.tx_done(false);
            }
            w
        });
        Ok(())
    }
}
