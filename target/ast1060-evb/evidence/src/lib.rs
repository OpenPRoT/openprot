// Licensed under the Apache-2.0 license

//! SPDM Evidence Implementation - AST1060-EVB Reference Implementation
//!
//! Provides device measurements for SPDM attestation operations. This reference
//! implementation returns fixed measurement values for testing and demonstration.
//!
//! ## Architecture
//!
//! - **Fixed measurements:** Two static string measurements
//! - **No hardware integration:** Placeholder for development
//! - **PCR quote generation:** Returns formatted measurement data
//!
//! ## Hardware-Backed Implementations
//!
//! This software implementation serves as a reference. Future hardware-backed
//! versions should:
//! - Integrate with platform boot measurements
//! - Use TPM for PCR values and quotes
//! - Include cryptographic signatures
//! - Implement dynamic measurement log
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ast1060_evidence::Ast1060Evidence;
//! use spdm_lib::platform::evidence::SpdmEvidence;
//!
//! let evidence = Ast1060Evidence::new();
//!
//! // Get PCR quote size
//! let size = evidence.pcr_quote_size(false)?;
//!
//! // Generate PCR quote
//! let mut buffer = vec![0u8; size];
//! let written = evidence.pcr_quote(&mut buffer, false)?;
//! ```

#![no_std]

use spdm_lib::platform::evidence::{SpdmEvidence, SpdmEvidenceError, SpdmEvidenceResult};

/// Fixed measurement data
const MEASUREMENT_1: &[u8] = b"OpenPRoT SPDM Responder";
const MEASUREMENT_2: &[u8] = b"OCP EMEA HELLO WORLD";

/// Total number of measurements
const MEASUREMENT_COUNT: u8 = 2;

/// SPDM evidence implementation for AST1060-EVB.
///
/// Provides fixed measurements for SPDM GET_MEASUREMENTS command.
/// This is a reference implementation for testing and demonstration.
pub struct Ast1060Evidence;

impl Ast1060Evidence {
    /// Create a new evidence provider instance.
    pub const fn new() -> Self {
        Self
    }

    /// Calculate the total size needed for PCR quote
    fn calculate_quote_size(&self, with_pqc_sig: bool) -> usize {
        // PCR quote format (simplified):
        // - Measurement count (1 byte)
        // - For each measurement:
        //   - Measurement index (1 byte)
        //   - Measurement size (2 bytes)
        //   - Measurement data (variable)
        // - Optional: Post-quantum signature (if with_pqc_sig)

        let mut size = 1; // Measurement count

        // Measurement 1
        size += 1; // Index
        size += 2; // Size field
        size += MEASUREMENT_1.len(); // Data

        // Measurement 2
        size += 1; // Index
        size += 2; // Size field
        size += MEASUREMENT_2.len(); // Data

        // Post-quantum signature (not implemented in this version)
        if with_pqc_sig {
            // Reserved for future PQC signature support
            size += 0;
        }

        size
    }

    /// Encode measurements into buffer
    fn encode_measurements(&self, buffer: &mut [u8]) -> SpdmEvidenceResult<usize> {
        let mut offset = 0;

        // Validate buffer size
        let required_size = self.calculate_quote_size(false);
        if buffer.len() < required_size {
            return Err(SpdmEvidenceError::InvalidEvidenceFormat);
        }

        // Measurement count
        buffer[offset] = MEASUREMENT_COUNT;
        offset += 1;

        // Measurement 1
        buffer[offset] = 0; // Index 0
        offset += 1;

        let len1 = MEASUREMENT_1.len() as u16;
        buffer[offset..offset + 2].copy_from_slice(&len1.to_le_bytes());
        offset += 2;

        buffer[offset..offset + MEASUREMENT_1.len()].copy_from_slice(MEASUREMENT_1);
        offset += MEASUREMENT_1.len();

        // Measurement 2
        buffer[offset] = 1; // Index 1
        offset += 1;

        let len2 = MEASUREMENT_2.len() as u16;
        buffer[offset..offset + 2].copy_from_slice(&len2.to_le_bytes());
        offset += 2;

        buffer[offset..offset + MEASUREMENT_2.len()].copy_from_slice(MEASUREMENT_2);
        offset += MEASUREMENT_2.len();

        Ok(offset)
    }
}

impl SpdmEvidence for Ast1060Evidence {
    fn pcr_quote(&self, buffer: &mut [u8], with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        if with_pqc_sig {
            // Post-quantum signatures not implemented in this version
            return Err(SpdmEvidenceError::UnsupportedEvidenceType);
        }

        self.encode_measurements(buffer)
    }

    fn pcr_quote_size(&self, with_pqc_sig: bool) -> SpdmEvidenceResult<usize> {
        if with_pqc_sig {
            // Post-quantum signatures not implemented in this version
            return Err(SpdmEvidenceError::UnsupportedEvidenceType);
        }

        Ok(self.calculate_quote_size(false))
    }
}

impl Default for Ast1060Evidence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let _evidence = Ast1060Evidence::new();
    }

    #[test]
    fn test_pcr_quote_size() {
        let evidence = Ast1060Evidence::new();

        // Without PQC signature
        let size = evidence.pcr_quote_size(false).unwrap();

        // Expected size:
        // 1 (count) + 1 (idx1) + 2 (len1) + 23 (data1) + 1 (idx2) + 2 (len2) + 20 (data2)
        // = 1 + 26 + 23 = 50
        assert_eq!(size, 50);
    }

    #[test]
    fn test_pcr_quote_size_with_pqc() {
        let evidence = Ast1060Evidence::new();

        // PQC signatures not supported
        let result = evidence.pcr_quote_size(true);
        assert!(matches!(
            result,
            Err(SpdmEvidenceError::UnsupportedEvidenceType)
        ));
    }

    #[test]
    fn test_pcr_quote() {
        let evidence = Ast1060Evidence::new();
        let mut buffer = [0u8; 100];

        let written = evidence.pcr_quote(&mut buffer, false).unwrap();
        assert_eq!(written, 50);

        // Verify measurement count
        assert_eq!(buffer[0], 2);

        // Verify measurement 1
        assert_eq!(buffer[1], 0); // Index 0
        let len1 = u16::from_le_bytes([buffer[2], buffer[3]]);
        assert_eq!(len1, 23);
        assert_eq!(&buffer[4..27], MEASUREMENT_1);

        // Verify measurement 2
        assert_eq!(buffer[27], 1); // Index 1
        let len2 = u16::from_le_bytes([buffer[28], buffer[29]]);
        assert_eq!(len2, 20);
        assert_eq!(&buffer[30..50], MEASUREMENT_2);
    }

    #[test]
    fn test_pcr_quote_buffer_too_small() {
        let evidence = Ast1060Evidence::new();
        let mut buffer = [0u8; 10]; // Too small

        let result = evidence.pcr_quote(&mut buffer, false);
        assert!(matches!(
            result,
            Err(SpdmEvidenceError::InvalidEvidenceFormat)
        ));
    }

    #[test]
    fn test_pcr_quote_with_pqc() {
        let evidence = Ast1060Evidence::new();
        let mut buffer = [0u8; 100];

        // PQC signatures not supported
        let result = evidence.pcr_quote(&mut buffer, true);
        assert!(matches!(
            result,
            Err(SpdmEvidenceError::UnsupportedEvidenceType)
        ));
    }

    #[test]
    fn test_measurement_values() {
        assert_eq!(MEASUREMENT_1, b"OpenPRoT SPDM Responder");
        assert_eq!(MEASUREMENT_2, b"OCP EMEA HELLO WORLD");
    }

    #[test]
    fn test_measurement_count() {
        assert_eq!(MEASUREMENT_COUNT, 2);
    }
}
