// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP IPC Client
//!
//! Provides an `MctpClient` implementation that communicates with the MCTP
//! server over a Pigweed IPC channel, using the wire protocol from
//! `openprot-mctp-api`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use openprot_mctp_client_ipc::IpcMctpClient;
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

use core::cell::{Ref, RefCell, RefMut};

use mctp_lib::config;
use openprot_mctp_api::wire::{self, MctpRequestHeader, MctpResponseHeader};
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};

const STACK_MAX_PAYLOAD_SIZE: usize = config::MAX_PAYLOAD;
const MAX_REQUEST_SIZE: usize = MctpRequestHeader::SIZE + STACK_MAX_PAYLOAD_SIZE;
const MAX_RESPONSE_SIZE: usize = MctpResponseHeader::SIZE + STACK_MAX_PAYLOAD_SIZE;

/// Internal mutable state for the IPC client.
struct ClientBuffers {
    request_buf: [u8; MAX_REQUEST_SIZE],
    response_buf: [u8; MAX_RESPONSE_SIZE],
}

/// MCTP client that communicates with the MCTP server over Pigweed IPC.
///
/// Uses `RefCell` for interior mutability so that `MctpClient` trait
/// methods (which take `&self`) can mutate the internal IPC buffers.
/// This keeps the caller-facing API simple while enabling efficient
/// buffer reuse across synchronous IPC transactions.
pub struct IpcMctpClient {
    handle: u32,
    inner: RefCell<ClientBuffers>,
}

/// The error for a failure of the client itself: a request that does not fit its
/// buffer, or a use of the buffers while they are already in use.
fn internal_error() -> MctpError {
    MctpError::from_code(ResponseCode::InternalError)
}

/// Copy as much of `src` as fits in `dst`. Returns the number of bytes copied.
///
/// The copy has no length check that can panic, unlike `copy_from_slice`.
fn copy_prefix(dst: &mut [u8], src: &[u8]) -> usize {
    let mut copied = 0;
    for (d, s) in dst.iter_mut().zip(src) {
        *d = *s;
        copied += 1;
    }
    copied
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

    /// Borrow the buffers for writing. The buffers are in use only within one
    /// call, so this fails only if a call re-enters the client, and it then
    /// returns an error instead of panicking.
    fn buffers_mut(&self) -> Result<RefMut<'_, ClientBuffers>, MctpError> {
        self.inner.try_borrow_mut().map_err(|_| internal_error())
    }

    /// Borrow the buffers for reading; fails as [`Self::buffers_mut`] does.
    fn buffers(&self) -> Result<Ref<'_, ClientBuffers>, MctpError> {
        self.inner.try_borrow().map_err(|_| internal_error())
    }

    /// Encode, send, and decode a transaction.
    fn transact(&self, req_len: usize) -> Result<(MctpResponseHeader, usize), MctpError> {
        let resp_len = self.send_recv(req_len)?;
        let inner = self.buffers()?;

        if resp_len < MctpResponseHeader::SIZE {
            return Err(internal_error());
        }
        let response = inner
            .response_buf
            .get(..resp_len)
            .ok_or_else(internal_error)?;

        let header = wire::decode_response_header(response).map_err(|_| internal_error())?;

        if !header.is_success() {
            return Err(MctpError::from_code(header.response_code()));
        }

        Ok((header, resp_len))
    }

    fn send_recv(&self, req_len: usize) -> Result<usize, MctpError> {
        let mut inner = self.buffers_mut()?;
        // The request and the response are separate fields, so both can be
        // borrowed at once, one shared and one mutable.
        let ClientBuffers {
            request_buf,
            response_buf,
        } = &mut *inner;
        let request = request_buf.get(..req_len).ok_or_else(internal_error)?;
        userspace::syscall::channel_transact(
            self.handle,
            request,
            response_buf,
            userspace::time::Instant::MAX,
        )
        .map_err(|_| internal_error())
    }
}

impl MctpClient for IpcMctpClient {
    fn req(&self, eid: u8) -> Result<Handle, MctpError> {
        let req_len = {
            let mut inner = self.buffers_mut()?;
            wire::encode_req(&mut inner.request_buf, eid)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, _) = self.transact(req_len)?;
        Ok(Handle(header.handle))
    }

    fn listener(&self, msg_type: u8) -> Result<Handle, MctpError> {
        let req_len = {
            let mut inner = self.buffers_mut()?;
            wire::encode_listener(&mut inner.request_buf, msg_type)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, _) = self.transact(req_len)?;
        Ok(Handle(header.handle))
    }

    fn get_eid(&self) -> u8 {
        let req_len = {
            let Ok(mut inner) = self.buffers_mut() else {
                return 0;
            };
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
            let mut inner = self.buffers_mut()?;
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
            let mut inner = self.buffers_mut()?;
            wire::encode_recv(&mut inner.request_buf, handle.0, timeout_millis)
                .map_err(|_| MctpError::from_code(ResponseCode::InternalError))?
        };
        let (header, resp_len) = self.transact(req_len)?;

        let inner = self.buffers()?;
        let response = inner
            .response_buf
            .get(..resp_len)
            .ok_or_else(internal_error)?;
        let payload =
            wire::get_response_payload(response, &header).map_err(|_| internal_error())?;

        // A payload longer than `buf` is cut to fit; `payload_size` below still
        // reports its full length.
        copy_prefix(buf, payload);

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
            let mut inner = self.buffers_mut()?;
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
            let Ok(mut inner) = self.buffers_mut() else {
                return;
            };
            match wire::encode_unbind(&mut inner.request_buf, handle.0) {
                Ok(len) => len,
                Err(_) => return,
            }
        };
        let _ = self.transact(req_len);
    }
}
