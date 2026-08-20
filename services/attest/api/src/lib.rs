// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Platform-independent API for the OpenPRoT attestation producer service.
//!
//! # Usage
//!
//! Applications depend only on this crate. Platform code provides a concrete
//! [`AttestProducer`] implementation (hardware-backed via Caliptra, or the
//! `test-support`-gated software stub in the `openprot-attest-producer` crate).
//!
//! ```text
//! ┌───────────────────────────────────────────────┐
//! │  application / verifier service               │
//! │      depends on: openprot-attest-api          │
//! │      calls:      AttestProducer::generate_token│
//! └──────────────────┬────────────────────────────┘
//!                    │ trait object / generic bound
//! ┌──────────────────▼────────────────────────────┐
//! │  openprot-attest-producer                     │
//! │      HwAttestProducer  (production)           │
//! │      SoftwareAttestProducer (test-support)    │
//! └───────────────────────────────────────────────┘
//! ```

#![forbid(unsafe_code)]

mod error;
mod traits;
mod types;

pub use error::AttestError;
pub use traits::AttestProducer;
pub use types::{
    AttestConfig, CaliptraSigner, CertChain, DigestAlgorithm, Measurement, MeasurementAuthority,
    MeasurementProvider, OemId,
};
