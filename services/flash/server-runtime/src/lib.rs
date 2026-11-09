// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Flash server IPC loop.
//!
//! Serves one flash device per IPC channel, such as one per chip select. The
//! channel a request arrives on picks the device, so the wire protocol carries
//! no device id and a client can only reach the devices it was given a channel
//! to. The server decides what a request means and builds the response frame.
//! This crate only moves the bytes.

#![no_std]

use hal_flash::Flash;
use openprot_flash_server::Server;
use pw_status::{Error, Result};
use userspace::time::Instant;
use util_error::ErrorCode;
use util_service::{ChannelId, Event, ServiceLoop};

pub use util_service::Channel;

/// Serve `servers[i]` on `channels[i]` through the WaitGroup `wg`.
///
/// `channels` must be non-empty, have distinct handles, and match `servers`
/// in length. `buf` holds one request and its reply, so it must fit the
/// largest of either plus the opcode or status header.
///
/// A request that cannot be served is answered with an error status and the
/// other channels keep being served. Only setup and WaitGroup failures return.
pub fn run<TFlash: Flash<Error = ErrorCode>>(
    wg: u32,
    channels: &mut [Channel],
    servers: &mut [Server<TFlash>],
    buf: &mut [u8],
) -> Result<()> {
    if channels.is_empty() || channels.len() != servers.len() {
        return Err(Error::InvalidArgument);
    }
    let mut service = ServiceLoop::new(wg, channels)?;
    loop {
        if let Event::Request { channel, len } = service.next(Instant::MAX, buf)? {
            serve(&mut service, servers, channel, buf, len);
        }
    }
}

fn serve<TFlash: Flash<Error = ErrorCode>>(
    service: &mut ServiceLoop<'_>,
    servers: &mut [Server<TFlash>],
    channel: ChannelId,
    buf: &mut [u8],
    len: usize,
) {
    let frame_len = match servers.get_mut(channel.index()) {
        Some(server) => server.handle_frame(buf, len),
        None => 0,
    };
    if service
        .reply(channel, buf.get(..frame_len).unwrap_or(&[]))
        .is_err()
    {
        pw_log::error!("flash server: reply failed");
    }
}
