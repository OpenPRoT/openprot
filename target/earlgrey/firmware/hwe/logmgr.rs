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
use util_zfmt::{render::render_event, FixedBuf, LogServer, StreamStart, Write, ZfmtU64};
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
    Newline,
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
            //pw_log::info!("rendered: {}", self.buf.as_str() as &str);
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
        // 1. Transmit current buffer (body or newline)
        if active_log.state == TxState::Body || active_log.state == TxState::Newline {
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
                        if active_log.state == TxState::Body {
                            // Body sent, load newline into buffer
                            active_log.buf.clear();
                            let _ = active_log.buf.write_str("\r\n");
                            active_log.sent = 0;
                            active_log.state = TxState::Newline;
                            continue; // Loop again to send newline
                        } else {
                            active_log.state = TxState::Idle;
                        }
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
            // No more logs to load
            break;
        }
    }
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
    // Add UART interrupt to wait group
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::UART0_INTERRUPTS,
        signals::UART0_TX_DONE,
        handle::UART0_INTERRUPTS as usize,
    )?;
    syscall::wait_group_add(
        handle::LOGMGR_WAIT_GROUP,
        handle::LOGGER_SYSMGR,
        Signals::READABLE,
        handle::LOGGER_SYSMGR as usize,
    )?;

    let mut server = LogServer::<2048>::new();
    let mut active_log = ActiveLog::new();
    let mut uart_cursor = 0u64;

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
            // Clear interrupt by re-enabling it (our implementation clears on enable)
            uart.enable_interrupts(IrqMask::TX_IDLE)
                .map_err(|_| Error::Internal)?;
            service_uart_tx(&mut uart, &mut active_log, &server, &mut uart_cursor)?;
            let _ = syscall::interrupt_ack(handle::UART0_INTERRUPTS, wait_result.pending_signals);
        } else {
            // IPC request
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
}

#[process_entry("logmgr")]
fn entry() -> Result<(), Error> {
    let ret = logmgr_server();
    pw_log::error!("logmgr status = {}", ret.status_code());
    let _ = syscall::debug_shutdown(ret);
    ret
}
