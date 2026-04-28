// Licensed under the Apache-2.0 license

//! # MCTP Service API
//!
//! This crate provides the client-side API for interacting with the MCTP service.
//! It defines the types, traits, and error handling for MCTP operations.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────┐
//! │   Application       │
//! │  (echo, SPDM, etc.) │
//! └─────────┬───────────┘
//!           │ uses MctpClient trait
//!           ▼
//! ┌─────────────────────┐
//! │   mctp-api           │◄── This crate
//! │  (types & traits)    │
//! └─────────┬───────────┘
//!           │ IPC (implementation specific)
//!           ▼
//! ┌─────────────────────┐
//! │   MCTP Server       │
//! │  (transport layer)  │
//! └─────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Listener mode**: Receive incoming MCTP messages by type
//! - **Request mode**: Send requests to a remote EID and receive responses
//! - **Platform independent**: No OS-specific dependencies

#![no_std]
#![warn(missing_docs)]

mod error;
pub mod stack;
mod traits;
pub mod wire;

pub use error::{MctpError, ResponseCode};
pub use stack::{Stack, StackListener, StackReqChannel, StackRespChannel};
pub use traits::{MctpClient, MctpListener, MctpReqChannel, MctpRespChannel};

/// An opaque handle for a listener, request, or response channel.
///
/// Handles are allocated by the MCTP server and must be released
/// via `drop` when no longer needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(pub u32);

/// Metadata returned by a successful receive operation.
#[derive(Clone, Copy, Debug)]
pub struct RecvMetadata {
    /// MCTP message type.
    pub msg_type: u8,
    /// Whether an integrity check was present.
    pub msg_ic: bool,
    /// The tag value for correlating request/response pairs.
    pub msg_tag: u8,
    /// The source endpoint ID.
    pub remote_eid: u8,
    /// The size of the payload in bytes.
    pub payload_size: usize,
}
