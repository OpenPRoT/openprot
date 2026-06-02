// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

pub struct DummyDelay;

impl embedded_hal::delay::DelayNs for DummyDelay {
    fn delay_ns(&mut self, ns: u32) {
        for _ in 0..(ns / 100) {
            cortex_m::asm::nop();
        }
    }
}

pub trait Logger {
    fn debug(&mut self, msg: &str);
    fn error(&mut self, msg: &str);
}

pub struct NoOpLogger;
impl Logger for NoOpLogger {
    fn debug(&mut self, _msg: &str) {}
    fn error(&mut self, _msg: &str) {}
}
