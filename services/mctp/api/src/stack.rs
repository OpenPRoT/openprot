// Licensed under the Apache-2.0 license

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
//! use openprot_mctp_client::IpcMctpClient;
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

use crate::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use crate::traits::{MctpListener, MctpReqChannel, MctpRespChannel};

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
}

impl<C: MctpClient> Stack<C> {
    /// Create a new stack backed by the given `MctpClient`.
    pub fn new(client: C) -> Self {
        Stack { client }
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
    pub fn req(
        &self,
        eid: u8,
        timeout_millis: u32,
    ) -> Result<StackReqChannel<'_, C>, MctpError> {
        let handle = self.client.req(eid)?;
        Ok(StackReqChannel {
            stack: self,
            handle,
            eid,
            sent_tag: None,
            timeout: timeout_millis,
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
}

impl<C: MctpClient> MctpReqChannel for StackReqChannel<'_, C> {
    fn send(&mut self, msg_type: u8, buf: &[u8]) -> Result<(), MctpError> {
        if self.sent_tag.is_some() {
            return Err(MctpError::from_code(ResponseCode::BadArgument));
        }
        let tag = self.stack.client.send(
            Some(self.handle),
            msg_type,
            None,
            None,
            false,
            buf,
        )?;
        self.sent_tag = Some(tag);
        Ok(())
    }

    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> Result<(RecvMetadata, &'f mut [u8]), MctpError> {
        if self.sent_tag.is_none() {
            return Err(MctpError::from_code(ResponseCode::BadArgument));
        }
        let meta = self.stack.client.recv(self.handle, self.timeout, buf)?;
        let len = meta.payload_size;
        Ok((meta, &mut buf[..len]))
    }

    fn remote_eid(&self) -> u8 {
        self.eid
    }
}

impl<C: MctpClient> Drop for StackReqChannel<'_, C> {
    fn drop(&mut self) {
        self.stack.client.drop_handle(self.handle);
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
            .send(None, self.msg_type, Some(self.eid), Some(self.tag), false, buf)
            .map(|_| ())
    }

    fn remote_eid(&self) -> u8 {
        self.eid
    }
}
