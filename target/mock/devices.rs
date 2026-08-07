// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock board: a device table exercising every device archetype the
//! orchestrator manages. Not a real board — consumed by host tests and QEMU
//! runs until a hardware target declares its own table.

#![no_std]

use core::time::Duration;

use orchestrator_config::{
    BootCheckpoint, ComponentKind, DeviceConfig, DeviceTable, FailurePolicy,
};

/// The mock board's boot-signal vocabulary. The schema carries these
/// opaquely; only this board's `EvidenceReader` gives them meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSignal {
    /// A boot-complete GPIO line, by index.
    Gpio(u8),
    /// The device's MCTP endpoint answers as ready.
    MctpReady,
    /// The device sends a heartbeat message (latched; the reset path
    /// clears it).
    Heartbeat,
}

/// Declaration order is the boot order: the orchestrator releases devices
/// top to bottom, one at a time. This table is the authority — the
/// orchestrator's chain of trust is built from it
/// (`Chain::from_table`), never beside it.
///
/// The mock board's reset controller addresses reset lines by plain index,
/// so the reset id type is `u8`.
pub const MANAGED_DEVICES: DeviceTable<u8, MockSignal> = DeviceTable::new(&[
    // Direct-flash SPI device (BMC archetype): the eRoT fronts its flash.
    // No iRoT, so the eRoT's check is the only trust gate (Passive), and
    // the platform is pointless without its BMC (Required). Single
    // checkpoint: it raises a boot-complete GPIO.
    DeviceConfig::new(
        "bmc",
        7,
        ComponentKind::Passive,
        FailurePolicy::Required,
        &[BootCheckpoint::new(
            "boot-complete",
            MockSignal::Gpio(12),
            Duration::from_secs(90),
        )],
    ),
    // PLDM device (NIC archetype): self-updating, SPDM-capable — an iRoT
    // of its own (Active), and the platform can serve degraded without it
    // (Isolable). Two checkpoints, exercising the multi-checkpoint path:
    // transport up first, then proof the workload is alive.
    DeviceConfig::new(
        "nic",
        3,
        ComponentKind::Active,
        FailurePolicy::Isolable,
        &[
            BootCheckpoint::new("mctp-ready", MockSignal::MctpReady, Duration::from_secs(20)),
            BootCheckpoint::new("heartbeat", MockSignal::Heartbeat, Duration::from_secs(10)),
        ],
    ),
]);

/// Derived, not declared: the orchestrator's chain capacity is exactly the
/// table's length.
pub const DEVICE_COUNT: usize = MANAGED_DEVICES.devices().len();

/// Derived, not declared: the orchestrator's proven effect-buffer floor
/// (`E >= 2 * N + 2`), with no headroom — headroom would be a second,
/// hand-picked number.
pub const EFFECT_CAP: usize = 2 * DEVICE_COUNT + 2;

/// Consecutive failed-restore attempts per device before its failure
/// policy is consulted. A genuine board fact — not derivable — so it is
/// declared here, next to the rest of the board's boot policy.
pub const MAX_RETRY: u8 = 3;

/// Board-local checks the schema constructors cannot do — they know the
/// schema's shape, not this board's meanings. Const-fence pattern: a bad
/// signal fails the build.
const fn validate_signals(devices: &[DeviceConfig<u8, MockSignal>]) {
    let mut i = 0;
    while i < devices.len() {
        let checkpoints = devices[i].checkpoints();
        let mut c = 0;
        while c < checkpoints.len() {
            if let MockSignal::Gpio(line) = *checkpoints[c].signal() {
                // The mock ready-line bank packs 32 lines, SGPIO-style.
                assert!(line < 32, "gpio signal names a line outside the bank");
            }
            c += 1;
        }
        i += 1;
    }
}

const _: () = validate_signals(MANAGED_DEVICES.devices());

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use openprot_orchestrator_sm::{
        Chain, ComponentId, Effect, Event, Orchestrator, PowerOnResult, State,
    };

    use super::*;

    /// End-to-end handoff: the table this board declares is everything the
    /// orchestrator needs. Kind and policy come from the table too — the
    /// bmc is `Passive`, so releasing it must advance the walk speculatively
    /// instead of blocking in `AwaitingReady`.
    #[test]
    fn table_feeds_the_orchestrator() {
        let chain = Chain::<DEVICE_COUNT>::from_table(&MANAGED_DEVICES);
        let mut orch = Orchestrator::<DEVICE_COUNT, EFFECT_CAP>::new(chain, MAX_RETRY);
        let bmc = ComponentId::new(0);
        let nic = ComponentId::new(1);

        let mut effects: Vec<Effect> = Vec::new();
        orch.dispatch_with(Event::PowerGood(PowerOnResult::Provisioned), |e| {
            effects.push(e);
            Ok(())
        });
        assert_eq!(
            effects,
            [Effect::ReadFirmware(bmc), Effect::VerifyFirmware(bmc)]
        );
        assert_eq!(orch.state(), State::PreSupervision);

        effects.clear();
        orch.dispatch_with(Event::VerificationPassed(bmc), |e| {
            effects.push(e);
            Ok(())
        });
        assert_eq!(
            effects,
            [
                Effect::ReleaseReset(bmc),
                Effect::ReadFirmware(nic),
                Effect::VerifyFirmware(nic),
            ]
        );
        assert_eq!(orch.state(), State::PreSupervision);
    }
}
