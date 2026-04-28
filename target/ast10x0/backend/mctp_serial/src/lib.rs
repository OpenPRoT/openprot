// Licensed under the Apache-2.0 license

#![no_std]

use ast10x0_peripherals::uart::Usart;
use openprot_mctp_transport_serial::SerialSender;

/// AST10x0 serial backend binding for MCTP transport.
///
/// This target owns peripheral construction and exposes a stable backend
/// type name (`Backend`) so higher layers can select it at build time.
pub struct Ast10x0MctpSerialBackend {
    sender: SerialSender<Usart>,
}

impl Ast10x0MctpSerialBackend {
    /// Create a backend using UART5 (AST10x0 debug/default UART).
    pub fn new() -> Self {
        // SAFETY: 0x7e78_4000 is UART5 MMIO base on AST10x0.
        let uart = unsafe { Usart::new(0x7e78_4000 as *const _) };
        Self {
            sender: SerialSender::new(uart),
        }
    }

    /// Access the serial sender used by the MCTP stack.
    pub fn sender_mut(&mut self) -> &mut SerialSender<Usart> {
        &mut self.sender
    }
}

impl Default for Ast10x0MctpSerialBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable type alias intended for compile-time backend selection.
pub type Backend = Ast10x0MctpSerialBackend;
