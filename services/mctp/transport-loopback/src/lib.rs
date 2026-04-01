// Licensed under the Apache-2.0 license

//! # MCTP Loopback Transport Binding
//!
//! This crate provides a loopback transport binding for the MCTP server,
//! enabling direct server-to-server communication without physical transport.
//!
//! It implements [`mctp_lib::Sender`] for outbound MCTP packets and provides
//! a shared buffer mechanism for bidirectional packet exchange between two
//! MCTP endpoints.
//!
//! ## Design
//!
//! Unlike I2C transport which requires encoding/decoding with transport headers
//! and PEC, the loopback transport works with raw MCTP packets. This makes it
//! simpler and more efficient for in-memory communication.
//!
//! The [`LoopbackPair`] manages two unidirectional packet queues (A→B and B→A).
//! Each endpoint gets a [`LoopbackSender`] that writes to the appropriate queue.
//! The application polls the pair to transfer packets to the receiving server.

#![no_std]
#![warn(missing_docs)]

mod sender;

pub use sender::{LoopbackPair, LoopbackSender};
