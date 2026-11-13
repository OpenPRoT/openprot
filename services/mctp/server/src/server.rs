// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Core MCTP server logic.
//!
//! [`Server`] is one MCTP endpoint as its clients see it: handles for
//! listeners and requests, send, and receive. A receive that finds no
//! message can be *registered* instead of failing; the server then reports
//! it later, from [`Server::update`], as either a message or a timeout.
//! Time is a caller-supplied `u64` of monotonic milliseconds, so nothing
//! here depends on a kernel clock.
//!
//! The router keeps the last time it was told and stamps what it creates
//! with it: a reassembly in progress, and the flow that matches a response
//! to its request, both of which expire six seconds after their stamp. So
//! the clock must be brought up to date *before* an event is handled, with
//! [`Server::advance`], not only afterwards. Stamped with a time from
//! before a quiet period, a new reassembly or flow looks old the moment
//! the clock catches up, and is expired before it can complete.
//!
//! What the server does not do is hold the caller's reply path. Whoever
//! drives it (see `openprot_mctp_server::dispatch` and the kernel runtime)
//! keeps that, keyed by the receive's [`Handle`].
//!
//! Derived from the Hubris `mctp-server`, with its IPC primitives removed.

use heapless::LinearMap;
use mctp::{Eid, MsgIC, MsgType, Tag, TagValue};
use mctp_lib::{config, AppCookie, MctpMessage, Router, Sender};
use openprot_mctp_api::{Handle, MctpError, RecvMetadata, ResponseCode};

/// Configuration constants for the MCTP server.
pub struct ServerConfig;

impl ServerConfig {
    /// Maximum number of concurrent requests the server can handle.
    pub const MAX_REQUESTS: usize = 8;
    /// Maximum number of listeners that can be registered concurrently.
    pub const MAX_LISTENERS: usize = 8;
    /// Maximum number of concurrent outstanding receive calls.
    pub const MAX_OUTSTANDING: usize = 16;
    /// Maximum payload size in bytes, defined by `mctp-estack` config.
    pub const MAX_PAYLOAD: usize = config::MAX_PAYLOAD;
}

/// A pending receive call waiting for a message or timeout.
#[derive(Debug, Clone, Copy)]
struct PendingRecv {
    /// Absolute deadline in milliseconds, or `None` to wait forever.
    deadline: Option<u64>,
    /// Set when this receive is one of several registered together by
    /// [`Server::register_recv_any`].
    group: Option<Group>,
}

/// Membership of a receive registered with [`Server::register_recv_any`].
#[derive(Debug, Clone, Copy)]
struct Group {
    /// The handle the caller's reply path is keyed by: the first of the
    /// handles it registered. All members resolve through this one key.
    key: u32,
    /// Position in the caller's list: lower is received first when several
    /// members have a message waiting at once.
    priority: u8,
}

/// Most handles one [`Server::register_recv_any`] can wait on.
pub const MAX_RECV_ANY: usize = openprot_mctp_api::wire::MAX_RECV_ANY_HANDLES;

/// The platform-independent MCTP server.
///
/// This struct wraps the `mctp-lib` [`Router`] and manages outstanding
/// receive calls with timeout tracking.
///
/// # Type Parameters
///
/// * `S` - The [`Sender`] implementation for outbound transport.
/// * `OUTSTANDING` - Maximum number of concurrent pending receive calls.
pub struct Server<S: Sender, const OUTSTANDING: usize> {
    /// The underlying MCTP router (from mctp-lib). Private: going around
    /// the server would bypass the outstanding-receive table.
    stack: Router<S, { ServerConfig::MAX_LISTENERS }, { ServerConfig::MAX_REQUESTS }>,
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
        match self.stack.req(Eid(eid)) {
            Ok(cookie) => Ok(Handle(cookie.0 as u32)),
            Err(e) => Err(mctp_error_to_server_error(e)),
        }
    }

    /// Register a listener for incoming messages of the given type.
    pub fn listener(&mut self, typ: u8) -> Result<Handle, MctpError> {
        match self.stack.listener(MsgType(typ)) {
            Ok(cookie) => Ok(Handle(cookie.0 as u32)),
            Err(e) => Err(mctp_error_to_server_error(e)),
        }
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
    ///
    /// A payload larger than `buf` is consumed but not copied.
    /// `payload_size` still reports its real length, so the caller detects
    /// this as `payload_size > buf.len()`.
    pub fn try_recv(&mut self, handle: Handle, buf: &mut [u8]) -> Option<RecvMetadata> {
        let cookie = AppCookie(handle.0 as usize);
        let msg = self.stack.recv(cookie)?;

        Some(recv_metadata_from_msg(&msg, buf))
    }

    /// Register a pending receive call for the given handle.
    ///
    /// The platform layer should call this when `try_recv` returns `None`
    /// and the client wants to block. `timeout_millis` of 0 waits forever.
    ///
    /// A handle has at most one receive outstanding. Fails with `AddrInUse`
    /// if one is already registered on it, and with `NoSpace` if the
    /// outstanding table is full. On either error nothing is registered, so
    /// the caller must answer the client now rather than wait.
    pub fn register_recv(
        &mut self,
        handle: Handle,
        timeout_millis: u32,
        now_millis: u64,
    ) -> Result<(), MctpError> {
        if self.outstanding.contains_key(&handle.0) {
            return Err(MctpError::from_code(ResponseCode::AddrInUse));
        }
        let deadline = match timeout_millis {
            0 => None,
            t => Some(now_millis.saturating_add(u64::from(t))),
        };
        self.outstanding
            .insert(
                handle.0,
                PendingRecv {
                    deadline,
                    group: None,
                },
            )
            .map_err(|_| MctpError::from_code(ResponseCode::NoSpace))?;
        Ok(())
    }

    /// Check for a message on any of `handles`, in the order given.
    ///
    /// Returns the handle it arrived on and its metadata, copying the payload
    /// into `buf` as [`try_recv`](Self::try_recv) does. If several handles
    /// have a message waiting, only the first one is taken; the others stay
    /// queued for the next receive.
    pub fn try_recv_any(
        &mut self,
        handles: &[Handle],
        buf: &mut [u8],
    ) -> Option<(Handle, RecvMetadata)> {
        for &handle in handles {
            if let Some(meta) = self.try_recv(handle, buf) {
                return Some((handle, meta));
            }
        }
        None
    }

    /// Register one pending receive that completes on whichever of `handles`
    /// gets a message first, or on the timeout.
    ///
    /// The caller's reply path is keyed by the returned handle (the first of
    /// `handles`); [`update`](Self::update) reports the outcome under that key
    /// as [`RecvResult::MessageOn`] (naming the handle that fired) or
    /// [`RecvResult::TimedOut`], once for the whole group, and drops every
    /// member's registration. `timeout_millis` of 0 waits forever.
    ///
    /// All-or-nothing: fails with `BadArgument` for an empty list, too many
    /// handles or a repeated handle, with `AddrInUse` if any handle already
    /// has a receive outstanding, and with `NoSpace` if the table cannot hold
    /// them all. On any error nothing is registered, so the caller must answer
    /// the client now rather than wait.
    pub fn register_recv_any(
        &mut self,
        handles: &[Handle],
        timeout_millis: u32,
        now_millis: u64,
    ) -> Result<Handle, MctpError> {
        let bad = || MctpError::from_code(ResponseCode::BadArgument);
        let key = handles.first().ok_or_else(bad)?.0;
        if handles.len() > MAX_RECV_ANY {
            return Err(bad());
        }
        for (i, handle) in handles.iter().enumerate() {
            if handles[..i].contains(handle) {
                return Err(bad());
            }
            if self.outstanding.contains_key(&handle.0) {
                return Err(MctpError::from_code(ResponseCode::AddrInUse));
            }
        }
        if self.outstanding.len() + handles.len() > self.outstanding.capacity() {
            return Err(MctpError::from_code(ResponseCode::NoSpace));
        }
        let deadline = match timeout_millis {
            0 => None,
            t => Some(now_millis.saturating_add(u64::from(t))),
        };
        for (priority, handle) in handles.iter().enumerate() {
            // Cannot fail: capacity was checked above.
            let _ = self.outstanding.insert(
                handle.0,
                PendingRecv {
                    deadline,
                    group: Some(Group {
                        key,
                        priority: priority as u8,
                    }),
                },
            );
        }
        Ok(Handle(key))
    }

    /// Drop the receive registered on `handle`, and, if it belongs to a
    /// [`register_recv_any`](Self::register_recv_any) group, every other
    /// member of that group.
    fn remove_pending(&mut self, handle: Handle) {
        let group = self.outstanding.get(&handle.0).and_then(|p| p.group);
        self.outstanding.remove(&handle.0);
        if let Some(group) = group {
            let mut members: heapless::Vec<u32, MAX_RECV_ANY> = heapless::Vec::new();
            for (handle_val, pending) in self.outstanding.iter() {
                if pending.group.is_some_and(|g| g.key == group.key) {
                    let _ = members.push(*handle_val);
                }
            }
            for member in members {
                self.outstanding.remove(&member);
            }
        }
    }

    /// Bring the router's clock up to `now_millis`.
    ///
    /// Call this with the current time before handling any event: before
    /// feeding a packet to [`inbound`](Self::inbound) and before a `send`.
    /// See the module documentation for why "before" matters.
    /// [`update`](Self::update) and `dispatch::dispatch_mctp_op` advance the
    /// clock themselves, so only a direct `inbound` or `send` needs this.
    /// A time earlier than the last one given is ignored.
    pub fn advance(&mut self, now_millis: u64) {
        let _ = self.stack.update(now_millis);
    }

    /// Withdraw a receive registered with [`register_recv`](Self::register_recv).
    ///
    /// For a platform layer that registered a receive and then found it
    /// cannot hold the caller's reply path after all. A no-op if nothing is
    /// registered on `handle`.
    pub fn cancel_recv(&mut self, handle: Handle) {
        self.remove_pending(handle);
    }

    /// The earliest deadline, in the caller's millisecond clock, among
    /// registered receives, or `None` if none of them has a timeout.
    ///
    /// The platform layer must call [`update`](Self::update) no later than
    /// this. It is the only timer the server asks for: the router's own
    /// housekeeping (reassembly and flow expiry) is advanced on every
    /// `update` and needs no wake-up of its own, since a stale entry only
    /// matters when the next packet or request arrives, and that is itself
    /// followed by an `update`.
    pub fn next_recv_deadline(&self) -> Option<u64> {
        self.outstanding.values().filter_map(|p| p.deadline).min()
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
        if buf.len() > ServerConfig::MAX_PAYLOAD {
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

        let result = self
            .stack
            .send(eid.map(Eid), MsgType(typ), tag, MsgIC(ic), cookie, buf);

        match result {
            Ok(tag) => Ok(tag.tag().0),
            Err(e) => Err(mctp_error_to_server_error(e)),
        }
    }

    /// Update the stack and resolve any fulfilled receive calls.
    ///
    /// Call this after every event the platform layer handles (an inbound
    /// packet, a client request) and no later than
    /// [`next_recv_deadline`](Self::next_recv_deadline). Returns the
    /// router's suggested housekeeping interval (ms), which callers may
    /// ignore for the reason given there.
    ///
    /// `recv_buf` is a single shared scratch buffer, reused for each
    /// resolved handle in turn. For every handle whose registered receive
    /// is now fulfilled, `on_ready(handle, result, recv_buf)` is called
    /// immediately — with `recv_buf` holding only *that* handle's payload
    /// (empty for `RecvResult::TimedOut`) — so the caller must copy or
    /// encode what it needs before `on_ready` returns. Holding onto
    /// `recv_buf` past the callback would alias a later handle's message,
    /// since the buffer is overwritten for the next resolved handle (if
    /// any). A message payload larger than `recv_buf` is consumed but not
    /// copied, as in [`try_recv`](Self::try_recv).
    pub fn update(
        &mut self,
        now_millis: u64,
        recv_buf: &mut [u8],
        mut on_ready: impl FnMut(Handle, RecvResult, &[u8]),
    ) -> u32 {
        // Update the mctp-stack; get the next timeout interval
        let stack_timeout = self.stack.update(now_millis).unwrap_or(60_000) as u32;

        let mut resolved: heapless::Vec<u32, OUTSTANDING> = heapless::Vec::new();

        for (handle_val, pending) in self.outstanding.iter() {
            if pending.group.is_some() {
                continue; // resolved below, one result per group
            }
            let handle = Handle(*handle_val);
            let cookie = AppCookie(*handle_val as usize);

            // Check if a message arrived for this handle
            if let Some(mctp_msg) = self.stack.recv(cookie) {
                let metadata = recv_metadata_from_msg(&mctp_msg, recv_buf);
                let _ = resolved.push(*handle_val);
                // Deliver now, while recv_buf still holds this handle's
                // payload; the next resolved handle (if any) overwrites it.
                on_ready(handle, RecvResult::Message(metadata), recv_buf);
                continue;
            }

            // Check for timeout
            if pending.deadline.is_some_and(|d| now_millis >= d) {
                let _ = resolved.push(*handle_val);
                on_ready(handle, RecvResult::TimedOut, &[]);
            }
        }

        // Receives registered together resolve as one: the first member in
        // priority order that has a message wins, and the others are left
        // untouched (their messages stay queued). With no message, the group
        // times out together.
        let mut keys: heapless::Vec<u32, OUTSTANDING> = heapless::Vec::new();
        for pending in self.outstanding.values() {
            if let Some(group) = pending.group
                && !keys.contains(&group.key)
            {
                let _ = keys.push(group.key);
            }
        }
        for key in keys {
            let mut members: heapless::Vec<(u8, u32, Option<u64>), MAX_RECV_ANY> =
                heapless::Vec::new();
            for (handle_val, pending) in self.outstanding.iter() {
                if let Some(group) = pending.group.filter(|g| g.key == key) {
                    let _ = members.push((group.priority, *handle_val, pending.deadline));
                }
            }
            members.sort_unstable_by_key(|&(priority, _, _)| priority);

            let mut result = None;
            for &(_, handle_val, _) in members.iter() {
                if let Some(meta) = self.try_recv(Handle(handle_val), recv_buf) {
                    result = Some(RecvResult::MessageOn {
                        handle: Handle(handle_val),
                        meta,
                    });
                    break;
                }
            }
            if result.is_none() {
                let due = members
                    .first()
                    .and_then(|&(_, _, deadline)| deadline)
                    .is_some_and(|d| now_millis >= d);
                if due {
                    result = Some(RecvResult::TimedOut);
                }
            }
            if let Some(result) = result {
                // Deliver now, while `recv_buf` still holds this group's payload
                // (empty for a timeout); the next resolved handle or group, if
                // any, overwrites it.
                let payload: &[u8] = if matches!(result, RecvResult::TimedOut) {
                    &[]
                } else {
                    recv_buf
                };
                on_ready(Handle(key), result, payload);
                for &(_, handle_val, _) in members.iter() {
                    self.outstanding.remove(&handle_val);
                }
            }
        }

        // Remove fulfilled/timed-out entries
        for handle_val in &resolved {
            self.outstanding.remove(handle_val);
        }

        stack_timeout
    }

    /// Unbind a handle previously allocated by `req` or `listener`.
    pub fn unbind(&mut self, handle: Handle) -> Result<(), MctpError> {
        let cookie = AppCookie(handle.0 as usize);
        let _ = self.stack.unbind(cookie);
        self.remove_pending(handle);
        Ok(())
    }

    /// Feed an inbound MCTP packet to the router.
    ///
    /// The platform layer calls this when data arrives from a transport
    /// binding. The packet should be a raw MCTP packet without transport
    /// headers (the transport binding strips those).
    ///
    /// The packet is timestamped with the router's clock as of the last
    /// [`advance`](Self::advance) or [`update`](Self::update), so call
    /// `advance` with the current time first.
    pub fn inbound(&mut self, pkt: &[u8]) -> Result<(), MctpError> {
        // The AppCookie of the matching listener/request is discarded;
        // pending receives are serviced by polling, not by this result.
        self.stack
            .inbound(pkt)
            .map(|_| ())
            .map_err(mctp_error_to_server_error)
    }
}

/// Result of a pending receive call.
#[derive(Debug, Clone, Copy)]
pub enum RecvResult {
    /// A message was received.
    Message(RecvMetadata),
    /// A message was received on `handle`, one of several registered with
    /// [`Server::register_recv_any`].
    MessageOn {
        /// The handle the message arrived on.
        handle: Handle,
        /// The message.
        meta: RecvMetadata,
    },
    /// The receive call timed out.
    TimedOut,
}

/// Build receive metadata and copy payload into `buf` when it fits.
fn recv_metadata_from_msg(msg: &MctpMessage<'_>, buf: &mut [u8]) -> RecvMetadata {
    let payload_len = msg.payload.len();
    if payload_len <= buf.len() {
        buf[..payload_len].copy_from_slice(msg.payload);
    }

    RecvMetadata {
        msg_type: msg.typ.0,
        msg_ic: msg.ic.0,
        msg_tag: msg.tag.tag().0,
        remote_eid: msg.source.0,
        payload_size: payload_len,
    }
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
