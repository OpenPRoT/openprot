// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::{AttestError, CertChain};

/// Platform-independent attestation producer interface.
///
/// Implementors assemble and sign an OCP-EAT COSE_Sign1 token containing
/// platform measurements, DICE identity claims, and optionally pre-serialized
/// SPDM evidence from the verifier service.
///
/// The `evidence` parameter accepted by `generate_token` is a raw CBOR byte
/// slice produced by the verifier service. Passing it as bytes rather than a
/// typed struct keeps this crate free of any verifier or spdm-lib dependency.
pub trait AttestProducer: Send + Sync {
    /// Generate a signed OCP-EAT COSE_Sign1 token bound to `nonce`.
    ///
    /// `evidence` is a CBOR-encoded blob from the verifier service, embedded
    /// verbatim as claim -70001. Pass an empty slice when no peer evidence is
    /// available.
    ///
    /// Returns the complete COSE_Sign1 structure as a byte vector.
    fn generate_token(&self, nonce: &[u8], evidence: &[u8]) -> Result<Vec<u8>, AttestError>;

    /// Return the current DICE certificate chain, ordered leaf → root.
    fn cert_chain(&self) -> Result<CertChain, AttestError>;
}
