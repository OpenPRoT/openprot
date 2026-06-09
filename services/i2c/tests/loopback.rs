// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test, no kernel: a consumer drives `I2cClient` purely
//! through the `embedded_hal::i2c::I2c` seam; `LoopbackTransport` routes the
//! *real* client encoders/decoders into `i2c_server::dispatch`, which replays
//! onto a mock bus. This exercises the exact marshalling the IPC client uses
//! — the property the structural template requires and that a hardwired-IPC
//! client would lose.
//!
//! Every assertion is made **through the seam only** (the bytes the consumer
//! reads back), never by reaching into the client/transport — so the client's
//! public surface stays `new()` + the `I2c` trait, nothing else.

use i2c_api::seam::{ErrorType, I2c, Operation, SevenBitAddress};
use i2c_client::I2cClient;
use i2c_server::loopback::LoopbackTransport;

/// Bus that encodes what it observed back into the read data, so the test can
/// verify address + write payload + op ordering purely from `rx`:
/// read byte 0 = address; the rest = the concatenated write bytes, cycled.
struct EchoBus;

#[derive(Debug)]
struct EchoErr;
impl i2c_api::seam::I2cBusError for EchoErr {
    fn kind(&self) -> i2c_api::seam::ErrorKind {
        i2c_api::seam::ErrorKind::Other
    }
}
impl ErrorType for EchoBus {
    type Error = EchoErr;
}
impl I2c<SevenBitAddress> for EchoBus {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Pass 1: gather all write bytes in operation order.
        let mut w = [0u8; 32];
        let mut wl = 0;
        for op in operations.iter() {
            if let Operation::Write(b) = op {
                w[wl..wl + b.len()].copy_from_slice(b);
                wl += b.len();
            }
        }
        // Pass 2: fill every read byte (continuing the counter across reads).
        let mut k = 0usize;
        for op in operations.iter_mut() {
            if let Operation::Read(r) = op {
                for byte in r.iter_mut() {
                    *byte = if k == 0 {
                        address
                    } else if wl > 0 {
                        w[(k - 1) % wl]
                    } else {
                        address
                    };
                    k += 1;
                }
            }
        }
        Ok(())
    }
}

#[test]
fn consumer_drives_client_over_loopback_no_kernel() {
    // Consumer only ever sees the embedded-hal seam.
    let mut client = I2cClient::new(LoopbackTransport::new(EchoBus));

    let mut rx = [0u8; 4];
    client.write_read(0x48, &[0xDE, 0xAD], &mut rx).unwrap();

    // rx[0] = address, rx[1..] = write bytes cycled — proves the address and
    // the write payload (in order) crossed the full client↔server path, and
    // the 4-byte read was scattered back correctly.
    assert_eq!(rx, [0x48, 0xDE, 0xAD, 0xDE]);
}

#[test]
fn multi_op_transaction_roundtrips_in_order() {
    let mut client = I2cClient::new(LoopbackTransport::new(EchoBus));

    let mut a = [0u8; 2];
    let mut b = [0u8; 3];
    client
        .transaction(
            0x20,
            &mut [
                Operation::Write(&[0x01]),
                Operation::Read(&mut a),
                Operation::Read(&mut b),
            ],
        )
        .unwrap();

    // Reads continue one counter across ops: a[0]=addr, then the single
    // write byte (0x01) repeats — proving multi-read scatter order.
    assert_eq!(a, [0x20, 0x01]);
    assert_eq!(b, [0x01, 0x01, 0x01]);
}
