// Licensed under the Apache-2.0 license

//! HAL trait implementations for I3C
//!
//! Implementations of external traits from `embedded-hal`, `proposed-traits`, etc.

use core::convert::Infallible;

use embedded_hal::i2c::SevenBitAddress;
use proposed_traits::i2c_target::I2CCoreTarget;
use proposed_traits::i3c_master::{ErrorKind, ErrorType, I3c, I3cSpeed};
use proposed_traits::i3c_target::{DynamicAddressable, I3CCoreTarget, IBICapable};

use super::ccc;
use super::config::I3cTargetConfig;
use super::controller::I3cController;
use super::error::I3cError;
use super::hardware::HardwareInterface;
use super::types::{I3cIbi, I3cIbiType};

// =============================================================================
// Error Type Implementation
// =============================================================================

impl<H: HardwareInterface> ErrorType for I3cController<H> {
    type Error = I3cError;
}

// =============================================================================
// I3C Master Trait Implementation
// =============================================================================

impl<H: HardwareInterface> I3c for I3cController<H> {
    fn assign_dynamic_address(
        &mut self,
        static_address: SevenBitAddress,
    ) -> Result<SevenBitAddress, Self::Error> {
        let slot = self
            .config
            .attached
            .pos_of_addr(static_address)
            .ok_or(I3cError::from(ErrorKind::DynamicAddressConflict))?;

        self.hw
            .do_entdaa(&mut self.config, slot.into())
            .map_err(|_| I3cError::from(ErrorKind::DynamicAddressConflict))?;

        let pid = ccc::ccc_getpid(&mut self.hw, &mut self.config, static_address)
            .map_err(|_| I3cError::from(ErrorKind::InvalidCcc))?;

        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(static_address)
            .ok_or(I3cError::Other)?;

        let old_pid = self
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .pid;

        if let Some(op) = old_pid {
            if pid != op {
                return Err(I3cError::Other);
            }
        }

        let bcr = ccc::ccc_getbcr(&mut self.hw, &mut self.config, static_address)
            .map_err(|_| I3cError::from(ErrorKind::InvalidCcc))?;

        {
            let dev = self
                .config
                .attached
                .devices
                .get_mut(dev_idx)
                .ok_or(I3cError::Other)?;

            dev.pid = Some(pid);
            dev.bcr = bcr;
        }

        let dyn_addr: SevenBitAddress = self
            .config
            .attached
            .devices
            .get(dev_idx)
            .ok_or(I3cError::Other)?
            .dyn_addr;

        self.hw
            .ibi_enable(&mut self.config, dyn_addr)
            .map_err(|_| I3cError::Other)?;

        Ok(dyn_addr)
    }

    fn acknowledge_ibi(&mut self, address: SevenBitAddress) -> Result<(), Self::Error> {
        let dev_idx = self
            .config
            .attached
            .find_dev_idx_by_addr(address)
            .ok_or(I3cError::Other)?;

        if self.config.attached.devices[dev_idx].pid.is_none() {
            return Err(I3cError::Other);
        }

        Ok(())
    }

    fn handle_hot_join(&mut self) -> Result<(), Self::Error> {
        // Only need to call assign_dynamic_address after receiving hot-join IBI
        Ok(())
    }

    fn set_bus_speed(&mut self, _speed: I3cSpeed) -> Result<(), Self::Error> {
        // AST1060 I3C driver doesn't support changing bus speed dynamically
        Ok(())
    }

    fn request_mastership(&mut self) -> Result<(), Self::Error> {
        // AST1060 controller doesn't support multi-master
        Ok(())
    }
}

// =============================================================================
// I2C Error Type Implementation
// =============================================================================

impl<H: HardwareInterface> embedded_hal::i2c::ErrorType for I3cController<H> {
    type Error = Infallible;
}

// =============================================================================
// I2C Target Trait Implementation
// =============================================================================

impl<H: HardwareInterface> I2CCoreTarget for I3cController<H> {
    #[inline]
    fn init(&mut self, own_addr: u8) -> Result<(), Self::Error> {
        if let Some(t) = self.config.target_config.as_mut() {
            if t.addr.is_none() {
                t.addr = Some(own_addr);
            }
        } else {
            self.config.target_config =
                Some(I3cTargetConfig::new(0, Some(own_addr), /* mdb */ 0xae));
        }
        Ok(())
    }

    #[inline]
    fn on_address_match(&mut self, addr: u8) -> bool {
        self.config.target_config.as_ref().and_then(|t| t.addr) == Some(addr)
    }

    #[inline]
    fn on_transaction_start(&mut self, _is_read: bool) {}

    #[inline]
    fn on_stop(&mut self) {}
}

// =============================================================================
// I3C Target Trait Implementation
// =============================================================================

impl<H: HardwareInterface> I3CCoreTarget for I3cController<H> {}

impl<H: HardwareInterface> DynamicAddressable for I3cController<H> {
    fn on_dynamic_address_assigned(&mut self, _new_address: u8) {
        self.config.sir_allowed_by_sw = true;
    }
}

impl<H: HardwareInterface> IBICapable for I3cController<H> {
    fn wants_ibi(&self) -> bool {
        true
    }

    fn get_ibi_payload(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        let (da, mdb) = match self.config.target_config.as_ref() {
            Some(t) => (
                match t.addr {
                    Some(da) => da,
                    None => return Ok(0),
                },
                t.mdb,
            ),
            None => return Ok(0),
        };

        let addr_rnw = (da << 1) | 0x1;
        let mut crc = crc8_ccitt(0, &[addr_rnw]);
        crc = crc8_ccitt(crc, &[mdb]);

        let payload = [mdb, crc];
        let mut ibi = I3cIbi {
            ibi_type: I3cIbiType::TargetIntr,
            payload: Some(&payload),
        };
        let rc = self
            .hw
            .target_pending_read_notify(&mut self.config, buffer, &mut ibi);

        match rc {
            Ok(()) => Ok(buffer.len() + payload.len()),
            _ => Ok(0),
        }
    }

    fn on_ibi_acknowledged(&mut self) {}
}

// =============================================================================
// Helper Functions
// =============================================================================

/// CRC-8 CCITT calculation
#[inline]
fn crc8_ccitt(mut crc: u8, data: &[u8]) -> u8 {
    for &b in data {
        let mut x = crc ^ b;
        for _ in 0..8 {
            x = if (x & 0x80) != 0 {
                (x << 1) ^ 0x07
            } else {
                x << 1
            };
        }
        crc = x;
    }
    crc
}

// =============================================================================
// Error Conversions
// =============================================================================

impl From<ErrorKind> for I3cError {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::DynamicAddressConflict => Self::AddrInUse,
            ErrorKind::InvalidCcc => Self::Invalid,
            _ => Self::Other,
        }
    }
}
