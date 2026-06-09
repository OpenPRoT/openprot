// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Pure dispatch for the **stateless** slave device ops:
//! `ConfigureSlave` / `EnableSlave` / `DisableSlave`.
//!
//! These map 1:1 onto the `openprot_hal_blocking::i2c_hardware::slave`
//! seam, so this function is generic over it (the SoC backend is the only
//! thing that names silicon) and host-testable with a mock slave — exactly
//! like the master `dispatch`.
//!
//! The runtime-stateful slave bits — arming notification, the IRQ→drain→USER
//! wake, and `SlaveReceive` returning the latched buffer — are **not** here:
//! they need per-bus runtime state and live in `i2c-server-runtime`
//! (kernel). One IPC channel per bus ⇒ no bus field on the wire.

use i2c_api::seam::{I2cBusError, I2cSlaveCore, SevenBitAddress};
use i2c_api::{I2cError, I2cOp, I2cRequestHeader};

use crate::{encode_error, encode_ok, kind_to_wire};

/// Decode one slave-control request and apply it to `slave`. Returns the
/// encoded response length. Never panics on malformed input.
///
/// Handles only the stateless device ops; `SlaveReceive` and the
/// notification arm/disarm ops are the runtime's responsibility and yield
/// `InvalidOperation` if they reach here.
pub fn dispatch_slave<S>(slave: &mut S, request: &[u8], response: &mut [u8]) -> usize
where
    S: I2cSlaveCore<SevenBitAddress>,
{
    if request.len() < I2cRequestHeader::SIZE {
        return encode_error(response, I2cError::InvalidOperation);
    }
    let Ok(hdr) =
        zerocopy::Ref::<_, I2cRequestHeader>::from_bytes(&request[..I2cRequestHeader::SIZE])
    else {
        return encode_error(response, I2cError::InvalidOperation);
    };

    let result = match hdr.operation() {
        Ok(I2cOp::ConfigureSlave) => {
            slave.configure_slave_address(hdr.address_value() as SevenBitAddress)
        }
        Ok(I2cOp::EnableSlave) => slave.enable_slave_mode(),
        Ok(I2cOp::DisableSlave) => slave.disable_slave_mode(),
        _ => return encode_error(response, I2cError::InvalidOperation),
    };

    match result {
        Ok(()) => encode_ok(response, 0),
        Err(e) => encode_error(response, kind_to_wire(e.kind())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use i2c_api::seam::ErrorType;
    use i2c_api::I2cResponseHeader;

    /// Minimal mock slave: records calls; never errors.
    #[derive(Default)]
    struct MockSlave {
        addr: Option<u8>,
        enabled: bool,
    }
    #[derive(Debug)]
    struct E;
    impl I2cBusError for E {
        fn kind(&self) -> i2c_api::seam::ErrorKind {
            i2c_api::seam::ErrorKind::Other
        }
    }
    impl ErrorType for MockSlave {
        type Error = E;
    }
    impl I2cSlaveCore<SevenBitAddress> for MockSlave {
        fn configure_slave_address(&mut self, a: SevenBitAddress) -> Result<(), E> {
            self.addr = Some(a);
            Ok(())
        }
        fn enable_slave_mode(&mut self) -> Result<(), E> {
            self.enabled = true;
            Ok(())
        }
        fn disable_slave_mode(&mut self) -> Result<(), E> {
            self.enabled = false;
            Ok(())
        }
        fn is_slave_mode_enabled(&self) -> bool {
            self.enabled
        }
        fn slave_address(&self) -> Option<SevenBitAddress> {
            self.addr
        }
    }

    fn req(op: I2cOp, addr: u16) -> [u8; I2cRequestHeader::SIZE] {
        let h = I2cRequestHeader::new(op, addr, 0, 0);
        let mut b = [0u8; I2cRequestHeader::SIZE];
        b.copy_from_slice(zerocopy::IntoBytes::as_bytes(&h));
        b
    }
    fn ok(resp: &[u8]) -> bool {
        zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(&resp[..I2cResponseHeader::SIZE])
            .unwrap()
            .is_success()
    }

    #[test]
    fn configure_enable_disable_apply_to_device() {
        let mut s = MockSlave::default();
        let mut r = [0u8; 64];

        let n = dispatch_slave(&mut s, &req(I2cOp::ConfigureSlave, 0x42), &mut r);
        assert!(ok(&r[..n]));
        assert_eq!(s.addr, Some(0x42));

        let n = dispatch_slave(&mut s, &req(I2cOp::EnableSlave, 0), &mut r);
        assert!(ok(&r[..n]));
        assert!(s.enabled);

        let n = dispatch_slave(&mut s, &req(I2cOp::DisableSlave, 0), &mut r);
        assert!(ok(&r[..n]));
        assert!(!s.enabled);
    }

    #[test]
    fn runtime_owned_and_malformed_ops_rejected() {
        let mut s = MockSlave::default();
        let mut r = [0u8; 64];
        // SlaveReceive is runtime-handled — not a device op here.
        let n = dispatch_slave(&mut s, &req(I2cOp::SlaveReceive, 0), &mut r);
        let h = zerocopy::Ref::<_, I2cResponseHeader>::from_bytes(&r[..I2cResponseHeader::SIZE])
            .unwrap();
        assert_eq!(h.error_code(), I2cError::InvalidOperation);
        let _ = n;

        // Too-short request: rejected, no panic.
        let n2 = dispatch_slave(&mut s, &[0u8; 3], &mut r);
        assert_eq!(n2, I2cResponseHeader::SIZE);
    }
}
