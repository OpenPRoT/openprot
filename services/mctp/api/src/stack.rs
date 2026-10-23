// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! High-level MCTP stack facade
//!
//! Bridges any [`MctpClient`] implementation to the abstract
//! [`MctpListener`], [`MctpReqChannel`], and [`MctpRespChannel`] traits,
//! hiding both the concrete MCTP stack implementation and the underlying
//! OS / transport mechanism.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use openprot_mctp_client_ipc::IpcMctpClient;
//! use openprot_mctp_api::stack::Stack;
//! use openprot_mctp_api::{MctpListener, MctpReqChannel, MctpRespChannel};
//!
//! let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
//! stack.set_eid(8).unwrap();
//!
//! // Server side: receive a request and reply
//! let mut listener = stack.listener(MSG_TYPE_SPDM, 0).unwrap();
//! let (meta, payload, mut resp) = listener.recv(&mut buf).unwrap();
//! resp.send(&reply).unwrap();
//!
//! // Client side: send a request and receive the response
//! let mut req = stack.req(remote_eid, 0).unwrap();
//! req.send(MSG_TYPE_SPDM, &msg).unwrap();
//! let (meta, response) = req.recv(&mut buf).unwrap();
//! ```

use core::cell::Cell;

use crate::traits::{MctpListener, MctpReqChannel, MctpRespChannel};
use crate::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};

// ============================================================================
// Stack
// ============================================================================

/// An MCTP stack facade backed by any [`MctpClient`] implementation.
///
/// `Stack` is the entry point for application code. It wraps a concrete
/// `MctpClient` and returns typed channel handles whose methods implement
/// the standard MCTP traits. Applications only depend on those traits;
/// the underlying stack implementation and OS transport are invisible.
pub struct Stack<C: MctpClient> {
    client: C,
    /// A request handle kept between [`Stack::req_reusable`] channels, so a
    /// caller that sends many requests to one EID does not allocate and unbind
    /// a handle (two IPC round trips) per request.
    cached_req: Cell<Option<(u8, Handle)>>,
}

impl<C: MctpClient> Stack<C> {
    /// Create a new stack backed by the given `MctpClient`.
    pub fn new(client: C) -> Self {
        Stack {
            client,
            cached_req: Cell::new(None),
        }
    }

    /// Get the local endpoint ID.
    pub fn get_eid(&self) -> u8 {
        self.client.get_eid()
    }

    /// Set the local endpoint ID.
    pub fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        self.client.set_eid(eid)
    }

    /// Open an outbound request channel to `eid`.
    ///
    /// `timeout_millis` of 0 means no timeout (block indefinitely).
    pub fn req(&self, eid: u8, timeout_millis: u32) -> Result<StackReqChannel<'_, C>, MctpError> {
        let handle = self.client.req(eid)?;
        Ok(StackReqChannel {
            stack: self,
            handle,
            eid,
            sent_tag: None,
            timeout: timeout_millis,
            reuse: false,
            healthy: true,
        })
    }

    /// Like [`req`](Self::req), but the request handle is kept for the next
    /// call instead of being released when the channel is dropped.
    ///
    /// The handle is only kept if the channel completed cleanly: every `send`
    /// was answered by a `recv` and neither returned an error. After an error
    /// (for example a receive timeout, which can leave a receive pending in the
    /// server) the handle is released as with [`req`](Self::req). A cached
    /// handle for a different `eid` is released and replaced.
    pub fn req_reusable(
        &self,
        eid: u8,
        timeout_millis: u32,
    ) -> Result<StackReqChannel<'_, C>, MctpError> {
        let handle = match self.cached_req.take() {
            Some((cached_eid, handle)) if cached_eid == eid => handle,
            stale => {
                if let Some((_, handle)) = stale {
                    self.client.drop_handle(handle);
                }
                self.client.req(eid)?
            }
        };
        Ok(StackReqChannel {
            stack: self,
            handle,
            eid,
            sent_tag: None,
            timeout: timeout_millis,
            reuse: true,
            healthy: true,
        })
    }

    /// Register a listener for incoming messages of the given MCTP type.
    ///
    /// `timeout_millis` of 0 means no timeout (block indefinitely).
    pub fn listener(
        &self,
        msg_type: u8,
        timeout_millis: u32,
    ) -> Result<StackListener<'_, C>, MctpError> {
        let handle = self.client.listener(msg_type)?;
        Ok(StackListener {
            stack: self,
            handle,
            timeout: timeout_millis,
        })
    }
}

// ============================================================================
// Request channel
// ============================================================================

/// A request channel for sending MCTP requests and receiving responses.
///
/// Obtained via [`Stack::req`]. Implements [`MctpReqChannel`].
pub struct StackReqChannel<'s, C: MctpClient> {
    stack: &'s Stack<C>,
    handle: Handle,
    eid: u8,
    /// Tag captured after the first `send`; required before `recv` may be called.
    sent_tag: Option<u8>,
    timeout: u32,
    /// Hand the handle back to the stack on drop instead of releasing it.
    reuse: bool,
    /// Cleared by any error, so a possibly-pending handle is never reused.
    healthy: bool,
}

impl<C: MctpClient> MctpReqChannel for StackReqChannel<'_, C> {
    fn send(&mut self, msg_type: u8, buf: &[u8]) -> Result<(), MctpError> {
        if self.sent_tag.is_some() {
            self.healthy = false;
            return Err(MctpError::from_code(ResponseCode::BadArgument));
        }
        let tag = self
            .stack
            .client
            .send(Some(self.handle), msg_type, None, None, false, buf)
            .inspect_err(|_| self.healthy = false)?;
        self.sent_tag = Some(tag);
        Ok(())
    }

    fn recv<'f>(&mut self, buf: &'f mut [u8]) -> Result<(RecvMetadata, &'f mut [u8]), MctpError> {
        if self.sent_tag.is_none() {
            return Err(MctpError::from_code(ResponseCode::BadArgument));
        }
        let meta = self
            .stack
            .client
            .recv(self.handle, self.timeout, buf)
            .inspect_err(|_| self.healthy = false)?;
        let len = meta.payload_size;
        // Clear sent_tag after receiving response, allowing another send/recv cycle
        self.sent_tag = None;
        Ok((meta, &mut buf[..len]))
    }

    fn remote_eid(&self) -> u8 {
        self.eid
    }
}

impl<C: MctpClient> Drop for StackReqChannel<'_, C> {
    fn drop(&mut self) {
        if self.reuse && self.healthy && self.sent_tag.is_none() {
            if let Some((_, previous)) = self.stack.cached_req.replace(Some((self.eid, self.handle)))
            {
                self.stack.client.drop_handle(previous);
            }
        } else {
            self.stack.client.drop_handle(self.handle);
        }
    }
}

impl<C: MctpClient> Drop for Stack<C> {
    fn drop(&mut self) {
        if let Some((_, handle)) = self.cached_req.take() {
            self.client.drop_handle(handle);
        }
    }
}

// ============================================================================
// Listener
// ============================================================================

/// A listener that receives incoming MCTP messages of a specific type.
///
/// Obtained via [`Stack::listener`]. Implements [`MctpListener`].
pub struct StackListener<'s, C: MctpClient> {
    stack: &'s Stack<C>,
    handle: Handle,
    timeout: u32,
}

impl<C: MctpClient> StackListener<'_, C> {
    /// Update the timeout used by subsequent [`recv`](MctpListener::recv) calls.
    ///
    /// Useful for a listener kept alive across many `recv` calls (e.g. to
    /// avoid dropping and re-registering the underlying listener handle
    /// between calls) whose desired wait time varies per call. `timeout_millis`
    /// of `0` blocks indefinitely.
    pub fn set_timeout(&mut self, timeout_millis: u32) {
        self.timeout = timeout_millis;
    }
}

impl<'s, C: MctpClient> MctpListener for StackListener<'s, C> {
    type RespChannel<'a>
        = StackRespChannel<'s, C>
    where
        Self: 'a;

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> Result<(RecvMetadata, &'f mut [u8], Self::RespChannel<'_>), MctpError> {
        let meta = self.stack.client.recv(self.handle, self.timeout, buf)?;
        let len = meta.payload_size;
        let resp = StackRespChannel {
            stack: self.stack,
            eid: meta.remote_eid,
            msg_type: meta.msg_type,
            tag: meta.msg_tag,
        };
        Ok((meta, &mut buf[..len], resp))
    }
}

impl<C: MctpClient> Drop for StackListener<'_, C> {
    fn drop(&mut self) {
        self.stack.client.drop_handle(self.handle);
    }
}

// ============================================================================
// Response channel
// ============================================================================

/// A response channel for replying to an incoming MCTP request.
///
/// Returned by [`StackListener::recv`]. Implements [`MctpRespChannel`].
pub struct StackRespChannel<'s, C: MctpClient> {
    stack: &'s Stack<C>,
    eid: u8,
    msg_type: u8,
    tag: u8,
}

impl<C: MctpClient> MctpRespChannel for StackRespChannel<'_, C> {
    fn send(&mut self, buf: &[u8]) -> Result<(), MctpError> {
        // Responses pass handle=None; the server distinguishes requests from
        // responses by the presence or absence of a handle.
        self.stack
            .client
            .send(
                None,
                self.msg_type,
                Some(self.eid),
                Some(self.tag),
                false,
                buf,
            )
            .map(|_| ())
    }

    fn remote_eid(&self) -> u8 {
        self.eid
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    // -----------------------------------------------------------------------
    // Minimal mock MctpClient
    // -----------------------------------------------------------------------

    /// Simple scripted mock: every call returns a fixed result; calls are
    /// counted via Cell so the mock is usable behind a shared reference.
    struct MockClient {
        eid: Cell<u8>,
        next_handle: Cell<u32>,
        /// Payload written into the recv buffer.
        recv_payload: &'static [u8],
        recv_meta: RecvMetadata,
        /// Tag returned by send.
        send_tag: u8,
        /// If set, all operations return this error.
        force_error: Option<ResponseCode>,
        drop_count: Cell<u32>,
    }

    impl MockClient {
        fn new() -> Self {
            MockClient {
                eid: Cell::new(8),
                next_handle: Cell::new(1),
                recv_payload: b"hello",
                recv_meta: RecvMetadata {
                    msg_type: 1,
                    msg_ic: false,
                    msg_tag: 3,
                    remote_eid: 42,
                    payload_size: 5,
                },
                send_tag: 3,
                force_error: None,
                drop_count: Cell::new(0),
            }
        }

        fn with_error(code: ResponseCode) -> Self {
            let mut c = Self::new();
            c.force_error = Some(code);
            c
        }
    }

    impl MctpClient for MockClient {
        fn req(&self, _eid: u8) -> Result<Handle, MctpError> {
            if let Some(e) = self.force_error {
                return Err(MctpError::from_code(e));
            }
            let h = self.next_handle.get();
            self.next_handle.set(h + 1);
            Ok(Handle(h))
        }

        fn listener(&self, _msg_type: u8) -> Result<Handle, MctpError> {
            if let Some(e) = self.force_error {
                return Err(MctpError::from_code(e));
            }
            let h = self.next_handle.get();
            self.next_handle.set(h + 1);
            Ok(Handle(h))
        }

        fn get_eid(&self) -> u8 {
            self.eid.get()
        }

        fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
            if let Some(e) = self.force_error {
                return Err(MctpError::from_code(e));
            }
            self.eid.set(eid);
            Ok(())
        }

        fn recv(
            &self,
            _handle: Handle,
            _timeout_millis: u32,
            buf: &mut [u8],
        ) -> Result<RecvMetadata, MctpError> {
            if let Some(e) = self.force_error {
                return Err(MctpError::from_code(e));
            }
            let len = self.recv_payload.len();
            buf[..len].copy_from_slice(self.recv_payload);
            Ok(self.recv_meta)
        }

        fn send(
            &self,
            _handle: Option<Handle>,
            _msg_type: u8,
            _eid: Option<u8>,
            _tag: Option<u8>,
            _integrity_check: bool,
            _buf: &[u8],
        ) -> Result<u8, MctpError> {
            if let Some(e) = self.force_error {
                return Err(MctpError::from_code(e));
            }
            Ok(self.send_tag)
        }

        fn drop_handle(&self, _handle: Handle) {
            self.drop_count.set(self.drop_count.get() + 1);
        }
    }

    // -----------------------------------------------------------------------
    // Stack::get_eid / set_eid
    // -----------------------------------------------------------------------

    #[test]
    fn stack_get_set_eid() {
        let stack = Stack::new(MockClient::new());
        assert_eq!(stack.get_eid(), 8);
        stack.set_eid(20).unwrap();
        assert_eq!(stack.get_eid(), 20);
    }

    #[test]
    fn stack_set_eid_error_propagates() {
        let stack = Stack::new(MockClient::with_error(ResponseCode::InternalError));
        let err = stack.set_eid(1).unwrap_err();
        assert_eq!(err.code, ResponseCode::InternalError);
    }

    // -----------------------------------------------------------------------
    // Stack::req / StackReqChannel
    // -----------------------------------------------------------------------

    #[test]
    fn req_channel_send_then_recv() {
        let stack = Stack::new(MockClient::new());
        let mut ch = stack.req(42, 0).unwrap();
        assert_eq!(ch.remote_eid(), 42);

        ch.send(1, b"ping").unwrap();

        let mut buf = [0u8; 32];
        let (meta, payload) = ch.recv(&mut buf).unwrap();
        assert_eq!(meta.remote_eid, 42);
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn req_channel_recv_before_send_is_error() {
        let stack = Stack::new(MockClient::new());
        let mut ch = stack.req(42, 0).unwrap();
        let mut buf = [0u8; 32];
        let err = ch.recv(&mut buf).unwrap_err();
        assert_eq!(err.code, ResponseCode::BadArgument);
    }

    #[test]
    fn req_channel_double_send_is_error() {
        let stack = Stack::new(MockClient::new());
        let mut ch = stack.req(42, 0).unwrap();
        ch.send(1, b"first").unwrap();
        let err = ch.send(1, b"second").unwrap_err();
        assert_eq!(err.code, ResponseCode::BadArgument);
    }

    #[test]
    fn req_channel_drop_releases_handle() {
        let stack = Stack::new(MockClient::new());
        let ch = stack.req(10, 0).unwrap();
        drop(ch);
        assert_eq!(stack.client.drop_count.get(), 1);
    }

    #[test]
    fn reusable_req_keeps_the_handle_between_channels() {
        let stack = Stack::new(MockClient::new());
        let mut buf = [0u8; 32];
        for _ in 0..3 {
            let mut ch = stack.req_reusable(42, 0).unwrap();
            ch.send(1, b"ping").unwrap();
            ch.recv(&mut buf).unwrap();
        }
        // One handle allocated, none released yet.
        assert_eq!(stack.client.next_handle.get(), 2);
        assert_eq!(stack.client.drop_count.get(), 0);
    }

    /// Client that only counts released handles, into a counter that outlives
    /// the stack.
    struct CountingClient<'a>(&'a Cell<u32>);

    impl MctpClient for CountingClient<'_> {
        fn req(&self, _eid: u8) -> Result<Handle, MctpError> {
            Ok(Handle(1))
        }
        fn listener(&self, _msg_type: u8) -> Result<Handle, MctpError> {
            Ok(Handle(2))
        }
        fn get_eid(&self) -> u8 {
            0
        }
        fn set_eid(&self, _eid: u8) -> Result<(), MctpError> {
            Ok(())
        }
        fn recv(
            &self,
            _handle: Handle,
            _timeout_millis: u32,
            _buf: &mut [u8],
        ) -> Result<RecvMetadata, MctpError> {
            Err(MctpError::from_code(ResponseCode::InternalError))
        }
        fn send(
            &self,
            _handle: Option<Handle>,
            _msg_type: u8,
            _eid: Option<u8>,
            _tag: Option<u8>,
            _integrity_check: bool,
            _buf: &[u8],
        ) -> Result<u8, MctpError> {
            Ok(0)
        }
        fn drop_handle(&self, _handle: Handle) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn reusable_req_releases_cached_handle_on_stack_drop() {
        let drops = Cell::new(0u32);
        {
            let stack = Stack::new(CountingClient(&drops));
            let ch = stack.req_reusable(42, 0).unwrap();
            drop(ch);
            assert_eq!(drops.get(), 0);
        }
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn reusable_req_for_another_eid_replaces_the_cached_handle() {
        let stack = Stack::new(MockClient::new());
        drop(stack.req_reusable(42, 0).unwrap());
        let ch = stack.req_reusable(43, 0).unwrap();
        assert_eq!(stack.client.next_handle.get(), 3);
        assert_eq!(stack.client.drop_count.get(), 1);
        drop(ch);
    }

    #[test]
    fn reusable_req_is_released_after_an_unanswered_send() {
        let stack = Stack::new(MockClient::new());
        let mut ch = stack.req_reusable(42, 0).unwrap();
        ch.send(1, b"ping").unwrap();
        drop(ch);
        // No response was received: the handle must not be reused.
        assert_eq!(stack.client.drop_count.get(), 1);
        let _next = stack.req_reusable(42, 0).unwrap();
        assert_eq!(stack.client.next_handle.get(), 3);
    }

    #[test]
    fn req_error_propagates() {
        let stack = Stack::new(MockClient::with_error(ResponseCode::NoSpace));
        let err = stack.req(10, 0).err().expect("should fail");
        assert_eq!(err.code, ResponseCode::NoSpace);
    }

    // -----------------------------------------------------------------------
    // Stack::listener / StackListener / StackRespChannel
    // -----------------------------------------------------------------------

    #[test]
    fn listener_recv_returns_payload_and_resp_channel() {
        let stack = Stack::new(MockClient::new());
        let mut listener = stack.listener(1, 0).unwrap();
        let mut buf = [0u8; 32];
        let (meta, payload, resp) = listener.recv(&mut buf).unwrap();

        assert_eq!(meta.msg_type, 1);
        assert_eq!(meta.remote_eid, 42);
        assert_eq!(payload, b"hello");
        assert_eq!(resp.remote_eid(), 42);
    }

    #[test]
    fn listener_resp_channel_send_ok() {
        let stack = Stack::new(MockClient::new());
        let mut listener = stack.listener(1, 0).unwrap();
        let mut buf = [0u8; 32];
        let (_, _, mut resp) = listener.recv(&mut buf).unwrap();
        resp.send(b"pong").unwrap();
    }

    #[test]
    fn listener_error_propagates() {
        let stack = Stack::new(MockClient::with_error(ResponseCode::TimedOut));
        let err = stack.listener(1, 0).err().expect("should fail");
        assert_eq!(err.code, ResponseCode::TimedOut);
    }

    #[test]
    fn listener_drop_releases_handle() {
        let stack = Stack::new(MockClient::new());
        let l = stack.listener(1, 0).unwrap();
        drop(l);
        assert_eq!(stack.client.drop_count.get(), 1);
    }
}
