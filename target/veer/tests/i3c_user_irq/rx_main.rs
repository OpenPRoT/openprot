// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C userspace-IRQ app.
//!
//! Proves the full userspace interrupt path on VeeR: a `type: "interrupt"`
//! object (I3C = PIC IRQ 2) delivers the TTI RX-descriptor interrupt to this
//! userspace process via a WaitGroup, the process drains the frame through the
//! [`I3cTarget`] facade, and `interrupt_ack` re-enables the level-triggered
//! source. No hardware register is polled to discover the frame — the IRQ is
//! the only path from arrival to reception.

#![no_main]
#![no_std]

use app_i3c_user_irq::{handle, signals};
use caliptra_i3c_target::CaliptraI3cTarget;
use openprot_hal_blocking::i3c_hardware::{I3cTarget, TargetEvent};
use userspace::entry;
use userspace::syscall;
use userspace::syscall::Signals;
use userspace::time::Instant;

/// The host sends exactly this private-write payload.
const EXPECTED: &[u8] = &[0x01, 0x02, 0x03, 0x04];

macro_rules! fail {
    ($msg:literal) => {{
        pw_log::error!($msg);
        let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
        loop {}
    }};
}

#[entry]
fn entry() {
    // SAFETY: this process exclusively owns the I3C peripheral, mapped as a
    // device region by system.json5; Caliptra ROM already initialized the core.
    let mut i3c = unsafe { CaliptraI3cTarget::new() };
    if i3c.enable().is_err() {
        fail!("i3c enable failed");
    }

    if syscall::wait_group_add(handle::WG, handle::I3C_IRQ, signals::I3C, 0).is_err() {
        fail!("wait_group_add failed");
    }

    pw_log::info!("i3c user-irq: waiting for private write");

    let mut buf = [0u8; 64];
    loop {
        let w = match syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX) {
            Ok(w) => w,
            Err(_) => fail!("object_wait failed"),
        };
        if !w.pending_signals.contains(signals::I3C) {
            continue;
        }

        let event = i3c.on_interrupt();
        let acked = w.pending_signals & signals::I3C;
        match event {
            Ok(TargetEvent::InboundReady) => {
                let frame = i3c.read_frame(&mut buf);
                let _ = syscall::interrupt_ack(handle::I3C_IRQ, acked);
                match frame {
                    Ok(Some(n)) => {
                        if n >= EXPECTED.len() && &buf[..EXPECTED.len()] == EXPECTED {
                            pw_log::info!("i3c user-irq: received expected payload");
                            let _ = syscall::debug_shutdown(Ok(()));
                            loop {}
                        }
                        pw_log::error!("i3c user-irq: payload mismatch len={}", n as u32);
                        let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
                        loop {}
                    }
                    // Spurious wake with no descriptor: keep waiting.
                    Ok(None) => {}
                    Err(_) => fail!("read_frame failed"),
                }
            }
            Ok(_) => {
                let _ = syscall::interrupt_ack(handle::I3C_IRQ, acked);
            }
            Err(_) => {
                let _ = syscall::interrupt_ack(handle::I3C_IRQ, acked);
                fail!("on_interrupt failed");
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
