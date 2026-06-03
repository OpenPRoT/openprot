// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot-hal-blocking` slave-trait implementations for the AST1060 I2C
//! driver — the slave-mode sibling of [`hal_impl`](super::hal_impl) (which
//! implements the embedded-hal master `I2c` trait).
//!
//! These delegate to the driver's existing inherent slave methods
//! (`configure_slave`/`enable_slave`/`slave_read`/`handle_slave_interrupt`).
//! No new hardware logic. `ErrorType` is already implemented for
//! `Ast1060I2c` by `hal_impl`, so — after the `I2cSlaveCore: ErrorType`
//! correction in `hal/blocking` — this is pure delegation with no
//! `I2cHardwareCore` (init/timing/recover) baggage.
//!
//! Scope: the thin notification slice (configure / enable / disable / poll /
//! read). Methods outside that path are honest best-effort and marked.

use embedded_hal::i2c::SevenBitAddress;
use openprot_hal_blocking::i2c_hardware::slave::{I2cIsrEvent, I2cSlaveBuffer, I2cSlaveCore};
use openprot_hal_blocking::i2c_hardware::I2cBusRecovery;

use super::controller::Ast1060I2c;
use super::error::I2cError;
use super::slave::{SlaveConfig, SlaveEvent};

/// Driver `SlaveEvent` → HAL `I2cIsrEvent`. The notification path only acts on
/// the data-received case; the rest map to their nearest HAL kind.
fn to_hal_event(ev: SlaveEvent) -> I2cIsrEvent {
    match ev {
        SlaveEvent::ReadRequest => I2cIsrEvent::SlaveRdReq,
        SlaveEvent::WriteRequest => I2cIsrEvent::SlaveWrReq,
        SlaveEvent::DataReceived { .. } | SlaveEvent::DataReceivedAndSent { .. } => {
            I2cIsrEvent::SlaveWrRecvd
        }
        SlaveEvent::DataSent { .. } => I2cIsrEvent::SlaveRdProc,
        SlaveEvent::Stop => I2cIsrEvent::SlaveStop,
    }
}

/// Bytes received in the event, if any (the only thing the drain path needs).
fn rx_len(ev: SlaveEvent) -> Option<usize> {
    match ev {
        SlaveEvent::DataReceived { len } => Some(len),
        SlaveEvent::DataReceivedAndSent { rx_len, .. } => Some(rx_len),
        _ => None,
    }
}

impl<Y: FnMut(u32)> I2cSlaveCore<SevenBitAddress> for Ast1060I2c<'_, Y> {
    fn configure_slave_address(&mut self, addr: SevenBitAddress) -> Result<(), Self::Error> {
        let cfg = SlaveConfig::new(addr)?;
        self.configure_slave(&cfg)
    }

    fn enable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.enable_slave();
        Ok(())
    }

    fn disable_slave_mode(&mut self) -> Result<(), Self::Error> {
        self.disable_slave();
        Ok(())
    }

    fn is_slave_mode_enabled(&self) -> bool {
        self.regs().i2cc00().read().enbl_slave_fn().bit()
    }

    fn slave_address(&self) -> Option<SevenBitAddress> {
        if self.is_slave_mode_enabled() {
            Some(self.regs().i2cs40().read().slave_dev_addr1().bits())
        } else {
            None
        }
    }
}

impl<Y: FnMut(u32)> I2cSlaveBuffer<SevenBitAddress> for Ast1060I2c<'_, Y> {
    fn read_slave_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        self.slave_read(buffer)
    }

    fn write_slave_response(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.slave_write(data).map(|_| ())
    }

    /// Process the slave interrupt status; `Some(len)` iff a write from the
    /// master was received (the data is then read via `read_slave_buffer`).
    /// This is the exact drain pattern the server-runtime IRQ path uses.
    fn poll_slave_data(&mut self) -> Result<Option<usize>, Self::Error> {
        Ok(self.handle_slave_interrupt().and_then(rx_len))
    }

    /// Best-effort: drain any pending RX so the next transaction starts clean.
    fn clear_slave_buffer(&mut self) -> Result<(), Self::Error> {
        if self.slave_has_data() {
            let mut scratch = [0u8; 32];
            let _ = self.slave_read(&mut scratch)?;
        }
        Ok(())
    }

    /// AST1060 slave uses the 32-byte packet-mode buffer.
    fn tx_buffer_space(&self) -> Result<usize, Self::Error> {
        Ok(32)
    }

    /// Coarse: the driver does not expose an exact pending count without
    /// consuming the buffer. Returns 0/1 (use `poll_slave_data` for the path
    /// that matters). Not used by the notification slice.
    fn rx_buffer_count(&self) -> Result<usize, Self::Error> {
        Ok(usize::from(self.slave_has_data()))
    }
}

impl<Y: FnMut(u32)> Ast1060I2c<'_, Y> {
    /// Return the next slave event and rx length, if any.
    /// This exposes the full hardware event (ReadRequest, Stop, etc.) alongside
    /// the receive count, so the server-runtime can store the actual event kind
    /// rather than always hardcoding DataReceived.
    pub fn try_next_slave_event(&mut self) -> Result<Option<(I2cIsrEvent, usize)>, I2cError> {
        let Some(ev) = self.handle_slave_interrupt() else {
            return Ok(None);
        };
        let kind = to_hal_event(ev);
        let len = rx_len(ev).unwrap_or(0);
        Ok(Some((kind, len)))
    }
}

impl<Y: FnMut(u32)> I2cBusRecovery for Ast1060I2c<'_, Y> {
    fn recover_bus(&mut self) -> Result<(), Self::Error> {
        Ast1060I2c::recover_bus(self)
    }
}
