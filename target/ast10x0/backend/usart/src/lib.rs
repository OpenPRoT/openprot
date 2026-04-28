// Licensed under the Apache-2.0 license

#![no_std]

use ast10x0_peripherals::uart::Usart;
use embedded_io::{Read, Write};
use usart_api::backend::{BackendError, LineStatus, Parity, UsartBackend, UsartConfig};

pub struct Ast10x0UsartBackend {
    uart: Usart,
}

impl Ast10x0UsartBackend {
    /// Create a backend using UART5 (the standard debug UART for AST10x0).
    ///
    /// # Safety
    ///
    /// This function is safe because it uses the well-known UART5 base address
    /// and will only be called once per process by the server loop.
    pub fn new() -> Self {
        // SAFETY: 0x7e78_4000 is UART5 MMIO base on AST10x0 (IRQ 8); this
        // process is the sole owner of the mapping per
        // target/ast10x0/usart/system.json5.
        let uart = unsafe { Usart::new(0x7e78_4000 as *const _) };

        Self { uart }
    }
}

impl Default for Ast10x0UsartBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable type alias used by the server binary for compile-time backend selection.
pub type Backend = Ast10x0UsartBackend;

impl UsartBackend for Ast10x0UsartBackend {
    fn configure(&mut self, config: UsartConfig) -> Result<(), BackendError> {
        // Current low-level driver supports a fixed 8N1 configuration and limited rates.
        // Keep behavior explicit until richer runtime configuration is plumbed through.
        if config.stop_bits != 1 {
            return Err(BackendError::InvalidConfiguration);
        }
        if config.parity != Parity::None {
            return Err(BackendError::InvalidConfiguration);
        }
        match config.baud_rate {
            9_600 | 19_200 | 1_500_000 => Ok(()),
            _ => Err(BackendError::InvalidConfiguration),
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, BackendError> {
        // Use embedded_io::Write to send bytes
        match self.uart.write_all(data) {
            Ok(()) => Ok(data.len()),
            Err(_) => Err(BackendError::Timeout),
        }
    }

    fn read(&mut self, out: &mut [u8]) -> Result<usize, BackendError> {
        match self.uart.read(out) {
            Ok(n) => Ok(n),
            Err(_) => Err(BackendError::InternalError),
        }
    }

    fn line_status(&self) -> Result<LineStatus, BackendError> {
        let status = self.uart.read_line_status();
        Ok(LineStatus(status.bits()))
    }

    fn enable_interrupts(&mut self, mask: u16) -> Result<(), BackendError> {
        // Map generic mask bits to UART-specific interrupt enables
        if mask & 0x01 != 0 {
            self.uart.set_rx_data_available_interrupt();
        }
        if mask & 0x02 != 0 {
            self.uart.set_tx_idle_interrupt();
        }
        Ok(())
    }

    fn disable_interrupts(&mut self, mask: u16) -> Result<(), BackendError> {
        // Map generic mask bits to UART-specific interrupt disables
        if mask & 0x01 != 0 {
            self.uart.clear_rx_data_available_interrupt();
        }
        if mask & 0x02 != 0 {
            self.uart.clear_tx_idle_interrupt();
        }
        Ok(())
    }
}
