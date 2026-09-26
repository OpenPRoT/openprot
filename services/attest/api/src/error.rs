// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum AttestError {
    #[error("Mailbox error: {0}")]
    Mailbox(&'static str),
    #[error("DER parse error: {0}")]
    Der(&'static str),
    #[error("DICE chain validation error: {0}")]
    ChainValidation(&'static str),
    #[error("Invalid nonce: {0}")]
    InvalidNonce(&'static str),
    #[error("CBOR encoding error")]
    Cbor,
    #[error("Fixed-size buffer capacity exceeded")]
    BufferFull,
    #[error("COSE signing error")]
    Cose,
    #[error("Measurement provider error: {0}")]
    Provider(&'static str),
    #[error("Invalid key material: {0}")]
    InvalidKey(&'static str),
}
