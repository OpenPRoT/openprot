// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OCP recovery driver for the Caliptra Subsystem i3c-core.
//!
//! The i3c-core's recovery handler answers the recovery agent's (the BMC's)
//! commands in hardware, at the recovery (virtual) target address. It answers
//! each read from a set of CSRs and applies the agent's writes to them. The
//! MCU reads and writes the same CSRs over AXI: it sets the status that the
//! agent will read next, and reads what the agent has written.
//!
//! At the end of cold boot, the MCU ROM sets `RECOVERY_STATUS` to "recovery
//! successful" and `DEVICE_STATUS` to 0. A ROM built with
//! `recovery_status_open` sets `RECOVERY_STATUS` to "booting recovery image"
//! instead. The runtime asks for another image by setting both again.
//!
//! Caliptra reads the image FIFO with its DMA engine. The MCU has no DMA
//! path to the FIFO, so [`I3cRecovery`] reads `INDIRECT_FIFO_DATA` one word at
//! a time and checks `INDIRECT_FIFO_STATUS_0.EMPTY` before each read.
//!
//! The recovery registers are separate from the TTI registers, so this
//! driver can run next to an I3C target driver on the same core.

#![cfg_attr(not(test), no_std)]

use core::mem::offset_of;
use core::num::NonZero;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::i3c::bits::{
    DeviceStatus0, IndirectFifoCtrl0, IndirectFifoStatus0, RecIntfCfg, RecoveryCtrl,
    RecoveryStatus as RecoveryStatusReg,
};
use caliptra_ss_registers::i3c::regs::I3c;
use hal_ocp_recovery::{DeviceStatus, ImageIndex, RecoveryDriver, RecoveryReason, RecoveryStatus};
use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

/// Error module for the i3c-core recovery driver.
pub const OCP_RECOVERY_VEER: ErrorModule = ErrorModule::new(0x4f52); // ascii `OR`.
/// The agent selected a component memory space other than 0, which is the
/// only one the i3c-core FIFO supports.
pub const OCP_RECOVERY_VEER_UNSUPPORTED_CMS: ErrorCode =
    OCP_RECOVERY_VEER.from_pw(0, Error::Unimplemented);

/// `RECOVERY_CTRL.ACTIVATE_REC_IMG` value with which the agent asks for
/// activation.
const ACTIVATE_RECOVERY_IMAGE: u32 = 0xf;

/// Value written to `RECOVERY_CTRL.ACTIVATE_REC_IMG` to clear an activation
/// request. The field is write-1-to-clear in hardware, so this clears bit 0
/// and the field no longer reads 0xf. Caliptra writes the same value.
const CLEAR_ACTIVATE: u32 = 0x1;

/// A recovery register, as its offset from the i3c-core base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Reg {
    DeviceStatus0 = offset_of!(I3c, sec_fw_recovery_if_device_status_0),
    RecoveryCtrl = offset_of!(I3c, sec_fw_recovery_if_recovery_ctrl),
    RecoveryStatus = offset_of!(I3c, sec_fw_recovery_if_recovery_status),
    IndirectFifoCtrl0 = offset_of!(I3c, sec_fw_recovery_if_indirect_fifo_ctrl_0),
    IndirectFifoCtrl1 = offset_of!(I3c, sec_fw_recovery_if_indirect_fifo_ctrl_1),
    IndirectFifoStatus0 = offset_of!(I3c, sec_fw_recovery_if_indirect_fifo_status_0),
    IndirectFifoData = offset_of!(I3c, sec_fw_recovery_if_indirect_fifo_data),
    RecIntfCfg = offset_of!(I3c, soc_mgmt_if_rec_intf_cfg),
}

/// Access to the i3c-core registers.
pub trait Mmio {
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// The i3c-core's memory-mapped registers.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of the i3c-core registers. They must be
    /// mapped for the caller, and no other code on the MCU may access the
    /// recovery registers.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }
}

impl Mmio for MmioBlock {
    fn read(&self, reg: Reg) -> u32 {
        let addr = self.base.wrapping_add(reg as usize);
        // SAFETY: every `Reg` is the offset of a register inside the block
        // that the caller of `new` guaranteed is mapped.
        unsafe { with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        let addr = self.base.wrapping_add(reg as usize);
        // SAFETY: every `Reg` is the offset of a register inside the block
        // that the caller of `new` guaranteed is mapped.
        unsafe { with_exposed_provenance_mut::<u32>(addr).write_volatile(value) }
    }
}

/// OCP recovery driver for the i3c-core.
pub struct I3cRecovery<M: Mmio> {
    mmio: M,
}

impl<M: Mmio> I3cRecovery<M> {
    pub const fn new(mmio: M) -> Self {
        Self { mmio }
    }

    /// Returns whether the recovery registers are connected to the I3C bus.
    ///
    /// When `REC_INTF_CFG.REC_INTF_BYPASS` is set (AXI bypass, used for
    /// flash boot), the BMC cannot reach the recovery registers.
    pub fn i3c_enabled(&self) -> bool {
        RecIntfCfg::RecIntfBypass.read(self.mmio.read(Reg::RecIntfCfg)) == 0
    }

    /// Returns the underlying register access.
    pub fn into_inner(self) -> M {
        self.mmio
    }
}

impl<M: Mmio> RecoveryDriver for I3cRecovery<M> {
    type Error = ErrorCode;

    fn set_device_status(
        &mut self,
        status: DeviceStatus,
        reason: RecoveryReason,
    ) -> Result<(), ErrorCode> {
        let value = DeviceStatus0::DevStatus.val(status as u32)
            + DeviceStatus0::RecReasonCode.val(u32::from(reason.0));
        self.mmio.write(Reg::DeviceStatus0, value.into());
        Ok(())
    }

    fn set_recovery_status(
        &mut self,
        status: RecoveryStatus,
        index: ImageIndex,
    ) -> Result<(), ErrorCode> {
        let value = RecoveryStatusReg::DevRecStatus.val(status as u32)
            + RecoveryStatusReg::RecImgIndex.val(u32::from(index.get()));
        self.mmio.write(Reg::RecoveryStatus, value.into());
        Ok(())
    }

    fn reset_fifo(&mut self) -> Result<(), ErrorCode> {
        self.mmio.write(
            Reg::IndirectFifoCtrl0,
            IndirectFifoCtrl0::Reset.val(1).into(),
        );
        // Clear the size of the previous image, so that `image_words` waits
        // for the agent to write a new one. MCU ROM does the same at boot.
        self.mmio.write(Reg::IndirectFifoCtrl1, 0);
        Ok(())
    }

    fn image_words(&mut self) -> Result<Option<NonZero<u32>>, ErrorCode> {
        let Some(words) = NonZero::new(self.mmio.read(Reg::IndirectFifoCtrl1)) else {
            return Ok(None);
        };
        if IndirectFifoCtrl0::Cms.read(self.mmio.read(Reg::IndirectFifoCtrl0)) != 0 {
            return Err(OCP_RECOVERY_VEER_UNSUPPORTED_CMS);
        }
        Ok(Some(words))
    }

    fn read_fifo(&mut self, buf: &mut [u32]) -> Result<usize, ErrorCode> {
        let mut count = 0;
        for word in buf.iter_mut() {
            let status = self.mmio.read(Reg::IndirectFifoStatus0);
            if IndirectFifoStatus0::Empty.is_set(status) {
                break;
            }
            *word = self.mmio.read(Reg::IndirectFifoData);
            count += 1;
        }
        Ok(count)
    }

    fn activation_requested(&mut self) -> Result<bool, ErrorCode> {
        let ctrl = self.mmio.read(Reg::RecoveryCtrl);
        Ok(RecoveryCtrl::ActivateRecImg.read(ctrl) == ACTIVATE_RECOVERY_IMAGE)
    }

    fn clear_activation(&mut self) -> Result<(), ErrorCode> {
        let ctrl = self.mmio.read(Reg::RecoveryCtrl);
        let ctrl = RecoveryCtrl::ActivateRecImg
            .val(CLEAR_ACTIVATE)
            .modify(ctrl);
        self.mmio.write(Reg::RecoveryCtrl, ctrl);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// Register offsets from caliptra-mcu-sw `registers/generated-firmware`.
    #[test]
    fn register_offsets() {
        assert_eq!(Reg::DeviceStatus0 as usize, 0x130);
        assert_eq!(Reg::RecoveryCtrl as usize, 0x13c);
        assert_eq!(Reg::RecoveryStatus as usize, 0x140);
        assert_eq!(Reg::IndirectFifoCtrl0 as usize, 0x148);
        assert_eq!(Reg::IndirectFifoCtrl1 as usize, 0x14c);
        assert_eq!(Reg::IndirectFifoStatus0 as usize, 0x150);
        assert_eq!(Reg::IndirectFifoData as usize, 0x168);
        assert_eq!(Reg::RecIntfCfg as usize, 0x30c);
    }

    /// `RECOVERY_CTRL.ACTIVATE_REC_IMG`.
    const ACTIVATE_MASK: u32 = 0x00ff_0000;

    /// Registers as plain values, with a FIFO behind `IndirectFifoData` and
    /// write-1-to-clear behavior on `RECOVERY_CTRL.ACTIVATE_REC_IMG`.
    #[derive(Default)]
    struct FakeMmio {
        device_status_0: u32,
        recovery_ctrl: u32,
        recovery_status: u32,
        fifo_ctrl_0: u32,
        fifo_ctrl_1: u32,
        rec_intf_cfg: u32,
        fifo: RefCell<VecDeque<u32>>,
        fifo_resets: usize,
    }

    impl Mmio for FakeMmio {
        fn read(&self, reg: Reg) -> u32 {
            match reg {
                Reg::DeviceStatus0 => self.device_status_0,
                Reg::RecoveryCtrl => self.recovery_ctrl,
                Reg::RecoveryStatus => self.recovery_status,
                Reg::IndirectFifoCtrl0 => self.fifo_ctrl_0,
                Reg::IndirectFifoCtrl1 => self.fifo_ctrl_1,
                Reg::IndirectFifoStatus0 => u32::from(self.fifo.borrow().is_empty()),
                Reg::IndirectFifoData => self.fifo.borrow_mut().pop_front().unwrap_or(u32::MAX),
                Reg::RecIntfCfg => self.rec_intf_cfg,
            }
        }

        fn write(&mut self, reg: Reg, value: u32) {
            match reg {
                Reg::DeviceStatus0 => self.device_status_0 = value,
                Reg::RecoveryCtrl => {
                    let activate = self.recovery_ctrl & ACTIVATE_MASK & !value;
                    self.recovery_ctrl = (value & !ACTIVATE_MASK) | activate;
                }
                Reg::RecoveryStatus => self.recovery_status = value,
                Reg::IndirectFifoCtrl0 => {
                    if IndirectFifoCtrl0::Reset.read(value) == 1 {
                        self.fifo.get_mut().clear();
                        self.fifo_resets += 1;
                    }
                    self.fifo_ctrl_0 = IndirectFifoCtrl0::Cms.read(value);
                }
                Reg::IndirectFifoCtrl1 => self.fifo_ctrl_1 = value,
                Reg::IndirectFifoStatus0 | Reg::IndirectFifoData => {}
                Reg::RecIntfCfg => self.rec_intf_cfg = value,
            }
        }
    }

    #[test]
    fn status_registers() {
        let mut d = I3cRecovery::new(FakeMmio::default());
        d.set_device_status(DeviceStatus::RecoveryMode, RecoveryReason::FLASHLESS_BOOT)
            .unwrap();
        assert_eq!(d.mmio.device_status_0, 0x0012_0003);
        d.set_recovery_status(RecoveryStatus::AwaitingImage, ImageIndex::new(5).unwrap())
            .unwrap();
        assert_eq!(d.mmio.recovery_status, 0x51);
    }

    #[test]
    fn image_size_and_fifo() {
        let mut d = I3cRecovery::new(FakeMmio {
            fifo_ctrl_1: 9,
            fifo: RefCell::new([7, 8].into()),
            ..Default::default()
        });
        d.reset_fifo().unwrap();
        assert_eq!(d.image_words().unwrap(), None);
        assert_eq!(d.mmio.fifo_resets, 1);

        // The agent writes INDIRECT_FIFO_CTRL and three words.
        d.mmio.fifo_ctrl_1 = 3;
        d.mmio.fifo.get_mut().extend([10, 11, 12]);
        assert_eq!(d.image_words().unwrap(), NonZero::new(3));

        let mut buf = [0_u32; 2];
        assert_eq!(d.read_fifo(&mut buf).unwrap(), 2);
        assert_eq!(buf, [10, 11]);
        assert_eq!(d.read_fifo(&mut buf).unwrap(), 1);
        assert_eq!(buf[0], 12);
        assert_eq!(d.read_fifo(&mut buf).unwrap(), 0);
    }

    #[test]
    fn rejects_nonzero_cms() {
        let mut d = I3cRecovery::new(FakeMmio {
            fifo_ctrl_0: 2,
            fifo_ctrl_1: 4,
            ..Default::default()
        });
        assert!(d.image_words() == Err(OCP_RECOVERY_VEER_UNSUPPORTED_CMS));
    }

    #[test]
    fn activation() {
        let mut d = I3cRecovery::new(FakeMmio {
            recovery_ctrl: 0x000f_0100,
            ..Default::default()
        });
        assert!(d.activation_requested().unwrap());
        d.clear_activation().unwrap();
        assert!(!d.activation_requested().unwrap());
        // CMS and REC_IMG_SEL are kept.
        assert_eq!(d.mmio.recovery_ctrl & 0xffff, 0x0100);
    }

    #[test]
    fn bypass() {
        let mut d = I3cRecovery::new(FakeMmio::default());
        assert!(d.i3c_enabled());
        d.mmio.rec_intf_cfg = 1;
        assert!(!d.i3c_enabled());
    }
}
