// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum AttestError {
    #[error("Caliptra mailbox error: {0}")]
    Caliptra(&'static str),
    #[error("CBOR encoding error")]
    Cbor,
    #[error("Fixed-size buffer capacity exceeded")]
    BufferFull,
    #[error("COSE signing error")]
    Cose,
    #[error("Measurement provider error: {0}")]
    Provider(&'static str),
}
