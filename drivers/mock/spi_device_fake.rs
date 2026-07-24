// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

extern crate std;

use std::borrow::Cow;
use std::cell::Ref;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use util_error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FakeSpiError(pub ErrorCode);

impl embedded_hal::spi::Error for FakeSpiError {
    fn kind(&self) -> embedded_hal::spi::ErrorKind {
        embedded_hal::spi::ErrorKind::Other
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeSpiTransfer {
    pub tx: Vec<u8>,
    pub rx: Vec<u8>,
}

pub struct ResponseProgram<'a, TError> {
    pub prefix: Cow<'a, [u8]>,
    pub response: Response<'a, TError>,
}

pub enum Response<'a, TError> {
    Data(Cow<'a, [u8]>),
    Error(TError),
}

#[derive(Clone)]
pub struct FakeSpiDevice<'a> {
    log: Rc<RefCell<Vec<FakeSpiTransfer>>>,
    response_programs: Rc<RefCell<Vec<ResponseProgram<'a, FakeSpiError>>>>,
}

impl<'a> FakeSpiDevice<'a> {
    pub fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(vec![])),
            response_programs: Rc::new(RefCell::new(vec![])),
        }
    }

    /// Adds a programmed response to the fake device. If the SPI request
    /// matches `prefix`, then `response` will be received after `prefix.len()`
    /// zeroes.
    pub fn preprogram_data_response(&self, prefix: Cow<'a, [u8]>, response: Cow<'a, [u8]>) {
        self.response_programs.borrow_mut().push(ResponseProgram {
            prefix,
            response: Response::Data(response),
        })
    }

    /// Adds a programmed error response to the fake device. If the SPI request
    /// matches `prefix`, then the supplied error will be returned.
    pub fn preprogram_error_response(&self, prefix: Cow<'a, [u8]>, err: ErrorCode) {
        self.response_programs.borrow_mut().push(ResponseProgram {
            prefix,
            response: Response::Error(FakeSpiError(err)),
        })
    }

    pub fn log(&self) -> impl Deref<Target = [FakeSpiTransfer]> + use<'a, '_> {
        Ref::map(self.log.borrow(), |log| log.as_slice())
    }

    pub fn assert_all_expectations_met(&self) {
        assert!(
            self.response_programs.borrow().is_empty(),
            "Not all expected SPI response programs were executed: remaining = {}",
            self.response_programs.borrow().len()
        );
    }
}

impl Default for FakeSpiDevice<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl embedded_hal::spi::ErrorType for FakeSpiDevice<'_> {
    type Error = FakeSpiError;
}

impl embedded_hal::spi::SpiDevice for FakeSpiDevice<'_> {
    fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        let mut tx = Vec::new();
        let mut read_ops = Vec::new();

        for op in operations.iter_mut() {
            match op {
                embedded_hal::spi::Operation::Write(buf) => {
                    tx.extend_from_slice(buf);
                }
                embedded_hal::spi::Operation::Read(buf) => {
                    read_ops.push(buf);
                }
                embedded_hal::spi::Operation::Transfer(read_buf, write_buf) => {
                    tx.extend_from_slice(write_buf);
                    read_ops.push(read_buf);
                }
                embedded_hal::spi::Operation::TransferInPlace(buf) => {
                    tx.extend_from_slice(buf);
                    // TransferInPlace is both write and read. In SpiFlash this is not typically used,
                    // but we treat it as read mapping to the same buffer.
                    read_ops.push(buf);
                }
                embedded_hal::spi::Operation::DelayNs(_) => {}
            }
        }

        let mut response_programs = self.response_programs.borrow_mut();
        let Some(response_index) = response_programs
            .iter()
            .position(|resp| tx.starts_with(&resp.prefix))
        else {
            panic!("Unexpected SPI transaction: tx = {tx:x?}");
        };

        let program = response_programs.remove(response_index);

        match program.response {
            Response::Data(data) => {
                let skipped_resp_bytes = tx.len() - program.prefix.len();
                let total_rx_requested: usize = read_ops.iter().map(|buf| buf.len()).sum();

                if data.len() != total_rx_requested + skipped_resp_bytes {
                    panic!(
                        "Expected transaction with prefix {:x?} to return {} bytes, but a response of len {} was requested ({} bytes skipped). Simulated data len = {}",
                        program.prefix,
                        data.len(),
                        total_rx_requested,
                        skipped_resp_bytes,
                        data.len()
                    );
                }

                let mut data_offset = skipped_resp_bytes;
                for buf in read_ops {
                    let len = buf.len();
                    buf.copy_from_slice(&data[data_offset..data_offset + len]);
                    data_offset += len;
                }

                let mut rx = vec![0; program.prefix.len()];
                rx.extend_from_slice(&data);

                let mut logged_tx = tx.clone();
                if logged_tx.len() < rx.len() {
                    logged_tx.resize(rx.len(), 0);
                }

                self.log
                    .borrow_mut()
                    .push(FakeSpiTransfer { tx: logged_tx, rx });
                Ok(())
            }
            Response::Error(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use embedded_hal::spi::{Operation, SpiDevice};
    use util_error::FLASH_GENERIC_BUSY;

    #[test]
    #[should_panic(expected = "Unexpected SPI transaction")]
    fn test_no_expectations() {
        let mut spi = FakeSpiDevice::new();
        let mut rx = [0; 4];
        spi.transaction(&mut [Operation::Write(b"hi"), Operation::Read(&mut rx)])
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "Unexpected SPI transaction")]
    fn test_no_matching_expectations() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hola".into(), b"adios".into());
        let mut rx = [0; 4];
        spi.transaction(&mut [Operation::Write(b"hi"), Operation::Read(&mut rx)])
            .unwrap();
    }

    #[test]
    fn test_matching_expectations_exact_req_len() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hola".into(), b"adios".into());
        spi.preprogram_data_response(b"hi".into(), b"goodbye".into());

        let mut rx1 = [0; 7];
        spi.transaction(&mut [Operation::Write(b"hi"), Operation::Read(&mut rx1)])
            .unwrap();
        assert_eq!(b"goodbye", &rx1);

        let mut rx2 = [0; 5];
        spi.transaction(&mut [Operation::Write(b"hola"), Operation::Read(&mut rx2)])
            .unwrap();
        assert_eq!(b"adios", &rx2);

        assert_eq!(
            *spi.log(),
            [
                FakeSpiTransfer {
                    tx: b"hi\0\0\0\0\0\0\0".into(),
                    rx: b"\0\0goodbye".into(),
                },
                FakeSpiTransfer {
                    tx: b"hola\0\0\0\0\0".into(),
                    rx: b"\0\0\0\0adios".into(),
                }
            ]
        );
        spi.assert_all_expectations_met();
    }

    #[test]
    fn test_matching_expectations_nonexact_req_len() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hi".into(), b"goodbye".into());
        let mut rx = [0; 6];
        spi.transaction(&mut [Operation::Write(b"hi!"), Operation::Read(&mut rx)])
            .unwrap();
        assert_eq!(b"oodbye", &rx);
        assert_eq!(
            *spi.log(),
            [FakeSpiTransfer {
                tx: b"hi!\0\0\0\0\0\0".into(),
                rx: b"\0\0goodbye".into(),
            }]
        );
        spi.assert_all_expectations_met();
    }

    #[test]
    fn test_matching_expectations_error() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_error_response(b"hi".into(), FLASH_GENERIC_BUSY);
        let mut rx = [0; 6];
        assert_eq!(
            spi.transaction(&mut [Operation::Write(b"hi!"), Operation::Read(&mut rx)]),
            Err(FakeSpiError(FLASH_GENERIC_BUSY))
        );
    }

    #[test]
    #[should_panic(
        expected = "Expected transaction with prefix [68, 69] to return 7 bytes, but a response of len 5 was requested"
    )]
    fn test_wrong_response_size() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hi".into(), b"goodbye".into());
        let mut rx = [0; 5];
        spi.transaction(&mut [Operation::Write(b"hi"), Operation::Read(&mut rx)])
            .unwrap();
    }

    #[test]
    fn test_transfer_success() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hi".into(), b"hello".into());
        let mut rx = [0; 5];
        spi.transaction(&mut [Operation::Transfer(&mut rx, b"hi")])
            .unwrap();
        assert_eq!(b"hello", &rx);
        spi.assert_all_expectations_met();
    }

    #[test]
    fn test_transfer_multi_success() {
        let mut spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"hi".into(), b"hello".into());
        spi.preprogram_data_response(b"bye".into(), b"world".into());
        let mut rx1 = [0; 5];
        let mut rx2 = [0; 5];
        spi.transaction(&mut [Operation::Transfer(&mut rx1, b"hi")])
            .unwrap();
        spi.transaction(&mut [Operation::Transfer(&mut rx2, b"bye")])
            .unwrap();
        assert_eq!(b"hello", &rx1);
        assert_eq!(b"world", &rx2);
        spi.assert_all_expectations_met();
    }

    #[test]
    #[should_panic(expected = "Unexpected SPI transaction")]
    fn test_transfer_panic_on_missing_program() {
        let mut spi = FakeSpiDevice::new();
        let mut rx = [0; 5];
        spi.transaction(&mut [Operation::Transfer(&mut rx, b"hi")])
            .unwrap();
    }

    #[test]
    #[should_panic(
        expected = "Not all expected SPI response programs were executed: remaining = 1"
    )]
    fn test_unmet_expectation_panics() {
        let spi = FakeSpiDevice::new();
        spi.preprogram_data_response(b"unused_cmd".into(), b"data".into());
        // We do NOT call transaction() to execute this preprogrammed response.
        // Calling assert_all_expectations_met() must panic!
        spi.assert_all_expectations_met();
    }
}
