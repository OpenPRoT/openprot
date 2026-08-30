// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

pub mod gpio;
pub mod sys;

use earlgrey_gpio::EarlGreyGpio;
use earlgrey_sysmgr_client::SysmgrClient;
use gpio::GpioCommandHandler;
use sys::SysCommandHandler;
use util_ipc::IpcHandle;

/// Zero-allocation whitespace-separated token iterator for CLI parsing.
pub struct TokenIter<'a> {
    input: &'a str,
}

impl<'a> TokenIter<'a> {
    pub const fn new(input: &'a str) -> Self {
        Self { input }
    }

    pub fn next_token(&mut self) -> Option<&'a str> {
        let bytes = self.input.as_bytes();
        let mut start = 0;
        while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
            start += 1;
        }
        if start >= bytes.len() {
            self.input = "";
            return None;
        }

        let mut end = start;
        while end < bytes.len() && bytes[end] != b' ' && bytes[end] != b'\t' {
            end += 1;
        }

        let token = &self.input[start..end];
        self.input = &self.input[end..];
        Some(token)
    }

    pub fn remaining(&self) -> &'a str {
        self.input.trim_start()
    }
}

/// Errors returned by CLI command handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliError {
    UnknownCommand,
    InvalidArguments,
    MissingArguments,
    HardwareError,
    NotImplemented,
}

/// Execution context passed to CLI command handlers.
pub struct CliContext<'a> {
    pub gpio: &'a mut EarlGreyGpio,
    pub sysmgr: &'a SysmgrClient<IpcHandle>,
    pub straps: u32,
}

/// Trait implemented by CLI command hierarchy handlers.
pub trait CommandHandler {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn execute(
        &mut self,
        tokens: &mut TokenIter<'_>,
        context: &mut CliContext<'_>,
    ) -> Result<(), CliError>;
}

/// Root CLI dispatcher for the platform service.
pub struct CliDispatcher {
    gpio_handler: GpioCommandHandler,
    sys_handler: SysCommandHandler,
}

impl CliDispatcher {
    pub const fn new() -> Self {
        Self {
            gpio_handler: GpioCommandHandler::new(),
            sys_handler: SysCommandHandler::new(),
        }
    }

    pub fn print_help(&self) {
        util_zfmt::debug!("Platform CLI Commands:");
        util_zfmt::debug!("  gpio  - GPIO pin configuration and control");
        util_zfmt::debug!("  sys   - System information and reset control");
        util_zfmt::debug!("  usb   - USB status and multiplexer control");
        util_zfmt::debug!("  flash - Flash status and memory info");
        util_zfmt::debug!("  help  - Display this help message");
    }

    pub fn dispatch(&mut self, line: &str, context: &mut CliContext<'_>) {
        let mut tokens = TokenIter::new(line);
        let Some(cmd) = tokens.next_token() else {
            return;
        };

        match cmd {
            "help" => {
                self.print_help();
            }
            "gpio" => {
                let _ = self.gpio_handler.execute(&mut tokens, context);
            }
            "sys" => {
                let _ = self.sys_handler.execute(&mut tokens, context);
            }
            "usb" => {
                util_zfmt::debug!("usb: not implemented yet");
            }
            "flash" => {
                util_zfmt::debug!("flash: not implemented yet");
            }
            _ => {
                util_zfmt::debug!("Unknown command. Type 'help' for available commands.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_iter_empty() {
        let mut iter = TokenIter::new("");
        assert_eq!(iter.next_token(), None);

        let mut iter2 = TokenIter::new("   ");
        assert_eq!(iter2.next_token(), None);
    }

    #[test]
    fn test_token_iter_single_token() {
        let mut iter = TokenIter::new("help");
        assert_eq!(iter.next_token(), Some("help"));
        assert_eq!(iter.next_token(), None);

        let mut iter2 = TokenIter::new("  help  ");
        assert_eq!(iter2.next_token(), Some("help"));
        assert_eq!(iter2.next_token(), None);
    }

    #[test]
    fn test_token_iter_multiple_tokens() {
        let mut iter = TokenIter::new("gpio   config  IOA2  in   none");
        assert_eq!(iter.next_token(), Some("gpio"));
        assert_eq!(iter.next_token(), Some("config"));
        assert_eq!(iter.next_token(), Some("IOA2"));
        assert_eq!(iter.next_token(), Some("in"));
        assert_eq!(iter.next_token(), Some("none"));
        assert_eq!(iter.next_token(), None);
    }
}
