// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # openprot-pldm-service
//!
//! PLDM-over-MCTP responder service built on top of
//! [`openprot-mctp-api`] and [`pldm-interface`].
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────┐
//! │   Application / Firmware │  creates PldmResponder / PldmRequester, calls run_responder()/run_requester()
//! └───────────┬──────────────┘
//!             │
//!             ▼
//! ┌──────────────────────────┐
//! │   openprot-pldm-service  │◄── this crate
//! │   PldmResponder          │  dispatches to CmdInterface (responder side)
//! │   PldmRequester          │  sends PLDM requests (initiator side)
//! └───────────┬──────────────┘
//!             │ MctpListener / MctpReqChannel / MctpRespChannel
//!             ▼
//! ┌──────────────────────────┐
//! │   openprot-mctp-api      │  Stack<C: MctpClient>
//! │   (Stack facade)         │
//! └───────────┬──────────────┘
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
//! use openprot_pldm_service::{MctpPldmTransport, PldmResponder, PldmRequester};
//!
//! // `client` is any `MctpClient`. `ua_cmd` implements `UaFdCmdChannel` and
//! // `fd_req` implements `UaFdRspChannel` — both are provided by the
//! // platform IPC glue (see the `firmware-device-ipc` crate).
//! let transport = MctpPldmTransport::new(client);
//! let mut buf = [0u8; 1024];
//!
//! // Responder task: forward inbound PLDM requests from the Update Agent to
//! // the FirmwareDevice process and return its responses. `run_responder`
//! // loops forever, returning only on error; a `timeout_millis` of `0`
//! // blocks indefinitely on each exchange.
//! let responder = PldmResponder::new();
//! if let Err(e) = responder.run_responder(&transport, &ua_cmd, &mut buf, 0) {
//!     // handle or log error
//! }
//!
//! // Requester task (typically a separate process): forward FD-initiated
//! // PLDM requests to a remote endpoint over MCTP.
//! const REMOTE_EID: u8 = 8;
//! let requester = PldmRequester::new();
//! if let Err(e) = requester.run_requester(&fd_req, &transport, REMOTE_EID, &mut buf, 0) {
//!     // handle or log error
//! }
//! ```

#![no_std]
#![warn(missing_docs)]

pub mod error;
pub mod firmware_device;
pub mod requester;
pub mod responder;
pub mod transport;

pub use error::PldmServiceError;
pub use requester::PldmRequester;
pub use transport::MctpPldmTransport;
pub use responder::{PldmResponder, PLDM_MSG_TYPE};
