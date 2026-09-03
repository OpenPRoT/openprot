// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

// Caliptra chain: VendorCA → IDevID → LDevID → AliasFMC → AliasRT (leaf)
pub const MAX_CHAIN_LEN: usize = 5;
pub const MAX_CERT_SIZE: usize = 2048;

pub const MAX_MEASUREMENTS: usize = 16;
pub const MAX_PROVIDERS: usize = 8;
pub const MAX_COMPONENT_LEN: usize = 64;
pub const MAX_VERSION_LEN: usize = 32;
pub const MAX_DIGEST_LEN: usize = 64; // SHA-512

pub const MAX_OEMID_LEN: usize = 16;
pub const MAX_HW_MODEL_LEN: usize = 64;
pub const MAX_HW_VERSION_LEN: usize = 32;
pub const MAX_NONCE_LEN: usize = 64;
pub const MAX_EVIDENCE_LEN: usize = 4096;

// Upper bound for a fully-populated COSE_Sign1 token
pub const MAX_TOKEN_SIZE: usize = 8192;
