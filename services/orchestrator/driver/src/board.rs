// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What the board supplies to the driver: traits and wiring data only.
//! Boards (or test mocks) implement these.

use openprot_orchestrator_sm::{BootFailureKind, ComponentId};
use orchestrator_capabilities::Updatable;
use orchestrator_config::DeviceConfig;
use util_io::ByteSource;

pub use orchestrator_capabilities::{AuthenticatedImage, BootControl, BootWatch, Measurement};
use orchestrator_capabilities::{IncrementalVerifier, Recovery, SelfReset, SelfUpdate, SvnFloor};

/// Access to one component's active firmware image, however it is reached —
/// interposed flash, a PLDM/MCTP transfer, a RAM copy in tests.
pub trait ImageSource {
    /// The error type reported by this source.
    type Error: core::error::Error;

    /// Makes the image readable (claim the flash, open the transfer).
    /// Idempotent; a later `open` re-stages the image.
    fn open(&mut self) -> Result<(), Self::Error>;

    /// Image length in bytes. Only meaningful after a successful `open`;
    /// sources that learn the size during `open` cache it and report the
    /// cached value here.
    fn size(&self) -> Result<usize, Self::Error>;

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
    fn size(&self) -> Result<usize, Self::Error> {
        (**self).size()
    }

    #[inline(always)]
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        (**self).read_at(offset, buf)
    }
}

/// Judges a component's firmware image; board wiring decides what counts
/// as authenticated.
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
    /// Reported as `Event::VerificationPassed`. Carries what the check
    /// learned about the image, including the manifest SVN the
    /// anti-rollback commit
    /// ([`Effect::CommitSvnFloor`](openprot_orchestrator_sm::Effect))
    /// takes. The same payload the update path's
    /// [`PollOutcome::Authenticated`](orchestrator_capabilities::PollOutcome)
    /// carries: one description of an authenticated image, whether it was
    /// checked at rest or staged.
    Authenticated(AuthenticatedImage),
    /// Reported as `Event::VerificationFailed`.
    Rejected,
}

/// Something the orchestrator tells management software about.
/// `#[non_exhaustive]` so a sink just ignores variants it doesn't
/// know, and adding one doesn't break anyone.
///
/// Payloads stay `Copy` and lifetime-free, like the effects these mirror. A
/// report names a component only where the effect it mirrors does.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    /// Held in reset and out of the trust chain, so the platform runs
    /// degraded. Reported once per component, as it is gated.
    Isolated(ComponentId),
    /// Recovery attempts exhausted and the platform halts. Reported before
    /// the lockdown latch, while there is still a platform to report from.
    RecoveryFailed(ComponentId),
    /// A boot walk failed at a named checkpoint. The `kind` tells whether
    /// this was silence (timed out), a retriable device fault, or a terminal
    /// device fault.
    BootFailed {
        id: ComponentId,
        checkpoint: &'static str,
        kind: BootFailureKind,
    },
    /// An update request declined because the platform was busy. Nothing
    /// staged, and the requester may ask again. Platform-wide, not
    /// per-component: the machine supervises one update at a time and
    /// `Event::UpdateRequest` names no component.
    UpdateDeferred,
    /// An update in flight superseded by recovery. Its staged image is
    /// discarded and no verdict for that request follows. Platform-wide for
    /// the same reason as [`Report::UpdateDeferred`].
    UpdateAborted,
    /// The verifier could not run its check: a crypto fault, or a
    /// staging region it could not read. The update is rejected either
    /// way, so without this report a broken verifier looks exactly like
    /// a bad image. Platform-wide for the same reason as
    /// [`Report::UpdateDeferred`].
    UpdateVerifierFault,
    /// A component's image digest, computed during verification.
    /// The driver sends it through the sink and forgets it.
    Measured {
        /// The component whose image this measures.
        id: ComponentId,
        /// The digest over that image.
        measurement: Measurement,
    },
    /// The spare slot could not be brought up to date after a commit.
    /// The running image is fine. The other slot still holds the
    /// firmware from before the update, so a fallback boots the old
    /// version.
    ///
    /// CSA 5.1.3 wants more than a report. A slot left unbootable has to
    /// be repaired from a known-good image. Nothing does that yet.
    SlotResyncFailed(ComponentId),
    /// The spare slot was brought up to date but the anti-rollback floor
    /// could not be written afterwards. The component runs the new image
    /// and both slots hold it; the floor still names the old SVN, so the
    /// old image could be installed again.
    SvnFloorCommitFailed(ComponentId),
    /// The eRoT's own update was rejected: its candidate authenticated
    /// without a manifest SVN, so the session has nothing to record and
    /// the floor would have nothing to advance to once the trial is
    /// confirmed. The update ends and the machine goes back to `Ready`.
    /// A managed component's versionless image is installed and holds
    /// the floor instead ([`Report::SvnFloorHeld`]); the eRoT verifies
    /// every other image, so it is the one that cannot lose
    /// anti-rollback.
    SelfImageVersionless,
    /// The component's anti-rollback floor stayed where it was, because
    /// the image that authenticated carried no SVN to move it to. The
    /// board binds an eRoT-held floor for this component, so it asked for
    /// anti-rollback its verifier cannot supply: nothing is wrong with
    /// the image, and nothing protects against installing an older one
    /// either. The eRoT's own update rejects such an image instead, and
    /// says so with [`Report::SelfImageVersionless`].
    SvnFloorHeld(ComponentId),
}

/// Where the driver hands its [`Report`]s. What a report becomes, a log
/// entry, a management transport message, a fault line, is board wiring.
///
/// Infallible by design: a report names something that already happened, so
/// an undeliverable one costs information, not containment. An error channel
/// would put reports on the fail-secure path, letting the act of reporting a
/// contained failure escalate it.
pub trait ReportSink {
    /// Receives one report. A sink that cannot deliver immediately queues on
    /// its own side rather than stalling the effect batch.
    fn report(&mut self, report: Report);
}

/// Drops every report, for a board with no management side to tell. Losing
/// reports costs visibility only, so this is a wiring choice, not a stub.
impl ReportSink for () {
    #[inline(always)]
    fn report(&mut self, _report: Report) {}
}

impl<S: ReportSink> ReportSink for &mut S {
    #[inline(always)]
    fn report(&mut self, report: Report) {
        (**self).report(report)
    }
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
    /// Boot-checkpoint supervision for the managed components.
    type BootWatch: BootWatch;
    /// The anti-rollback floor of the managed components. The SVN number
    /// survives reset and power loss, otherwise a power cycle would
    /// re-admit images below the floor.
    type SvnFloor: SvnFloor;
    /// Where reports go. `()` for a board with no management side to tell.
    type ReportSink: ReportSink;
    /// Stages and activates update payloads on the managed components.
    type Updatable: Updatable;
    /// Restores a managed component's image from its recovery source.
    /// `()` for a board with no recovery path: every attempt reports
    /// source exhaustion immediately.
    type Recovery: Recovery;
    /// The region an update source writes a candidate into. One region,
    /// because one update runs at a time.
    type Staging: ByteSource;
    /// Incremental verification of staged update candidates.
    type UpdateVerifier: IncrementalVerifier;
    /// Durable storage that records the eRoT's own update. It has to
    /// survive the reboot that decides whether the update worked.
    type SelfUpdate: SelfUpdate;
    /// Resets the eRoT, which is how its own update is activated. A board
    /// that never updates the eRoT still names a type; it is reached only
    /// through [`Board::self_image`].
    type SelfReset: SelfReset;
    /// The board's reset signal ids, as the device table spells them.
    type ResetSignal: 'static;
    /// The board's boot-probe vocabulary, as the device table spells it.
    type Probe: 'static;
}

/// The eRoT's own update session plus the anti-rollback floor it advances.
///
/// The two are kept in one struct because nothing uses one without the
/// other. Both [`settle_self_update`](crate::settle_self_update) and
/// [`commit_self_svn_floor`](crate::commit_self_svn_floor) need the pair.
///
/// This is not a [`SvnFloorBinding`]. That type exists to say who keeps a
/// component's floor, and the eRoT always keeps its own.
pub struct SelfImage<S: SelfUpdate, F: SvnFloor, R: SelfReset> {
    /// Storage the session lives in across the reboot.
    pub session: S,
    /// The eRoT's own anti-rollback floor.
    pub floor: F,
    /// The reset that boots the trial image. Activating the eRoT's own
    /// update ends here and does not come back.
    pub reset: R,
}

/// Who keeps one component's anti-rollback floor. Spelled as its own type
/// so a board must state the choice; an eRoT floor and a device tracking
/// its own SVN are different wirings, not a present or absent value.
pub enum SvnFloorBinding<F: SvnFloor> {
    /// The eRoT holds the floor and `CommitSvnFloor` advances it.
    Erot(F),
    /// The component tracks its own SVN (iRoT, or a PLDM device
    /// committing internally). The eRoT keeps no second floor and its
    /// `CommitSvnFloor` is a no-op.
    SelfManaged,
}

/// Everything one component needs, in one place. `components[i]` is
/// `ComponentId(i)`: table position assigns the id, the same order the
/// board's device table declares.
///
/// One struct per component rather than one array per capability, so a
/// component is written and read in one place and its parts cannot be
/// shuffled against each other.
pub struct ComponentWiring<B: BoardCapabilities> {
    /// The device table row this wiring claims. `bring_up` checks it
    /// against the table, so a pair of components wired into each other's
    /// slots is caught there rather than driving the wrong reset line.
    ///
    /// The check reaches this field and no further. A capability built
    /// against some other row, a boot walk handed `&DEVICES[1]` while this
    /// says row 0, still passes. Build the array with
    /// `core::array::from_fn` so one closure writes the index once.
    pub device: &'static DeviceConfig<B::ResetSignal, B::Probe>,
    /// The component's active firmware image.
    pub image: B::Image,
    /// Actuates the component's reset.
    pub boot_control: B::BootControl,
    /// Supervises the component's boot walk.
    pub boot_watch: B::BootWatch,
    /// Who keeps the component's anti-rollback floor.
    pub svn_floor: SvnFloorBinding<B::SvnFloor>,
    /// Stages updates for the component. A device with no update path
    /// wires an adapter whose `poll_stage` errors.
    pub updatable: B::Updatable,
    /// Restores the component's image from its configured sources. `()`
    /// for a board with no recovery path.
    pub recovery: B::Recovery,
}

/// Everything the board supplies, built once at bring-up and handed to
/// [`bring_up`](crate::bring_up). Holds only what a board composes by hand:
/// anything the chain already states, the driver derives instead of taking it
/// here. Fields are public because a board writes this as a literal, and
/// because executors may need two parts at once (disjoint borrows).
///
/// ```ignore
/// struct Ast1060Board;
/// impl BoardCapabilities for Ast1060Board {
///     type Image = SpiFlashImage;         // interposed flash, offsets from the slot layout
///     type Verifier = ManifestVerifier;   // signature + SVN via the crypto engine
///     type BootControl = ExtrstGpio;      // per-component reset line
///     type BootWatch = CheckpointWalk;    // GPIO checkpoint walk over the boot window
///     type SvnFloor = OtpSvnFloor;        // fuse-backed anti-rollback floor
///     type ReportSink = MctpReports;      // reports out over the management transport
///     type Updatable = PldmDevice;        // device pulls its own chunks
///     type Recovery = SlotRecovery;       // A/B + golden, attempt-indexed
///     type Staging = StagingFlash;        // where the update source writes
///     type UpdateVerifier = CryptoVerifier; // incremental signature check
///     type SelfUpdate = SelfUpdateSession; // session record in the eRoT's own flash
///     type SelfReset = WatchdogReset;     // the reset that boots the trial image
///     type ResetSignal = ExtrstLine;      // as DEVICES spells its reset ids
///     type Probe = BootProbe;             // as DEVICES spells its boot probes
/// }
///
/// // The table is a static: bring_up checks each wiring against its row by
/// // address, and a const table can be promoted afresh at every use.
/// static DEVICES: [orchestrator_config::DeviceConfig<ExtrstLine, BootProbe>; 2] =
///     [bmc_row, cpld_row];
///
/// // One chain, from the board's device table, for both halves. The const
/// // item is load-bearing: it forces chain_of's validation at build time.
/// const CHAIN: orchestrator_config::ChainEntries<2> = orchestrator_config::chain_of(&DEVICES);
/// let (orchestrator, driver) = bring_up::<Ast1060Board, 2, 6>(
///     &CHAIN,
///     &DEVICES,
///     Board {
///         components: [
///             ComponentWiring {
///                 device: &DEVICES[0],
///                 image: bmc_image,
///                 boot_control: bmc_reset,
///                 boot_watch: bmc_walk,
///                 svn_floor: SvnFloorBinding::Erot(bmc_floor),
///                 updatable: bmc_update,
///                 recovery: bmc_recovery,
///             },
///             ComponentWiring {
///                 device: &DEVICES[1],
///                 image: cpld_image,
///                 boot_control: cpld_reset,
///                 boot_watch: cpld_walk,
///                 svn_floor: SvnFloorBinding::SelfManaged,
///                 updatable: cpld_update,
///                 recovery: cpld_recovery,
///             },
///         ],
///         verifier,
///         report_sink,
///         update_staging,
///         update_stall_budget_millis: 30_000,
///         update_verifier: Some(crypto_verifier),
///         self_image: Some(SelfImage {
///             session: self_update,
///             floor: self_floor,
///             reset: self_reset,
///         }),
///     },
///     MAX_RETRY,
/// );
/// ```
pub struct Board<B: BoardCapabilities, const N: usize> {
    /// `components[i]` is `ComponentId(i)`: device index, chain position
    /// and table declaration order are the same number.
    pub components: [ComponentWiring<B>; N],
    /// Judges images for every component.
    pub verifier: B::Verifier,
    /// Where the driver hands the SM's reports. One per platform, not one
    /// per component: two of the four reports name no component.
    pub report_sink: B::ReportSink,
    /// Where an update source leaves the candidate. Not per component:
    /// the SM allows one update at a time, so one region serves all of
    /// them.
    pub update_staging: B::Staging,
    /// How long an update may make no progress before the pump abandons
    /// it. Board policy, not the update source's: a device that stops
    /// answering has to lose the job in bounded time whatever the source
    /// would prefer.
    pub update_stall_budget_millis: u64,
    /// Incremental verifier for staged update candidates. The driver
    /// takes ownership at construction; boards supply `Some(verifier)`.
    pub update_verifier: Option<B::UpdateVerifier>,
    /// The eRoT's own update session and floor. Set this to `None` on a
    /// board that never updates itself, meaning one that does nothing but
    /// supervise other components.
    pub self_image: Option<SelfImage<B::SelfUpdate, B::SvnFloor, B::SelfReset>>,
}
