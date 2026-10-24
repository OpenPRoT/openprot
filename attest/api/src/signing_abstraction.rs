// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;

use crate::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_MEASUREMENTS};
use crate::error::AttestError;
use crate::types::Measurement;

/// Hardware-backed signing operations implemented by a platform security subsystem.
///
/// The private Alias Key never leaves the hardware security boundary.
/// Testing: implement with a software key (`SoftwareAttestProducer` in the
/// producer crate behind `test-support`).
pub trait Signer {
    /// Sign `payload` with the platform alias key. Returns raw (r‖s) bytes.
    fn sign(&self, payload: &[u8]) -> Result<[u8; 96], AttestError>;
    /// Return the full DER-encoded certificate chain, leaf → root.
    fn cert_chain_der(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
    /// Return hardware-internal firmware measurements (ROM, FMC, runtime, etc.).
    fn measurements(&self, out: &mut Vec<Measurement, MAX_MEASUREMENTS>)
        -> Result<(), AttestError>;
}
