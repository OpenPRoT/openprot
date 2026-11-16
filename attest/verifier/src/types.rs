// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use heapless::{String, Vec};

// Fixed-capacity constants
pub const MAX_MEASUREMENTS: usize = 16;
pub const MAX_COMPONENT_LEN: usize = 64;
pub const MAX_VERSION_LEN: usize = 32;
pub const MAX_DIGEST_LEN: usize = 64; // SHA-512

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Sha384,
    Sha512,
}

#[derive(Clone, Copy, Debug)]
pub enum MeasurementAuthority {
    Caliptra,
    Platform,
}

/// A single firmware measurement record extracted from an EAT token.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub component: String<MAX_COMPONENT_LEN>,
    pub version: String<MAX_VERSION_LEN>,
    pub digest_alg: DigestAlgorithm,
    pub digest: Vec<u8, MAX_DIGEST_LEN>,
    pub authority: MeasurementAuthority,
}
