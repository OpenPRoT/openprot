// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the board supplies to the driver: traits and wiring data only.
//! Boards (or test mocks) implement these.

use openprot_orchestrator_sm::ComponentId;
use orchestrator_capabilities::BootControl;

/// Access to one component's active firmware image, however it is reached —
/// interposed flash, a PLDM/MCTP transfer, a RAM copy in tests.
pub trait ImageSource {
    /// The error type reported by this source.
    type Error: core::error::Error;

    /// Makes the image readable (claim the flash, open the transfer).
    /// Idempotent; a later `open` re-stages the image.
    fn open(&mut self) -> Result<(), Self::Error>;

    /// Image length in bytes.
    fn size(&mut self) -> Result<usize, Self::Error>;

    /// Reads `buf.len()` bytes starting at byte `offset` of the image.
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), Self::Error>;
}

impl<S: ImageSource> ImageSource for &mut S {
    type Error = S::Error;

    #[inline(always)]
    fn open(&mut self) -> Result<(), Self::Error> {
        (**self).open()
    }

    #[inline(always)]
    fn size(&mut self) -> Result<usize, Self::Error> {
        (**self).size()
    }

    #[inline(always)]
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        (**self).read_at(offset, buf)
    }
}

/// Judges a component's firmware image; board wiring decides what
/// "authentic" means.
pub trait Verifier {
    /// The error type reported by this verifier.
    type Error: core::error::Error;

    /// Judges `id`'s image, reading it from `image`.
    ///
    /// # Errors
    ///
    /// Only when the check could not be performed (crypto fault, missing
    /// key, unreadable source). A checked-and-bad image is
    /// `Ok(Verdict::Rejected)` — an actuation fault must not forge a
    /// verdict.
    fn verify(
        &mut self,
        id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, Self::Error>;
}

impl<V: Verifier> Verifier for &mut V {
    type Error = V::Error;

    #[inline(always)]
    fn verify(
        &mut self,
        id: ComponentId,
        image: &mut impl ImageSource,
    ) -> Result<Verdict, Self::Error> {
        (**self).verify(id, image)
    }
}

/// A [`Verifier`]'s judgment of one image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Reported as `Event::VerificationPassed`.
    Authenticated,
    /// Reported as `Event::VerificationFailed`.
    Rejected,
}

/// The set of platform capabilities one board composes into the
/// `PlatformDriver`, named by a marker type. A new seam adds an associated
/// type here and a field on [`Board`] — never another parameter.
pub trait BoardCapabilities {
    /// Image access for the managed components.
    type Image: ImageSource;
    /// Judges images for every component.
    type Verifier: Verifier;
    /// Reset actuation for the managed components.
    type BootControl: BootControl;
    // Later seams: Evidence (checkpoint walk), Recovery, Staging.
}

/// Everything the board supplies, built once at bring-up and handed to
/// `PlatformDriver::new`. Fields are public: executors may need two parts at once
/// (disjoint borrows).
///
/// ```ignore
/// struct Ast1060Board;
/// impl BoardCapabilities for Ast1060Board {
///     type Image = SpiFlashImage;         // interposed flash, offsets from the slot layout
///     type Verifier = ManifestVerifier;   // signature + SVN via the crypto engine
///     type BootControl = ExtrstGpio;      // per-component reset line
/// }
/// let board = Board::<Ast1060Board, 2> {
///     images: [bmc_image, cpld_image],
///     verifier,
///     boot_controls: [bmc_reset, cpld_reset],
/// };
/// ```
pub struct Board<B: BoardCapabilities, const N: usize> {
    /// `images[i]` belongs to `ComponentId(i)` — device index = chain
    /// position = table declaration order.
    pub images: [B::Image; N],
    /// Judges images for every component.
    pub verifier: B::Verifier,
    /// `boot_controls[i]` actuates `ComponentId(i)`'s reset, same indexing
    /// as `images`.
    pub boot_controls: [B::BootControl; N],
    // Later seams add fields, e.g. evidence: [B::Evidence; N].
}
