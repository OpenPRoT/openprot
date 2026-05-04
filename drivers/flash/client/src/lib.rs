// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use core::time::Duration;

use flash_api::{
    FlashError, FlashGeometry, FlashOp, FlashRegion, FlashRequestHeader, FlashResponseHeader,
};
pub use flash_api::MAX_PAYLOAD_SIZE;
use userspace::syscall;
use userspace::time::{Clock, Duration as KDuration, Instant, SystemClock};
use zerocopy::FromBytes;

const MAX_BUF_SIZE: usize = 512;

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

pub struct FlashClient {
    handle: u32,
    req: [u8; MAX_BUF_SIZE],
    resp: [u8; MAX_BUF_SIZE],
    default_timeout: Option<Duration>,
}

impl FlashClient {
    /// Build a client with no default timeout. Calls without an explicit
    /// timeout will block until the server responds.
    pub const fn new(handle: u32) -> Self {
        Self {
            handle,
            req: [0u8; MAX_BUF_SIZE],
            resp: [0u8; MAX_BUF_SIZE],
            default_timeout: None,
        }
    }

    /// Build a client with a default timeout applied to calls that don't
    /// specify one. `None` means block forever.
    pub const fn with_default_timeout(handle: u32, timeout: Option<Duration>) -> Self {
        Self {
            handle,
            req: [0u8; MAX_BUF_SIZE],
            resp: [0u8; MAX_BUF_SIZE],
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
        let to = self.default_timeout;
        self.geometry_with_timeout(to)
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
            &mut self.resp,
            deadline_from(timeout),
        )?;

        parse_geometry_response(&self.resp[..resp_len])
    }

    /// Discover the logical regions exposed by the backend. Writes up
    /// to `out.len()` records into `out` and returns the count.
    pub fn regions(&mut self, out: &mut [FlashRegion]) -> Result<usize, ClientError> {
        let to = self.default_timeout;
        self.regions_with_timeout(out, to)
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
            &mut self.resp,
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
        let to = self.default_timeout;
        self.read_with_timeout(address, out, to)
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
            &mut self.resp,
            deadline_from(timeout),
        )?;

        parse_payload_response(&self.resp[..resp_len], out)
    }

    /// Write `data` starting at `address`, applying the client's default
    /// timeout. The caller is responsible for ensuring
    /// `data.len() <= chunk_size()`.
    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<usize, ClientError> {
        let to = self.default_timeout;
        self.write_with_timeout(address, data, to)
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
            &mut self.resp,
            deadline_from(timeout),
        )?;

        parse_value_response(&self.resp[..resp_len]).map(|n| n as usize)
    }

    /// Erase `length` bytes starting at `address`, applying the client's
    /// default timeout. Both must be aligned to and a multiple of the
    /// backend's erase granule.
    pub fn erase(&mut self, address: u32, length: u32) -> Result<(), ClientError> {
        let to = self.default_timeout;
        self.erase_with_timeout(address, length, to)
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
            &mut self.resp,
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
            &mut self.resp,
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
