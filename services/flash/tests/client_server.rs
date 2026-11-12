// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The real flash client and server talking over an in-memory transport,
//! with no kernel and no IPC. The flash behind the server is RAM.

use core::cell::RefCell;
use core::num::NonZero;
use std::rc::Rc;

use hal_flash::{Flash, FlashAddress};
use openprot_flash_client::{FlashClient, FlashTransport};
use openprot_flash_server::{Outcome, Server};
use util_error::{self as error, ErrorCode};
use util_types::PowerOf2Usize;

const TOTAL: usize = 0x1_0000;
const SECTOR: usize = 4096;

/// Panics on a request the server should have rejected before it got here.
struct RamFlash {
    data: Vec<u8>,
}

impl Flash for RamFlash {
    type Error = ErrorCode;

    fn geometry(&mut self) -> Result<(NonZero<usize>, PowerOf2Usize, u32), ErrorCode> {
        let total = NonZero::new(self.data.len()).unwrap();
        Ok((total, PowerOf2Usize::new(SECTOR).unwrap(), 1 << 12))
    }

    fn read(&mut self, addr: FlashAddress, buf: &mut [u8]) -> Result<(), ErrorCode> {
        let start = addr.offset() as usize;
        buf.copy_from_slice(&self.data[start..start + buf.len()]);
        Ok(())
    }

    fn erase(&mut self, addr: FlashAddress, size: PowerOf2Usize) -> Result<(), ErrorCode> {
        let start = addr.offset() as usize;
        assert_eq!(start % size.get(), 0);
        self.data[start..start + size.get()].fill(0xff);
        Ok(())
    }

    fn program(&mut self, addr: FlashAddress, data: &[u8]) -> Result<(), ErrorCode> {
        let start = addr.offset() as usize;
        for (dst, src) in self.data[start..start + data.len()].iter_mut().zip(data) {
            *dst &= *src;
        }
        Ok(())
    }
}

/// Joins a request's pieces, runs the server on them, and splits the response
/// across the caller's pieces, as the kernel channel does.
struct Loopback {
    server: Server<RamFlash>,
    scratch: Vec<u8>,
}

impl FlashTransport for Loopback {
    fn transact(&mut self, send: &[&[u8]], recv: &mut [&mut [u8]]) -> Result<usize, ErrorCode> {
        let mut len = 0;
        for piece in send {
            self.scratch[len..len + piece.len()].copy_from_slice(piece);
            len += piece.len();
        }
        let Outcome::Done(frame_len) = self.server.handle_frame(&mut self.scratch, len) else {
            panic!("RamFlash's erase never defers");
        };

        let mut rest = &self.scratch[..frame_len];
        for piece in recv.iter_mut() {
            let take = piece.len().min(rest.len());
            let (head, tail) = rest.split_at(take);
            piece[..take].copy_from_slice(head);
            rest = tail;
        }
        if !rest.is_empty() {
            return Err(error::IPC_ERROR_RSP_TOO_LARGE);
        }
        Ok(frame_len)
    }
}

/// Reports that `received` bytes came back without writing any of them.
struct Silent {
    received: usize,
}

impl FlashTransport for Silent {
    fn transact(&mut self, _: &[&[u8]], _: &mut [&mut [u8]]) -> Result<usize, ErrorCode> {
        Ok(self.received)
    }
}

/// Delivers every response intact but misreports its length by `delta`, once
/// the client has connected.
struct Skewed {
    inner: Loopback,
    delta: isize,
    calls: u32,
}

impl FlashTransport for Skewed {
    fn transact(&mut self, send: &[&[u8]], recv: &mut [&mut [u8]]) -> Result<usize, ErrorCode> {
        let len = self.inner.transact(send, recv)?;
        self.calls += 1;
        Ok(if self.calls == 1 {
            len
        } else {
            len.checked_add_signed(self.delta).unwrap()
        })
    }
}

type Servers = Rc<RefCell<Vec<Server<RamFlash>>>>;

/// One channel of a server that serves one device per channel. Every channel
/// shares the same devices; `device` is the one this channel reaches.
struct Channel {
    servers: Servers,
    device: usize,
    scratch: Vec<u8>,
}

impl FlashTransport for Channel {
    fn transact(&mut self, send: &[&[u8]], recv: &mut [&mut [u8]]) -> Result<usize, ErrorCode> {
        let mut len = 0;
        for piece in send {
            self.scratch[len..len + piece.len()].copy_from_slice(piece);
            len += piece.len();
        }
        let frame_len = match self.servers.borrow_mut().get_mut(self.device) {
            Some(server) => match server.handle_frame(&mut self.scratch, len) {
                Outcome::Done(frame_len) => frame_len,
                Outcome::Pending => panic!("RamFlash's erase never defers"),
            },
            None => 0,
        };

        let mut rest = &self.scratch[..frame_len];
        for piece in recv.iter_mut() {
            let take = piece.len().min(rest.len());
            let (head, tail) = rest.split_at(take);
            piece[..take].copy_from_slice(head);
            rest = tail;
        }
        Ok(frame_len)
    }
}

/// Devices of different sizes behind one server, and a channel to each.
fn two_devices() -> (Servers, [Channel; 2]) {
    let sizes = [TOTAL, 2 * TOTAL];
    let servers: Vec<_> = sizes
        .iter()
        .map(|&size| {
            Server::new(RamFlash {
                data: vec![0xff; size],
            })
        })
        .collect();
    let servers = Rc::new(RefCell::new(servers));
    let channel = |device| Channel {
        servers: servers.clone(),
        device,
        scratch: vec![0; 8192],
    };
    let channels = [channel(0), channel(1)];
    (servers, channels)
}

fn loopback() -> Loopback {
    let server = Server::new(RamFlash {
        data: vec![0xff; TOTAL],
    });
    Loopback {
        server,
        scratch: vec![0; 8192],
    }
}

fn client() -> FlashClient<Loopback> {
    FlashClient::new(loopback()).unwrap()
}

fn sector() -> PowerOf2Usize {
    PowerOf2Usize::new(SECTOR).unwrap()
}

#[test]
fn the_client_learns_the_geometry_from_the_server() {
    let mut client = client();
    let (total, page, bitmap) = client.geometry().unwrap();
    assert_eq!(total.get(), TOTAL);
    assert_eq!(page.get(), SECTOR);
    assert_eq!(bitmap, 1 << 12);
}

#[test]
fn programmed_data_reads_back_and_the_rest_of_the_sector_is_untouched() {
    let mut client = client();
    let base = FlashAddress::new(0x1000);
    let data: Vec<u8> = (0..300).map(|i| i as u8).collect();

    client.erase(base, sector()).unwrap();
    client.program(base + 250, &data).unwrap();

    let mut back = vec![0u8; 600];
    client.read(base, &mut back).unwrap();
    assert!(back[..250].iter().all(|&b| b == 0xff));
    assert_eq!(&back[250..550], &data[..]);
    assert!(back[550..].iter().all(|&b| b == 0xff));
}

#[test]
fn erase_returns_a_programmed_block_to_ones() {
    let mut client = client();
    let base = FlashAddress::new(0x2000);

    client.program(base, &[0; 16]).unwrap();
    client.erase(base, sector()).unwrap();

    let mut back = [0u8; 16];
    client.read(base, &mut back).unwrap();
    assert_eq!(back, [0xff; 16]);
}

#[test]
fn a_misaligned_erase_reaches_the_client_as_the_servers_error() {
    let mut client = client();
    let result = client.erase(FlashAddress::new(0x1100), sector());
    assert_eq!(result, Err(error::FLASH_GENERIC_ERASE_INVALID_ADDR));
}

#[test]
fn an_unsupported_erase_size_reaches_the_client_as_the_servers_error() {
    let mut client = client();
    let small = PowerOf2Usize::new(512).unwrap();
    let result = client.erase(FlashAddress::new(0), small);
    assert_eq!(result, Err(error::FLASH_GENERIC_ERASE_INVALID_SIZE));
}

#[test]
fn a_read_past_the_end_reaches_the_client_as_the_servers_error() {
    let mut client = client();
    let mut buf = [0u8; 16];
    let result = client.read(FlashAddress::new(TOTAL as u32 - 8), &mut buf);
    assert_eq!(result, Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS));
    assert_eq!(buf, [0u8; 16]);
}

#[test]
fn a_response_with_no_status_is_an_error_not_a_success() {
    let result = FlashClient::new(Silent { received: 0 });
    assert!(matches!(result, Err(e) if e == error::IPC_ERROR_RSP_BAD_LEN));
}

#[test]
fn a_read_response_that_is_too_short_is_an_error_not_stale_data() {
    let transport = Skewed {
        inner: loopback(),
        delta: -1,
        calls: 0,
    };
    let mut client = FlashClient::new(transport).unwrap();
    let mut buf = [0u8; 16];
    let result = client.read(FlashAddress::new(0), &mut buf);
    assert_eq!(result, Err(error::IPC_ERROR_RSP_BAD_LEN));
}

#[test]
fn an_erase_response_that_is_too_long_is_an_error() {
    let transport = Skewed {
        inner: loopback(),
        delta: 1,
        calls: 0,
    };
    let mut client = FlashClient::new(transport).unwrap();
    let result = client.erase(FlashAddress::new(0), sector());
    assert_eq!(result, Err(error::IPC_ERROR_RSP_BAD_LEN));
}

#[test]
fn each_channel_reports_the_geometry_of_its_own_device() {
    let (_servers, [first, second]) = two_devices();
    let mut first = FlashClient::new(first).unwrap();
    let mut second = FlashClient::new(second).unwrap();

    assert_eq!(first.geometry().unwrap().0.get(), TOTAL);
    assert_eq!(second.geometry().unwrap().0.get(), 2 * TOTAL);
}

#[test]
fn a_write_on_one_channel_does_not_reach_the_other_device() {
    let (_servers, [first, second]) = two_devices();
    let mut first = FlashClient::new(first).unwrap();
    let mut second = FlashClient::new(second).unwrap();
    let addr = FlashAddress::new(0x1000);

    first.program(addr, &[0x12; 16]).unwrap();

    let mut on_first = [0u8; 16];
    let mut on_second = [0u8; 16];
    first.read(addr, &mut on_first).unwrap();
    second.read(addr, &mut on_second).unwrap();
    assert_eq!(on_first, [0x12; 16]);
    assert_eq!(on_second, [0xff; 16]);
}

#[test]
fn a_range_valid_on_a_larger_device_is_rejected_on_a_smaller_one() {
    let (_servers, [first, second]) = two_devices();
    let mut first = FlashClient::new(first).unwrap();
    let mut second = FlashClient::new(second).unwrap();
    let past_first = FlashAddress::new(TOTAL as u32 + 0x100);
    let mut buf = [0u8; 16];

    assert_eq!(
        first.read(past_first, &mut buf),
        Err(error::FLASH_GENERIC_ADDR_OUT_OF_BOUNDS)
    );
    assert!(second.read(past_first, &mut buf).is_ok());
}

#[test]
fn a_channel_with_no_device_behind_it_gets_no_frame() {
    let (servers, [first, _]) = two_devices();
    let orphan = Channel {
        servers,
        device: 2,
        scratch: first.scratch,
    };
    let result = FlashClient::new(orphan);
    assert!(matches!(result, Err(e) if e == error::IPC_ERROR_RSP_BAD_LEN));
}
