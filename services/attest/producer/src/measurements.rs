// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Firmware measurement collection.
//!
//! Aggregates Caliptra-internal measurements (ROM, FMC, RT digests) with any
//! platform-registered [`MeasurementProvider`] instances.

use heapless::Vec;

use openprot_attest_api::consts::MAX_MEASUREMENTS;
use openprot_attest_api::{AttestError, Measurement, MeasurementProvider};

/// Collect all measurements: `caliptra` first, then registered providers.
pub fn collect(
    caliptra: &[Measurement],
    providers: &[&dyn MeasurementProvider],
    out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
) -> Result<(), AttestError> {
    for m in caliptra {
        out.push(m.clone()).map_err(|_| AttestError::BufferFull)?;
    }
    for p in providers {
        p.measurements(out)?;
    }
    Ok(())
}

/// Stub Caliptra measurements for use in tests (requires `test-support` feature).
#[cfg(feature = "test-support")]
pub fn test_caliptra_measurements() -> Vec<Measurement, MAX_MEASUREMENTS> {
    use heapless::String;
    use openprot_attest_api::consts::{MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_VERSION_LEN};
    use openprot_attest_api::{DigestAlgorithm, MeasurementAuthority};

    let mut v: Vec<Measurement, MAX_MEASUREMENTS> = Vec::new();

    for (name, ver, fill) in &[
        ("Caliptra ROM", "1.0.0", 0xAAu8),
        ("Caliptra FMC", "2.3.1", 0xBBu8),
        ("Caliptra RT", "2.3.1", 0xCCu8),
    ] {
        let mut component: String<MAX_COMPONENT_LEN> = String::new();
        component.push_str(name).unwrap();
        let mut version: String<MAX_VERSION_LEN> = String::new();
        version.push_str(ver).unwrap();
        let mut digest: Vec<u8, MAX_DIGEST_LEN> = Vec::new();
        digest.extend_from_slice(&[*fill; 48]).unwrap();
        v.push(Measurement {
            component,
            version,
            digest_alg: DigestAlgorithm::Sha384,
            digest,
            authority: MeasurementAuthority::Caliptra,
        })
        .unwrap();
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::{String, Vec};
    use openprot_attest_api::consts::{
        MAX_COMPONENT_LEN, MAX_DIGEST_LEN, MAX_MEASUREMENTS, MAX_VERSION_LEN,
    };
    use openprot_attest_api::{DigestAlgorithm, MeasurementAuthority};

    struct StubProvider {
        name: &'static str,
        fail: bool,
    }

    impl MeasurementProvider for StubProvider {
        fn component_name(&self) -> &str {
            self.name
        }
        fn measurements(
            &self,
            out: &mut Vec<Measurement, MAX_MEASUREMENTS>,
        ) -> Result<(), AttestError> {
            if self.fail {
                return Err(AttestError::Provider("intentional failure"));
            }
            let mut component: String<MAX_COMPONENT_LEN> = String::new();
            component.push_str(self.name).unwrap();
            let mut version: String<MAX_VERSION_LEN> = String::new();
            version.push_str("0.1").unwrap();
            let mut digest: Vec<u8, MAX_DIGEST_LEN> = Vec::new();
            digest.extend_from_slice(&[0xBBu8; 48]).unwrap();
            out.push(Measurement {
                component,
                version,
                digest_alg: DigestAlgorithm::Sha384,
                digest,
                authority: MeasurementAuthority::Platform,
            })
            .map_err(|_| AttestError::BufferFull)
        }
    }

    fn rom() -> Measurement {
        let mut component: String<MAX_COMPONENT_LEN> = String::new();
        component.push_str("ROM").unwrap();
        let mut version: String<MAX_VERSION_LEN> = String::new();
        version.push_str("1.0").unwrap();
        let mut digest: Vec<u8, MAX_DIGEST_LEN> = Vec::new();
        digest.extend_from_slice(&[0xAAu8; 48]).unwrap();
        Measurement {
            component,
            version,
            digest_alg: DigestAlgorithm::Sha384,
            digest,
            authority: MeasurementAuthority::Caliptra,
        }
    }

    #[test]
    fn no_providers_returns_caliptra_measurements_unchanged() {
        let mut out = Vec::new();
        collect(&[rom()], &[], &mut out).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].component.as_str(), "ROM");
    }

    #[test]
    fn provider_measurements_are_appended() {
        let p = StubProvider {
            name: "UEFI",
            fail: false,
        };
        let mut out = Vec::new();
        collect(&[rom()], &[&p as &dyn MeasurementProvider], &mut out).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].component.as_str(), "UEFI");
    }

    #[test]
    fn multiple_providers_all_appended() {
        let p1 = StubProvider {
            name: "UEFI",
            fail: false,
        };
        let p2 = StubProvider {
            name: "BMC",
            fail: false,
        };
        let mut out = Vec::new();
        collect(
            &[],
            &[
                &p1 as &dyn MeasurementProvider,
                &p2 as &dyn MeasurementProvider,
            ],
            &mut out,
        )
        .unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn failing_provider_propagates_error() {
        let p = StubProvider {
            name: "BMC",
            fail: true,
        };
        let mut out = Vec::new();
        let err = collect(&[], &[&p as &dyn MeasurementProvider], &mut out).unwrap_err();
        assert!(matches!(err, AttestError::Provider(_)));
    }
}
