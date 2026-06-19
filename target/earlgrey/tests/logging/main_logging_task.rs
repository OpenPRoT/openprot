// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use main_logging_task_codegen::handle;
use pw_status::{Error, Result};
use registers::rv_timer::RvTimer;
use userspace::syscall::Signals;
use userspace::time::Instant;
use userspace::{entry, syscall};
use zfmt::events::{DebugMessage, EventHeader, StreamStart};
use zfmt::{leb128, FixedBuf, Write};
use zfmt::{log_bare_event, log_info, FlatAdapter, FlatSend, ZfmtU64};

// TODO: enhance this test to support both text output (current) and
// binary output with the `zfmt-host` tools (todo).

#[inline(always)]
fn rv_timer_value(rv_timer: &RvTimer) -> u64 {
    let regs = rv_timer.regs();
    loop {
        let hi1 = regs.timer_v_upper0().read();
        let low = regs.timer_v_lower0().read();
        let hi2 = regs.timer_v_upper0().read();
        if hi1 == hi2 {
            return ((hi1 as u64) << 32) | (low as u64);
        }
    }
}

#[derive(zfmt::Zfmt)]
#[zfmt(format = "main_event count={count}")]
pub struct MainEvent {
    pub count: u32,
}

// --- Binary stream parser helpers ---

struct Frame<'a> {
    tag: u32,
    payload: &'a [u8],
}

fn next_frame<'a>(buf: &mut &'a [u8]) -> Option<Frame<'a>> {
    if buf.len() < 4 {
        return None;
    }
    let tag = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
    *buf = &buf[4..];

    let (len, consumed) = leb128::decode(buf)?;
    *buf = buf.get(consumed..)?;

    let len = len as usize;
    if buf.len() < len {
        return None;
    }
    let payload = &buf[..len];
    *buf = &buf[len..];

    Some(Frame { tag, payload })
}

fn parse_and_log_binary_stream(mut data: &[u8]) {
    let mut pending_header: Option<FixedBuf<64>> = None;

    while let Some(frame) = next_frame(&mut data) {
        match frame.tag {
            StreamStart::ZFMT_TAG => {
                let _ = syscall::debug_log(b"[Stream Start]\n");
            }
            EventHeader::ZFMT_TAG => {
                if frame.payload.len() == core::mem::size_of::<EventHeader>() {
                    let hdr = unsafe { &*(frame.payload.as_ptr() as *const EventHeader) };
                    let mut header_buf = FixedBuf::<64>::new();
                    if hdr.format_into(&mut header_buf).is_ok() {
                        pending_header = Some(header_buf);
                    }
                }
            }
            DebugMessage::ZFMT_TAG => {
                if let Some(header_buf) = pending_header.take() {
                    let payload = frame.payload;
                    if let Some((msg_len, consumed)) = leb128::decode(payload) {
                        let msg_len = msg_len as usize;
                        if let Some(msg_bytes) = payload.get(consumed..consumed + msg_len) {
                            if let Ok(msg_str) = core::str::from_utf8(msg_bytes) {
                                let mut final_buf = FixedBuf::<256>::new();
                                let _ = final_buf.write_str(header_buf.as_str());
                                let _ = final_buf.write_str(" ");
                                let _ = final_buf.write_str(msg_str);
                                let _ = final_buf.write_str("\n");
                                let _ = syscall::debug_log(final_buf.as_str().as_bytes());
                            }
                        }
                    }
                }
            }
            _ => {
                pw_log::warn!("Unknown tag in stream: 0x{:08x}", frame.tag);
            }
        }
    }
}

// --- Logger implementation ---

struct KernelDebugLogger {
    rv_timer: RvTimer,
}

impl FlatSend for KernelDebugLogger {
    fn timestamp(&self) -> ZfmtU64 {
        ZfmtU64::from_u64(rv_timer_value(&self.rv_timer))
    }
    fn send(&self, data: &[u8]) {
        parse_and_log_binary_stream(data);
    }
}

static LOGGER: FlatAdapter<KernelDebugLogger, 256> = FlatAdapter::new(KernelDebugLogger {
    rv_timer: unsafe { RvTimer::new() },
});

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("Main logging task started");

    log_bare_event!(
        LOGGER,
        StreamStart {
            protocol_version: StreamStart::PROTOCOL_VERSION,
            _pad0: [0; 2],
            tick_rate_hz: ZfmtU64::from_u64(1_000_000),
            firmware_build_id: ZfmtU64::from_u64(0),
        }
    );

    log_info!(LOGGER, MainEvent { count: 0 });

    let mut count = 1;
    let mut next_log_time = syscall::debug_clock_now().ticks() + 1_000_000;

    let mut ipc_buffer = [0u8; 256];
    let mut ipc_events_received = 0;

    while ipc_events_received < 5 {
        let deadline = Instant::from_ticks(next_log_time);
        let wait_result = syscall::object_wait(handle::IPC, Signals::READABLE, deadline);

        match wait_result {
            Ok(wait_return) => {
                if wait_return.pending_signals.contains(Signals::READABLE) {
                    match syscall::channel_read(handle::IPC, 0, &mut ipc_buffer) {
                        Ok(len) => {
                            let len = len.min(ipc_buffer.len());
                            parse_and_log_binary_stream(&ipc_buffer[..len]);
                            ipc_events_received += 1;
                        }
                        Err(e) => {
                            pw_log::error!("Failed to read from IPC: {}", e as u32);
                        }
                    }
                    let resp_res = syscall::channel_respond(handle::IPC, &ipc_buffer[..0]);
                    if let Err(e) = resp_res {
                        pw_log::error!("channel_respond failed: {}", e as u32);
                    }
                }
            }
            Err(Error::DeadlineExceeded) => {
                log_info!(LOGGER, MainEvent { count });
                count += 1;
                next_log_time = syscall::debug_clock_now().ticks() + 1_000_000;
            }
            Err(e) => {
                pw_log::error!("Wait failed: {}", e as u32);
                return Err(e);
            }
        }
    }

    pw_log::info!("Main logging task finished");
    let _ = syscall::debug_shutdown(Ok(()));
    Ok(())
}

util_panic::make_panic_handler!();
