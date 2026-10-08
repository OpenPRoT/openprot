// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Attestation verifier: appraises peer EAT tokens and firmware measurements.

pub mod corim;
pub mod eat_verifier;
pub mod evidence;

use std::collections::HashMap;
use std::time::Duration;

pub use corim::CorimRvp;
pub use evidence::AttestEvidence;

use openprot_attest_api::Measurement;

/// Appraise a list of measurements against one or more Reference Value
/// Providers and return the overall disposition and per-component results.
pub fn appraise_measurements(
    measurements: &[Measurement],
    rvps: &[Box<dyn ReferenceValueProvider>],
) -> (Disposition, HashMap<String, ComponentDisposition>) {
    let mut component_results = HashMap::new();
    let mut any_fail = false;
    let mut any_unknown = false;

    use openprot_attest_api::DigestAlgorithm;

    for m in measurements {
        let mut resolved = ComponentDisposition::Unknown;

        'rvp: for rvp in rvps {
            if let Some(refs) = rvp.reference_values(&m.component) {
                resolved = ComponentDisposition::Fail;
                for r in &refs {
                    let alg_match = matches!(
                        (&m.digest_alg, &r.digest_alg),
                        (DigestAlgorithm::Sha384, DigestAlgorithm::Sha384)
                            | (DigestAlgorithm::Sha512, DigestAlgorithm::Sha512)
                    );
                    if alg_match && m.digest == r.digest {
                        resolved = ComponentDisposition::Pass;
                        break 'rvp;
                    }
                }
                break 'rvp;
            }
        }

        match resolved {
            ComponentDisposition::Fail => any_fail = true,
            ComponentDisposition::Unknown => any_unknown = true,
            ComponentDisposition::Pass => {}
        }
        component_results.insert(m.component.as_str().to_string(), resolved);
    }

    let disposition = if any_fail {
        Disposition::Fail
    } else if any_unknown {
        Disposition::Indeterminate
    } else {
        Disposition::Pass
    };

    (disposition, component_results)
}

// ── Reference value provider ──────────────────────────────────────────────

/// Pluggable source of expected (reference) measurement values.
///
/// Implement this trait to supply reference values from a local CoRIM file,
/// a remote Reference Value Provider service, or a hard-coded policy table.
pub trait ReferenceValueProvider: Send + Sync {
    /// Return the reference measurements for `component`, or `None` if the
    /// component is not known to this provider.
    fn reference_values(&self, component: &str) -> Option<Vec<Measurement>>;
}

// ── Appraisal result types ────────────────────────────────────────────────

/// Overall appraisal outcome for a peer attestation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// All measurements matched reference values and all checks passed.
    Pass,
    /// One or more checks failed.
    Fail,
    /// The verifier could not reach a definitive conclusion (e.g., no
    /// reference values available for one or more components).
    Indeterminate,
}

/// Per-component appraisal outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentDisposition {
    Pass,
    Fail,
    /// No reference value was available for this component.
    Unknown,
}

// ── Configuration ─────────────────────────────────────────────────────────

/// Verifier configuration, typically set once at platform initialisation.
pub struct VerifyConfig {
    /// DER-encoded trust anchor certificates (Vendor CA roots) used to
    /// validate peer DICE certificate chains.
    pub trust_anchors: Vec<Vec<u8>>,
    /// Maximum age of a peer token before it is considered stale.
    pub max_token_age: Duration,
    /// Maximum DICE certificate chain depth accepted (prevents DoS via
    /// pathologically deep chains).
    pub max_chain_depth: usize,
}

// ── Verifier ──────────────────────────────────────────────────────────────

/// Attestation verifier. Construct once; thread-safe.
pub struct Verifier {
    config: VerifyConfig,
    rvps: Vec<Box<dyn ReferenceValueProvider>>,
}

impl Verifier {
    /// Create a new verifier with the given configuration.
    pub fn new(config: VerifyConfig) -> Result<Self, VerifyError> {
        if config.trust_anchors.is_empty() {
            return Err(VerifyError::Configuration(
                "at least one trust anchor certificate is required".into(),
            ));
        }
        Ok(Self {
            config,
            rvps: Vec::new(),
        })
    }

    /// Register a Reference Value Provider.
    pub fn add_rvp(&mut self, rvp: Box<dyn ReferenceValueProvider>) {
        self.rvps.push(rvp);
    }

    /// Appraise a pre-received COSE_Sign1 EAT token byte buffer.
    pub fn verify_token(
        &self,
        token: &[u8],
        expected_nonce: &[u8],
    ) -> Result<AttestEvidence, VerifyError> {
        eat_verifier::verify(token, expected_nonce, &self.config, &self.rvps)
    }
}

// ── Errors ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Verifier configuration error: {0}")]
    Configuration(String),
    #[error("Malformed CBOR: {0}")]
    MalformedCbor(String),
    #[error("Certificate chain validation failed: {0}")]
    CertChain(String),
    #[error("Signature verification failed")]
    SignatureInvalid,
    #[error("Nonce mismatch")]
    NonceMismatch,
    #[error("Token is expired or too old")]
    Stale,
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
    #[error("Measurement appraisal failed for component: {0}")]
    AppraisalFailed(String),
    #[error("Evidence serialization error: {0}")]
    EvidenceCbor(String),
}
