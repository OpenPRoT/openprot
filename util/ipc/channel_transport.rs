// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `util_service::AsyncTransport` over a kernel channel.
//!
//! `AsyncTransaction` lends the kernel `'static` buffers for the duration of
//! a transaction, which a caller holding an ordinary `&[u8]` request cannot
//! satisfy. This type owns that pair of buffers, copies each request in and
//! each response out, and hands callers the plain-slice seam every service
//! shares.
//!
//! Buffer sizes are the wiring's choice: a request longer than the send
//! buffer, or a response longer than the caller's, is `TooLarge` rather than
//! a truncated frame.

use util_service::{AsyncTransport, TransportError};

use super::async_transaction::{AsyncTransaction, Buffers};
use super::IpcInitiator;

/// One channel's worth of async round-trip, with the buffers it lends the
/// kernel.
pub struct AsyncChannelTransport<H: IpcInitiator> {
    txn: AsyncTransaction<H>,
    /// Held while idle, lent to `txn` while a round-trip is in flight.
    idle: Option<Buffers>,
}

impl<H: IpcInitiator> AsyncChannelTransport<H> {
    /// Wrap an initiator with the buffers it lends the kernel. `send` must
    /// fit the largest request this channel carries, `recv` the largest
    /// response.
    pub fn new(handle: H, send: &'static mut [u8], recv: &'static mut [u8]) -> Self {
        Self {
            txn: AsyncTransaction::new(handle),
            idle: Some(Buffers { send, recv }),
        }
    }

    /// The raw channel handle, to register with a WaitGroup so the event
    /// loop wakes when the response lands.
    pub fn as_raw(&self) -> u32 {
        self.txn.as_raw()
    }

    /// Whether a round-trip is in flight.
    pub fn is_pending(&self) -> bool {
        self.txn.is_pending()
    }
}

impl<H: IpcInitiator> AsyncTransport for AsyncChannelTransport<H> {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError> {
        let Some(buffers) = self.idle.take() else {
            return Err(TransportError::WrongState);
        };
        let Buffers { send, recv } = buffers;

        if req.len() > send.len() {
            self.idle = Some(Buffers { send, recv });
            return Err(TransportError::TooLarge);
        }
        send[..req.len()].copy_from_slice(req);

        match self.txn.start(send, req.len(), recv) {
            Ok(()) => Ok(()),
            Err(e) => {
                // The buffers came back, so the next start can reuse them.
                self.idle = Some(Buffers {
                    send: e.send,
                    recv: e.recv,
                });
                Err(TransportError::Failed)
            }
        }
    }

    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError> {
        if self.idle.is_some() {
            return Err(TransportError::WrongState);
        }

        match self.txn.try_recv() {
            Ok(None) => Ok(None),
            Ok(Some(completion)) => {
                let len = completion.len;
                let too_large = len > resp.len();
                if !too_large {
                    resp[..len].copy_from_slice(&completion.recv[..len]);
                }
                self.idle = Some(Buffers {
                    send: completion.send,
                    recv: completion.recv,
                });
                if too_large {
                    // recv goes back to idle without being copied out, so
                    // the response is gone. The next call is start.
                    return Err(TransportError::TooLarge);
                }
                Ok(Some(len))
            }
            Err(e) => {
                // try_recv hands the buffers out on every failure, so the
                // channel is idle again and the next call is start.
                self.idle = e.buffers;
                Err(TransportError::Failed)
            }
        }
    }

    fn cancel(&mut self) -> Result<(), TransportError> {
        if self.idle.is_some() {
            return Err(TransportError::WrongState);
        }
        match self.txn.cancel() {
            Ok(buffers) => {
                self.idle = Some(buffers);
                Ok(())
            }
            Err(_) => Err(TransportError::Failed),
        }
    }
}
