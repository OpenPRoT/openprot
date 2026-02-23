// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Async HAL traits for OpenPRoT
//!
//! This crate re-exports embedded-hal-async 1.0 traits for async/await-based
//! hardware abstraction layer operations compatible with modern async runtimes.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// Re-export embedded-hal-async 1.0 traits
pub use embedded_hal_async::*;
