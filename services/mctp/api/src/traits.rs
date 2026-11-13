// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP client traits
//!
//! Platform-independent traits for interacting with an MCTP server.
//! Implementations are provided per-platform (Hubris IPC, Linux sockets, etc.).

use crate::{Handle, MctpError, RecvMetadata, ResponseCode};

/// A client interface to an MCTP stack/server.
///
/// Provides methods for obtaining listener and request handles, and
/// for managing the endpoint ID (EID).
pub trait MctpClient {
    /// Obtain a request handle for sending messages to the given EID.
    fn req(&self, eid: u8) -> Result<Handle, MctpError>;

    /// Register a listener for incoming messages of the given MCTP type.
    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError>;

    /// Get the local endpoint ID.
    fn get_eid(&self) -> u8;

    /// Set the local endpoint ID.
    fn set_eid(&self, eid: u8) -> Result<(), MctpError>;

    /// Receive a message on the given handle into `buf`.
    ///
    /// `timeout_millis` of 0 means no timeout (block indefinitely).
    fn recv(
        &self,
        handle: Handle,
        timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError>;

    /// Receive a message on whichever of `handles` gets one first, into `buf`.
    ///
    /// Returns the handle the message arrived on. If more than one handle has
    /// a message waiting, the earliest in `handles` is received and the others
    /// stay queued. `timeout_millis` of 0 means no timeout. At most
    /// [`MAX_RECV_ANY_HANDLES`](crate::wire::MAX_RECV_ANY_HANDLES) handles; a
    /// handle may not repeat or already have a receive outstanding.
    ///
    /// The handles must belong to the server this client talks to, but need
    /// not have been allocated through this client.
    ///
    /// The default returns [`ResponseCode::BadArgument`], so clients that
    /// cannot wait on several handles need not implement it.
    fn recv_any(
        &self,
        handles: &[Handle],
        timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<(Handle, RecvMetadata), MctpError> {
        let _ = (handles, timeout_millis, buf);
        Err(MctpError::from_code(ResponseCode::BadArgument))
    }

    /// Send a message through the given handle.
    ///
    /// For requests, `handle` is `Some`. For responses, `handle` is `None`.
    /// Returns the tag value used.
    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError>;

    /// Release a handle previously obtained from `req` or `listener`.
    fn drop_handle(&self, handle: Handle);
}

/// A listener that receives incoming MCTP messages of a specific type.
pub trait MctpListener {
    /// The response channel type returned when a message is received.
    type RespChannel<'a>: MctpRespChannel
    where
        Self: 'a;

    /// Wait for an incoming message, writing the payload into `buf`.
    ///
    /// Returns the message metadata, payload slice, and a response channel.
    fn recv<'f>(
        &mut self,
        buf: &'f mut [u8],
    ) -> Result<(RecvMetadata, &'f mut [u8], Self::RespChannel<'_>), MctpError>;
}

/// A request channel for sending MCTP requests and receiving responses.
pub trait MctpReqChannel {
    /// Send a request message.
    fn send(&mut self, msg_type: u8, buf: &[u8]) -> Result<(), MctpError>;

    /// Receive the response to a previously sent request.
    fn recv<'f>(&mut self, buf: &'f mut [u8]) -> Result<(RecvMetadata, &'f mut [u8]), MctpError>;

    /// The remote endpoint ID this channel targets.
    fn remote_eid(&self) -> u8;
}

/// A response channel for replying to an incoming MCTP request.
pub trait MctpRespChannel {
    /// Send a response message.
    fn send(&mut self, buf: &[u8]) -> Result<(), MctpError>;

    /// The remote endpoint ID that sent the original request.
    fn remote_eid(&self) -> u8;
}
