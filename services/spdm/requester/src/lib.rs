// Licensed under the Apache-2.0 license

//! SPDM Requester Service
//!
//! This service implements the SPDM requester (client) role, which initiates
//! attestation and measurement operations with SPDM responders.
//!
//! ## Overview
//!
//! The SPDM requester sends requests to responders to:
//! - Get version information
//! - Negotiate capabilities and algorithms
//! - Challenge the responder for attestation
//! - Retrieve measurements and certificates
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────┐
//! │  Application            │
//! │  (attestation manager)  │
//! └───────────┬─────────────┘
//!             │
//!             ▼
//! ┌─────────────────────────┐
//! │  SPDM Requester         │◄── This crate
//! │  (request builder)      │
//! └───────────┬─────────────┘
//!             │ SPDM messages
//!             ▼
//! ┌─────────────────────────┐
//! │  MCTP Transport         │
//! └─────────────────────────┘
//! ```

#![no_std]
#![warn(missing_docs)]

/// SPDM requester state and configuration.
#[derive(Debug)]
pub struct SpdmRequester {
    /// Remote endpoint ID for MCTP transport.
    pub remote_eid: u8,
}

impl SpdmRequester {
    /// Create a new SPDM requester targeting the given endpoint.
    pub fn new(remote_eid: u8) -> Self {
        Self { remote_eid }
    }

    /// Get the remote endpoint ID.
    pub fn remote_eid(&self) -> u8 {
        self.remote_eid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requester_creation() {
        let requester = SpdmRequester::new(42);
        assert_eq!(requester.remote_eid(), 42);
    }
}
