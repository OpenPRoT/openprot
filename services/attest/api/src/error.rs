// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum AttestError {
    #[error("Caliptra mailbox error: {0}")]
    Caliptra(String),
    #[error("CBOR encoding error: {0}")]
    Cbor(String),
    #[error("COSE signing error: {0}")]
    Cose(String),
    #[error("Measurement provider error: {0}")]
    Provider(String),
}
