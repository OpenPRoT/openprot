// Licensed under the Apache-2.0 license

//! MCTP IPC Client
//!
//! Provides an `MctpClient` implementation that communicates with the MCTP
//! server over a Pigweed IPC channel, using the wire protocol from
//! `openprot-mctp-api`.
//!
//! This is the MCTP equivalent of `i2c_client::IpcI2cClient`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use openprot_mctp_client::IpcMctpClient;
//! use openprot_mctp_api::MctpClient;
//!
//! let client = IpcMctpClient::new(handle::MCTP);
//!
//! client.set_eid(8).unwrap();
//! let listener = client.listener(1).unwrap();
//! let meta = client.recv(listener, 0, &mut buf).unwrap();
//! ```

#![no_std]
#![warn(missing_docs)]

use core::cell::RefCell;

use openprot_mctp_api::wire::{self, MctpResponseHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};

/// Internal mutable state for the IPC client.
struct ClientBuffers {
    request_buf: [u8; MAX_REQUEST_SIZE],
    response_buf: [u8; MAX_RESPONSE_SIZE],
}

/// MCTP client that communicates with the MCTP server over Pigweed IPC.
///
/// Uses `RefCell` for interior mutability so that `MctpClient` trait
/// methods (which take `&self`) can mutate the internal IPC buffers.
/// This matches the Hubris pattern where IPC calls are logically
/// stateless from the caller's perspective.
pub struct IpcMctpClient {
    handle: u32,
    inner: RefCell<ClientBuffers>,
}

impl IpcMctpClient {
    /// Create a new IPC MCTP client bound to the given channel handle.
    ///
    /// The handle comes from the application's `app_package`-generated
    /// handle module (e.g., `handle::MCTP`).
    pub fn new(handle: u32) -> Self {
        Self {
            handle,
            inner: RefCell::new(ClientBuffers {
                request_buf: [0u8; MAX_REQUEST_SIZE],
                response_buf: [0u8; MAX_RESPONSE_SIZE],
            }),
        }
    }

    /// Get the IPC channel handle.
    pub fn channel_handle(&self) -> u32 {
        self.handle
    }

    /// Encode, send, and decode a transaction.
    fn transact(&self, req_len: usize) -> Result<(MctpResponseHeader, usize), MctpError> {
        let resp_len = self.send_recv(req_len)?;
        let inner = self.inner.borrow();

        if resp_len < MctpResponseHeader::SIZE {
            return Err(MctpError::from_code(ResponseCode::InternalError));
        }

        let header = wire::decode_response_header(&inner.response_buf[..resp_len])
            .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?;

        if !header.is_success() {
            return Err(MctpError::from_code(header.response_code()));
        }

        Ok((header, resp_len))
    }

    /// Platform-specific send/receive via Pigweed IPC.
    ///
    /// Uses `syscall::channel_transact` when built under Pigweed (Bazel).
    /// Returns a stub error when built under Cargo (no `userspace` crate).
    #[cfg(feature = "pigweed")]
    fn send_recv(&self, req_len: usize) -> Result<usize, MctpError> {
        let mut inner = self.inner.borrow_mut();
        userspace::syscall::channel_transact(
            self.handle,
            &inner.request_buf[..req_len],
            &mut inner.response_buf,
            userspace::time::Instant::MAX,
        )
        .map_err(|e| {
            pw_log::error!("IpcMctpClient: channel_transact failed: err={}", e as u32);
            MctpError::from_code(ResponseCode::InternalError)
        })
    }

    /// Stub for non-Pigweed builds (Cargo workspace).
    #[cfg(not(feature = "pigweed"))]
    fn send_recv(&self, _req_len: usize) -> Result<usize, MctpError> {
        Err(MctpError::from_code(ResponseCode::InternalError))
    }
}

impl MctpClient for IpcMctpClient {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            wire::encode_req(&mut inner.request_buf, eid)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, _) = self.transact(req_len)?;
        Ok(Handle(header.handle))
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            wire::encode_listener(&mut inner.request_buf, msg_type)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, _) = self.transact(req_len)?;
        Ok(Handle(header.handle))
    }

    fn get_eid(&self) -> u8 {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            match wire::encode_get_eid(&mut inner.request_buf) {
                Ok(len) => len,
                Err(_) => return 0,
            }
        };
        match self.transact(req_len) {
            Ok((header, _)) => header.eid,
            Err(_) => 0,
        }
    }

    fn set_eid(&self, eid: u8) -> Result<(), MctpError> {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            wire::encode_set_eid(&mut inner.request_buf, eid)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        self.transact(req_len)?;
        Ok(())
    }

    fn recv(
        &self,
        handle: Handle,
        timeout_millis: u32,
        buf: &mut [u8],
    ) -> Result<RecvMetadata, MctpError> {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            wire::encode_recv(&mut inner.request_buf, handle.0, timeout_millis)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, resp_len) = self.transact(req_len)?;

        let inner = self.inner.borrow();
        let payload = wire::get_response_payload(&inner.response_buf[..resp_len], &header)
            .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?;

        let copy_len = core::cmp::min(payload.len(), buf.len());
        buf[..copy_len].copy_from_slice(&payload[..copy_len]);

        Ok(RecvMetadata {
            msg_type: header.msg_type,
            msg_ic: header.flags & wire::flags::IC != 0,
            msg_tag: header.tag,
            remote_eid: header.eid,
            payload_size: payload.len(),
        })
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> Result<u8, MctpError> {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            wire::encode_send(
                &mut inner.request_buf,
                handle.map(|h| h.0),
                msg_type,
                eid,
                tag,
                integrity_check,
                buf,
            )
            .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, _) = self.transact(req_len)?;
        Ok(header.tag)
    }

    fn drop_handle(&self, handle: Handle) {
        let req_len = {
            let mut inner = self.inner.borrow_mut();
            match wire::encode_unbind(&mut inner.request_buf, handle.0) {
                Ok(len) => len,
                Err(_) => return,
            }
        };
        let _ = self.transact(req_len);
    }
}
