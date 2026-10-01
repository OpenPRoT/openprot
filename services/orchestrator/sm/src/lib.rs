// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot_orchestrator_sm` — the eRoT boot-sequence state machine.
//!
//! This is the pure decision core: it describes side effects as [`Effect`]
//! values rather than performing them; the surrounding OpenPRoT platform
//! driver carries them out via a [`Platform`] impl. No concrete hardware
//! appears here — the machine is generic over an opaque [`ComponentId`].
//!
//! Three invariants define the boundary:
//!   1. **Effects flow through [`Sink`]** — fresh per event, drained afterward.
//!   2. **Feedback as data ([`Effect::Emit`])** — follow-up events are effects,
//!      visible in the trace; used for the retry cap (INV7).
//!   3. **Reads as events** — outside information arrives in [`Event`] payloads;
//!      the core never reads anything directly.
//!
//! The crate is split by role: `model` holds the vocabulary (states, events,
//! effects, chain config), [`sink`] the effect buffer and its capacities,
//! `status` the per-component service record, [`rot`] the storage and the
//! transition handlers, [`platform`] the executor seam, and [`orchestrator`]
//! the event loop a caller drives.

#![no_std]
#![forbid(unsafe_code)]

mod model;
pub mod orchestrator;
pub mod platform;
pub mod rot;
pub mod sink;
mod status;

#[doc(inline)]
pub use model::*;
#[doc(inline)]
pub use orchestrator::Orchestrator;
#[doc(inline)]
pub use platform::{EffectError, Platform};
#[doc(inline)]
pub use rot::Rot;
#[doc(inline)]
pub use sink::Sink;

#[cfg(test)]
mod tests;
