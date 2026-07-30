// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Firmware measurement collection.
//!
//! Aggregates Caliptra-internal measurements (ROM, FMC, RT digests) with any
//! platform-registered [`MeasurementProvider`] instances.

use openprot_attest_api::{AttestError, Measurement, MeasurementProvider};

/// Collect all measurements: Caliptra-internal first, then registered providers.
///
/// `caliptra_measurements` is pre-fetched from the Caliptra driver.
/// Platform providers are queried here and their results appended.
pub fn collect(
    caliptra_measurements: Vec<Measurement>,
    providers: &[Box<dyn MeasurementProvider>],
) -> Result<Vec<Measurement>, AttestError> {
    let mut all = caliptra_measurements;
    for provider in providers {
        let entries = provider.measurements().map_err(|e| {
            AttestError::Provider(format!(
                "provider '{}' failed: {e}",
                provider.component_name()
            ))
        })?;
        all.extend(entries);
    }
    Ok(all)
}

/// Stub Caliptra measurements for use in tests (requires `test-support` feature).
#[cfg(feature = "test-support")]
pub fn test_caliptra_measurements() -> Vec<Measurement> {
    use openprot_attest_api::{DigestAlgorithm, MeasurementAuthority};

    vec![
        Measurement {
            component: "Caliptra ROM".into(),
            version: "1.0.0".into(),
            digest_alg: DigestAlgorithm::Sha384,
            digest: vec![0xAAu8; 48],
            authority: MeasurementAuthority::Caliptra,
        },
        Measurement {
            component: "Caliptra FMC".into(),
            version: "2.3.1".into(),
            digest_alg: DigestAlgorithm::Sha384,
            digest: vec![0xBBu8; 48],
            authority: MeasurementAuthority::Caliptra,
        },
        Measurement {
            component: "Caliptra RT".into(),
            version: "2.3.1".into(),
            digest_alg: DigestAlgorithm::Sha384,
            digest: vec![0xCCu8; 48],
            authority: MeasurementAuthority::Caliptra,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_attest_api::{DigestAlgorithm, MeasurementAuthority};

    struct StubProvider {
        name: &'static str,
        fail: bool,
    }

    impl MeasurementProvider for StubProvider {
        fn component_name(&self) -> &str {
            self.name
        }
        fn measurements(&self) -> Result<Vec<Measurement>, AttestError> {
            if self.fail {
                Err(AttestError::Provider("intentional failure".into()))
            } else {
                Ok(vec![Measurement {
                    component: self.name.into(),
                    version: "0.1".into(),
                    digest_alg: DigestAlgorithm::Sha384,
                    digest: vec![0xBBu8; 48],
                    authority: MeasurementAuthority::Platform,
                }])
            }
        }
    }

    fn rom() -> Measurement {
        Measurement {
            component: "ROM".into(),
            version: "1.0".into(),
            digest_alg: DigestAlgorithm::Sha384,
            digest: vec![0xAAu8; 48],
            authority: MeasurementAuthority::Caliptra,
        }
    }

    #[test]
    fn no_providers_returns_caliptra_measurements_unchanged() {
        let result = collect(vec![rom()], &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].component, "ROM");
    }

    #[test]
    fn provider_measurements_are_appended() {
        let providers: Vec<Box<dyn MeasurementProvider>> =
            vec![Box::new(StubProvider {
                name: "UEFI",
                fail: false,
            })];
        let result = collect(vec![rom()], &providers).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].component, "ROM");
        assert_eq!(result[1].component, "UEFI");
    }

    #[test]
    fn multiple_providers_all_appended() {
        let providers: Vec<Box<dyn MeasurementProvider>> = vec![
            Box::new(StubProvider {
                name: "UEFI",
                fail: false,
            }),
            Box::new(StubProvider {
                name: "BMC",
                fail: false,
            }),
        ];
        let result = collect(vec![], &providers).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn failing_provider_propagates_error() {
        let providers: Vec<Box<dyn MeasurementProvider>> =
            vec![Box::new(StubProvider {
                name: "BMC",
                fail: true,
            })];
        let err = collect(vec![], &providers).unwrap_err();
        assert!(err.to_string().contains("BMC"));
    }
}
