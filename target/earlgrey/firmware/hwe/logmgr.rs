// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_clock_domain::PERIPHERAL_CLOCK_HZ;
use logmgr_codegen::{handle, signals};
use pw_status::{Error, StatusCode};
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{process_entry, syscall};
use util_ipc::{IpcChannel, IpcHandle};
use util_zfmt::{render::render_event, FixedBuf, LogServer, StreamStart, ZfmtU64};
use zerocopy::IntoBytes;

use earlgrey_uart_driver::UartDriver;
use uart::Uart0;
use usart_api::backend::{BackendError, IrqMask, Parity, UsartBackend, UsartConfig};

// NOTE: logmgr is not permitted to perform logging via `zfmt`, as that would
// require logmgr to have a channel to itself.

#[derive(Clone, Copy, PartialEq, Eq)]
enum TxState {
    Idle,
    Body,
}

struct ActiveLog {
    buf: FixedBuf<512>,
    sent: usize,
    state: TxState,
}

impl ActiveLog {
    const fn new() -> Self {
        Self {
            buf: FixedBuf::new(),
            sent: 0,
            state: TxState::Idle,
        }
    }

    fn clear(&mut self) {
        self.buf.clear();
        self.sent = 0;
        self.state = TxState::Idle;
    }

    fn fill_from_event(&mut self, event: &[u8]) -> Option<usize> {
        self.clear();
        let consumed = render_event(event, &mut self.buf);
        if consumed.is_some() {
            self.state = TxState::Body;
            consumed
        } else {
            self.clear();
            None
        }
    }
}

fn service_uart_tx<const N: usize>(
    uart: &mut UartDriver,
    active_log: &mut ActiveLog,
    server: &LogServer<N>,
    uart_cursor: &mut u64,
) -> Result<(), Error> {
    loop {
        // 1. Transmit current buffer
        if active_log.state == TxState::Body {
            let data = active_log.buf.as_slice();
            // We maintain the invariant that active_log.sent <= data.len() and
            // should never get None. If we do, we halt transmission.
            let remaining = match data.get(active_log.sent..) {
                Some(r) if !r.is_empty() => r,
                _ => {
                    active_log.state = TxState::Idle;
                    break;
                }
            };
            match uart.write(remaining) {
                Ok(n) => {
                    active_log.sent += n;
                    if active_log.sent == data.len() {
                        active_log.state = TxState::Idle;
                    } else {
                        // Partial write, FIFO full. Enable interrupt and wait.
                        uart.enable_interrupts(IrqMask::TX_IDLE)
                            .map_err(|_| Error::Internal)?;
                        return Ok(());
                    }
                }
                Err(BackendError::WouldBlock) => {
                    uart.enable_interrupts(IrqMask::TX_IDLE)
                        .map_err(|_| Error::Internal)?;
                    return Ok(()); // Stop, wait for interrupt
                }
                Err(_) => return Err(Error::Internal),
            }
        }

        // 2. If active log is empty, try to load next one
        if active_log.state == TxState::Idle {
            uart.disable_interrupts(IrqMask::TX_IDLE)
                .map_err(|_| Error::Internal)?; // No more data to send for now

            let cursor = if *uart_cursor < server.buffer.read {
                server.buffer.read
            } else {
                *uart_cursor
            };
            *uart_cursor = cursor;

            if cursor < server.buffer.write {
                if let Some((_tag, s1, s2)) = server.buffer.next_frame_slice(cursor) {
                    let mut temp_frame = [0u8; 260];
                    let frame_len = s1.len() + s2.len();
                    if frame_len > temp_frame.len() {
                        *uart_cursor += frame_len as u64;
                        continue; // Skip too large frame
                    }
                    temp_frame[..s1.len()].copy_from_slice(s1);
                    temp_frame[s1.len()..frame_len].copy_from_slice(s2);

                    if let Some(consumed) = active_log.fill_from_event(&temp_frame[..frame_len]) {
                        *uart_cursor += consumed as u64;
                        // Loop again to transmit the newly loaded log
                        continue;
                    } else {
                        // Failed to render, skip
                        *uart_cursor += frame_len as u64;
                        continue;
                    }
                }
            }
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "cli")]
pub const CMDLINE_MAX_LINE_LEN: usize = 128;

#[cfg(feature = "cli")]
#[derive(Debug, PartialEq, Eq)]
pub struct CommandLineBuffer<const N: usize> {
    buf: [u8; N],
    len: usize,
    ready: bool,
}

#[cfg(feature = "cli")]
impl<const N: usize> CommandLineBuffer<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
            ready: false,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn push_char(&mut self, ch: u8) -> bool {
        if self.ready || self.len >= N {
            false
        } else {
            self.buf[self.len] = ch;
            self.len += 1;
            true
        }
    }

    pub fn pop_char(&mut self) -> bool {
        if self.ready || self.len == 0 {
            false
        } else {
            self.len -= 1;
            true
        }
    }

    pub fn finish_line(&mut self) -> bool {
        if self.ready {
            false
        } else {
            self.ready = true;
            true
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn is_blank(&self) -> bool {
        self.len == 0
            || self.buf[..self.len]
                .iter()
                .all(|&b| b == b' ' || b == b'\t')
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.ready = false;
    }
}

#[cfg(feature = "cli")]
fn process_input_char<const N: usize, F>(
    ch: u8,
    cmd_buf: &mut CommandLineBuffer<N>,
    cli_platform: &IpcHandle,
    last_was_cr: &mut bool,
    mut echo: F,
) where
    F: FnMut(&[u8]),
{
    if ch == b'\n' && *last_was_cr {
        *last_was_cr = false;
        return;
    }
    *last_was_cr = ch == b'\r';

    if ch == b'\r' || ch == b'\n' {
        if cmd_buf.is_blank() {
            cmd_buf.clear();
            echo(b"\r\nhwe> ");
        } else {
            echo(b"\r\n");
            cmd_buf.finish_line();
            let _ = cli_platform.set_peer_user_signal(true);
        }
    } else if ch == 0x08 || ch == 0x7f {
        if cmd_buf.pop_char() {
            echo(b"\x08 \x08");
        }
    } else if (0x20..0x7f).contains(&ch) {
        if cmd_buf.push_char(ch) {
            echo(&[ch]);
        }
    }
}

#[cfg(feature = "cli")]
fn uart_write_all(uart: &mut UartDriver, mut data: &[u8]) {
    while !data.is_empty() {
        match uart.write(data) {
            Ok(0) => continue,
            Ok(n) => data = &data[n..],
            Err(BackendError::WouldBlock) => continue,
            Err(_) => break,
        }
    }
}

#[cfg(feature = "cli")]
fn service_uart_rx(
    uart: &mut UartDriver,
    cmd_buf: &mut CommandLineBuffer<CMDLINE_MAX_LINE_LEN>,
    cli_platform: &IpcHandle,
    last_was_cr: &mut bool,
    rx_buf: &mut [u8],
) -> Result<(), Error> {
    loop {
        match uart.read(rx_buf) {
            Ok(0) => break,
            Ok(n) => {
                for &byte in &rx_buf[..n] {
                    process_input_char(byte, cmd_buf, cli_platform, last_was_cr, |echo_slice| {
                        uart_write_all(uart, echo_slice);
                    });
                }
            }
            Err(_) => break,
        }
    }
    uart.enable_interrupts(IrqMask::RX_DATA_AVAILABLE)
        .map_err(|_| Error::Internal)?;
    Ok(())
}

fn logmgr_server() -> Result<(), Error> {
    // UART0 physical address is mapped in our address space.
    // Since we use identity mapping for devices, we can use the physical address directly.
    let mut uart = unsafe { UartDriver::new(Uart0::PTR) };

    // Configure UART: 115200 baud, 8N1.
    uart.configure(UsartConfig {
        baud_rate: 0,
        parity: Parity::None,
        stop_bits: 1,
    })
    .map_err(|_| Error::Internal)?;

    #[cfg(feature = "cli")]
    uart.enable_interrupts(IrqMask::RX_DATA_AVAILABLE)
        .map_err(|_| Error::Internal)?;

    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::LOGGER_USB,
        Signals::READABLE,
        handle::LOGGER_USB as usize,
    )?;
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::LOGGER_PLATFORM,
        Signals::READABLE,
        handle::LOGGER_PLATFORM as usize,
    )?;
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::LOGGER_FLASH,
        Signals::READABLE,
        handle::LOGGER_FLASH as usize,
    )?;
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::LOGGER_SYSMGR,
        Signals::READABLE,
        handle::LOGGER_SYSMGR as usize,
    )?;
    #[cfg(feature = "cli")]
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::CLI_PLATFORM,
        Signals::READABLE,
        handle::CLI_PLATFORM as usize,
    )?;
    #[cfg(feature = "cli")]
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::CLI_USB,
        Signals::READABLE,
        handle::CLI_USB as usize,
    )?;
    #[cfg(feature = "cli")]
    let uart0_irq_signals =
        signals::UART0_TX_DONE | signals::UART0_RX_WATERMARK | signals::UART0_RX_TIMEOUT;
    #[cfg(not(feature = "cli"))]
    let uart0_irq_signals = signals::UART0_TX_DONE;

    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::UART0_INTERRUPTS,
        uart0_irq_signals,
        handle::UART0_INTERRUPTS as usize,
    )?;

    let mut server = LogServer::<2048>::new();
    let mut active_log = ActiveLog::new();
    let mut uart_cursor = 0u64;
    #[cfg(feature = "cli")]
    let mut cmd_buf_uart = CommandLineBuffer::<CMDLINE_MAX_LINE_LEN>::new();
    #[cfg(feature = "cli")]
    let mut cmd_buf_usb = CommandLineBuffer::<CMDLINE_MAX_LINE_LEN>::new();
    #[cfg(feature = "cli")]
    let cli_platform_handle = IpcHandle::new(handle::CLI_PLATFORM);
    #[cfg(feature = "cli")]
    let mut last_was_cr_uart = false;
    #[cfg(feature = "cli")]
    let mut last_was_cr_usb = false;

    // Log StreamStart event to the buffer on startup.
    let ss = StreamStart {
        protocol_version: StreamStart::PROTOCOL_VERSION,
        _pad0: [0; 2],
        tick_rate_hz: ZfmtU64::from_u64(PERIPHERAL_CLOCK_HZ),
        firmware_build_id: ZfmtU64::from_u64(0),
    };
    let mut ss_frame = [0u8; 4 + 1 + 20]; // tag(4) + len(1) + payload(20)
    ss_frame[0..4].copy_from_slice(&StreamStart::ZFMT_TAG.to_le_bytes());
    ss_frame[4] = 20; // len
    ss.serialize_into(&mut ss_frame[5..]);
    server.buffer.push_frame(&ss_frame);

    // Kick off UART transmission.
    if let Err(e) = service_uart_tx(&mut uart, &mut active_log, &server, &mut uart_cursor) {
        pw_log::error!("Failed to kick off UART: {}", e as u32);
    }

    let mut req = [0u8; 260];

    loop {
        let wait_result =
            syscall::object_wait(handle::LOGMGR_WAIT_GROUP, Signals::READABLE, Instant::MAX)?;
        let active_handle = wait_result.user_data as u32;

        if active_handle == handle::UART0_INTERRUPTS {
            #[cfg(feature = "cli")]
            if (wait_result.pending_signals
                & (signals::UART0_RX_WATERMARK | signals::UART0_RX_TIMEOUT))
                != Signals::empty()
            {
                service_uart_rx(
                    &mut uart,
                    &mut cmd_buf_uart,
                    &cli_platform_handle,
                    &mut last_was_cr_uart,
                    &mut req[..32],
                )?;
            }
            if (wait_result.pending_signals & signals::UART0_TX_DONE) != Signals::empty() {
                uart.enable_interrupts(IrqMask::TX_IDLE)
                    .map_err(|_| Error::Internal)?;
                service_uart_tx(&mut uart, &mut active_log, &server, &mut uart_cursor)?;
            }
            let _ = syscall::interrupt_ack(handle::UART0_INTERRUPTS, wait_result.pending_signals);
            continue;
        }

        #[cfg(feature = "cli")]
        if active_handle == handle::CLI_PLATFORM {
            let channel = IpcHandle::new(handle::CLI_PLATFORM);
            let n = channel.read(0, &mut req)?;
            if n > 0 && &req[..n] == b"DONE" {
                channel.respond(&[0u8; 0])?;
                service_uart_tx(&mut uart, &mut active_log, &server, &mut uart_cursor)?;
                continue;
            }

            if cmd_buf_uart.is_ready() {
                let line = cmd_buf_uart.as_bytes();
                channel.respond(line)?;
                cmd_buf_uart.clear();
            } else if cmd_buf_usb.is_ready() {
                let line = cmd_buf_usb.as_bytes();
                channel.respond(line)?;
                cmd_buf_usb.clear();
            } else {
                channel.respond(&[0u8; 0])?;
            }
            if !cmd_buf_uart.is_ready() && !cmd_buf_usb.is_ready() {
                let _ = cli_platform_handle.set_peer_user_signal(false);
            }
            continue;
        }

        #[cfg(feature = "cli")]
        if active_handle == handle::CLI_USB {
            let channel = IpcHandle::new(handle::CLI_USB);
            // Split `req` [260 bytes] into:
            // - `echo_buf` (196 bytes): Destination for cooked echo characters.
            // - `input_buf` (64 bytes): Source for raw input characters (matches USB FS max packet size).
            //
            // Worst-case character expansion occurs on backspace ('\x08' or '\x7f' [1 byte] ->
            // "\x08 \x08" [3 bytes]) to visually erase characters on standard VT100/ANSI terminals.
            // With 64 bytes input, maximum echo is 64 * 3 = 192 bytes <= 196 bytes, guaranteeing
            // that `echo_buf` will never overflow or drop echo characters.
            const MAX_INPUT: usize = 64;
            let mid = req.len() - MAX_INPUT;
            let (echo_buf, input_buf) = req.split_at_mut(mid);
            let n = channel.read(0, input_buf)?;
            let input_slice = &input_buf[..n.min(input_buf.len())];

            let mut echo_len = 0;
            for &byte in input_slice {
                process_input_char(
                    byte,
                    &mut cmd_buf_usb,
                    &cli_platform_handle,
                    &mut last_was_cr_usb,
                    |echo_slice| {
                        if echo_len + echo_slice.len() <= echo_buf.len() {
                            echo_buf[echo_len..echo_len + echo_slice.len()]
                                .copy_from_slice(echo_slice);
                            echo_len += echo_slice.len();
                        }
                    },
                );
            }
            let _ = channel.respond(&echo_buf[..echo_len]);
            continue;
        }

        // IPC request from a logger client
        let channel = IpcHandle::new(active_handle);
        let n = channel.read(0, &mut req)?;
        let n = n.min(req.len());
        let raise = match server.handle_request(&channel, &mut req[..n]) {
            Ok(processed) => processed,
            Err(e) => {
                channel.respond(e.as_bytes())?;
                false
            }
        };
        if raise {
            // Try to service TX (load new logs if idle)
            service_uart_tx(&mut uart, &mut active_log, &server, &mut uart_cursor)?;
            // Signal the USB task that there are logs available.
            let _ = syscall::object_set_peer_user_signal(handle::LOGGER_USB, raise);
        }
    }
}

#[process_entry("logmgr")]
fn entry() -> Result<(), Error> {
    let ret = logmgr_server();
    pw_log::error!("logmgr status = {}", ret.status_code());
    let _ = syscall::debug_shutdown(ret);
    ret
}
