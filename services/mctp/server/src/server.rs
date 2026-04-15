// Licensed under the Apache-2.0 license

//! Core MCTP server logic.
//!
//! This is a direct port of the Hubris `mctp-server/src/server.rs`.
//! The `Router` integration, handle management, timeout logic, and message
//! routing are preserved as-is. Only Hubris IPC primitives (`sys_reply`,
//! `Leased`, `RecvMessage`) have been replaced with platform-independent
//! equivalents.

use heapless::LinearMap;
use mctp::{Eid, MsgIC, MsgType, Tag, TagValue};
use mctp_lib::{AppCookie, Router, Sender};
use openprot_mctp_api::{Handle, MctpError, RecvMetadata, ResponseCode};

/// Maximum payload size in bytes.
// TODO: Use configuration from mctp-lib (mctp-estack)
//       see https://github.com/OpenPRoT/mctp-lib/issues/4
const MAX_PAYLOAD: usize = 1023;

/// Configuration constants for the MCTP server.
pub struct ServerConfig;

impl ServerConfig {
    /// Maximum number of concurrent requests the server can handle.
    pub const MAX_REQUESTS: usize = 8;
    /// Maximum number of listeners that can be registered concurrently.
    pub const MAX_LISTENERS: usize = 8;
    /// Maximum number of concurrent outstanding receive calls.
    pub const MAX_OUTSTANDING: usize = 16;
    /// Maximum payload size in bytes.
    pub const MAX_PAYLOAD: usize = MAX_PAYLOAD;
}

/// A pending receive call waiting for a message or timeout.
#[derive(Debug, Clone, Copy)]
struct PendingRecv {
    /// Deadline in milliseconds (0 = no timeout).
    deadline: u64,
}

/// The platform-independent MCTP server.
///
/// This struct wraps the `mctp-lib` [`Router`] and manages outstanding
/// receive calls with timeout tracking. It is a direct port of the
/// Hubris `Server` struct with OS-specific IPC removed.
///
/// # Type Parameters
///
/// * `S` - The [`Sender`] implementation for outbound transport.
/// * `OUTSTANDING` - Maximum number of concurrent pending receive calls.
pub struct Server<S: Sender, const OUTSTANDING: usize> {
    /// The underlying MCTP router (from mctp-lib).
    pub stack: Router<S, { ServerConfig::MAX_LISTENERS }, { ServerConfig::MAX_REQUESTS }>,
    /// Currently outstanding recv calls, keyed by handle value.
    ///
    /// Maps the handle to a deadline. The platform layer is responsible
    /// for storing any additional per-recv state (e.g., reply channels).
    outstanding: LinearMap<u32, PendingRecv, OUTSTANDING>,
}

impl<S: Sender, const OUTSTANDING: usize> Server<S, OUTSTANDING> {
    /// Create a new MCTP server instance.
    pub fn new(own_eid: Eid, now_millis: u64, outbound: S) -> Self {
        let stack = Router::new(own_eid, now_millis, outbound);
        Self {
            stack,
            outstanding: LinearMap::new(),
        }
    }

    /// Allocate a request handle for sending messages to the given EID.
    pub fn req(&mut self, eid: u8) -> Result<Handle, MctpError> {
        self.stack.req(Eid(eid)).map(|cookie| Handle(cookie.0 as u32)).map_err(|e| {
            let err = mctp_error_to_server_error(e);
            pw_log::error!("server: req(eid={}) ResponseCode={}", eid as u32, err.code as u8);
            err
        })
    }

    /// Register a listener for incoming messages of the given type.
    pub fn listener(&mut self, typ: u8) -> Result<Handle, MctpError> {
        self.stack.listener(MsgType(typ)).map(|cookie| Handle(cookie.0 as u32)).map_err(|e| {
            let err = mctp_error_to_server_error(e);
            pw_log::error!("server: listener(typ={}) ResponseCode={}", typ as u32, err.code as u8);
            err
        })
    }

    /// Get the currently configured EID.
    pub fn get_eid(&self) -> u8 {
        self.stack.get_eid().0
    }

    /// Set the EID for this endpoint.
    pub fn set_eid(&mut self, eid: u8) -> Result<(), MctpError> {
        self.stack
            .set_eid(Eid(eid))
            .map_err(mctp_error_to_server_error)
    }

    /// Check for an available message on the given handle.
    ///
    /// If a message is available, returns the metadata and copies the
    /// payload into `buf`. Otherwise returns `None` and the caller
    /// should register a pending recv via [`register_recv`](Self::register_recv).
    pub fn try_recv(
        &mut self,
        handle: Handle,
        buf: &mut [u8],
    ) -> Option<RecvMetadata> {
        let cookie = AppCookie(handle.0 as usize);
        let msg = self.stack.recv(cookie)?;

        let payload_len = msg.payload.len();
        if payload_len <= buf.len() {
            buf[..payload_len].copy_from_slice(msg.payload);
        }

        Some(RecvMetadata {
            msg_type: msg.typ.0,
            msg_ic: msg.ic.0,
            msg_tag: msg.tag.tag().0,
            remote_eid: msg.source.0,
            payload_size: payload_len,
        })
    }

    /// Register a pending receive call for the given handle.
    ///
    /// The platform layer should call this when `try_recv` returns `None`
    /// and the client wants to block. Returns an error if the outstanding
    /// table is full.
    pub fn register_recv(
        &mut self,
        handle: Handle,
        timeout_millis: u32,
        now_millis: u64,
    ) -> Result<(), MctpError> {
        let deadline = if timeout_millis != 0 {
            now_millis + timeout_millis as u64
        } else {
            0
        };

        // Don't overwrite existing entries
        if self.outstanding.contains_key(&handle.0) {
            return Ok(());
        }

        self.outstanding
            .insert(handle.0, PendingRecv { deadline })
            .map_err(|_| MctpError::from_code(ResponseCode::NoSpace))?;
        Ok(())
    }

    /// Send a message.
    ///
    /// For requests, `handle` is `Some`. For responses, `handle` is `None`.
    /// When responding to a request received by a listener, `eid` and `tag`
    /// must be set. Returns the tag value used.
    pub fn send(
        &mut self,
        handle: Option<Handle>,
        typ: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        ic: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        if buf.len() > MAX_PAYLOAD {
            return Err(MctpError::from_code(ResponseCode::NoSpace));
        }

        let tag = if handle.is_none() {
            // Responses use unowned tags
            tag.map(|x| Tag::Unowned(TagValue(x)))
        } else {
            // Requests use owned tags (or allocate a new one)
            tag.map(|x| Tag::Owned(TagValue(x)))
        };

        // Responses need no handle, use 255 as dummy
        let cookie = AppCookie(handle.unwrap_or(Handle(255)).0 as usize);

        let result = self.stack.send(
            eid.map(Eid),
            MsgType(typ),
            tag,
            MsgIC(ic),
            cookie,
            buf,
        );

        match result {
            Ok(tag) => Ok(tag.tag().0),
            Err(e) => Err(mctp_error_to_server_error(e)),
        }
    }

    /// Update the stack and check for fulfilled receive calls.
    ///
    /// Should be called on timer events. Returns the interval (ms) until
    /// the next required update, and a list of handles that now have
    /// messages available (the platform layer should deliver them).
    pub fn update(
        &mut self,
        now_millis: u64,
        recv_buf: &mut [u8],
    ) -> (u32, heapless::Vec<(Handle, RecvResult), OUTSTANDING>) {
        // Update the mctp-stack; get the next timeout interval
        let stack_timeout = self
            .stack
            .update(now_millis)
            .unwrap_or(60_000) as u32;

        let mut ready: heapless::Vec<(Handle, RecvResult), OUTSTANDING> = heapless::Vec::new();

        for (handle_val, pending) in self.outstanding.iter() {
            let handle = Handle(*handle_val);
            let cookie = AppCookie(*handle_val as usize);

            // Check if a message arrived for this handle
            if let Some(mctp_msg) = self.stack.recv(cookie) {
                let payload_len = mctp_msg.payload.len();
                if payload_len <= recv_buf.len() {
                    recv_buf[..payload_len].copy_from_slice(mctp_msg.payload);
                }
                let metadata = RecvMetadata {
                    msg_type: mctp_msg.typ.0,
                    msg_ic: mctp_msg.ic.0,
                    msg_tag: mctp_msg.tag.tag().0,
                    remote_eid: mctp_msg.source.0,
                    payload_size: payload_len,
                };
                let _ = ready.push((handle, RecvResult::Message(metadata)));
                continue;
            }

            // Check for timeout
            if pending.deadline != 0 && now_millis >= pending.deadline {
                let _ = ready.push((handle, RecvResult::TimedOut));
            }
        }

        // Remove fulfilled/timed-out entries
        for (handle, _) in &ready {
            self.outstanding.remove(&handle.0);
        }

        (stack_timeout, ready)
    }

    /// Unbind a handle previously allocated by `req` or `listener`.
    pub fn unbind(&mut self, handle: Handle) -> Result<(), MctpError> {
        let cookie = AppCookie(handle.0 as usize);
        let _ = self.stack.unbind(cookie);
        self.outstanding.remove(&handle.0);
        Ok(())
    }

    /// Feed an inbound MCTP packet to the router.
    ///
    /// The platform layer calls this when data arrives from a transport
    /// binding. The packet should be a raw MCTP packet without transport
    /// headers (the transport binding strips those).
    pub fn inbound(&mut self, pkt: &[u8]) -> Result<(), MctpError> {
        self.stack
            .inbound(pkt)
            .map_err(mctp_error_to_server_error)
    }
}

/// Result of a pending receive call.
#[derive(Debug, Clone, Copy)]
pub enum RecvResult {
    /// A message was received.
    Message(RecvMetadata),
    /// The receive call timed out.
    TimedOut,
}

/// Map mctp::Error to our MctpError.
fn mctp_error_to_server_error(e: mctp::Error) -> MctpError {
    use mctp::Error::*;
    let code = match e {
        InternalError => ResponseCode::InternalError,
        NoSpace => ResponseCode::NoSpace,
        AddrInUse => ResponseCode::AddrInUse,
        TimedOut => ResponseCode::TimedOut,
        BadArgument => ResponseCode::BadArgument,
        _ => ResponseCode::InternalError,
    };
    MctpError::from_code(code)
}
