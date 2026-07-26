// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use earlgrey_gpio::{EarlGreyGpio, GpioMask, GpioPin};
use openprot_hal_blocking::gpio_port::{
    EdgeSensitivity, GpioInterrupt, GpioPort, InterruptOperation, PinMask,
};
use userspace::time::{Clock, Duration, Instant, SystemClock};
use util_error::ErrorCode;
use zfmt::Zfmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetCpuState {
    ColdBoot,
    LatchReset,
    Measure,
    ReleaseReset,
    Running,
}

#[derive(Zfmt, Clone)]
#[zfmt(format = "Platform State: {state}")]
pub struct StateTransition {
    pub state: &'static str,
}

#[derive(Zfmt, Clone)]
#[zfmt(format = "Reset Monitor: {monitor}")]
pub struct ResetMonitor {
    pub monitor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetEvent {
    Start { is_low_power_exit: bool },
    MonitorFallingEdge { monitor_index: usize },
    Timeout,
}

pub struct TargetCpuReset {
    pub rst_ctrl0_n: GpioPin,
    pub rst_mon0_n: GpioPin,
    pub rst_mon1_n: GpioPin,
    pub state: TargetCpuState,
    next_deadline: Instant,
}

impl TargetCpuReset {
    pub const fn new(rst_ctrl0_n: GpioPin, rst_mon0_n: GpioPin, rst_mon1_n: GpioPin) -> Self {
        Self {
            rst_ctrl0_n,
            rst_mon0_n,
            rst_mon1_n,
            state: TargetCpuState::ColdBoot,
            next_deadline: Instant::MAX,
        }
    }

    pub fn setup_interrupts(&self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        let rst_mon0 = GpioMask::from(self.rst_mon0_n);
        gpio.irq_configure(rst_mon0, EdgeSensitivity::FallingEdge)
            .map_err(ErrorCode::from)?;
        gpio.irq_control(rst_mon0, InterruptOperation::Enable)
            .map_err(ErrorCode::from)?;

        let rst_mon1 = GpioMask::from(self.rst_mon1_n);
        gpio.irq_configure(rst_mon1, EdgeSensitivity::FallingEdge)
            .map_err(ErrorCode::from)?;
        gpio.irq_control(rst_mon1, InterruptOperation::Enable)
            .map_err(ErrorCode::from)?;

        Ok(())
    }

    pub fn handle_event(
        &mut self,
        event: ResetEvent,
        gpio: &mut EarlGreyGpio,
    ) -> Result<(), ErrorCode> {
        match event {
            ResetEvent::Start { is_low_power_exit } => {
                if !is_low_power_exit {
                    self.transition_to_latch_reset(gpio)?;
                } else {
                    self.transition_to_running();
                }
            }
            ResetEvent::MonitorFallingEdge { monitor_index } => {
                let pin = if monitor_index == 0 {
                    self.rst_mon0_n
                } else {
                    self.rst_mon1_n
                };
                let pin_mask = GpioMask::from(pin);

                gpio.irq_control(pin_mask, InterruptOperation::Clear)
                    .map_err(ErrorCode::from)?;

                util_zfmt::info!(ResetMonitor {
                    monitor: monitor_index as u32
                });

                self.transition_to_latch_reset(gpio)?;
            }
            ResetEvent::Timeout => match self.state {
                TargetCpuState::Measure => {
                    self.transition_to_release_reset(gpio)?;
                }
                _ => {
                    self.next_deadline = Instant::MAX;
                }
            },
        }
        Ok(())
    }

    fn transition_to_latch_reset(&mut self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        self.state = TargetCpuState::LatchReset;
        util_zfmt::info!(StateTransition {
            state: "LatchReset"
        });
        gpio.set_reset(GpioMask::empty(), GpioMask::from(self.rst_ctrl0_n))
            .map_err(ErrorCode::from)?;

        self.transition_to_measure();
        Ok(())
    }

    fn transition_to_measure(&mut self) {
        self.state = TargetCpuState::Measure;
        util_zfmt::info!(StateTransition { state: "Measure" });
        self.next_deadline = SystemClock::now() + Duration::from_secs(1);
    }

    fn transition_to_release_reset(&mut self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        self.state = TargetCpuState::ReleaseReset;
        util_zfmt::info!(StateTransition {
            state: "ReleaseReset"
        });
        gpio.set_reset(GpioMask::from(self.rst_ctrl0_n), GpioMask::empty())
            .map_err(ErrorCode::from)?;

        self.transition_to_running();
        Ok(())
    }

    fn transition_to_running(&mut self) {
        self.state = TargetCpuState::Running;
        util_zfmt::info!(StateTransition { state: "Running" });
        self.next_deadline = Instant::MAX;
    }

    pub fn next_deadline(&self) -> Instant {
        self.next_deadline
    }

    pub fn state(&self) -> TargetCpuState {
        self.state
    }
}

pub enum ResetPolicy {
    TargetCpu(TargetCpuReset),
}

impl ResetPolicy {
    pub fn setup_interrupts(&self, gpio: &mut EarlGreyGpio) -> Result<(), ErrorCode> {
        match self {
            Self::TargetCpu(policy) => policy.setup_interrupts(gpio),
        }
    }

    pub fn handle_event(
        &mut self,
        event: ResetEvent,
        gpio: &mut EarlGreyGpio,
    ) -> Result<(), ErrorCode> {
        match self {
            Self::TargetCpu(policy) => policy.handle_event(event, gpio),
        }
    }

    pub fn next_deadline(&self) -> Instant {
        match self {
            Self::TargetCpu(policy) => policy.next_deadline(),
        }
    }

    pub fn state(&self) -> TargetCpuState {
        match self {
            Self::TargetCpu(policy) => policy.state(),
        }
    }
}
