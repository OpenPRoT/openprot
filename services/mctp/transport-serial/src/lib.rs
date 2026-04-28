// Licensed under the Apache-2.0 license

//! MCTP over Serial Transport
//!
//! Transport binding for sending and receiving MCTP packets over
//! a serial (UART) link using the `mctp-lib` serial framing.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────┐
//! │   MCTP Server       │
//! │  (openprot-mctp-    │
//! │   server)           │
//! └────────┬────────────┘
//!          │ mctp_lib::Sender / inbound()
//!          ▼
//! ┌─────────────────────┐
//! │  SerialSender       │  ← This crate
//! │  SerialReceiver     │
//! └────────┬────────────┘
//!          │ embedded-io::Write + serial RX source
//!          ▼
//! ┌─────────────────────┐
//! │  Selected backend   │
//! │  (per-target bind)  │
//! └─────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use openprot_mctp_transport_serial::{SerialSender, SerialReceiver};
//!
//! let sender = SerialSender::new(serial_writer);
//! let mut receiver = SerialReceiver::new();
//!
//! // Feed bytes from the selected serial backend into the receiver:
//! while let Ok(count) = serial_read(&mut buf) {
//!     for &byte in &buf[..count] {
//!     if let Some(pkt) = receiver.feed(byte) {
//!         server.inbound(pkt).ok();
//!     }
//!     }
//! }
//! ```

#![no_std]
#![warn(missing_docs)]

mod receiver;
mod sender;

pub use receiver::SerialReceiver;
pub use sender::SerialSender;
