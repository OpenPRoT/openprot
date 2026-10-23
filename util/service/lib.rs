// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Transport and dispatch traits for IPC services.
//!
//! Three traits: `Transport` (blocking), `AsyncTransport` (split-phase),
//! and `Dispatch` (server). A service defines its wire format in its own
//! crate and plugs into these.

#![no_std]

mod delayed;
mod dispatch;
mod loopback;
mod transport;

pub use delayed::Delayed;
pub use dispatch::{Dispatch, DispatchError};
pub use loopback::Loopback;
pub use transport::{AsyncTransport, Transport, TransportError};
