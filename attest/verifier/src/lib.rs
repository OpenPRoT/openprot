// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

pub mod verifier;

pub use verifier::{
    appraise_measurements, AttestEvidence, ComponentDisposition, CorimRvp, Disposition,
    ReferenceValueProvider, Verifier, VerifyConfig, VerifyError,
};
