// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! The orchestrator's side of the PLDM update gate.
//!
//! The firmware device parks at each phase and raises a nudge; the
//! orchestrator reads `FdStatus` and answers with one operation. This crate
//! turns a status into that answer.
//!
//! [`AlwaysPerform`] is the policy that answers every phase with proceed.
//! It exists
//! so the update path can run end to end before any real policy is
//! written, and so tests have a gate that never blocks. It makes no
//! checks: no isolation, no SVN floor, no component identity. Nothing here
//! belongs on a shipping device.

#![no_std]

mod always_perform;
mod decision;

pub use always_perform::AlwaysPerform;
pub use decision::Decision;
