// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot checkpoints: what the orchestrator probes, and how long it waits.

/// One boot checkpoint: a probe the orchestrator evaluates, and how long
/// it waits. Retry policy is deliberately not table data: a retry
/// re-resets the device and re-runs the whole walk, so budgets are
/// per boot attempt and owned by the orchestrator state machine.
///
/// The probe is board-defined, the schema attaches no meaning to it and
/// names no probe kinds. Each board defines its own vocabulary (a small
/// enum: a GPIO line, a progress-register threshold, a message-path
/// readiness) and gives it meaning in its `EvidenceReader`. The probe is
/// a defunctionalized evidence check: data in the table instead of a
/// function, so the table stays printable, comparable, const-checkable,
/// and could one day be generated instead of written.
///
/// Fields are private so a checkpoint that violates the schema is
/// unrepresentable: [`new`](Self::new) is the only way in, and it checks.
///
/// # Example: three GPIO checkpoints
///
/// A BMC behind three GPIO ready lines (bl1 on pin 4, kernel on pin 5,
/// service on pin 6), all on the same SGPIOM bank. Each probe variant
/// maps to one `GpioBootMonitor` in the board's `EvidenceReader`, and
/// the walker (`CheckpointWalk`) walks them in declaration order.
///
/// ```ignore
/// #[derive(Debug, Clone, Copy)]
/// enum BmcProbe { Bl1, Kernel, Service }
///
/// const BMC: DeviceConfig<u8, BmcProbe> = DeviceConfig::new(
///     "bmc", 0,
///     &[
///         BootCheckpoint::new("bl1",     BmcProbe::Bl1,     Duration::from_millis(500)),
///         BootCheckpoint::new("kernel",  BmcProbe::Kernel,  Duration::from_secs(5)),
///         BootCheckpoint::new("service", BmcProbe::Service, Duration::from_secs(30)),
///     ],
///     None,
/// );
///
/// // Pin binding at bring-up: one GpioBootMonitor per probe.
/// let bl1     = GpioBootMonitor::new(&sgpiom, Mask(1 << 4), ActivePolarity::ActiveHigh);
/// let kernel  = GpioBootMonitor::new(&sgpiom, Mask(1 << 5), ActivePolarity::ActiveHigh);
/// let service = GpioBootMonitor::new(&sgpiom, Mask(1 << 6), ActivePolarity::ActiveHigh);
///
/// // The board's EvidenceReader dispatches probe to monitor.
/// // See EvidenceReader's docs for the full impl pattern.
/// let bmc_walk = CheckpointWalk::new(bmc_reader, &BMC);
/// ```
///
/// The GPIO wiring above is illustrative only (`ignore`d: this crate is
/// deliberately dependency-free, so it can't compile against
/// `GpioBootMonitor`/`CheckpointWalk`, which live downstream). The part of
/// the shape this crate *can* check — declaring a checkpoint list and
/// building a table from it — is a real, compiled example:
///
/// ```rust
/// use orchestrator_config::{BootCheckpoint, DeviceConfig};
/// use core::time::Duration;
///
/// const CHECKPOINTS: &[BootCheckpoint<u8>] = &[
///     BootCheckpoint::new("bl1", 0, Duration::from_millis(500)),
///     BootCheckpoint::new("kernel", 1, Duration::from_secs(5)),
/// ];
/// const BMC: DeviceConfig<u8, u8> = DeviceConfig::new("bmc", 0, CHECKPOINTS, None);
///
/// assert_eq!(BMC.checkpoints().len(), 2);
/// assert_eq!(BMC.checkpoints()[0].name(), "bl1");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BootCheckpoint<P> {
    name: &'static str,
    probe: P,
    timeout: core::time::Duration,
}

impl<P> BootCheckpoint<P> {
    /// Declares a checkpoint. `const`, so board tables run the checks at
    /// build time.
    ///
    /// # Panics
    ///
    /// Panics, a build error in const context, if `name` is empty or
    /// `timeout` is shorter than 1 ms (the walk rounds to whole
    /// milliseconds, so a sub-millisecond window would silently become
    /// zero).
    #[must_use]
    pub const fn new(name: &'static str, probe: P, timeout: core::time::Duration) -> Self {
        assert!(!name.is_empty(), "checkpoint name must not be empty");
        assert!(
            timeout.as_millis() >= 1,
            "checkpoint timeout must be at least 1 ms"
        );
        Self {
            name,
            probe,
            timeout,
        }
    }

    /// Names the checkpoint in failure reports ("bl1", "kernel", ...).
    /// Unique within a device's checkpoint list.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Board-defined probe, resolved by the board's `EvidenceReader`
    /// (in `orchestrator-capabilities`). Data rather than a function, so
    /// the table stays pure data, the type-level docs say why.
    #[must_use]
    pub const fn probe(&self) -> &P {
        &self.probe
    }

    /// Window for one attempt at this checkpoint. Expiry is the boot
    /// walk's own judgment; hung devices report nothing.
    ///
    /// The orchestrator state machine never sees this value, it is
    /// clockless. The walk consumes the windows and reports expiry as a
    /// failed attempt; a component's whole boot timeout is nothing more
    /// than its walk over these windows, in order.
    #[must_use]
    pub const fn timeout(&self) -> core::time::Duration {
        self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    // Board tables run the constructors at compile time, where a
    // rejection is a build error nobody can assert on. These tests call
    // them at runtime to prove the reject paths actually fire.

    #[test]
    #[should_panic(expected = "checkpoint name must not be empty")]
    fn rejects_an_empty_checkpoint_name() {
        let _ = BootCheckpoint::new("", 0u8, Duration::from_secs(1));
    }

    #[test]
    #[should_panic(expected = "checkpoint timeout must be at least 1 ms")]
    fn rejects_a_zero_checkpoint_timeout() {
        let _ = BootCheckpoint::new("boot-complete", 0u8, Duration::ZERO);
    }

    #[test]
    #[should_panic(expected = "checkpoint timeout must be at least 1 ms")]
    fn rejects_a_sub_millisecond_checkpoint_timeout() {
        let _ = BootCheckpoint::new("boot-complete", 0u8, Duration::from_micros(999));
    }
}
