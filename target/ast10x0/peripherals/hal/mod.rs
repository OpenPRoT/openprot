// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mode-agnostic OpenPRoT HAL vocabulary: capability + role traits, and the pin-token generators.
//! Unlike the blocking/nb/async crates, nothing here is flavored by an execution mode.
//!
//! Lives in `target/ast10x0` for the time being, not `//hal`, until a second target needs it —
//! this keeps the abstraction from claiming a portability it hasn't earned yet.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod field_mux;
pub mod gpio;
pub mod resource;
