// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM IPC API: wire format, types, and transport seam.
//!
//! Defines the binary protocol the orchestrator uses to talk to the
//! PLDM Firmware Device over IPC. Host-buildable, no kernel
//! dependencies. The server and client crates depend on this for
//! shared types; neither re-invents the encoding.

#![no_std]

pub mod error;
pub mod status;
pub mod transport;
pub mod wire;

pub use error::{DenyReason, PldmIpcError, ResponseCode, WireError};
pub use status::{FdStatus, TransferMode};
pub use transport::{Transport, TransportError};
pub use wire::{
    PldmOp, RequestHeader, ResponseHeader, MAX_PAYLOAD_SIZE, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
