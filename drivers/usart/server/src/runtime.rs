// Licensed under the Apache-2.0 license

use usart_api::backend::{IrqMask, UsartBackend};
use usart_api::{UsartResponseHeader};
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use crate::{DispatchOutcome, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, dispatch_request};

/// Holds at most one in-flight `TryRead` request that is waiting for the RX
/// interrupt to fire before it can be completed.
pub struct PendingRead {
    /// IPC channel handle of the waiting client, or `None` if idle.
    channel: Option<u32>,
    /// Max bytes the client asked for.
    requested_size: usize,
}

impl PendingRead {
    pub const fn new() -> Self {
        Self {
            channel: None,
            requested_size: 0,
        }
    }

    /// Park a `TryRead` request; the server will complete it on the next RX IRQ.
    ///
    /// Returns `false` if a read is already parked — UART RX is a single serial
    /// stream and cannot be multiplexed between concurrent consumers.  The
    /// caller should respond with `UsartError::Busy` in that case.
    pub fn park(&mut self, channel: u32, requested_size: usize) -> bool {
        if self.channel.is_some() {
            return false;
        }
        self.channel = Some(channel);
        self.requested_size = requested_size;
        true
    }

    /// Take the pending request, leaving the slot empty.
    pub fn take(&mut self) -> Option<(u32, usize)> {
        self.channel.take().map(|ch| (ch, self.requested_size))
    }

    pub fn is_pending(&self) -> bool {
        self.channel.is_some()
    }
}

impl Default for PendingRead {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the USART server dispatch loop forever.
///
/// The caller is responsible for populating `wg` ahead of time. The
/// convention this runtime relies on:
///
/// - For each IPC channel the binary serves, register it with its own
///   handle as `user_data`:
///   `wait_group_add(wg, ch, Signals::READABLE, ch as usize)`.
/// - Register the IRQ with `irq_signals` and `irq` as its `user_data`:
///   `wait_group_add(wg, irq, irq_signals, irq as usize)`.
///
/// The loop then routes wake-ups using `wait_return.user_data` directly:
/// the IRQ branch matches on the IRQ handle, every other wake-up is
/// treated as a channel and `user_data` is the channel handle to
/// read/respond on. This keeps the runtime topology-agnostic — adding
/// another client task is one more `wait_group_add` call in the binary.
pub fn run<B: UsartBackend>(backend: &mut B, wg: u32, irq: u32, irq_signals: Signals) -> ! {
    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut pending = PendingRead::new();

    let wait_mask = Signals::READABLE | irq_signals;

    loop {
        let Ok(wait_return) = syscall::object_wait(wg, wait_mask, Instant::MAX) else {
            continue;
        };

        if wait_return.user_data as u32 == irq
            && wait_return.pending_signals.contains(irq_signals)
        {
            let acked = wait_return.pending_signals & irq_signals;
            let _ = syscall::interrupt_ack(irq, acked);

            // If a TryRead is parked, complete it now that data may be ready.
            if let Some((client_channel, req_size)) = pending.take() {
                let payload_offset = UsartResponseHeader::SIZE;
                let payload_capacity = response_buf.len().saturating_sub(payload_offset);
                let read_buf_len = core::cmp::min(req_size, payload_capacity);

                // Disable the RX interrupt; it will be re-armed if the next
                // try_read also finds no data (unlikely at this point).
                let _ = backend.disable_interrupts(IrqMask::RX_DATA_AVAILABLE);

                let resp_len = match backend.try_read(
                    &mut response_buf[payload_offset..payload_offset + read_buf_len],
                ) {
                    Ok(n) => {
                        let hdr = UsartResponseHeader::success(n as u16);
                        response_buf[..UsartResponseHeader::SIZE]
                            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
                        UsartResponseHeader::SIZE + n
                    }
                    Err(e) => {
                        // Still not ready (shouldn't happen after IRQ, but handle gracefully).
                        // Re-arm and re-park with the original size.
                        let _ = backend.enable_interrupts(IrqMask::RX_DATA_AVAILABLE);
                        pending.park(client_channel, req_size);
                        let _ = e; // suppress
                        continue;
                    }
                };

                let _ = syscall::channel_respond(client_channel, &response_buf[..resp_len]);
            }
            continue;
        }

        if !wait_return.pending_signals.contains(Signals::READABLE) {
            continue;
        }

        let channel = wait_return.user_data as u32;
        let Ok(req_len) = syscall::channel_read(channel, 0, &mut request_buf) else {
            continue;
        };

        match dispatch_request(
            backend,
            &mut pending,
            channel,
            &request_buf[..req_len],
            &mut response_buf,
        ) {
            DispatchOutcome::Respond(resp_len) => {
                let _ = syscall::channel_respond(channel, &response_buf[..resp_len]);
            }
            DispatchOutcome::Queued => {
                // Response will be sent from the IRQ completion path above.
            }
        }
    }
}
