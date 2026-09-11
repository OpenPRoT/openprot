// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! End-to-end host test for PLDM firmware-update inventory commands.
//!
//! The Update Agent queries a Firmware Device over in-memory MCTP and verifies
//! the type 5 device identifiers and firmware parameters supplied by `FdOps`.

use core::cell::RefCell;

use mctp::Eid;
use openprot_mctp_server::Server;
use openprot_pldm_service::firmware_device::{FirmwareDevice, RunTerminusResult};
use openprot_pldm_service::{MctpPldmTransport, PldmServiceError};
use pldm_common::codec::PldmCodec;
use pldm_common::message::firmware_update::apply_complete::ApplyResult;
use pldm_common::message::firmware_update::get_fw_params::{
    FirmwareParameters, GetFirmwareParametersRequest, GetFirmwareParametersResponse,
};
use pldm_common::message::firmware_update::get_status::ProgressPercent;
use pldm_common::message::firmware_update::query_devid::{
    QueryDeviceIdentifiersRequest, QueryDeviceIdentifiersResponse,
};
use pldm_common::message::firmware_update::transfer_complete::TransferResult;
use pldm_common::message::firmware_update::verify_complete::VerifyResult;
use pldm_common::protocol::base::{PldmBaseCompletionCode, PldmMsgType};
use pldm_common::protocol::firmware_update::{
    ComponentActivationMethods, ComponentClassification, ComponentParameterEntry,
    ComponentResponseCode, Descriptor, DescriptorType, FirmwareDeviceCapability,
    PldmFirmwareString, PldmFirmwareVersion,
};
use pldm_common::util::fw_component::FirmwareComponent;
use pldm_interface::firmware_device::fd_ops::{ComponentOperation, FdOps, FdOpsError};

mod common;
use common::{transfer, BufferSender, DirectClientWithPump, FD_EID, TIMEOUT_MILLIS, UA_EID};

struct InventoryFdOps {
    descriptors: [Descriptor; 2],
    firmware_parameters: FirmwareParameters,
}

impl FdOps for InventoryFdOps {
    fn get_device_identifiers(
        &self,
        device_identifiers: &mut [Descriptor],
    ) -> Result<usize, FdOpsError> {
        let destination = device_identifiers
            .get_mut(..self.descriptors.len())
            .ok_or(FdOpsError::DeviceIdentifiersError)?;
        destination.copy_from_slice(&self.descriptors);
        Ok(self.descriptors.len())
    }

    fn get_firmware_parms(
        &self,
        firmware_params: &mut FirmwareParameters,
    ) -> Result<(), FdOpsError> {
        *firmware_params = self.firmware_parameters.clone();
        Ok(())
    }

    fn get_xfer_size(&self, _ua_transfer_size: usize) -> Result<usize, FdOpsError> {
        Err(FdOpsError::TransferSizeError)
    }

    fn handle_component(
        &self,
        _component: &FirmwareComponent,
        _fw_params: &FirmwareParameters,
        _op: ComponentOperation,
    ) -> Result<ComponentResponseCode, FdOpsError> {
        Err(FdOpsError::ComponentError)
    }

    fn query_download_offset_and_length(
        &self,
        _component: &FirmwareComponent,
    ) -> Result<(usize, usize), FdOpsError> {
        Err(FdOpsError::FwDownloadError)
    }

    fn download_fw_data(
        &self,
        _offset: usize,
        _data: &[u8],
        _component: &FirmwareComponent,
    ) -> Result<TransferResult, FdOpsError> {
        Err(FdOpsError::FwDownloadError)
    }

    fn is_download_complete(&self, _component: &FirmwareComponent) -> bool {
        false
    }

    fn query_download_progress(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<(), FdOpsError> {
        Err(FdOpsError::FwDownloadError)
    }

    fn verify(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<VerifyResult, FdOpsError> {
        Err(FdOpsError::VerifyError)
    }

    fn apply(
        &self,
        _component: &FirmwareComponent,
        _progress_percent: &mut ProgressPercent,
    ) -> Result<ApplyResult, FdOpsError> {
        Err(FdOpsError::ApplyError)
    }

    fn activate(
        &self,
        _self_contained_activation: u8,
        _estimated_time: &mut u16,
    ) -> Result<u8, FdOpsError> {
        Err(FdOpsError::ActivateError)
    }

    fn cancel_update_component(&self, _component: &FirmwareComponent) -> Result<(), FdOpsError> {
        Err(FdOpsError::CancelUpdateError)
    }
}

fn firmware_string(value: &str) -> PldmFirmwareString {
    match PldmFirmwareString::new("ASCII", value) {
        Ok(value) => value,
        Err(error) => panic!("invalid firmware string {value}: {error:?}"),
    }
}

#[test]
fn firmware_update_inventory_commands_return_fd_data() {
    let pci_vendor_id = match Descriptor::new(DescriptorType::PciVendorId, &[0x34, 0x12]) {
        Ok(descriptor) => descriptor,
        Err(error) => panic!("invalid PCI vendor descriptor: {error:?}"),
    };
    let uuid = match Descriptor::new(
        DescriptorType::Uuid,
        &[
            0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ],
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => panic!("invalid UUID descriptor: {error:?}"),
    };
    let descriptors = [pci_vendor_id, uuid];

    let active_image_set_version = firmware_string("platform-2.4.0");
    let pending_image_set_version = firmware_string("platform-2.5.0");
    let active_component_version = firmware_string("controller-2.4.0");
    let pending_component_version = firmware_string("controller-2.5.0");
    let active_version =
        PldmFirmwareVersion::new(0x0002_0400, &active_component_version, Some("20260801"));
    let pending_version =
        PldmFirmwareVersion::new(0x0002_0500, &pending_component_version, Some("20260901"));
    let component = ComponentParameterEntry::new(
        ComponentClassification::Firmware,
        0x1001,
        0,
        &active_version,
        &pending_version,
        ComponentActivationMethods(0x0009),
        FirmwareDeviceCapability(0x0000_0003),
    );
    let firmware_parameters = FirmwareParameters::new(
        FirmwareDeviceCapability(0x0000_0003),
        1,
        &active_image_set_version,
        &pending_image_set_version,
        &[component],
    );
    let fd_ops = InventoryFdOps {
        descriptors,
        firmware_parameters: firmware_parameters.clone(),
    };

    let ua_to_fd_packets = RefCell::new(Vec::new());
    let ua_sender = BufferSender {
        packets: &ua_to_fd_packets,
    };
    let ua_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(UA_EID), 0, ua_sender));

    let fd_to_ua_packets = RefCell::new(Vec::new());
    let fd_sender = BufferSender {
        packets: &fd_to_ua_packets,
    };
    let fd_server: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(FD_EID), 0, fd_sender));

    // Create the DirectClientWithPump for the responder role, which handles requests from the UA to the FD.
    let responder_client = DirectClientWithPump::new(&fd_server, || {
        transfer(&ua_to_fd_packets, &mut fd_server.borrow_mut());
        ua_to_fd_packets.borrow_mut().clear();
    });

    // Inventory Commands do not invoke the FW FSM and do not invoke FD as a initiator.  Therefore the closure is empty.
    let requester_client = DirectClientWithPump::new(&fd_server, || {});
    let mut fd = FirmwareDevice::init(
        &fd_ops,
        &pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES,
        MctpPldmTransport::new(responder_client),
        MctpPldmTransport::new(requester_client),
    );
    let mut fd_buf = [0u8; 1024];

    let mut transact = |request: &[u8]| -> Vec<u8> {
        let handle = ua_server
            .borrow_mut()
            .req(FD_EID)
            .expect("allocate UA request handle");
        ua_server
            .borrow_mut()
            .send(Some(handle), 0x01, None, None, false, request)
            .expect("send inventory request");

        match fd.run_terminus(UA_EID, &mut fd_buf, TIMEOUT_MILLIS, TIMEOUT_MILLIS, &mut ()) {
            RunTerminusResult::Completed => {}
            RunTerminusResult::StoppedByError(PldmServiceError::Mctp(error))
                if error.is_timeout() => {}
            RunTerminusResult::StoppedByError(error) => {
                panic!("firmware device failed: {error:?}")
            }
        }

        transfer(&fd_to_ua_packets, &mut ua_server.borrow_mut());
        fd_to_ua_packets.borrow_mut().clear();
        let mut response = [0u8; 1024];
        let metadata = ua_server
            .borrow_mut()
            .try_recv(handle, &mut response)
            .expect("inventory response should be available");
        let payload = response
            .get(..metadata.payload_size)
            .expect("response payload should fit the receive buffer")
            .to_vec();
        let _ = ua_server.borrow_mut().unbind(handle);
        payload
    };

    let mut request_buf = [0u8; 64];
    let mut instance_id = 0u8;
    let query_device_identifiers =
        QueryDeviceIdentifiersRequest::new(instance_id, PldmMsgType::Request);
    let request_len = query_device_identifiers
        .encode(&mut request_buf)
        .expect("encode QueryDeviceIdentifiers");
    let response = transact(
        request_buf
            .get(..request_len)
            .expect("encoded request should fit the request buffer"),
    );
    let response = QueryDeviceIdentifiersResponse::decode(&response)
        .expect("decode QueryDeviceIdentifiers response");
    assert_eq!(
        response.completion_code,
        PldmBaseCompletionCode::Success as u8
    );
    assert_eq!(response.descriptor_count, 2);
    let [expected_initial, expected_additional] = descriptors;
    assert_eq!(response.initial_descriptor, expected_initial);
    let additional = response
        .additional_descriptors
        .as_ref()
        .expect("UUID descriptor should be present");
    let [actual_additional, ..] = additional;
    assert_eq!(*actual_additional, expected_additional);
    assert_eq!(
        response.device_identifiers_len as usize,
        descriptors
            .iter()
            .map(Descriptor::codec_size_in_bytes)
            .sum()
    );

    instance_id += 1;
    let get_firmware_parameters =
        GetFirmwareParametersRequest::new(instance_id, PldmMsgType::Request);
    let request_len = get_firmware_parameters
        .encode(&mut request_buf)
        .expect("encode GetFirmwareParameters");
    let response = transact(
        request_buf
            .get(..request_len)
            .expect("encoded request should fit the request buffer"),
    );
    let response = GetFirmwareParametersResponse::decode(&response)
        .expect("decode GetFirmwareParameters response");
    assert_eq!(
        response.completion_code,
        PldmBaseCompletionCode::Success as u8
    );
    assert_eq!(response.parms, firmware_parameters);
}
