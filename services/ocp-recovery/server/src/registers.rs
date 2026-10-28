// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The register file the eRoT serves to the Recovery Agent.

use openprot_ocp_recovery_api::regs::{
    DeviceReset, DeviceState, DeviceStatus, HwStatus, ImageIndex, ImageSelection, IndirectFifoCtrl,
    IndirectFifoStatus, ProtCap, RecoveryCtrl, RecoveryReason, RecoveryState, RecoveryStatus,
    ResetControl,
};

/// An activation the Recovery Agent requested through `RECOVERY_CTRL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationRequest {
    /// The component memory space the agent selected.
    pub cms: u8,
    /// Where the agent pointed the device for the image.
    pub selection: Option<ImageSelection>,
}

/// One instance of every register of the recovery interface.
///
/// The i2c server-runtime owns the instance and calls [`dispatch`] with it
/// for every agent access; the policy side reaches it through the setters and
/// the event takes. The takes are the device half of the "Write 1, Device
/// Clears" contract: reading the event clears the field, so the agent sees
/// the command value only until the device has acted.
///
/// [`dispatch`]: crate::dispatch
pub struct RecoveryRegisterFile {
    /// Preformatted `DEVICE_ID` response, laid out by the board.
    device_id: &'static [u8],
    prot_cap: ProtCap,
    device_status: DeviceStatus,
    device_reset: DeviceReset,
    recovery_ctrl: RecoveryCtrl,
    recovery_status: RecoveryStatus,
    hw_status: HwStatus,
    fifo_ctrl: IndirectFifoCtrl,
    fifo_status: IndirectFifoStatus,
}

impl RecoveryRegisterFile {
    /// Builds the register file a device starts with.
    ///
    /// `prot_cap` carries the capabilities the board advertises, `device_id`
    /// the preformatted identity block, and `fifo_status` describes the code
    /// region the staging memory backs. The device status starts at pending
    /// and the recovery status at not-in-recovery; the policy side moves both.
    pub fn new(
        prot_cap: ProtCap,
        device_id: &'static [u8],
        fifo_status: IndirectFifoStatus,
    ) -> Self {
        Self {
            device_id,
            prot_cap,
            device_status: DeviceStatus::pending(),
            device_reset: DeviceReset::default(),
            recovery_ctrl: RecoveryCtrl::default(),
            recovery_status: RecoveryStatus::default(),
            hw_status: HwStatus::default(),
            fifo_ctrl: IndirectFifoCtrl::default(),
            fifo_status,
        }
    }

    /// Reports a device state and the reason code that belongs with it.
    pub fn set_device_state(&mut self, state: DeviceState, reason: RecoveryReason) {
        self.device_status.set_state(state, reason);
    }

    /// Reports how far the device has come with one image.
    pub fn set_recovery_status(&mut self, state: RecoveryState, image: ImageIndex) {
        self.recovery_status.set(state, image);
    }

    /// Advances the DEVICE_STATUS heartbeat counter.
    pub fn tick_heartbeat(&mut self) {
        self.device_status.tick_heartbeat();
    }

    /// The hardware status register, for the board to fill.
    pub fn hw_status_mut(&mut self) -> &mut HwStatus {
        &mut self.hw_status
    }

    /// Takes a pending activation request and clears the activate field, the
    /// device half of the "Write 1, Device Clears" contract. The agent reads
    /// 0x0 from `RECOVERY_CTRL` byte 2 afterwards.
    pub fn take_activation(&mut self) -> Option<ActivationRequest> {
        if !self.recovery_ctrl.activation_requested() {
            return None;
        }
        self.recovery_ctrl.clear_activation();
        Some(ActivationRequest {
            cms: self.recovery_ctrl.cms(),
            selection: self.recovery_ctrl.image_selection(),
        })
    }

    /// Takes a pending reset command and clears `DEVICE_RESET` byte 0. The
    /// forced-recovery and interface bytes keep their values, which is what
    /// the spec has the device clear.
    pub fn take_reset_request(&mut self) -> Option<ResetControl> {
        match self.device_reset.reset_control() {
            Some(ResetControl::None) | None => None,
            Some(reset) => {
                self.device_reset.clear_reset_control();
                Some(reset)
            }
        }
    }

    /// The mode the agent armed for the next platform reset. The field stays
    /// set across the take: the spec clears it through a platform reset, not
    /// through the device acting on it.
    pub fn armed_forced_recovery(
        &mut self,
    ) -> Option<openprot_ocp_recovery_api::regs::ForcedRecovery> {
        self.device_reset.forced_recovery()
    }

    pub(crate) fn device_id(&self) -> &'static [u8] {
        self.device_id
    }

    pub(crate) fn prot_cap(&self) -> &ProtCap {
        &self.prot_cap
    }

    pub(crate) fn device_status_mut(&mut self) -> &mut DeviceStatus {
        &mut self.device_status
    }

    pub(crate) fn device_reset_mut(&mut self) -> &mut DeviceReset {
        &mut self.device_reset
    }

    pub(crate) fn recovery_ctrl_mut(&mut self) -> &mut RecoveryCtrl {
        &mut self.recovery_ctrl
    }

    pub(crate) fn recovery_status(&self) -> &RecoveryStatus {
        &self.recovery_status
    }

    pub(crate) fn hw_status(&self) -> &HwStatus {
        &self.hw_status
    }

    pub(crate) fn fifo_ctrl_mut(&mut self) -> &mut IndirectFifoCtrl {
        &mut self.fifo_ctrl
    }

    pub(crate) fn fifo_status_mut(&mut self) -> &mut IndirectFifoStatus {
        &mut self.fifo_status
    }
}
