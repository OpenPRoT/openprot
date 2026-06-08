// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Server side of the i2c userspace driver.
//!
//! Mirror image of `i2c_client`: the client marshals **one whole
//! `embedded_hal::i2c::I2c::transaction`** into a single request; this server
//! decodes it, replays the ordered op list on the real bus via *any*
//! `embedded_hal::i2c::I2c` implementation, and scatters the read results
//! back. One request ⇒ one `transaction` ⇒ one response: the exclusive-atomic
//! `I2c` contract is preserved across the process boundary, never fragmented.
//!
//! [`dispatch`] is a pure function (no `userspace`/IPC) and generic over the
//! bus, so it is unit-testable on the host with a mock `I2c`. The IPC loop
//! lives in [`runtime`]. One IPC channel per bus — see [`runtime::run`].
//!
//! Scope note: the wire protocol (`i2c_api::protocol`) carries only
//! `Transaction`. There is deliberately no slave/target or interrupt
//! machinery here — that does not exist in this protocol and is not copied
//! in from other drivers.

#![no_std]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
// Tests use .unwrap() on zerocopy::Ref of fixed-size buffers we just wrote —
// safe by construction, but clippy can't see that.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod loopback;
pub mod slave;

use i2c_api::seam::{ErrorKind, I2c, I2cBusError, NoAcknowledgeSource, Operation, SevenBitAddress};
use i2c_api::{
    I2cError, I2cOp, I2cOpDesc, I2cOpKind, I2cRequestHeader, I2cResponseHeader, MAX_OPS,
    MAX_PAYLOAD_SIZE,
};

/// One request/response buffer size. Matches the client's `MAX_BUF_SIZE`: a
/// whole transaction must fit one round-trip and is never fragmented.
pub const MAX_BUF_SIZE: usize = 512;

/// Map the embedded-hal error taxonomy onto the wire status code. The server
/// stays decoupled from any concrete backend: it only needs `B::Error: Error`.
pub(crate) fn kind_to_wire(kind: ErrorKind) -> I2cError {
    match kind {
        ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address) => I2cError::AddressNack,
        ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data) => I2cError::DataNack,
        ErrorKind::NoAcknowledge(_) => I2cError::Nack,
        ErrorKind::ArbitrationLoss => I2cError::ArbitrationLoss,
        ErrorKind::Bus => I2cError::Bus,
        ErrorKind::Overrun => I2cError::Overrun,
        _ => I2cError::InternalError,
    }
}

pub(crate) fn encode_error(response: &mut [u8], err: I2cError) -> usize {
    let hdr = I2cResponseHeader::error(err);
    response[..I2cResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
    I2cResponseHeader::SIZE
}

/// Success header with `payload_len` bytes already placed at
/// `response[I2cResponseHeader::SIZE..]` by the caller (0 for ack-only).
pub(crate) fn encode_ok(response: &mut [u8], payload_len: usize) -> usize {
    let hdr = I2cResponseHeader::success(payload_len as u16);
    response[..I2cResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
    I2cResponseHeader::SIZE + payload_len
}

/// Decode one wire request, replay it on `bus` as a single
/// `I2c::transaction`, and encode the response into `response`.
///
/// Returns the number of bytes written to `response` (always `>=
/// I2cResponseHeader::SIZE`). Never panics on malformed input — it answers
/// with an error header instead. Pure: no IPC, no globals.
pub fn dispatch<B>(bus: &mut B, request: &[u8], response: &mut [u8]) -> usize
where
    B: I2c<SevenBitAddress>,
{
    // ---- header ----
    if request.len() < I2cRequestHeader::SIZE {
        return encode_error(response, I2cError::InvalidOperation);
    }
    let Ok(hdr) =
        zerocopy::Ref::<_, I2cRequestHeader>::from_bytes(&request[..I2cRequestHeader::SIZE])
    else {
        return encode_error(response, I2cError::InvalidOperation);
    };
    if hdr.operation() != Ok(I2cOp::Transaction) {
        return encode_error(response, I2cError::InvalidOperation);
    }
    let address = hdr.address_value() as SevenBitAddress;
    let op_count = hdr.op_count_value();
    if op_count > MAX_OPS {
        return encode_error(response, I2cError::TooManyOperations);
    }

    let desc_bytes = op_count * I2cOpDesc::SIZE;
    if request.len() < I2cRequestHeader::SIZE + desc_bytes {
        return encode_error(response, I2cError::BufferTooSmall);
    }

    // ---- single pass: validate, size, and stash (kind, len) ----
    let desc_base = I2cRequestHeader::SIZE;
    let write_base = desc_base + desc_bytes;
    let mut write_total = 0usize;
    let mut read_total = 0usize;
    // Stash validated (is_read, len) pairs so the build pass needs no re-parse.
    // Using bool avoids matching a #[non_exhaustive] enum a second time.
    let mut op_meta = [(false, 0usize); MAX_OPS];
    for i in 0..op_count {
        let off = desc_base + i * I2cOpDesc::SIZE;
        let Ok(desc) =
            zerocopy::Ref::<_, I2cOpDesc>::from_bytes(&request[off..off + I2cOpDesc::SIZE])
        else {
            return encode_error(response, I2cError::InvalidOperation);
        };
        match desc.op_kind() {
            Ok(I2cOpKind::Write) => {
                op_meta[i] = (false, desc.length());
                write_total += desc.length();
            }
            Ok(I2cOpKind::Read) => {
                op_meta[i] = (true, desc.length());
                read_total += desc.length();
            }
            Ok(_) | Err(_) => return encode_error(response, I2cError::InvalidOperation),
        }
    }
    if write_total > MAX_PAYLOAD_SIZE
        || read_total > MAX_PAYLOAD_SIZE
        || request.len() < write_base + write_total
        || I2cResponseHeader::SIZE + read_total > response.len()
    {
        return encode_error(response, I2cError::BufferTooSmall);
    }

    // ---- build the Operation list from stashed metadata ----
    // Reads land in a private scratch (carved into disjoint &mut subslices in
    // op order) so the borrows of `request` (write data, shared) and the read
    // area never overlap. The filler `Operation::Write(&[])` is `'static`.
    let mut read_scratch = [0u8; MAX_PAYLOAD_SIZE];
    let mut ops: [Operation; MAX_OPS] = core::array::from_fn(|_| Operation::Write(&[]));

    let mut w_off = write_base;
    let mut read_rem: &mut [u8] = &mut read_scratch[..read_total];
    for (i, op) in ops.iter_mut().enumerate().take(op_count) {
        let (is_read, len) = op_meta[i];
        if is_read {
            let (head, tail) = read_rem.split_at_mut(len);
            *op = Operation::Read(head);
            read_rem = tail;
        } else {
            *op = Operation::Write(&request[w_off..w_off + len]);
            w_off += len;
        }
    }

    // ---- one transaction, run to completion ----
    if let Err(e) = bus.transaction(address, &mut ops[..op_count]) {
        return encode_error(response, kind_to_wire(e.kind()));
    }

    // ---- scatter reads back, in operation order ----
    let hdr = I2cResponseHeader::success(read_total as u16);
    response[..I2cResponseHeader::SIZE].copy_from_slice(zerocopy::IntoBytes::as_bytes(&hdr));
    response[I2cResponseHeader::SIZE..I2cResponseHeader::SIZE + read_total]
        .copy_from_slice(&read_scratch[..read_total]);
    I2cResponseHeader::SIZE + read_total
}

#[cfg(test)]
mod tests {
    use super::*;
    use i2c_api::seam::ErrorType;

    /// Heapless mock: records the first write + address, fills reads with a
    /// constant, or fails with a chosen `ErrorKind`.
    #[derive(Default)]
    struct MockBus {
        last_addr: u8,
        write_buf: [u8; 32],
        write_len: usize,
        read_fill: u8,
        fail: Option<ErrorKind>,
    }

    #[derive(Debug)]
    struct MockErr(ErrorKind);
    impl I2cBusError for MockErr {
        fn kind(&self) -> ErrorKind {
            self.0
        }
    }
    impl core::fmt::Display for MockErr {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl ErrorType for MockBus {
        type Error = MockErr;
    }
    impl I2c<SevenBitAddress> for MockBus {
        fn transaction(
            &mut self,
            address: SevenBitAddress,
            operations: &mut [Operation<'_>],
        ) -> Result<(), Self::Error> {
            if let Some(k) = self.fail {
                return Err(MockErr(k));
            }
            self.last_addr = address;
            for op in operations {
                match op {
                    Operation::Write(w) => {
                        if self.write_len == 0 {
                            self.write_buf[..w.len()].copy_from_slice(w);
                            self.write_len = w.len();
                        }
                    }
                    Operation::Read(r) => r.iter_mut().for_each(|b| *b = self.read_fill),
                }
            }
            Ok(())
        }
    }

    /// Build `[hdr][writeDesc][readDesc][write]` into `buf`, return its length.
    fn write_read_req(buf: &mut [u8], addr: u16, write: &[u8], read_len: u16) -> usize {
        let h = I2cRequestHeader::new(
            I2cOp::Transaction,
            addr,
            2,
            (2 * I2cOpDesc::SIZE as u16) + write.len() as u16,
        );
        let dw = I2cOpDesc::new(I2cOpKind::Write, write.len() as u16);
        let dr = I2cOpDesc::new(I2cOpKind::Read, read_len);
        let mut n = 0;
        for part in [
            zerocopy::IntoBytes::as_bytes(&h),
            zerocopy::IntoBytes::as_bytes(&dw),
            zerocopy::IntoBytes::as_bytes(&dr),
            write,
        ] {
            buf[n..n + part.len()].copy_from_slice(part);
            n += part.len();
        }
        n
    }

    #[test]
    fn write_then_read_roundtrips() {
        let mut bus = MockBus {
            read_fill: 0xAB,
            ..Default::default()
        };
        let mut req = [0u8; 512];
        let req_len = write_read_req(&mut req, 0x48, &[0x10, 0x20], 3);
        let mut resp = [0u8; 512];
        let n = dispatch(&mut bus, &req[..req_len], &mut resp);

        assert_eq!(bus.last_addr, 0x48);
        assert_eq!(&bus.write_buf[..bus.write_len], &[0x10, 0x20]);
        let rh =
            zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(&resp[..I2cResponseHeader::SIZE])
                .unwrap();
        assert!(rh.is_success());
        assert_eq!(rh.payload_length(), 3);
        assert_eq!(n, I2cResponseHeader::SIZE + 3);
        assert_eq!(
            &resp[I2cResponseHeader::SIZE..I2cResponseHeader::SIZE + 3],
            &[0xAB, 0xAB, 0xAB]
        );
    }

    #[test]
    fn bus_error_maps_to_wire_code() {
        let mut bus = MockBus {
            fail: Some(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)),
            ..Default::default()
        };
        let mut req = [0u8; 512];
        let req_len = write_read_req(&mut req, 0x50, &[0x00], 1);
        let mut resp = [0u8; 512];
        dispatch(&mut bus, &req[..req_len], &mut resp);
        let rh =
            zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(&resp[..I2cResponseHeader::SIZE])
                .unwrap();
        assert!(!rh.is_success());
        assert_eq!(rh.error_code(), I2cError::AddressNack);
    }

    #[test]
    fn short_request_is_rejected_not_panicked() {
        let mut bus = MockBus::default();
        let mut resp = [0u8; 512];
        let n = dispatch(&mut bus, &[0u8; 3], &mut resp);
        assert_eq!(n, I2cResponseHeader::SIZE);
        let rh =
            zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(&resp[..I2cResponseHeader::SIZE])
                .unwrap();
        assert_eq!(rh.error_code(), I2cError::InvalidOperation);
    }
}
