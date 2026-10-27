// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Safe async IPC transaction wrapper, layered on `IpcInitiator`.
//!
//! `AsyncTransaction` wraps the unsafe `async_transact_start`/`_complete`/
//! `_cancel` trio into a state machine (Idle / Pending) that enforces the
//! kernel's one-transaction-per-channel rule locally and keeps the
//! raw-pointer safety contract in one place.
//!
//! Buffers are `&'static` because the kernel holds raw pointers into them
//! for the duration of the transaction. If `AsyncTransaction` owned the buffers
//! inline and the struct moved after `start()`, those pointers would
//! dangle. Static borrows make soundness independent of moves and
//! `mem::forget`.
//!
//! Every exit from Pending returns both buffers to the caller so they can
//! be reused for the next transaction (the standard embedded-DMA ownership
//! pattern).

use super::IpcInitiator;
use pw_status::{Error, Result};

/// Buffers lent to the kernel for the duration of one async transaction.
#[derive(Debug)]
pub struct Buffers {
    pub send: &'static [u8],
    pub recv: &'static mut [u8],
}

/// One-at-a-time async IPC transaction on an `IpcInitiator`.
///
/// Transitions: Idle -> `start()` -> Pending -> `try_recv()`/`cancel()` -> Idle.
pub struct AsyncTransaction<H: IpcInitiator> {
    handle: H,
    inflight: Option<Buffers>,
}

impl<H: IpcInitiator> AsyncTransaction<H> {
    /// Wrap an initiator handle with no outstanding async transaction.
    pub fn new(handle: H) -> Self {
        Self {
            handle,
            inflight: None,
        }
    }

    /// The raw channel handle, e.g. to register with a WaitGroup or pass to
    /// `object_wait`.
    pub fn as_raw(&self) -> u32 {
        self.handle.as_raw()
    }

    /// Whether a transaction is in flight.
    pub fn is_pending(&self) -> bool {
        self.inflight.is_some()
    }

    /// Start an async transaction.
    ///
    /// `send` is the request payload the server will read. `recv` is the
    /// buffer the kernel writes the server's response into. Both must be
    /// `'static` because the kernel holds raw pointers into them until
    /// `try_recv` or `cancel` completes the transaction.
    ///
    /// On success the buffers are held until `try_recv` or `cancel`
    /// returns them. On failure (already pending, or a kernel error) both
    /// buffers come back in the `Err` so nothing is lost.
    pub fn start(
        &mut self,
        send: &'static [u8],
        recv: &'static mut [u8],
    ) -> core::result::Result<(), StartError> {
        if self.inflight.is_some() {
            return Err(StartError {
                error: Error::FailedPrecondition,
                buffers: Buffers { send, recv },
            });
        }

        // Safety: send/recv are 'static, so the kernel's raw pointers
        // stay valid regardless of what happens to `self`, and they are
        // not read, written, or dropped again until try_recv/cancel.
        // nosemgrep
        let result = unsafe { self.handle.async_transact_start(send, recv) };

        match result {
            Ok(()) => {
                self.inflight = Some(Buffers { send, recv });
                Ok(())
            }
            Err(error) => Err(StartError {
                error,
                buffers: Buffers { send, recv },
            }),
        }
    }

    /// Try to complete a pending transaction.
    ///
    /// Returns `Ok(Completion { len, send, recv })` when the server has
    /// responded. `recv[..len]` holds the response payload; the full
    /// buffer is returned so it can be reused.
    ///
    /// Returns `Err(Error::Unavailable)` if READABLE is not set (server
    /// has not responded yet); the buffers stay held and another
    /// `try_recv` is expected after the next READABLE signal.
    ///
    /// Returns `Err(Error::FailedPrecondition)` if no transaction is
    /// pending. Any other kernel error leaves the transaction pending;
    /// use `cancel()` to reclaim the buffers.
    pub fn try_recv(&mut self) -> Result<Completion> {
        if self.inflight.is_none() {
            return Err(Error::FailedPrecondition);
        }

        let len = self.handle.async_transact_complete()?;
        let buffers = self.inflight.take().unwrap();
        Ok(Completion { len, buffers })
    }

    /// Cancel a pending transaction and reclaim the buffers.
    ///
    /// If the server has already responded, the response is silently
    /// discarded.
    ///
    /// Returns `Err(Error::FailedPrecondition)` if no transaction is
    /// pending. The buffers come back even if the cancel syscall errors: the
    /// kernel clears its transaction slot on every path, so it no longer
    /// holds pointers into them.
    pub fn cancel(&mut self) -> Result<Buffers> {
        let Some(buffers) = self.inflight.take() else {
            return Err(Error::FailedPrecondition);
        };

        // Only failure for a started transaction is Unavailable, meaning the
        // transaction was already dropped. The buffers are free either way.
        let _ = self.handle.async_cancel();
        Ok(buffers)
    }
}

impl<H: IpcInitiator> Drop for AsyncTransaction<H> {
    fn drop(&mut self) {
        if self.inflight.is_some() {
            let _ = self.handle.async_cancel();
        }
    }
}

/// Successful completion of an async transaction.
#[derive(Debug)]
pub struct Completion {
    /// Number of response bytes written into `buffers.recv`.
    pub len: usize,
    /// The buffers, returned for reuse. `recv[..len]` holds the response
    /// payload.
    pub buffers: Buffers,
}

/// Error from `start()`, carrying the buffers back so they are not lost.
#[derive(Debug)]
pub struct StartError {
    pub error: Error,
    pub buffers: Buffers,
}

impl From<StartError> for Error {
    /// Drops the reclaimed buffers; use `StartError` directly to reuse them.
    fn from(e: StartError) -> Self {
        e.error
    }
}
