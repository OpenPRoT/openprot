// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device-facing capability traits for the Boot Orchestrator.
//!
//! `BootControl` is the actuation capability: the orchestrator drives a
//! single managed device's reset without knowing which controller line it
//! maps to.
//!
//! `SvnFloor` is the anti-rollback capability: one device's durable SVN
//! floor, read at verification and advanced only after a confirmed boot.
//!
//! `BootStatus` is the shared vocabulary for boot-liveness evidence, and
//! `EvidenceReader` resolves a board-defined signal id to it. The schema
//! names no signal kinds: each board's device table declares its
//! checkpoints as data (`BootCheckpoint` in `orchestrator-config`), and the
//! board's reader gives the ids meaning.
//!
//! `Updatable` is the update capability: stage a payload on one device
//! (polled, one bounded step at a time) and mark it the boot candidate —
//! always tentatively; the commit gate lives elsewhere. `PayloadSource` is
//! the chunked read seam staging pulls from — transports and slot
//! bookkeeping stay behind the adapter.
//!
//! `TrialBoot` is the commit gate activation leaves open, whether `Updatable`
//! did the activating or a PLDM firmware device did it for the eRoT: keep the
//! activated image once its boot was judged, or drop it. It sits on the
//! downstream devices whose slots the eRoT drives, where the eRoT watches the
//! boot and decides within one of its own lifetimes.
//!
//! `SelfUpdate` is the same gate for the eRoT's own image, where the judging
//! outlives the judge: the eRoT resets into the candidate, so the verdict is
//! reached by a boot that has to read what the previous one left behind. It is
//! one durable session, one at a time, carrying the state and the verified SVN
//! together, so the next boot can tell a session that was never armed from a
//! confirmed trial whose floor advance had not run yet. Downstream devices
//! need no session: a reset loses the observation that would judge them, so
//! abandoning at boot gets the same result with no storage.
//!
//! `IncrementalVerifier` is the polled verification seam: `start`
//! consumes the verifier into a `VerifySession` whose `poll` does one
//! bounded hash step per call. See the trait docs for the full lifecycle.
//!
//! `BootWatch` is the seam the orchestrator polls: one device's boot walk,
//! erased of every device-specific type, answering with a `WalkVerdict`.
//!
//! `Recovery` is the restore capability: rewrite one device's active image
//! from its board-configured recovery source, mechanism unnamed, source
//! chosen per attempt.
//!
//! `LockdownLatch` is the terminal capability: latch the platform into its safe
//! state, one-way, at the top of the escalation ladder.
//!
//! This crate is a dependency-free leaf: it holds the capability contracts,
//! and everything depends downward on it. Concrete adapters bind a capability
//! to a signal source and live in their own crates, so naming a capability
//! never drags in the stack behind it — the HAL-backed `HalBootControl` and
//! the `GpioBootMonitor` read helper are in `orchestrator-hal-adapters`. The
//! per-board device table schema lives in the separate `orchestrator-config`
//! crate; board tables (`target/<board>/devices.rs`) declare the values.

#![cfg_attr(not(test), no_std)]

mod boot_control;
mod boot_watch;
mod evidence;
mod incremental_verifier;
mod lockdown_latch;
mod recovery;
mod self_update;
mod svn_floor;
mod trial_boot;
mod updatable;

pub use boot_control::BootControl;
pub use boot_watch::{BootWatch, FailureCause, WalkVerdict};
pub use evidence::{BootStatus, EvidenceReader};
pub use incremental_verifier::{IncrementalVerifier, PollOutcome, VerifySession};
pub use lockdown_latch::LockdownLatch;
pub use recovery::{Recovery, RestoreOutcome};
pub use self_update::{trial_outcome, RunningImage, SelfUpdate, SelfUpdateState, TrialOutcome};
pub use svn_floor::{Svn, SvnFloor};
pub use trial_boot::TrialBoot;
pub use updatable::{PayloadReadError, PayloadSource, StageProgress, Updatable, UpdateError};
