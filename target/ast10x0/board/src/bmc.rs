// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The mock BMC as a managed device: this board's supervision vocabulary.
//!
//! Everything here is the RoT's view. The RoT drives the BMC's reset line and
//! reads its ready line, so the ids naming those two wires, the window the RoT
//! is willing to wait, and the reader that resolves an id to evidence all live
//! on this side. The BMC itself knows none of it — it boots and drives one pin.
//!
//! Both ids are enums rather than pin numbers: the table stays printable and
//! comparable, and a forgotten signal is a non-exhaustive `match`, which is a
//! build error rather than a runtime hole.

use core::time::Duration;
use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::{DelayNs, InputPin, OutputPin};
use orchestrator_capabilities::{BootStatus, EvidenceReader};
use orchestrator_config::{BootCheckpoint, DeviceConfig};
use orchestrator_hal_adapters::{
    GpioReadyMonitor, GpioResetControl, ReadyLineError, ResetLineError,
};

/// Reset lines this board drives. `RoT_BMC_RESET_L` is the only one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmcReset {
    /// The mock BMC's reset input.
    Bmc,
}

/// Boot evidence this board can read from the mock BMC.
///
/// One ready line answers only "up yet?", so the vocabulary has one entry;
/// a second checkpoint would need a second wire or a message path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmcProbe {
    /// The BMC's boot-complete line, driven high once it has booted.
    Ready,
}

/// How long the RoT waits for the BMC to report ready before calling the
/// attempt failed. Generous: a missed window costs a reset cycle, and nothing
/// downstream is racing it.
const READY_WINDOW: Duration = Duration::from_secs(10);

const CHECKPOINTS: &[BootCheckpoint<BmcProbe>] = &[BootCheckpoint::new(
    "bmc-ready",
    BmcProbe::Ready,
    READY_WINDOW,
)];

/// The mock BMC's entry in this board's device table.
pub const BMC: DeviceConfig<BmcReset, BmcProbe> =
    DeviceConfig::new("bmc", BmcReset::Bmc, CHECKPOINTS, None);

/// The BMC's ready line is driven high when it has booted.
///
/// Active high is the fail-safe direction here: held in reset, unpowered, and
/// a disconnected wire all read low, so the RoT waits out its window and
/// reports honestly instead of believing a device that is not there.
const READY_ACTIVE: ActivePolarity = ActivePolarity::ActiveHigh;

/// `RoT_BMC_RESET_L` holds the BMC in reset when driven low.
const RESET_ACTIVE: ActivePolarity = ActivePolarity::ActiveLow;

/// This board's reset control, once a pin and a delay are bound to it.
pub type BmcResetControl<P, D> = GpioResetControl<P, D, BmcReset>;

/// Binds `reset_pin` as `RoT_BMC_RESET_L`, driving the BMC into reset.
///
/// # Errors
///
/// Returns an error if the line cannot be driven.
pub fn bind_reset<P: OutputPin, D: DelayNs>(
    reset_pin: P,
    delay: D,
) -> Result<BmcResetControl<P, D>, ResetLineError<P::Error>> {
    GpioResetControl::new(reset_pin, delay, BmcReset::Bmc, RESET_ACTIVE)
}

/// Resolves this board's probe ids to evidence.
///
/// Generic over the input pin so the vocabulary is independent of how the pin
/// is obtained; bring-up supplies the real GPIO.
pub struct BmcReader<P> {
    ready: GpioReadyMonitor<P>,
}

impl<P: InputPin> BmcReader<P> {
    /// Binds `ready_pin` as the BMC's boot-complete line.
    pub fn new(ready_pin: P) -> Self {
        Self {
            ready: GpioReadyMonitor::new(ready_pin, READY_ACTIVE),
        }
    }
}

impl<P: InputPin> EvidenceReader<BmcProbe> for BmcReader<P>
where
    P::Error: 'static,
{
    type Error = ReadyLineError<P::Error>;

    fn read(&mut self, probe: &BmcProbe) -> Result<BootStatus, Self::Error> {
        match probe {
            BmcProbe::Ready => self.ready.boot_status(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::digital;
    use openprot_hal_blocking::system_control::ResetControl;
    use orchestrator_capabilities::{BootWatch, FailureCause, WalkVerdict};
    use orchestrator_checkpoint_walk::CheckpointWalk;

    #[derive(Debug)]
    struct PinFault;

    impl digital::Error for PinFault {
        fn kind(&self) -> digital::ErrorKind {
            digital::ErrorKind::Other
        }
    }

    /// The BMC's ready line as the RoT sees it: low until the device has
    /// booted, which it does after `boots_after` reads. `None` is a device
    /// that never comes up.
    struct ReadyLine {
        boots_after: Option<usize>,
        reads: usize,
    }

    impl ReadyLine {
        fn booted() -> Self {
            Self {
                boots_after: Some(0),
                reads: 0,
            }
        }

        fn hung() -> Self {
            Self {
                boots_after: None,
                reads: 0,
            }
        }

        fn late(reads: usize) -> Self {
            Self {
                boots_after: Some(reads),
                reads: 0,
            }
        }
    }

    impl digital::ErrorType for ReadyLine {
        type Error = PinFault;
    }

    impl digital::InputPin for ReadyLine {
        fn is_high(&mut self) -> Result<bool, PinFault> {
            self.reads += 1;
            Ok(matches!(self.boots_after, Some(n) if self.reads > n))
        }

        fn is_low(&mut self) -> Result<bool, PinFault> {
            Ok(!self.is_high()?)
        }
    }

    /// The BMC's reset line as the RoT drives it, remembering its level.
    struct ResetLine {
        high: bool,
    }

    impl digital::ErrorType for ResetLine {
        type Error = PinFault;
    }

    impl digital::OutputPin for ResetLine {
        fn set_low(&mut self) -> Result<(), PinFault> {
            self.high = false;
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), PinFault> {
            self.high = true;
            Ok(())
        }
    }

    /// Takes the pulse's wait instantly; the hold time is the HAL's business.
    struct NoDelay;

    impl DelayNs for NoDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    #[test]
    fn the_table_declares_one_ready_checkpoint() {
        assert_eq!(BMC.name(), "bmc");
        assert_eq!(*BMC.reset_signal(), BmcReset::Bmc);

        let checkpoints = BMC.checkpoints();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].name(), "bmc-ready");
        assert_eq!(*checkpoints[0].probe(), BmcProbe::Ready);
        assert_eq!(checkpoints[0].timeout(), READY_WINDOW);
    }

    #[test]
    fn the_reader_resolves_ready_to_the_line_level() {
        let mut booting = BmcReader::new(ReadyLine::hung());
        let mut booted = BmcReader::new(ReadyLine::booted());

        assert_eq!(
            booting.read(&BmcProbe::Ready).expect("read failed"),
            BootStatus::Booting
        );
        assert_eq!(
            booted.read(&BmcProbe::Ready).expect("read failed"),
            BootStatus::Booted
        );
    }

    /// The table's signals must be exactly what the reader answers — the walk
    /// reads whatever the checkpoints name.
    #[test]
    fn the_reader_answers_every_signal_the_table_names() {
        let mut reader = BmcReader::new(ReadyLine::booted());

        for checkpoint in BMC.checkpoints() {
            assert_eq!(
                reader.read(checkpoint.probe()).expect("read failed"),
                BootStatus::Booted
            );
        }
    }

    /// Binding drives the line rather than assuming it: the BMC is held from
    /// the moment the RoT owns its reset.
    #[test]
    fn binding_the_reset_line_holds_the_bmc() {
        let reset = bind_reset(ResetLine { high: true }, NoDelay).expect("bind failed");

        assert!(!reset.pin().high);
        assert!(reset
            .reset_is_asserted(&BmcReset::Bmc)
            .expect("read failed"));
    }

    #[test]
    fn a_pulse_leaves_the_bmc_released_to_boot() {
        let mut reset = bind_reset(ResetLine { high: true }, NoDelay).expect("bind failed");

        reset
            .reset_pulse(&BmcReset::Bmc, Duration::from_millis(10))
            .expect("pulse failed");

        assert!(reset.pin().high);
        assert!(!reset
            .reset_is_asserted(&BmcReset::Bmc)
            .expect("read failed"));
    }

    fn walk<P: InputPin>(ready: P) -> CheckpointWalk<BmcReader<P>, BmcProbe>
    where
        P::Error: 'static,
    {
        CheckpointWalk::new(BmcReader::new(ready), &BMC)
    }

    #[test]
    fn a_booted_bmc_completes_the_walk_on_the_first_poll() {
        let mut walk = walk(ReadyLine::booted());
        walk.arm();

        assert_eq!(walk.poll(0), WalkVerdict::Complete);
    }

    #[test]
    fn a_bmc_still_booting_holds_the_walk_until_its_line_rises() {
        let mut walk = walk(ReadyLine::late(2));
        walk.arm();

        let waiting = WalkVerdict::Waiting {
            deadline_millis: READY_WINDOW.as_millis() as u64,
        };
        assert_eq!(walk.poll(0), waiting);
        assert_eq!(walk.poll(1), waiting);
        assert_eq!(walk.poll(2), WalkVerdict::Complete);
    }

    /// A BMC that never comes up costs the window and is reported by name.
    #[test]
    fn a_bmc_that_never_reports_ready_times_out() {
        let mut walk = walk(ReadyLine::hung());
        walk.arm();

        let deadline = READY_WINDOW.as_millis() as u64;
        assert_eq!(
            walk.poll(0),
            WalkVerdict::Waiting {
                deadline_millis: deadline
            }
        );
        assert_eq!(
            walk.poll(deadline),
            WalkVerdict::Failed {
                checkpoint: "bmc-ready",
                cause: FailureCause::TimedOut,
            }
        );
    }
}
