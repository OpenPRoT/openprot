// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! PLDM IPC client: the orchestrator's side of the IPC channel.
//!
//! One typed method per operation the firmware device answers, each
//! encoding a request and starting a round-trip. The reply is collected
//! by [`FdIpcClient::poll`], so the orchestrator's event loop never waits
//! on the device: the FD shares its loop with MCTP traffic from the
//! update agent and can take as long as that traffic makes it take.
//!
//! One round-trip at a time, which the transport enforces and this crate
//! relies on: the response frame carries a code and a payload but not
//! the opcode it answers, so the client remembers what it asked.
//!
//! Generic over `util_service::AsyncTransport`, so the same encode and
//! decode paths run behind a kernel channel in production and inside
//! `util_service::Loopback` in host tests.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pldm_client::{ClientError, FdIpcClient, Reply};
//!
//! // One turn of the orchestrator's event loop starts the round-trip.
//! client.perform_verify()?;
//!
//! // A later turn, once the channel signals readable, collects it.
//! match client.poll() {
//!     Ok(None) => {}                  // not answered yet, poll again
//!     Ok(Some(Reply::Acked)) => {}    // the FD took the request
//!     Ok(Some(Reply::Status(s))) => {} // only QueryStatus answers this
//!     Err(ClientError::Refused(code)) => {} // the FD said no, and why
//!     Err(e) => {}                    // the channel failed, round-trip over
//! }
//! ```

#![no_std]

mod client;
mod error;

pub use client::{FdIpcClient, Reply};
pub use error::ClientError;
