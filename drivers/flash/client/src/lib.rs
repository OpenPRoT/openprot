// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use core::time::Duration;

use flash_api::{
    FlashError, FlashGeometry, FlashOp, FlashRegion, FlashRequestHeader, FlashResponseHeader,
};
pub use flash_api::MAX_PAYLOAD_SIZE;
use userspace::syscall;
use userspace::time::{Duration as KDuration, Instant, SystemClock};
use zerocopy::FromBytes;

/// Minimum buffer size required for Flash protocol messages.
/// Clients must provide request and response buffers of at least this size.
pub const MIN_BUFFER_SIZE: usize = 512;

/// Convenience macro to call a `_with_timeout` method using the client's default timeout.
/// 
/// Usage: `with_default_timeout!(self, method_name, arg1, arg2, ...)`
macro_rules! with_default_timeout {
    ($self:expr, $method:ident) => {{
        let timeout = $self.default_timeout;
        $self.$method(timeout)
    }};
    ($self:expr, $method:ident, $($arg:expr),+) => {{
        let timeout = $self.default_timeout;
        $self.$method($($arg,)* timeout)
    }};
}

/// Convert a public `Option<core::time::Duration>` into the kernel
/// `Instant` deadline used by the IPC syscall. `None` and any duration
/// that would overflow the clock both saturate to `Instant::MAX`
/// (block-forever). Kept private so the kernel clock type does not
/// appear in the public API.
fn deadline_from(timeout: Option<Duration>) -> Instant {
    let Some(d) = timeout else { return Instant::MAX };
    let millis = d.as_millis().min(i64::MAX as u128) as i64;
    let kd = KDuration::from_millis(millis);
    SystemClock::now()
        .checked_add_duration(kd)
        .unwrap_or(Instant::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    IpcError(pw_status::Error),
    ServerError(FlashError),
    InvalidResponse,
    BufferTooSmall,
}

impl From<pw_status::Error> for ClientError {
    fn from(e: pw_status::Error) -> Self {
        Self::IpcError(e)
    }
}

/// Flash driver client for performing read, write, erase, and discovery operations.
///
/// This client uses caller-provided buffers for request and response messages,
/// giving you full control over memory allocation and lifetime. This eliminates
/// hidden stack or heap allocations and is ideal for embedded and no_std environments.
///
/// # Buffer Requirements
/// You must provide two buffers of at least [`MIN_BUFFER_SIZE`] bytes each:
/// - `req`: Used to serialize outgoing requests to the server
/// - `resp`: Used to receive and parse responses from the server
///
/// Both buffers may be reused across multiple client operations. They do not need
/// to persist between calls. Consider a static buffer or placing them on the stack
/// in a scope that encompasses all flash operations.
///
/// # Timeout Behavior
/// A default timeout can be set at construction or updated later with
/// [`set_default_timeout`](Self::set_default_timeout). All methods without an explicit
/// timeout parameter will use this default. `None` means block until the server responds.
///
/// # Examples
/// ```ignore
/// let mut req = [0u8; MIN_BUFFER_SIZE];
/// let mut resp = [0u8; MIN_BUFFER_SIZE];
/// let mut client = FlashClient::new(FLASH_HANDLE, &mut req, &mut resp);
///
/// let capacity = client.capacity()?;
/// let mut data = [0u8; 256];
/// client.read(0x1000, &mut data)?;
/// ```
pub struct FlashClient<'a> {
    handle: u32,
    req: &'a mut [u8],
    resp: &'a mut [u8],
    default_timeout: Option<Duration>,
}

impl<'a> FlashClient<'a> {
    /// Create a new client with caller-provided buffers and no default timeout.
    ///
    /// # Panics
    /// Panics if either `req` or `resp` is smaller than [`MIN_BUFFER_SIZE`].
    pub fn new(handle: u32, req: &'a mut [u8], resp: &'a mut [u8]) -> Self {
        assert!(
            req.len() >= MIN_BUFFER_SIZE && resp.len() >= MIN_BUFFER_SIZE,
            "buffers must be at least {} bytes",
            MIN_BUFFER_SIZE
        );
        Self {
            handle,
            req,
            resp,
            default_timeout: None,
        }
    }

    /// Create a client with caller-provided buffers and a default timeout.
    ///
    /// # Panics
    /// Panics if either `req` or `resp` is smaller than [`MIN_BUFFER_SIZE`].
    pub fn with_default_timeout(
        handle: u32,
        req: &'a mut [u8],
        resp: &'a mut [u8],
        timeout: Option<Duration>,
    ) -> Self {
        assert!(
            req.len() >= MIN_BUFFER_SIZE && resp.len() >= MIN_BUFFER_SIZE,
            "buffers must be at least {} bytes",
            MIN_BUFFER_SIZE
        );
        Self {
            handle,
            req,
            resp,
            default_timeout: timeout,
        }
    }

    /// Update the default timeout used by `read`, `write`, `erase`, and
    /// the discovery calls when no explicit timeout is supplied.
    pub fn set_default_timeout(&mut self, timeout: Option<Duration>) {
        self.default_timeout = timeout;
    }

    /// Probe flash presence through the server.
    ///
    /// Returns `Ok(true)` when backend reports responsive flash,
    /// `Ok(false)` when backend reports no device present.
    pub fn exists(&mut self) -> Result<bool, ClientError> {
        let to = self.default_timeout;
        self.call_value(FlashOp::Exists, 0, 0, to).map(|v| v != 0)
    }

    /// Total bytes of flash exposed by the backend.
    pub fn capacity(&mut self) -> Result<u32, ClientError> {
        let to = self.default_timeout;
        self.call_value(FlashOp::GetCapacity, 0, 0, to)
    }

    /// Wire-side geometry: capacity, page size, supported erase
    /// granularities (bitmap), address width, capability flags. Uses
    /// the client's default timeout.
    pub fn geometry(&mut self) -> Result<FlashGeometry, ClientError> {
        with_default_timeout!(self, geometry_with_timeout)
    }

    pub fn geometry_with_timeout(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<FlashGeometry, ClientError> {
        let hdr = FlashRequestHeader::new(FlashOp::GetGeometry, 0, 0, 0);
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_geometry_response(&self.resp[..resp_len])
    }

    /// Discover the logical regions exposed by the backend. Writes up
    /// to `out.len()` records into `out` and returns the count.
    pub fn regions(&mut self, out: &mut [FlashRegion]) -> Result<usize, ClientError> {
        with_default_timeout!(self, regions_with_timeout, out)
    }

    pub fn regions_with_timeout(
        &mut self,
        out: &mut [FlashRegion],
        timeout: Option<Duration>,
    ) -> Result<usize, ClientError> {
        // Cap the requested count at what fits in MAX_PAYLOAD_SIZE.
        let max_records = MAX_PAYLOAD_SIZE / FlashRegion::SIZE;
        let request_len = out.len().min(max_records) as u32;
        let hdr = FlashRequestHeader::new(FlashOp::GetRegions, 0, request_len, 0);
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_regions_response(&self.resp[..resp_len], out)
    }

    /// Largest single read or write the backend will accept. Larger
    /// requests must be issued by the caller as a sequence of
    /// chunk-sized operations.
    ///
    /// This is a protocol constant (`MAX_PAYLOAD_SIZE`); no IPC is
    /// issued. The value is the same for every backend.
    pub const fn chunk_size() -> usize {
        MAX_PAYLOAD_SIZE
    }

    /// Read up to `out.len()` bytes starting at `address`, applying the
    /// client's default timeout. The caller is responsible for ensuring
    /// `out.len() <= chunk_size()`.
    pub fn read(&mut self, address: u32, out: &mut [u8]) -> Result<usize, ClientError> {
        with_default_timeout!(self, read_with_timeout, address, out)
    }

    /// Read up to `out.len()` bytes starting at `address`, bounded by
    /// `timeout`. `None` means block until the server responds.
    pub fn read_with_timeout(
        &mut self,
        address: u32,
        out: &mut [u8],
        timeout: Option<Duration>,
    ) -> Result<usize, ClientError> {
        if out.len() > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let hdr = FlashRequestHeader::new(FlashOp::Read, address, out.len() as u32, 0);
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_payload_response(&self.resp[..resp_len], out)
    }

    /// Write `data` starting at `address`, applying the client's default
    /// timeout. The caller is responsible for ensuring
    /// `data.len() <= chunk_size()`.
    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, ClientError> {
        with_default_timeout!(self, write_with_timeout, address, data)
    }

    /// Write `data` starting at `address`, bounded by `timeout`. `None`
    /// blocks until the server responds. The caller is responsible for
    /// ensuring `data.len() <= chunk_size()`.
    pub fn write_with_timeout(
        &mut self,
        address: u32,
        data: &[u8],
        timeout: Option<Duration>,
    ) -> Result<usize, ClientError> {
        if data.len() > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }
        if FlashRequestHeader::SIZE + data.len() > self.req.len() {
            return Err(ClientError::BufferTooSmall);
        }

        let hdr = FlashRequestHeader::new(
            FlashOp::Write,
            address,
            data.len() as u32,
            data.len() as u16,
        );
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
        self.req[FlashRequestHeader::SIZE..FlashRequestHeader::SIZE + data.len()]
            .copy_from_slice(data);

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE + data.len()],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_value_response(&self.resp[..resp_len]).map(|n| n as usize)
    }

    /// Erase `length` bytes starting at `address`, applying the client's
    /// default timeout. Both must be aligned to and a multiple of the
    /// backend's erase granule.
    pub fn erase(&mut self, address: u32, length: u32) -> Result<(), ClientError> {
        with_default_timeout!(self, erase_with_timeout, address, length)
    }

    /// Erase `length` bytes starting at `address`, bounded by `timeout`.
    /// `None` blocks until the server responds. Both must be aligned to
    /// and a multiple of the backend's erase granule.
    pub fn erase_with_timeout(
        &mut self,
        address: u32,
        length: u32,
        timeout: Option<Duration>,
    ) -> Result<(), ClientError> {
        let hdr = FlashRequestHeader::new(FlashOp::Erase, address, length, 0);
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_value_response(&self.resp[..resp_len]).map(|_| ())
    }

    fn call_value(
        &mut self,
        op: FlashOp,
        address: u32,
        length: u32,
        timeout: Option<Duration>,
    ) -> Result<u32, ClientError> {
        let hdr = FlashRequestHeader::new(op, address, length, 0);
        self.req[..FlashRequestHeader::SIZE]
            .copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));

        let resp_len = syscall::channel_transact(
            self.handle,
            &self.req[..FlashRequestHeader::SIZE],
            &mut self.resp[..],
            deadline_from(timeout),
        )?;

        parse_value_response(&self.resp[..resp_len])
    }
}

fn parse_value_response(resp: &[u8]) -> Result<u32, ClientError> {
    if resp.len() < FlashResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let hdr_bytes = &resp[..FlashResponseHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, FlashResponseHeader>::from_bytes(hdr_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };

    if hdr.is_success() {
        Ok(hdr.value_word())
    } else {
        Err(ClientError::ServerError(hdr.error_code()))
    }
}

fn parse_geometry_response(resp: &[u8]) -> Result<FlashGeometry, ClientError> {
    if resp.len() < FlashResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let hdr_bytes = &resp[..FlashResponseHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, FlashResponseHeader>::from_bytes(hdr_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };

    if !hdr.is_success() {
        return Err(ClientError::ServerError(hdr.error_code()));
    }

    let len = hdr.payload_length();
    if len != FlashGeometry::SIZE
        || resp.len() < FlashResponseHeader::SIZE + FlashGeometry::SIZE
    {
        return Err(ClientError::InvalidResponse);
    }

    let geom_bytes =
        &resp[FlashResponseHeader::SIZE..FlashResponseHeader::SIZE + FlashGeometry::SIZE];
    FlashGeometry::read_from_bytes(geom_bytes).map_err(|_| ClientError::InvalidResponse)
}

fn parse_regions_response(resp: &[u8], out: &mut [FlashRegion]) -> Result<usize, ClientError> {
    if resp.len() < FlashResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let hdr_bytes = &resp[..FlashResponseHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, FlashResponseHeader>::from_bytes(hdr_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };

    if !hdr.is_success() {
        return Err(ClientError::ServerError(hdr.error_code()));
    }

    let count = hdr.value_word() as usize;
    let payload_len = hdr.payload_length();
    if count > out.len()
        || payload_len != count * FlashRegion::SIZE
        || resp.len() < FlashResponseHeader::SIZE + payload_len
    {
        return Err(ClientError::InvalidResponse);
    }

    for i in 0..count {
        let offset = FlashResponseHeader::SIZE + i * FlashRegion::SIZE;
        let region_bytes = &resp[offset..offset + FlashRegion::SIZE];
        out[i] = FlashRegion::read_from_bytes(region_bytes)
            .map_err(|_| ClientError::InvalidResponse)?;
    }

    Ok(count)
}

fn parse_payload_response(resp: &[u8], out: &mut [u8]) -> Result<usize, ClientError> {
    if resp.len() < FlashResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let hdr_bytes = &resp[..FlashResponseHeader::SIZE];
    let Some(hdr) = zerocopy::Ref::<_, FlashResponseHeader>::from_bytes(hdr_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };

    if !hdr.is_success() {
        return Err(ClientError::ServerError(hdr.error_code()));
    }

    let len = hdr.payload_length();
    if len > out.len() || resp.len() < FlashResponseHeader::SIZE + len {
        return Err(ClientError::InvalidResponse);
    }

    out[..len].copy_from_slice(&resp[FlashResponseHeader::SIZE..FlashResponseHeader::SIZE + len]);
    Ok(len)
}
