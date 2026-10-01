// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Crypto IPC client: the orchestrator's side of the crypto channel.
//!
//! Two operations: `start_verify` names a flash region (absolute
//! address + length), and `query_status` polls for hashing progress
//! or a verdict. The crypto service reads the firmware bytes from the
//! flash service itself; nothing crosses this interface but a region
//! descriptor and a verdict.
//!
//! One round-trip at a time. The response frame does not name the
//! operation, so the client remembers what it asked.
//!
//! Generic over `util_service::AsyncTransport`, so the same encode
//! and decode paths run behind a kernel channel in production and
//! inside `util_service::Loopback` in host tests.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crypto_client::{ClientError, CryptoIpcClient, Reply};
//!
//! client.start_verify(0x2000_0000, 0x0008_0000)?;
//!
//! match client.poll() {
//!     Ok(None) => {}                       // not answered yet
//!     Ok(Some(Reply::Accepted)) => {}      // ack only, not a verdict
//!     Err(ClientError::Refused(code)) => {} // the service said no
//!     Err(e) => {}                         // channel failed
//! }
//!
//! // Later, poll for progress:
//! client.query_status()?;
//! match client.poll() {
//!     Ok(Some(Reply::Status(s))) => {}     // hashing or done
//!     _ => {}
//! }
//! ```

#![no_std]

mod client;
mod error;

pub use client::{CryptoIpcClient, Reply};
pub use error::ClientError;
