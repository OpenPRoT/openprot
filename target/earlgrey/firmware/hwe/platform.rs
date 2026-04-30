// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]
#![allow(unused_imports)]
use platform_codegen::{handle, signals};
use pw_status::Error;
use userspace::{entry, syscall};
use userspace::time::Instant;


use earlgrey_sysmgr_client::{SysmgrClient, BootInfo};
use uart_receiver::UartReceiver;
use uart::Uart0;
use util_error::ErrorCode;
use util_ipc::IpcChannel;

struct CommandProcessor {
    line: [u8; 160],
    pos: usize,
}

impl Default for CommandProcessor {
    fn default() -> Self {
        Self {
            line: [0u8; 160],
            pos: 0,
        }
    }
}

impl CommandProcessor {

    fn push(&mut self, byte: u8) -> bool {
        match byte {
            8 | 127 => {
                // Backspace
                if self.pos > 0 {
                    self.pos -= 1;
                    util_console::print!("\x08 \x08");
                }
                false
            }
            21 => {
                // Ctrl-U: Kill line
                while self.pos > 0 {
                    self.pos -= 1;
                    util_console::print!("\x08 \x08");
                }
                false
            }
            23 => {
                // Ctrl-W: Kill word
                if self.pos > 0 && self.line[self.pos-1] == b' ' {
                    self.pos -= 1;
                    util_console::print!("\x08 \x08");
                }
                while self.pos > 0 && self.line[self.pos-1] != b' ' {
                    self.pos -= 1;
                    util_console::print!("\x08 \x08");
                }
                false
            }
            13 => {
                // Enter: accept line
                util_console::println!("");
                true
            }
            _ => {
                // Any other ASCII character: append to line.
                if self.pos < self.line.len() && byte < 127 {
                    self.line[self.pos] = byte;
                    self.pos += 1;
                    util_console::print!("{}", byte as char);
                }
                false
            }
        }
    }

    fn clear(&mut self) {
        self.pos = 0;
    }

    fn execute(&self) -> Result<(), ErrorCode> {
        let mut n = 0;
        let mut cmd = [""; 20];
        // SAFETY: this is safe because only ascii input is permitted.
        let line = unsafe { core::str::from_utf8_unchecked(&self.line[..self.pos]) };
        for part in line.split(' ').filter(|x| !x.is_empty()) {
            if n < cmd.len() {
                cmd[n] = part;
                n += 1;
            } else {
                util_console::println!("ERROR: too many arguments");
            }
        }

        RootCommandHierarchy::exec(&cmd[..n])
    }
}

struct RootCommandHierarchy;
impl RootCommandHierarchy {
    fn exec(cmd: &[&str]) -> Result<(), ErrorCode> {
        match cmd {
            ["hello"] => util_console::println!("Hello world!"),
            ["info"] => Self::handle_info()?,
            ["reboot"] => Self::handle_reboot()?,
            [_, ..] => util_console::println!("Unknown command: {}", cmd[0]),
            [] => {},
        }
        Ok(())
    }

    fn handle_info() -> Result<(), ErrorCode> {
        let sysmgr = SysmgrClient::new(IpcChannel::new(handle::SYSMGR_PLATFORM));
        let info = sysmgr.get_boot_info()?;
        util_console::println!("{:#?}", info);
        Ok(())
    }
    fn handle_reboot() -> Result<(), ErrorCode> {
        let sysmgr = SysmgrClient::new(IpcChannel::new(handle::SYSMGR_PLATFORM));
        sysmgr.request_reboot()?;
        Ok(())
    }

}

fn platform_server() -> Result<(), ErrorCode> {
    let mut uart = unsafe { UartReceiver::new(Uart0::PTR) };

    let mut cmd = CommandProcessor::default();

    uart.enable_receiver();
    uart.enable_interrupt();
    loop {
        let w= syscall::object_wait(
            handle::UART_INTERRUPTS,
            signals::UART0_RX_WATERMARK,
            Instant::MAX,
        ).map_err(ErrorCode::kernel_error)?;
        if w.pending_signals.contains(signals::UART0_RX_WATERMARK) {
            if let Some(byte) = uart.receive() {
                if cmd.push(byte) {
                    let _ = cmd.execute();
                    cmd.clear();
                }
            }
            let _ = syscall::interrupt_ack(handle::UART_INTERRUPTS, signals::UART0_RX_WATERMARK);
        }
    }
}

fn uart_setup_pinmux() {
    use top_earlgrey::{PinmuxInsel, PinmuxPeripheralIn};
    let mut pinmux = unsafe { pinmux::PinmuxAon::new() };
    pinmux
        .regs_mut()
        .mio_periph_insel()
        .at(PinmuxPeripheralIn::Uart0Rx as usize)
        .modify(|_| (PinmuxInsel::Ioc3 as u32).into());
}

#[entry]
fn entry() -> ! {
    uart_setup_pinmux();
    let ret = platform_server().map_err(|e| {
        pw_log::error!("❌ FAILED: {:08x}", u32::from(e) as u32);
        Error::Unknown
    });
    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in {}", module_path!() as &str);
    loop {}
}
