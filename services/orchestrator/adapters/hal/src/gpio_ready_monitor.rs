// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! HAL-backed boot-status reader over a single owned GPIO input line.

use openprot_hal_blocking::gpio_port::ActivePolarity;
use openprot_hal_blocking::InputPin;
use orchestrator_capabilities::BootStatus;

/// The ready line could not be read.
///
/// Carries the pin's own error so the implementation's detail survives; the
/// `Display`/`core::error::Error` machinery is what `EvidenceReader` asks of a
/// reader error and what `InputPin::Error` does not supply.
#[derive(Debug)]
pub struct ReadyLineError<E>(pub E);

impl<E: core::fmt::Debug> core::fmt::Display for ReadyLineError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ready line unreadable: {:?}", self.0)
    }
}

impl<E: core::fmt::Debug + 'static> core::error::Error for ReadyLineError<E> {}

/// Binds one owned GPIO input line to a managed device's boot-complete signal.
///
/// The sibling of [`GpioBootMonitor`], differing only in how the line is named:
/// that one borrows a `GpioPort` and selects a line by mask, because an SGPIOM
/// bank packs unrelated lines several devices share. A dedicated ready line on
/// a parallel bank is one pin, so owning it is the exclusivity guarantee — the
/// same reasoning that shapes [`GpioResetControl`] on the output side.
///
/// A single ready line can only answer "up yet?", so this reader reports the
/// [`BootStatus::Booting`]/[`BootStatus::Booted`] subset.
///
/// [`GpioBootMonitor`]: crate::GpioBootMonitor
/// [`GpioResetControl`]: crate::GpioResetControl
pub struct GpioReadyMonitor<P> {
    pin: P,
    active: ActivePolarity,
}

impl<P: InputPin> GpioReadyMonitor<P> {
    /// Binds `pin`, asserted per `active`, as a device's boot-complete signal.
    pub fn new(pin: P, active: ActivePolarity) -> Self {
        Self { pin, active }
    }

    /// Returns the current liveness of the device.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the pin's level read.
    pub fn boot_status(&mut self) -> Result<BootStatus, ReadyLineError<P::Error>> {
        let high = self.pin.is_high().map_err(ReadyLineError)?;
        let booted = match self.active {
            ActivePolarity::ActiveHigh => high,
            ActivePolarity::ActiveLow => !high,
        };
        Ok(if booted {
            BootStatus::Booted
        } else {
            BootStatus::Booting
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::digital;

    #[derive(Debug, PartialEq, Eq)]
    struct PinFault;

    impl digital::Error for PinFault {
        fn kind(&self) -> digital::ErrorKind {
            digital::ErrorKind::Other
        }
    }

    struct MockPin {
        high: bool,
        fail: bool,
    }

    impl MockPin {
        fn at(high: bool) -> Self {
            Self { high, fail: false }
        }

        fn failing() -> Self {
            Self {
                high: false,
                fail: true,
            }
        }
    }

    impl digital::ErrorType for MockPin {
        type Error = PinFault;
    }

    impl digital::InputPin for MockPin {
        fn is_high(&mut self) -> Result<bool, PinFault> {
            if self.fail {
                return Err(PinFault);
            }
            Ok(self.high)
        }

        fn is_low(&mut self) -> Result<bool, PinFault> {
            Ok(!self.is_high()?)
        }
    }

    #[test]
    fn active_high_reads_a_high_line_as_booted() {
        let mut bmc = GpioReadyMonitor::new(MockPin::at(true), ActivePolarity::ActiveHigh);

        assert_eq!(bmc.boot_status().expect("read failed"), BootStatus::Booted);
    }

    // The fail-safe direction: a line held low by reset, by an unpowered
    // device, or by a disconnected wire reads Booting, so the walk times out
    // honestly instead of reporting a device that was never there.
    #[test]
    fn active_high_reads_a_low_line_as_booting() {
        let mut bmc = GpioReadyMonitor::new(MockPin::at(false), ActivePolarity::ActiveHigh);

        assert_eq!(bmc.boot_status().expect("read failed"), BootStatus::Booting);
    }

    #[test]
    fn active_low_inverts_both_levels() {
        let mut low = GpioReadyMonitor::new(MockPin::at(false), ActivePolarity::ActiveLow);
        let mut high = GpioReadyMonitor::new(MockPin::at(true), ActivePolarity::ActiveLow);

        assert_eq!(low.boot_status().expect("read failed"), BootStatus::Booted);
        assert_eq!(high.boot_status().expect("read failed"), BootStatus::Booting);
    }

    #[test]
    fn a_pin_fault_surfaces_as_a_reader_error() {
        let mut bmc = GpioReadyMonitor::new(MockPin::failing(), ActivePolarity::ActiveHigh);

        let err = bmc
            .boot_status()
            .expect_err("expected the pin fault to propagate");

        // Display comes from the core::error::Error bound EvidenceReader wants,
        // not a Debug dump.
        assert_eq!(err.to_string(), "ready line unreadable: PinFault");
    }

    /// Compile-time fence: the reader error must satisfy what
    /// `EvidenceReader::Error` demands of it.
    fn _assert_reader_error<E: core::error::Error>() {}

    #[test]
    fn the_error_satisfies_the_evidence_reader_bound() {
        _assert_reader_error::<ReadyLineError<PinFault>>();
    }
}
