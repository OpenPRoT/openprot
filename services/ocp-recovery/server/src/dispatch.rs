// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! One agent access in, one answer out.

use openprot_ocp_recovery_api::command::RecoveryCommand;
use openprot_ocp_recovery_api::error::WireError;
use openprot_ocp_recovery_api::regs::{DeviceReset, DeviceStatus, IndirectFifoCtrl, RecoveryCtrl};
use zerocopy::IntoBytes;

use crate::registers::RecoveryRegisterFile;
use crate::staging::{FifoSink, StagingError};

/// Serves one access from the Recovery Agent.
///
/// `command_byte` is the command of the transaction, `write` carries the
/// payload of a write and is `None` for a read, and `transport_max` is the
/// largest payload one access of the transport carries
/// ([`SMBUS_MAX_PAYLOAD`](openprot_ocp_recovery_api::SMBUS_MAX_PAYLOAD) on
/// SMBus). The return value is how many bytes of `resp` a read filled; a
/// write fills none.
///
/// A refused access latches its [`ProtocolError`] into DEVICE_STATUS byte 1
/// and changes no other state, so the agent reads the error on its next
/// DEVICE_STATUS poll, where the read clears it. The dispatch never decides
/// policy: an accepted `RECOVERY_CTRL` or `DEVICE_RESET` write only lands in
/// the register, and the runtime finds it through the event takes.
pub fn dispatch<S: FifoSink>(
    file: &mut RecoveryRegisterFile,
    sink: &mut S,
    command_byte: u8,
    write: Option<&[u8]>,
    resp: &mut [u8],
    transport_max: usize,
) -> usize {
    let result = match RecoveryCommand::try_from(command_byte) {
        Ok(command) => match write {
            None => serve_read(file, command, resp),
            Some(bytes) => serve_write(file, sink, command, bytes, transport_max).map(|()| 0),
        },
        Err(e) => Err(e),
    };
    match result {
        Ok(len) => len,
        Err(e) => {
            file.device_status_mut().set_protocol_error(e.into());
            0
        }
    }
}

/// Copies register bytes into `resp`, or reports why the command serves none.
fn serve_read(
    file: &mut RecoveryRegisterFile,
    command: RecoveryCommand,
    resp: &mut [u8],
) -> Result<usize, WireError> {
    match command {
        RecoveryCommand::ProtCap => copy(file.prot_cap().as_bytes(), resp),
        RecoveryCommand::DeviceId => copy(file.device_id(), resp),
        RecoveryCommand::DeviceStatus => {
            // The register serves its bytes with the latched protocol error,
            // and the read clears the error: the one clear-on-read of the
            // interface, done here so no other path forgets it.
            let status = file.device_status_mut();
            let len = copy(status.as_bytes(), resp)?;
            status.take_protocol_error();
            Ok(len)
        }
        RecoveryCommand::DeviceReset => copy(file.device_reset_mut().as_bytes(), resp),
        RecoveryCommand::RecoveryCtrl => copy(file.recovery_ctrl_mut().as_bytes(), resp),
        RecoveryCommand::RecoveryStatus => copy(file.recovery_status().as_bytes(), resp),
        RecoveryCommand::HwStatus => copy(file.hw_status().as_bytes(), resp),
        RecoveryCommand::IndirectFifoCtrl => copy(file.fifo_ctrl_mut().as_bytes(), resp),
        RecoveryCommand::IndirectFifoStatus => copy(file.fifo_status_mut().as_bytes(), resp),
        // The eRoT serves the FIFO code region and no indirect memory window
        // or vendor command; reading the write aperture back is not defined
        // either. PROT_CAP advertises what exists, and an agent that asks
        // anyway reads the unsupported-command error from DEVICE_STATUS.
        RecoveryCommand::IndirectCtrl
        | RecoveryCommand::IndirectStatus
        | RecoveryCommand::IndirectData
        | RecoveryCommand::Vendor
        | RecoveryCommand::IndirectFifoData => Err(WireError::UnknownCommand(command as u8)),
    }
}

/// Applies one agent write, or reports why the register refused it.
fn serve_write<S: FifoSink>(
    file: &mut RecoveryRegisterFile,
    sink: &mut S,
    command: RecoveryCommand,
    bytes: &[u8],
    transport_max: usize,
) -> Result<(), WireError> {
    match command {
        RecoveryCommand::DeviceReset => {
            let sized = DeviceReset::from_wire(bytes, transport_max)?;
            file.device_reset_mut().apply_agent_write(sized)
        }
        RecoveryCommand::RecoveryCtrl => {
            let sized = RecoveryCtrl::from_wire(bytes, transport_max)?;
            file.recovery_ctrl_mut().apply_agent_write(sized)
        }
        RecoveryCommand::IndirectFifoCtrl => {
            let sized = IndirectFifoCtrl::from_wire(bytes, transport_max)?;
            let ctrl = file.fifo_ctrl_mut();
            ctrl.apply_agent_write(sized)?;
            if ctrl.reset_requested() {
                let size = ctrl.image_size_bytes();
                sink.begin_image(size).map_err(staging_refusal(command))?;
                // The reset ran, so the device clears the field and zeroes
                // the indices, which is what the agent polls for.
                ctrl.clear_reset();
                file.fifo_status_mut().reset_indices();
            }
            Ok(())
        }
        RecoveryCommand::IndirectFifoData => {
            command.check_write(bytes.len(), transport_max)?;
            sink.append(bytes).map_err(staging_refusal(command))?;
            file.fifo_status_mut().record_drained_write(bytes.len());
            Ok(())
        }
        // ReadWrite in the spec, not served by the eRoT: no indirect memory
        // window, no vendor command.
        RecoveryCommand::IndirectCtrl | RecoveryCommand::IndirectData | RecoveryCommand::Vendor => {
            Err(WireError::UnknownCommand(command as u8))
        }
        // Read-only commands; check_write names the refusal.
        RecoveryCommand::ProtCap
        | RecoveryCommand::DeviceId
        | RecoveryCommand::DeviceStatus
        | RecoveryCommand::RecoveryStatus
        | RecoveryCommand::HwStatus
        | RecoveryCommand::IndirectStatus
        | RecoveryCommand::IndirectFifoStatus => command.check_write(bytes.len(), transport_max),
    }
}

/// A staging refusal reaches the agent as an unsupported parameter: the
/// write was well-formed and the announced size or the region capacity is
/// what refused it.
fn staging_refusal(command: RecoveryCommand) -> impl Fn(StagingError) -> WireError {
    move |_| WireError::UnsupportedParameter {
        command,
        offset: 0,
        value: 0,
    }
}

fn copy(src: &[u8], resp: &mut [u8]) -> Result<usize, WireError> {
    let dst = resp
        .get_mut(..src.len())
        .ok_or(WireError::UnknownCommand(0))?;
    dst.copy_from_slice(src);
    Ok(src.len())
}

const _: () = {
    // serve_read copies at most a DEVICE_ID block; the dispatch caller sizes
    // resp to the largest register, which the api command table caps at 255.
    assert!(DeviceStatus::SIZE <= 255);
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::RecoveryRegisterFile;
    use crate::staging::RamStaging;
    use openprot_ocp_recovery_api::regs::{
        capability, DeviceState, ImageIndex, ProtCap, RecoveryReason, RecoveryState,
        ACTIVATE_RECOVERY_IMAGE, FIFO_RESET,
    };
    use openprot_ocp_recovery_api::{ProtocolError, SMBUS_MAX_PAYLOAD};

    const DEVICE_ID: &[u8] = &[0u8; 24];

    fn file() -> RecoveryRegisterFile {
        let prot_cap = ProtCap::new(
            capability::IDENTIFICATION | capability::PUSH_C_IMAGE | capability::FIFO_CMS,
            1,
            0,
            0,
        );
        let fifo_status =
            openprot_ocp_recovery_api::regs::IndirectFifoStatus::code_region(0x100, 252)
                .expect("sizes divide by 4");
        RecoveryRegisterFile::new(prot_cap, DEVICE_ID, fifo_status)
    }

    fn read(
        file: &mut RecoveryRegisterFile,
        sink: &mut RamStaging<0x100>,
        command: RecoveryCommand,
        resp: &mut [u8],
    ) -> usize {
        dispatch(file, sink, command as u8, None, resp, SMBUS_MAX_PAYLOAD)
    }

    fn write(
        file: &mut RecoveryRegisterFile,
        sink: &mut RamStaging<0x100>,
        command: RecoveryCommand,
        bytes: &[u8],
    ) {
        let mut resp = [0u8; 0];
        dispatch(
            file,
            sink,
            command as u8,
            Some(bytes),
            &mut resp,
            SMBUS_MAX_PAYLOAD,
        );
    }

    /// Byte 1 of a DEVICE_STATUS read is the latched protocol error.
    fn protocol_error_byte(file: &mut RecoveryRegisterFile, sink: &mut RamStaging<0x100>) -> u8 {
        let mut resp = [0u8; 64];
        let len = read(file, sink, RecoveryCommand::DeviceStatus, &mut resp);
        assert!(len >= 2);
        resp[1]
    }

    #[test]
    fn an_unknown_command_latches_unsupported_and_the_read_clears_it() {
        let (mut file, mut sink) = (file(), RamStaging::new());
        let mut resp = [0u8; 8];
        let len = dispatch(
            &mut file,
            &mut sink,
            0x21,
            None,
            &mut resp,
            SMBUS_MAX_PAYLOAD,
        );
        assert_eq!(len, 0);
        assert_eq!(
            protocol_error_byte(&mut file, &mut sink),
            ProtocolError::UnsupportedCommand as u8
        );
        // The DEVICE_STATUS read cleared the latch.
        assert_eq!(
            protocol_error_byte(&mut file, &mut sink),
            ProtocolError::None as u8
        );
    }

    #[test]
    fn a_write_to_a_read_only_command_latches_unsupported_command() {
        let (mut file, mut sink) = (file(), RamStaging::new());
        write(&mut file, &mut sink, RecoveryCommand::ProtCap, &[0u8; 15]);
        assert_eq!(
            protocol_error_byte(&mut file, &mut sink),
            ProtocolError::UnsupportedCommand as u8
        );
    }

    #[test]
    fn the_streaming_boot_sequence_lands_the_image_and_raises_the_activation() {
        let (mut file, mut sink) = (file(), RamStaging::new());
        file.set_device_state(
            DeviceState::RecoveryModeReady,
            RecoveryReason::FlashlessStreamingBoot,
        );
        file.set_recovery_status(RecoveryState::AwaitingImage, ImageIndex::FIRST);

        // The agent reads PROT_CAP and finds the push capability.
        let mut resp = [0u8; 64];
        let len = read(&mut file, &mut sink, RecoveryCommand::ProtCap, &mut resp);
        assert_eq!(len, ProtCap::SIZE);
        assert_eq!(&resp[..8], b"OCP RECV");

        // The agent programs the FIFO: CMS 0, reset, 8-byte image.
        let mut ctrl = [0u8; 6];
        ctrl[1] = FIFO_RESET;
        ctrl[2..].copy_from_slice(&2u32.to_le_bytes()); // 2 dwords
        write(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoCtrl,
            &ctrl,
        );
        assert_eq!(
            protocol_error_byte(&mut file, &mut sink),
            ProtocolError::None as u8
        );

        // The reset ran and the device cleared the field.
        let len = read(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoCtrl,
            &mut resp,
        );
        assert_eq!(len, 6);
        assert_eq!(resp[1], 0x0);

        // The image arrives in two chunks and the indices follow.
        write(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoData,
            &[1, 2, 3, 4],
        );
        write(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoData,
            &[5, 6, 7, 8],
        );
        assert!(sink.is_complete());

        // The agent activates; the device sees the request once and the
        // field reads back cleared.
        write(
            &mut file,
            &mut sink,
            RecoveryCommand::RecoveryCtrl,
            &[0, 0x1, ACTIVATE_RECOVERY_IMAGE],
        );
        let request = file.take_activation().expect("activation pending");
        assert_eq!(request.cms, 0);
        assert!(file.take_activation().is_none());
        let len = read(
            &mut file,
            &mut sink,
            RecoveryCommand::RecoveryCtrl,
            &mut resp,
        );
        assert_eq!(len, 3);
        assert_eq!(resp[2], 0x0);
    }

    #[test]
    fn a_data_chunk_past_the_announced_size_is_refused_and_not_staged() {
        let (mut file, mut sink) = (file(), RamStaging::new());
        let mut ctrl = [0u8; 6];
        ctrl[1] = FIFO_RESET;
        ctrl[2..].copy_from_slice(&1u32.to_le_bytes()); // 4 bytes announced
        write(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoCtrl,
            &ctrl,
        );

        write(
            &mut file,
            &mut sink,
            RecoveryCommand::IndirectFifoData,
            &[1, 2, 3, 4, 5],
        );
        assert_eq!(
            protocol_error_byte(&mut file, &mut sink),
            ProtocolError::UnsupportedParameter as u8
        );
        assert!(sink.is_empty());
    }
}
