// Licensed under the Apache-2.0 license

//! # MCTP Server
//!
//! Platform-independent MCTP server implementation for OpenPRoT.
//!
//! This crate provides the core MCTP server logic that manages:
//! - Listener and request handle allocation (via `mctp-lib` [`Router`](mctp_lib::Router))
//! - Inbound message routing to registered listeners
//! - Outbound message fragmentation and sending
//! - Timeout management for pending receive calls
//!
//! ## Transport Bindings
//!
//! The server is generic over the `mctp-lib` [`Sender`](mctp_lib::Sender) trait
//! for outbound transport. Transport-specific bindings (I2C, serial) implement
//! this trait and feed inbound packets via [`Server::inbound`].
//!
//! ## Platform Integration
//!
//! The server does not depend on any OS primitives. The platform layer
//! is responsible for:
//! - Driving the event loop (notifications, IPC dispatch)
//! - Providing a time source via [`Server::update`]
//! - Wiring up transport bindings

#![no_std]
#![warn(missing_docs)]

pub mod dispatch;
mod server;

pub use mctp_lib::Sender;
pub use server::{RecvResult, Server, ServerConfig};
