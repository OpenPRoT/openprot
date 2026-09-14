// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;

use crate::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS};
use crate::error::AttestError;
use crate::types::Measurement;

/// Hardware-backed signing operations implemented by a Caliptra mailbox driver.
///
/// The private Alias Key never leaves the Caliptra hardware boundary.
/// Testing: implement with a software key (`SoftwareAttestProducer` in the
/// producer crate behind `test-support`).
pub trait HwSigner {
    /// Sign `payload` with the platform alias key. Returns raw (r‖s) bytes.
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    /// Return the full DER-encoded certificate chain, leaf → root.
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    /// Return Caliptra-internal firmware measurements (ROM, FMC, runtime, etc.).
    fn caliptra_measurements(
        &self,
        out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
    ) -> Result<(), AttestError>;
}
