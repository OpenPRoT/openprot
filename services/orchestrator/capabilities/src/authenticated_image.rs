// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What an authenticating pass learned about the image it checked.

use crate::{Measurement, Svn};

/// What a verifier read out of an image it authenticated, apart from the
/// pass or fail itself.
///
/// One type for both passes. The boot walk checks an image at rest in one
/// call and the update path checks a candidate in bounded steps, but what
/// either learns about the image is the same, and two spellings of it in
/// two crates would drift.
///
/// `Copy` and lifetime-free, like every capability vocabulary type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedImage {
    /// The image's manifest SVN, or `None` from a verifier that
    /// authenticates without reading a manifest.
    ///
    /// The authenticated pass is the only reading of the manifest a
    /// caller may act on, so an implementor that reads one reports it
    /// here and the caller never goes back to the image. The alternative
    /// is `None` rather than a stand-in value, because the anti-rollback
    /// commit takes this number and a stand-in would move the floor to
    /// something no signature check covered.
    ///
    /// `None` is an ordinary answer, not a fault: DSP0267's component
    /// comparison stamp is optional and a plain signed image has no
    /// manifest at all. A commit behind one holds the floor where it is
    /// and reports that, rather than failing.
    ///
    /// `None` and `Some(Svn(0))` are different answers. `None` means the
    /// verifier read no manifest, so there is no version to speak of.
    /// `Some(Svn(0))` is a version it read, including the zero DSP0267
    /// uses for an unused comparison stamp, and the commit takes it; the
    /// floor is monotonic, so advancing to zero is a no-op. An
    /// implementor that would rather a zero stamp held the floor than
    /// committed it reports `None` for that image.
    pub svn: Option<Svn>,
    /// SHA-256 digest of the image, or `None` when no hash engine is
    /// wired up. Optional because measuring is a separate capability a
    /// board may not have, while an SVN comes from the manifest the
    /// check already read.
    pub measurement: Option<Measurement>,
}
