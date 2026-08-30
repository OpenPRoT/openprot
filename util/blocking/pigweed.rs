use userspace::syscall;
use userspace::time::Instant;

use crate::Blocking;
use crate::BlockingAckToken;

/// A struct for blocking on interrupts.
///
/// This struct allows threads to block until a particular interrupt occurs.
/// The interrupt will be ack'ed (thereby allowing new interrupt requests to be
/// latched) when the struct returned from `wait_for_notification()` is dropped.
pub struct BlockingInterrupt {
    pub handle: u32,
    pub signals: syscall::Signals,
}

impl Blocking for BlockingInterrupt {
    fn wait_for_notification(&self) -> impl BlockingAckToken {
        loop {
            if let Ok(w) = syscall::object_wait(
                self.handle,
                self.signals,
                Instant::MAX,
            ) {
                if w.pending_signals.contains(self.signals) {
                    return InterruptAckToken {
                        handle: self.handle,
                        signals: w.pending_signals,
                    };
                }
            }
        }
    }
}

/// A struct for ack'ing an interrupt with the PLIC, when the struct is dropped.
struct InterruptAckToken {
    pub handle: u32,
    pub signals: syscall::Signals,
}

impl BlockingAckToken for InterruptAckToken {}

impl Drop for InterruptAckToken {
    fn drop(&mut self) {
        let _ = syscall::interrupt_ack(self.handle, self.signals);
    }
}
