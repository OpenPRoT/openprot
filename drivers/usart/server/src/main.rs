// Licensed under the Apache-2.0 license

#![no_main]
#![no_std]

use app_usart_server::{handle, signals};
use usart_api::backend::UsartBackend;
use usart_backend::Backend;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

// IER bits exposed by the backend's interrupt mask convention
// (see backend/usart/src/lib.rs):
//   bit 0 = RX data available (ERBFI)
//   bit 1 = TX idle / THRE     (ETBEI)
const IRQ_MASK_TX_IDLE: u16 = 0x02;

#[entry]
fn entry() -> ! {
    let mut backend = Backend::new();
    let mut request_buf = [0u8; usart_server::MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; usart_server::MAX_RESPONSE_SIZE];

    let _ = syscall::wait_group_add(handle::WG, handle::USART, Signals::READABLE, 0);
    let _ = syscall::wait_group_add(handle::WG, handle::UART5_IRQ, signals::UART, 1);

    loop {
        let Ok(wait_return) = syscall::object_wait(
            handle::WG,
            Signals::READABLE | signals::UART,
            Instant::MAX,
        ) else {
            continue;
        };

        if wait_return.user_data == 1 && wait_return.pending_signals.contains(signals::UART) {
            let irq_signals = wait_return.pending_signals & signals::UART;
            // Mask the THRE source at the device. Usart::new enables ETBEI,
            // which keeps THRE asserting after every TX completes — without
            // this, kernel re-dispatch loops forever on a perpetually
            // pending IRQ. Clients that want TX-completion notifications
            // re-enable ETBEI via the EnableInterrupts opcode.
            let _ = backend.disable_interrupts(IRQ_MASK_TX_IDLE);
            let _ = syscall::interrupt_ack(handle::UART5_IRQ, irq_signals);
            continue;
        }

        if wait_return.user_data != 0 || !wait_return.pending_signals.contains(Signals::READABLE) {
            continue;
        }

        let Ok(req_len) = syscall::channel_read(handle::USART, 0, &mut request_buf) else {
            continue;
        };

        let resp_len = usart_server::dispatch_request(
            &mut backend,
            &request_buf[..req_len],
            &mut response_buf,
        );
        let _ = syscall::channel_respond(handle::USART, &response_buf[..resp_len]);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
