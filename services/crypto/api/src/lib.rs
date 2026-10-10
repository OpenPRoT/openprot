// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Crypto service IPC API: wire format and types.
//!
//! Defines the binary protocol the orchestrator uses to talk to the
//! crypto service over IPC. The caller names a flash region by
//! absolute address and length; the crypto service reads the bytes
//! from the flash service itself and reports a verdict. Firmware
//! never crosses this interface.
//!
//! Host-buildable, no kernel dependencies. Server and client share
//! these types; neither side re-invents the encoding. The transport that carries these frames is a
//! `util_service::AsyncTransport`, shared with every other IPC service.

#![no_std]

pub mod error;
pub mod wire;

pub use error::{ResponseCode, WireError};
pub use wire::{
    CryptoOp, RequestHeader, ResponseHeader, VerifyRegion, VerifyStatus, MAX_PAYLOAD_SIZE,
    MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
