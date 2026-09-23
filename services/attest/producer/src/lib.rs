// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Concrete attestation producer for OpenPRoT.
//!
//! Provides [`HwAttestProducer`] and [`SwAttestProducer`], which implement
//! [`openprot_attest_api::AttestProducer`] backed by a hardware signer (Caliptra
//! mailbox) or a caller-supplied software key respectively.  Under the
//! `test-support` feature, [`SoftwareAttestProducer`] provides a fully
//! software-backed stub for unit and integration tests.

#![no_std]
#![forbid(unsafe_code)]

pub mod builder;
pub mod cert_ueid;
pub mod der;
pub mod dice_identity;
pub mod measurements;
mod signer;

pub use signer::HwAttestProducer;
pub use signer::SwAttestProducer;

#[cfg(feature = "test-support")]
pub use signer::SoftwareAttestProducer;
