// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::reset::{ResetEvent, ResetPolicy, TargetCpuState};
use crate::spimux::{SpiMuxEvent, SpiMuxHandler};
use crate::usbmux::{UsbMuxEvent, UsbMuxHandler};
use earlgrey_gpio::EarlGreyGpio;
use userspace::time::{Clock, Instant, SystemClock};
use util_error::ErrorCode;

pub use crate::reset::TargetCpuState as State;

pub struct PlatformServer {
    gpio: EarlGreyGpio,
    usb_mux: UsbMuxHandler,
    spi_mux: SpiMuxHandler,
    reset_policy: ResetPolicy,
    exit_deadline: Instant,
}

impl PlatformServer {
    pub fn new(
        gpio: EarlGreyGpio,
        usb_mux: UsbMuxHandler,
        spi_mux: SpiMuxHandler,
        reset_policy: ResetPolicy,
    ) -> Self {
        Self {
            gpio,
            usb_mux,
            spi_mux,
            reset_policy,
            exit_deadline: Instant::MAX,
        }
    }

    pub fn state(&self) -> TargetCpuState {
        self.reset_policy.state()
    }

    pub fn next_deadline(&self) -> Instant {
        self.reset_policy.next_deadline().min(self.exit_deadline)
    }

    pub fn set_exit_deadline(&mut self, deadline: Instant) {
        self.exit_deadline = deadline;
    }

    pub fn should_exit(&self) -> bool {
        SystemClock::now() >= self.exit_deadline
    }

    pub fn start(&mut self, is_low_power_exit: bool) -> Result<(), ErrorCode> {
        // 1. Configure interrupts on reset monitors and USB presence.
        self.usb_mux.setup_interrupts(&mut self.gpio)?;
        self.reset_policy.setup_interrupts(&mut self.gpio)?;

        // 2. Dispatch initial startup events.
        if !is_low_power_exit {
            self.spi_mux
                .handle_event(SpiMuxEvent::ColdBoot, &mut self.gpio)?;
        }
        self.reset_policy
            .handle_event(ResetEvent::Start { is_low_power_exit }, &mut self.gpio)?;

        Ok(())
    }

    pub fn handle_timeout(&mut self) -> Result<(), ErrorCode> {
        self.reset_policy
            .handle_event(ResetEvent::Timeout, &mut self.gpio)
    }

    pub fn route_spi_mux(&mut self, route: crate::spimux::SpiMuxRoute) -> Result<(), ErrorCode> {
        self.spi_mux
            .handle_event(SpiMuxEvent::Route(route), &mut self.gpio)
    }

    /// Executes the safe 4-step SPI MUX handshake and switching sequence.
    pub fn switch_mux(
        &mut self,
        route: crate::spimux::SpiMuxRoute,
        flash_mux: &crate::flash_mux::FlashMuxClient,
    ) -> Result<(), ErrorCode> {
        let bitmap = match route {
            crate::spimux::SpiMuxRoute::HostCpu0Earlgrey1 => 0x2,
            crate::spimux::SpiMuxRoute::HostCpu1Earlgrey0 => 0x1,
        };
        // Step 1: Notify Flash Service of impending MUX switch (quiesce access).
        flash_mux.switch_mux_notice()?;

        // Step 2: Perform physical GPIO reset pulse and MUX channel selection.
        // If hardware switching fails, leave Flash Service in quiescent state (bitmap = 0)
        // to prevent accessing an uninitialized or unstable SPI bus.
        self.route_spi_mux(route)?;

        // Step 3: Notify Flash Service that MUX switch finished (reinit 4-byte mode & restore access).
        flash_mux.switch_mux_fin_notice(bitmap)?;

        Ok(())
    }

    pub fn handle_usb_presence_interrupt(&mut self) -> Result<(), ErrorCode> {
        self.usb_mux
            .handle_event(UsbMuxEvent::PinChanged, &mut self.gpio)
    }

    pub fn handle_rst_mon_interrupt(&mut self, index: usize) -> Result<(), ErrorCode> {
        self.reset_policy.handle_event(
            ResetEvent::MonitorFallingEdge {
                monitor_index: index,
            },
            &mut self.gpio,
        )
    }
}
