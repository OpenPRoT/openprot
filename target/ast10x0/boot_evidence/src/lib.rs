// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Boot-evidence wiring for the AST10x0 eRoT board.
//!
//! Binds the board's boot-checkpoint signals to concrete hardware: each
//! signal in the device table maps to one `GpioBootMonitor` on a
//! specific SGPIOM pin. The orchestrator never learns which pin belongs
//! to which device; this crate makes that binding once.

#![no_std]

use openprot_hal_blocking::gpio_port::{ActivePolarity, GpioPort};
use orchestrator_capabilities::{BootStatus, EvidenceReader};
use orchestrator_hal_adapters::{GpioBootMonitor, MonitorError};

/// Bit offset of BL1's ready line in SGPIOM bank EH (pin 42, bank base 32).
/// The typed `SgpiomMask` binding lives in the platform driver; this crate
/// records the offset so the table and the wiring agree on a single source.
pub const BL1_PIN_OFFSET: u32 = 10;

/// BL1 ready line is active-high: the BMC asserts the pin when bl1 is up.
pub const BL1_POLARITY: ActivePolarity = ActivePolarity::ActiveHigh;

/// Boot-checkpoint signal vocabulary for the BMC. Each variant names
/// one checkpoint in the device table; the `EvidenceReader` impl
/// dispatches it to the right `GpioBootMonitor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmcSignal {
    /// First-stage bootloader ready (SGPIOM pin 42, bank EH bit 10).
    Bl1,
}

/// Reads the BMC's boot evidence off the SGPIOM. One `GpioBootMonitor`
/// per checkpoint signal, all sharing the same bank port.
pub struct BmcBootReader<'a, P: GpioPort> {
    bl1: GpioBootMonitor<'a, P>,
}

impl<'a, P: GpioPort> BmcBootReader<'a, P> {
    /// Binds the reader to its monitors. Each monitor is constructed by
    /// the platform driver at bring-up from a `(port, pin, polarity)`
    /// triple.
    pub fn new(bl1: GpioBootMonitor<'a, P>) -> Self {
        Self { bl1 }
    }
}

impl<P: GpioPort> EvidenceReader<BmcSignal> for BmcBootReader<'_, P>
where
    P::Error: 'static,
{
    type Error = MonitorError<P::Error>;

    fn read(&mut self, signal: &BmcSignal) -> Result<BootStatus, Self::Error> {
        match signal {
            BmcSignal::Bl1 => self.bl1.boot_status(),
        }
    }
}

/// The BMC's device table entry. One checkpoint: bl1 on SGPIOM pin 42,
/// 500 ms window.
pub const BMC_DEVICE: orchestrator_config::DeviceConfig<u8, BmcSignal> =
    orchestrator_config::DeviceConfig::new(
        "bmc",
        0,
        &[orchestrator_config::BootCheckpoint::new(
            "bl1",
            BmcSignal::Bl1,
            core::time::Duration::from_millis(500),
        )],
    );

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_hal_blocking::gpio_port::{GpioError, GpioErrorKind, GpioErrorType, PinMask};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Mask(u32);

    impl PinMask for Mask {
        fn empty() -> Self {
            Self(0)
        }
        fn all() -> Self {
            Self(u32::MAX)
        }
        fn is_empty(&self) -> bool {
            self.0 == 0
        }
        fn contains(&self, other: Self) -> bool {
            self.0 & other.0 == other.0
        }
        fn union(&self, other: Self) -> Self {
            Self(self.0 | other.0)
        }
        fn intersection(&self, other: Self) -> Self {
            Self(self.0 & other.0)
        }
        fn toggle(&self) -> Self {
            Self(!self.0)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MockError;

    impl GpioError for MockError {
        fn kind(&self) -> GpioErrorKind {
            GpioErrorKind::HardwareFailure
        }
    }

    struct MockPort {
        input: Mask,
    }

    impl GpioErrorType for MockPort {
        type Error = MockError;
    }

    impl GpioPort for MockPort {
        type Config = ();
        type Mask = Mask;

        fn read_input(&self) -> Result<Mask, MockError> {
            Ok(self.input)
        }
        fn configure(&mut self, _: Mask, _: ()) -> Result<(), MockError> {
            panic!("reader must not configure pins");
        }
        fn set_reset(&mut self, _: Mask, _: Mask) -> Result<(), MockError> {
            panic!("reader must not drive outputs");
        }
        fn toggle(&mut self, _: Mask) -> Result<(), MockError> {
            panic!("reader must not drive outputs");
        }
    }

    const BL1_PIN: Mask = Mask(1 << BL1_PIN_OFFSET);

    #[test]
    fn bl1_reads_booted_when_pin42_is_high() {
        let port = MockPort {
            input: Mask(1 << 10),
        };
        let mon = GpioBootMonitor::new(&port, BL1_PIN, BL1_POLARITY);
        let mut reader = BmcBootReader::new(mon);

        assert_eq!(
            reader.read(&BmcSignal::Bl1).expect("read failed"),
            BootStatus::Booted,
        );
    }

    #[test]
    fn bl1_reads_booting_when_pin42_is_low() {
        let port = MockPort { input: Mask(0) };
        let mon = GpioBootMonitor::new(&port, BL1_PIN, BL1_POLARITY);
        let mut reader = BmcBootReader::new(mon);

        assert_eq!(
            reader.read(&BmcSignal::Bl1).expect("read failed"),
            BootStatus::Booting,
        );
    }

    #[test]
    fn bmc_device_table_has_one_bl1_checkpoint() {
        assert_eq!(BMC_DEVICE.name(), "bmc");
        assert_eq!(BMC_DEVICE.checkpoints().len(), 1);
        assert_eq!(BMC_DEVICE.checkpoints()[0].name(), "bl1");
        assert_eq!(*BMC_DEVICE.checkpoints()[0].signal(), BmcSignal::Bl1);
    }
}
