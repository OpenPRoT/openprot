// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared vocabulary for boot-liveness evidence.

/// Liveness of a managed device's boot: Boot Confirmation only.
///
/// Reports only that a device came up, never what booted; confirming the
/// running image is the one the RoT staged is attestation, a separate step.
/// `Failed` is optional device-reported evidence and never the only failure
/// path, since a hung device reports nothing — a stuck boot is caught by the
/// orchestrator's timeout, not by this enum.
///
/// Any given evidence source may only ever produce a *subset* of these
/// statuses: a single ready pin yields only `Booting`/`Booted`, while a
/// fault-channel backend can also report `Failed`. That is a capability
/// difference between sources, not an incomplete implementation — consumers
/// must handle the full set.
///
/// A status must describe the **current** boot cycle. Where the underlying
/// signal is an edge or pulse, it is latched beneath the read, and the latch
/// must be cleared whenever the device re-enters reset — by hardware tying
/// the latch to the device's reset line, or by the platform code that drives
/// `BootControl` — so evidence left over from a previous boot never reads as
/// [`Booted`](BootStatus::Booted). Clearing is deliberately the reset path's
/// job, not the reader's: a reader that could clear its own evidence would
/// let a read race a reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStatus {
    /// Released, but boot completion not yet observed.
    Booting,
    /// Boot completion observed.
    Booted,
    /// Device reported a boot failure.
    Failed,
}
