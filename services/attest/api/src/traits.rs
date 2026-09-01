// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;

use crate::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_TOKEN_SIZE};
use crate::AttestError;

/// Platform-independent attestation producer interface.
///
/// Implementors assemble and sign an OCP-EAT COSE_Sign1 token containing
/// platform measurements, DICE identity claims, and optionally pre-serialized
/// SPDM evidence from the verifier service.
///
/// The `evidence` parameter is a raw CBOR byte slice from the verifier service.
/// Passing it as bytes rather than a typed struct keeps this crate free of any
/// verifier or spdm-lib dependency.
pub trait AttestProducer: Send + Sync {
    /// Generate a signed OCP-EAT COSE_Sign1 token bound to `nonce`.
    ///
    /// `evidence` is a CBOR-encoded blob from the verifier service, embedded
    /// verbatim as claim -70001. Pass an empty slice when no peer evidence is
    /// available. `iat` is a Unix timestamp (seconds since epoch) supplied by
    /// the caller. The encoded token is appended to `out`.
    fn generate_token(
        &self,
        nonce: &[u8],
        evidence: &[u8],
        iat: u64,
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError>;

    /// Return the current DICE certificate chain, ordered leaf → root.
    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
}
