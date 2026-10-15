// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What an authenticating pass learned about the image it checked.

use crate::{Measurement, Svn};

/// The facts an authenticated image carries, separate from the pass or
/// fail itself: a verdict says whether to act, this says what a later
/// decision acts on.
///
/// One type for both passes. The boot walk checks an image at rest in one
/// call and the update path checks a candidate in bounded steps, but what
/// either learns about the image is the same, and two spellings of it in
/// two crates would drift.
///
/// `Copy` and lifetime-free, like every capability vocabulary type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedImage {
    /// The image's manifest SVN. The implementor reports what the
    /// manifest the authenticated pass covered holds, so a caller never
    /// has to read the image for it, and must not: any other reading is
    /// of bytes no signature check vouched for. It is the one value the
    /// anti-rollback commit may trust.
    pub svn: Svn,
    /// SHA-256 digest of the image, or `None` when no hash engine is
    /// wired up. Optional because measuring is a separate capability a
    /// board may not have, while an SVN comes from the manifest the
    /// check already read.
    pub measurement: Option<Measurement>,
}
