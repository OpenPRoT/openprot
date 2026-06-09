// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! In-process transport: client marshalling → [`dispatch`] → a real
//! `embedded_hal::i2c::I2c` bus, no kernel.
//!
//! First-class, not a test scaffold. It is:
//!  - the host-test path — `I2cClient::new(LoopbackTransport::new(mock_bus))`
//!    exercises the *same* encoders/decoders the IPC client uses; and
//!  - the correct early-boot path before IPC exists (drive a real bus
//!    in-process through the identical protocol code).
//!
//! Swapping `LoopbackTransport` for `i2c_client_ipc::IpcTransport` is a wiring
//! choice, never a code fork — that is the whole point of the seam.

use i2c_api::seam::{I2c, SevenBitAddress};
use i2c_api::{Transport, TransportError};

use crate::dispatch;

/// A [`Transport`] that runs [`dispatch`] against an owned in-process bus.
pub struct LoopbackTransport<B> {
    bus: B,
}

impl<B> LoopbackTransport<B> {
    pub const fn new(bus: B) -> Self {
        Self { bus }
    }
}

impl<B: I2c<SevenBitAddress>> Transport for LoopbackTransport<B> {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError> {
        // `dispatch` always produces a valid response frame (it encodes i2c
        // errors *into* the payload), so the transport layer never fails.
        Ok(dispatch(&mut self.bus, req, resp))
    }
}
