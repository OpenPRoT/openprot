// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Platform-independent flash server.
//!
//! Nothing here touches the kernel. The loop that carries requests over a
//! channel is `flash_server_runtime`.

#![no_std]

mod server;

pub use server::{Outcome, Server};
