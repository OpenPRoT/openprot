// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::Vec;

use crate::consts::{MAX_CERT_SIZE, MAX_CHAIN_LEN, MAX_TOKEN_SIZE};
use crate::AttestError;

/// Platform-independent attestation producer interface.
pub trait AttestProducer {
    /// Generate a signed OCP-EAT COSE_Sign1 token bound to `nonce`.
    ///
    /// The encoded token is appended to `out`.
    fn generate_token(
        &self,
        nonce: &[u8],
        out: &mut Vec<u8, MAX_TOKEN_SIZE>,
    ) -> Result<(), AttestError>;

    /// Return the current DICE certificate chain, ordered leaf → root.
    ///
    /// On success `buf` is **cleared** and then populated with the chain.
    /// Any contents in `buf` before the call are discarded.
    fn cert_chain(
        &self,
        buf: &mut Vec<Vec<u8, MAX_CERT_SIZE>, MAX_CHAIN_LEN>,
    ) -> Result<(), AttestError>;
}
