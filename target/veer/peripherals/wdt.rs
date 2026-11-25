// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for the MCU watchdog in the Caliptra-SS MCI block.
//!
//! The watchdog has two timers that count clock cycles. Timer 1 counts while
//! it is enabled. When it reaches its period it has timed out: the MCI sets
//! the timer 1 timeout bit of the `error0_internal_intr_r` error status
//! register, and timer 2 starts to count. When timer 2 reaches its period the
//! MCI raises an NMI on the MCU and sets `nmi_pin` in `HW_ERROR_FATAL`.
//!
//! When `WDT_TIMER2_EN` is set, timer 2 counts independently of timer 1, and
//! its timeout does not raise an NMI. The driver expects that bit to be 0,
//! which is its value after an MCI reset and after the caliptra-mcu-sw ROM
//! has run.
//!
//! The caliptra-mcu-sw ROM starts the watchdog before it jumps to the runtime
//! image (`Mci::configure_wdt` in `romtime/src/mci.rs`). The image has to
//! restart the watchdog periodically or stop it.
//!
//! The driver depends on this behavior of the timers (`wdt.sv` in
//! caliptra-rtl, and its use in `mci_top.sv` in caliptra-ss):
//!
//! - Timer 1 holds its count while it is disabled.
//! - The timer 1 restart bit takes effect only while timer 1 is enabled and
//!   has not timed out.
//! - Timer 2 counts while timer 1 is timed out, whether or not timer 1 is
//!   enabled.
//! - Clearing the timer 1 timeout bit of `error0_internal_intr_r` restarts
//!   both timers if timer 1 has timed out and timer 2 has not.

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::mci;
use mci::bits::{Error0IntrT, WdtStatus, WdtTimer1Ctrl, WdtTimer1En};
use tock_registers::LocalRegisterCopy;

/// The MCI registers this driver uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reg {
    /// `WDT_TIMER1_EN`.
    Timer1En,
    /// `WDT_TIMER1_CTRL`: the timer 1 restart bit.
    Timer1Ctrl,
    /// `WDT_TIMER1_TIMEOUT_PERIOD[0]`: low word of the timer 1 period.
    Timer1PeriodLow,
    /// `WDT_TIMER1_TIMEOUT_PERIOD[1]`: high word of the timer 1 period.
    Timer1PeriodHigh,
    /// `WDT_TIMER2_TIMEOUT_PERIOD[0]`: low word of the timer 2 period.
    Timer2PeriodLow,
    /// `WDT_TIMER2_TIMEOUT_PERIOD[1]`: high word of the timer 2 period.
    Timer2PeriodHigh,
    /// `WDT_STATUS`: whether each timer has timed out.
    Status,
    /// `error0_internal_intr_r`: write-one-to-clear error status bits, one
    /// of which is the timer 1 timeout.
    Error0Status,
}

impl Reg {
    /// Byte offset of the register from the MCI block base.
    pub const fn offset(self) -> usize {
        match self {
            Reg::Timer1En => offset_of!(mci::regs::Mci, mci_reg_wdt_timer1_en),
            Reg::Timer1Ctrl => offset_of!(mci::regs::Mci, mci_reg_wdt_timer1_ctrl),
            Reg::Timer1PeriodLow => offset_of!(mci::regs::Mci, mci_reg_wdt_timer1_timeout_period),
            Reg::Timer1PeriodHigh => {
                offset_of!(mci::regs::Mci, mci_reg_wdt_timer1_timeout_period) + 4
            }
            Reg::Timer2PeriodLow => offset_of!(mci::regs::Mci, mci_reg_wdt_timer2_timeout_period),
            Reg::Timer2PeriodHigh => {
                offset_of!(mci::regs::Mci, mci_reg_wdt_timer2_timeout_period) + 4
            }
            Reg::Status => offset_of!(mci::regs::Mci, mci_reg_wdt_status),
            Reg::Error0Status => offset_of!(mci::regs::Mci, intr_block_rf_error0_internal_intr_r),
        }
    }
}

/// Access to the MCI registers. The driver is generic over this trait so that
/// host tests can use a model in place of the hardware.
pub trait Mmio {
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// The MCI's memory-mapped registers.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of the MCI register block. The block must be
    /// mapped for the caller, and no other code may access the registers in
    /// [`Reg`].
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }
}

impl Mmio for MmioBlock {
    fn read(&self, reg: Reg) -> u32 {
        let addr = self.base.wrapping_add(reg.offset());
        // SAFETY: every `Reg` is the offset of a register inside the block that
        // the caller of `new` guaranteed is mapped.
        unsafe { with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        let addr = self.base.wrapping_add(reg.offset());
        // SAFETY: as for `read`.
        unsafe { with_exposed_provenance_mut::<u32>(addr).write_volatile(value) }
    }
}

/// The MCU watchdog.
pub struct Wdt<M: Mmio = MmioBlock> {
    mmio: M,
    // !Send + !Sync: exclusive ownership of one physical peripheral
    _not_send_sync: PhantomData<*mut ()>,
}

impl Wdt<MmioBlock> {
    /// `mci_base` is the address of the MCI register block (`mci_offset` in
    /// the caliptra-mcu-sw memory map).
    ///
    /// # Safety
    ///
    /// The MCI register block must be at `mci_base` and stay mapped for the
    /// caller, and no other code may use the watchdog.
    pub const unsafe fn new(mci_base: usize) -> Self {
        // SAFETY: the caller guarantees that the block is mapped and that
        // nothing else uses the watchdog registers.
        Self::with_mmio(unsafe { MmioBlock::new(mci_base) })
    }
}

impl<M: Mmio> Wdt<M> {
    /// Constructs the driver over `mmio`.
    pub const fn with_mmio(mmio: M) -> Self {
        Self {
            mmio,
            _not_send_sync: PhantomData,
        }
    }

    /// Starts the watchdog. Unless [`restart`](Self::restart) is called,
    /// timer 1 times out `timer1_period` clock cycles from now, and the MCI
    /// raises the NMI `timer2_period` cycles after that.
    pub fn start(&mut self, timer1_period: u64, timer2_period: u64) {
        self.stop();
        // If other code disabled timer 1, the count that timer 1 holds can
        // be any value. It can be a value at which timer 1 times out as soon
        // as it has its new period and is enabled. Timer 2 has the longest
        // period until timer 1 has been restarted, so that this timeout does
        // not lead to the NMI.
        self.write_period(Reg::Timer2PeriodLow, Reg::Timer2PeriodHigh, u64::MAX);
        self.write_period(Reg::Timer1PeriodLow, Reg::Timer1PeriodHigh, timer1_period);
        self.mmio
            .write(Reg::Timer1En, WdtTimer1En::Timer1En::SET.into());
        self.restart();
        self.write_period(Reg::Timer2PeriodLow, Reg::Timer2PeriodHigh, timer2_period);
    }

    /// Makes timer 1 count from 0 again. This also works after timer 1 has
    /// timed out. It has no effect on the timers while the watchdog is
    /// stopped or after timer 2 has timed out.
    pub fn restart(&mut self) {
        // The first write restarts timer 1 if it has not timed out. The
        // second write restarts both timers if timer 1 has timed out. If
        // timer 1 times out between the two writes, the second write restarts
        // it. With the writes in the other order, neither would.
        self.mmio
            .write(Reg::Timer1Ctrl, WdtTimer1Ctrl::Timer1Restart::SET.into());
        self.mmio.write(
            Reg::Error0Status,
            Error0IntrT::ErrorWdtTimer1TimeoutSts::SET.into(),
        );
    }

    /// Stops the watchdog.
    pub fn stop(&mut self) {
        // Timer 2 counts while timer 1 is timed out, even after timer 1 is
        // disabled. Timer 1 is restarted first so that it is not left timed
        // out.
        self.restart();
        self.mmio.write(Reg::Timer1En, 0);
    }

    /// Whether the watchdog is running.
    pub fn is_enabled(&self) -> bool {
        LocalRegisterCopy::<u32, WdtTimer1En::Register>::new(self.mmio.read(Reg::Timer1En))
            .is_set(WdtTimer1En::Timer1En)
    }

    /// Whether timer 1 has timed out. [`restart`](Self::restart) clears it.
    pub fn timer1_expired(&self) -> bool {
        self.status().is_set(WdtStatus::T1Timeout)
    }

    /// Whether timer 2 has timed out, which raises the NMI.
    pub fn timer2_expired(&self) -> bool {
        self.status().is_set(WdtStatus::T2Timeout)
    }

    fn status(&self) -> LocalRegisterCopy<u32, WdtStatus::Register> {
        LocalRegisterCopy::new(self.mmio.read(Reg::Status))
    }

    fn write_period(&mut self, low: Reg, high: Reg, period: u64) {
        self.mmio.write(low, period as u32);
        self.mmio.write(high, (period >> 32) as u32);
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;

    const T1_STATUS: u32 = 1 << 4;

    /// The watchdog logic of `wdt.sv`, `mci_wdt_top.sv` and `mci_reg_top.sv`
    /// with `WDT_TIMER2_EN` clear. [`tick`](Self::tick) is one clock cycle.
    struct Timers {
        timer1_en: bool,
        period1: u64,
        period2: u64,
        count1: u64,
        count2: u64,
        t1_timeout_prev: bool,
        error0_status: u32,
        nmi: bool,
    }

    /// Register writes whose effect on the timers lasts one clock cycle.
    #[derive(Default)]
    struct Pulses {
        /// The timer 1 restart bit was written.
        restart: bool,
        /// The timer 1 timeout bit of the error status was cleared.
        serviced: bool,
    }

    impl Timers {
        fn t1_timeout(&self) -> bool {
            self.count1 == self.period1
        }

        fn t2_timeout(&self) -> bool {
            self.count2 == self.period2
        }

        fn tick(&mut self, pulses: Pulses) {
            let t1 = self.t1_timeout();
            let t2 = self.t2_timeout();
            if t1 && !self.t1_timeout_prev {
                self.error0_status |= T1_STATUS;
            }
            if t2 {
                self.nmi = true;
            }
            self.t1_timeout_prev = t1;

            let serviced_restart = pulses.serviced && t1 && !t2;
            if self.timer1_en {
                if (pulses.restart && !t1) || serviced_restart {
                    self.count1 = 0;
                } else if !t1 {
                    self.count1 = self.count1.wrapping_add(1);
                }
            }
            if t1 {
                if serviced_restart {
                    self.count2 = 0;
                } else if !t2 {
                    self.count2 = self.count2.wrapping_add(1);
                }
            }
        }
    }

    /// The MCI registers in front of [`Timers`]. Each register access takes
    /// one clock cycle.
    struct FakeMci {
        timers: RefCell<Timers>,
    }

    impl FakeMci {
        /// The state after an MCI reset.
        fn new() -> Self {
            Self {
                timers: RefCell::new(Timers {
                    timer1_en: false,
                    period1: u64::MAX,
                    period2: u64::MAX,
                    count1: 0,
                    count2: 0,
                    t1_timeout_prev: false,
                    error0_status: 0,
                    nmi: false,
                }),
            }
        }

        fn run(&self, cycles: u64) {
            for _ in 0..cycles {
                self.timers.borrow_mut().tick(Pulses::default());
            }
        }
    }

    fn set_low(period: &mut u64, value: u32) {
        *period = (*period & !0xffff_ffff) | u64::from(value);
    }

    fn set_high(period: &mut u64, value: u32) {
        *period = (*period & 0xffff_ffff) | (u64::from(value) << 32);
    }

    impl Mmio for FakeMci {
        fn read(&self, reg: Reg) -> u32 {
            let mut timers = self.timers.borrow_mut();
            timers.tick(Pulses::default());
            match reg {
                Reg::Timer1En => u32::from(timers.timer1_en),
                Reg::Timer1Ctrl => 0,
                Reg::Timer1PeriodLow => timers.period1 as u32,
                Reg::Timer1PeriodHigh => (timers.period1 >> 32) as u32,
                Reg::Timer2PeriodLow => timers.period2 as u32,
                Reg::Timer2PeriodHigh => (timers.period2 >> 32) as u32,
                Reg::Status => {
                    u32::from(timers.t1_timeout()) | (u32::from(timers.t2_timeout()) << 1)
                }
                Reg::Error0Status => timers.error0_status,
            }
        }

        fn write(&mut self, reg: Reg, value: u32) {
            let timers = self.timers.get_mut();
            let mut pulses = Pulses::default();
            match reg {
                Reg::Timer1En => timers.timer1_en = value & 1 != 0,
                Reg::Timer1Ctrl => pulses.restart = value & 1 != 0,
                Reg::Timer1PeriodLow => set_low(&mut timers.period1, value),
                Reg::Timer1PeriodHigh => set_high(&mut timers.period1, value),
                Reg::Timer2PeriodLow => set_low(&mut timers.period2, value),
                Reg::Timer2PeriodHigh => set_high(&mut timers.period2, value),
                Reg::Status => {}
                Reg::Error0Status => {
                    pulses.serviced = timers.error0_status & value & T1_STATUS != 0;
                    timers.error0_status &= !value;
                }
            }
            timers.tick(pulses);
        }
    }

    const PERIOD1: u64 = 1000;
    const PERIOD2: u64 = 100;
    /// More clock cycles than any driver function takes.
    const SLACK: u64 = 32;

    fn wdt() -> Wdt<FakeMci> {
        Wdt::with_mmio(FakeMci::new())
    }

    fn nmi(wdt: &Wdt<FakeMci>) -> bool {
        wdt.mmio.timers.borrow().nmi
    }

    #[test]
    fn register_offsets_match_the_hardware_map() {
        assert_eq!(Reg::Timer1En.offset(), 0xb0);
        assert_eq!(Reg::Timer1Ctrl.offset(), 0xb4);
        assert_eq!(Reg::Timer1PeriodLow.offset(), 0xb8);
        assert_eq!(Reg::Timer1PeriodHigh.offset(), 0xbc);
        assert_eq!(Reg::Timer2PeriodLow.offset(), 0xc8);
        assert_eq!(Reg::Timer2PeriodHigh.offset(), 0xcc);
        assert_eq!(Reg::Status.offset(), 0xd0);
        assert_eq!(Reg::Error0Status.offset(), 0x101c);
    }

    #[test]
    fn is_stopped_after_reset() {
        let wdt = wdt();
        assert!(!wdt.is_enabled());
        wdt.mmio.run(10 * PERIOD1);
        assert!(!wdt.timer1_expired());
        assert!(!nmi(&wdt));
    }

    #[test]
    fn times_out_after_both_periods() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        assert!(wdt.is_enabled());

        wdt.mmio.run(PERIOD1 - SLACK);
        assert!(!wdt.timer1_expired());
        wdt.mmio.run(SLACK);
        assert!(wdt.timer1_expired());
        assert!(!wdt.timer2_expired());

        wdt.mmio.run(PERIOD2 - SLACK);
        assert!(!wdt.timer2_expired());
        assert!(!nmi(&wdt));
        wdt.mmio.run(SLACK);
        assert!(wdt.timer2_expired());
        assert!(nmi(&wdt));
    }

    #[test]
    fn restart_keeps_the_watchdog_from_timing_out() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        for _ in 0..10 {
            wdt.mmio.run(PERIOD1 - SLACK);
            assert!(!wdt.timer1_expired());
            wdt.restart();
        }
        assert!(!nmi(&wdt));
    }

    #[test]
    fn restart_after_timer1_timeout_restarts_both_timers() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 + PERIOD2 / 2);
        assert!(wdt.timer1_expired());

        wdt.restart();
        assert!(!wdt.timer1_expired());
        // Timer 2 counts from 0 again after the next timer 1 timeout.
        wdt.mmio.run(PERIOD1 + PERIOD2 - SLACK);
        assert!(wdt.timer1_expired());
        assert!(!nmi(&wdt));
        wdt.mmio.run(SLACK);
        assert!(nmi(&wdt));
    }

    #[test]
    fn stop_keeps_the_watchdog_from_timing_out() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 / 2);
        wdt.stop();
        assert!(!wdt.is_enabled());
        wdt.mmio.run(10 * PERIOD1);
        assert!(!wdt.timer1_expired());
        assert!(!nmi(&wdt));
    }

    #[test]
    fn stop_after_timer1_timeout_stops_timer2() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 + PERIOD2 / 2);
        wdt.stop();
        wdt.mmio.run(10 * PERIOD2);
        assert!(!wdt.timer1_expired());
        assert!(!nmi(&wdt));
    }

    #[test]
    fn start_after_stop_runs_for_the_whole_period() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 - SLACK);
        wdt.stop();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 - SLACK);
        assert!(!wdt.timer1_expired());
        wdt.mmio.run(SLACK);
        assert!(wdt.timer1_expired());
    }

    #[test]
    fn start_changes_the_periods_of_a_running_watchdog() {
        let mut wdt = wdt();
        wdt.start(PERIOD1, PERIOD2);
        wdt.mmio.run(PERIOD1 / 2);
        wdt.start(4 * PERIOD1, PERIOD2);
        wdt.mmio.run(4 * PERIOD1 - SLACK);
        assert!(!wdt.timer1_expired());
        wdt.mmio.run(SLACK);
        assert!(wdt.timer1_expired());
    }

    /// Other code, such as `Mci::disable_wdt` in caliptra-mcu-sw, disables
    /// timer 1 without restarting it, so timer 1 holds the count that it had.
    /// The test leaves timer 1 holding each count near the period that `start`
    /// then sets, and checks that the watchdog runs for that whole period.
    #[test]
    fn start_handles_a_count_held_near_the_period() {
        for held in PERIOD1 - SLACK..=PERIOD1 {
            let mut wdt = wdt();
            wdt.mmio.write(Reg::Timer1En, 1);
            wdt.mmio.run(held);
            wdt.mmio.write(Reg::Timer1En, 0);

            wdt.start(PERIOD1, 1);
            assert!(!wdt.timer1_expired(), "held count {held}");
            wdt.mmio.run(PERIOD1 - SLACK);
            assert!(!wdt.timer1_expired(), "held count {held}");
            assert!(!nmi(&wdt), "held count {held}");
            wdt.mmio.run(SLACK + 1);
            assert!(nmi(&wdt), "held count {held}");
        }
    }
}
