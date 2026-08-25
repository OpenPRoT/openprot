// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # openprot-pldm-service
//!
//! PLDM Firmware Device (FD) service built on top of
//! [`openprot-mctp-api`] and [`pldm-interface`], talking PLDM-over-MCTP
//! directly to a remote Update Agent (UA).
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────┐
//! │   Application / Firmware │  constructs FirmwareDevice, calls run_terminus()
//! └───────────┬──────────────┘
//!             │
//!             ▼
//! ┌───────────────────────────────────────────┐
//! │   openprot-pldm-service                   │◄── this crate
//! │   FirmwareDevice<'a, O: FdOps, Cr, Cq>    │
//! │     - cmd_interface: CmdInterface<'a, O>  │  PLDM FW-update state machine
//! │     - responder_transport: inbound UA→FD  │
//! │     - requester_transport: outbound FD→UA │
//! └───────────┬──────────────┬────────────────┘
//!             │              │ MctpPldmTransport<C: MctpClient>
//!             ▼              ▼
//! ┌───────────────────────────────────────────┐
//! │   openprot-mctp-api                       │  Stack<C: MctpClient>
//! └───────────┬───────────────────────────────┘
//!             │ IPC / transport
//!             ▼
//! ┌──────────────────────────┐
//! │   MCTP Server            │
//! └──────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use openprot_pldm_service::firmware_device::FirmwareDevice;
//! use openprot_pldm_service::MctpPldmTransport;
//! use pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES;
//!
//! // `fd_ops` implements `FdOps` (platform-specific flash / component
//! // logic). `responder_client` / `requester_client` are `MctpClient`
//! // implementations (they may share the same underlying MCTP endpoint).
//! let responder_transport = MctpPldmTransport::new(responder_client);
//! let requester_transport = MctpPldmTransport::new(requester_client);
//!
//! let mut fd = FirmwareDevice::init(
//!     &fd_ops,
//!     &PLDM_PROTOCOL_CAPABILITIES,
//!     responder_transport,
//!     requester_transport,
//! );
//!
//! const UA_EID: u8 = 8;
//! let mut buf = [0u8; 1024];
//!
//! // `run_terminus` loops forever, interleaving inbound UA commands with any
//! // FD-initiated requests (e.g. RequestFirmwareData) once an update begins.
//! // It returns only on error; a `timeout_millis`/`requester_timeout_millis`
//! // of `0` blocks indefinitely while idle. The final argument is an
//! // `UpdateEventSink` notified once per accepted RequestUpdate; `&mut ()`
//! // drops the notifications.
//! if let Err(e) = fd.run_terminus(UA_EID, &mut buf, 0, 0, &mut ()) {
//!     // handle or log error
//! }
//! ```

#![no_std]
#![warn(missing_docs)]

pub mod error;
pub mod firmware_device;
pub mod transport;

pub use error::PldmServiceError;
pub use transport::MctpPldmTransport;
