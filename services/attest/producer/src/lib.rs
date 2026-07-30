// SPDX-License-Identifier: Apache-2.0

//! Concrete attestation producer for OpenPRoT.
//!
//! Provides [`HwAttestProducer`], which implements [`openprot_attest_api::AttestProducer`]
//! backed by a Caliptra hardware signer.  Under the `test-support` feature,
//! [`SoftwareAttestProducer`] provides a fully software-backed substitute
//! that requires no Caliptra hardware.

#![forbid(unsafe_code)]

pub mod builder;
pub mod dice_identity;
pub mod measurements;
mod signer;

pub use signer::HwAttestProducer;

#[cfg(feature = "test-support")]
pub use signer::SoftwareAttestProducer;
