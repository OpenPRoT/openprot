// Licensed under the Apache-2.0 license

//! Shared EID configuration for the PLDM req/resp loopback test.
//!
//! These constants are the single source of truth for the MCTP endpoint IDs
//! used by all three processes in the test image: the loopback server, the
//! PLDM requester, and (if needed) the PLDM responder.

#![no_std]
use pldm_common::protocol::base::{PldmControlCmd, PldmSupportedType};
use pldm_interface::control_context::ProtocolCapability;

/// EID assigned to the MCTP loopback server endpoint facing the requester.
pub const REQUESTER_EID: u8 = 8;

/// EID assigned to the MCTP loopback server endpoint facing the responder.
pub const RESPONDER_EID: u8 = 42;

/// Per-exchange MCTP timeout in milliseconds (0 = block indefinitely).
pub const TIMEOUT_MILLIS: u32 = 0;

const CTRL_CMDS: [u8; 5] = [
    PldmControlCmd::SetTid as u8,
    PldmControlCmd::GetTid as u8,
    PldmControlCmd::GetPldmCommands as u8,
    PldmControlCmd::GetPldmVersion as u8,
    PldmControlCmd::GetPldmTypes as u8,
];

pub static CAPS: [ProtocolCapability<'static>; 1] = [
    ProtocolCapability {
        pldm_type: PldmSupportedType::Base,
        protocol_version: 0xF1F1F000, // "1.1.0" BCD-encoded
        supported_commands: &CTRL_CMDS,
    },
];

