// Licensed under the Apache-2.0 license

use usart_api::backend::UsartBackend;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use crate::{MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, dispatch_request};

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
            continue;
        }

        if !wait_return.pending_signals.contains(Signals::READABLE) {
            continue;
        }

        let channel = wait_return.user_data as u32;
        let Ok(req_len) = syscall::channel_read(channel, 0, &mut request_buf) else {
            continue;
        };

        let resp_len = dispatch_request(backend, &request_buf[..req_len], &mut response_buf);
        let _ = syscall::channel_respond(channel, &response_buf[..resp_len]);
    }
}
