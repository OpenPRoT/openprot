// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use ast1060_pac as device;
use core::marker::PhantomData;

use super::types::{
    Bank, BankDevice, Direction, Error, InitialLevel, InterruptMode, InterruptTrigger,
    SgpiomPinConfig,
};

/// Snapshot of live SGPIOM register state for one bank, returned by
/// [`Sgpiom::dump_state`]. The caller decides how to log or inspect the values.
#[derive(Debug, Clone, Copy)]
pub struct SgpiomBankState {
    pub bank: Bank,
    pub config: u32,
    pub data: u32,
    pub latch: u32,
    pub int_en: u32,
    pub int_status: u32,
}

pub struct Sgpiom {
    sgpiom: *const device::sgpiom::RegisterBlock,
    /// Prevent `Send` and `Sync`.
    ///
    /// MMIO register blocks must not be transferred across threads or
    /// shared by reference due to potential side effects and lack of
    /// synchronization guarantees.
    _not_send_sync: PhantomData<*const ()>,
}

impl Sgpiom {
    /// Create an SGPIOM instance from a raw register-block pointer.
    ///
    /// # Safety
    ///
    /// - `sgpiom` must be a valid, non-null pointer to the AST1060 SGPIOM register block.
    /// - The pointed register block must remain valid for the lifetime of this `Sgpiom`.
    /// - Caller must enforce global ownership so concurrent mutable access does not occur.
    pub const unsafe fn new(sgpiom: *const device::sgpiom::RegisterBlock) -> Self {
        Self {
            sgpiom,
            _not_send_sync: PhantomData,
        }
    }

    /// Create an instance pointing to the global AST1060 SGPIOM register block.
    ///
    /// # Safety
    ///
    /// Caller must ensure access to the singleton SGPIOM is coordinated.
    pub unsafe fn new_global() -> Self {
        // SAFETY: Caller upholds the singleton access contract.
        unsafe { Self::new(device::Sgpiom::ptr()) }
    }

    #[inline]
    fn regs(&self) -> &device::sgpiom::RegisterBlock {
        // SAFETY: `Sgpiom` construction is `unsafe`, so caller upholds pointer validity,
        // non-nullness, and aliasing/ownership requirements.
        unsafe { &*self.sgpiom }
    }

    /// Read the global configuration register (`gpio554`) back from hardware.
    #[must_use]
    pub fn read_config(&self) -> u32 {
        self.regs().gpio554().read().bits()
    }

    /// Snapshot the live SGPIOM register state for a bank.
    ///
    /// All values are read back from hardware (not the last written value), so
    /// this confirms whether writes actually stuck and the engine reflects them.
    /// The caller is responsible for logging or otherwise consuming the result.
    #[must_use]
    pub fn dump_state(&self, bank: Bank) -> SgpiomBankState {
        SgpiomBankState {
            bank,
            config: self.read_config(),
            data: self.port_get_raw(bank),
            latch: self.read_output_latch(bank),
            int_en: self.int_en_read(bank),
            int_status: self.interrupt_status(bank),
        }
    }

    /// Configures SGPIOM global settings.
    ///
    /// `ngpios` is total SGPIO count across banks.
    pub fn configure_global(&self, ngpios: u16, clock_div: u16) -> Result<(), Error> {
        // Four 32-pin banks (A-P) => 128 max. Zephyr's AST10x0 DTS uses
        // ngpios = 128; reject out-of-range instead of silently masking the
        // 5-bit `numbers` hardware field.
        if ngpios == 0 || ngpios > 128 {
            return Err(Error::InvalidNgpios);
        }

        let numbers = ((ngpios as u32 + 7) / 8) as u8;

        self.regs().gpio554().modify(|_, w| {
            w.enbl_of_serial_gpio().set_bit();
            // SAFETY: writing the datasheet-defined numbers and clock-division fields.
            unsafe { w.numbers_of_serial_gpiopins().bits(numbers) };
            unsafe { w.serial_gpioclk_division().bits(clock_div) };
            w
        });
        Ok(())
    }

    /// Read the raw 32-bit Data Value register for a bank.
    ///
    /// This returns the SGPIOM sampled *input* state, NOT the last value driven
    /// out. For read-modify-write of outputs use [`Self::read_output_latch`].
    #[must_use]
    pub fn port_get_raw(&self, bank: Bank) -> u32 {
        match bank {
            Bank::Ad => self.regs().gpio500().read().bits(),
            Bank::Eh => self.regs().gpio51c().read().bits(),
            Bank::Il => self.regs().gpio538().read().bits(),
            Bank::Mp => self.regs().gpio590().read().bits(),
        }
    }

    /// Read the output-latch readback register for a bank.
    ///
    /// The Data Value register (`port_get_raw`) returns the sampled *input*
    /// state, not the value last driven out. For read-modify-write of outputs
    /// we must read the dedicated Data Read (output-latch) register so that
    /// untouched bits retain their previously driven value rather than being
    /// overwritten with input samples.
    ///
    /// Bank `Mp` (M/N/O/P) has no `gpio57c()` accessor in the PAC, so its latch
    /// readback (`wr_latch[3]` in Zephyr, controller base + 0x7c) is read raw to
    /// stay aligned with the Zephyr RMW across all four banks.
    #[must_use]
    pub fn read_output_latch(&self, bank: Bank) -> u32 {
        match bank {
            Bank::Ad => self.regs().gpio570().read().bits(),
            Bank::Eh => self.regs().gpio574().read().bits(),
            Bank::Il => self.regs().gpio578().read().bits(),
            // SAFETY: `self.sgpiom` is a valid RegisterBlock pointer (construction
            // contract). The register block base is the controller base (0x..0500);
            // +0x7c (= 0x..057c) is the MP write-latch readback, inside the mapped
            // 0x100 SGPIOM register file.
            Bank::Mp => unsafe {
                let base = self.sgpiom.cast::<u8>();
                core::ptr::read_volatile(base.add(0x7c).cast::<u32>())
            },
        }
    }

    pub fn port_set_masked_raw(&self, bank: Bank, mask: u32, value: u32) {
        let current = self.read_output_latch(bank);
        let next = (current & !mask) | (value & mask);
        self.port_write_raw(bank, next);
    }

    pub fn port_set_bits_raw(&self, bank: Bank, mask: u32) {
        self.port_set_masked_raw(bank, mask, mask);
    }

    pub fn port_clear_bits_raw(&self, bank: Bank, mask: u32) {
        self.port_set_masked_raw(bank, mask, 0);
    }

    pub fn port_toggle_bits(&self, bank: Bank, mask: u32) {
        let current = self.read_output_latch(bank);
        self.port_write_raw(bank, current ^ mask);
    }

    pub fn pin_set_raw(&self, dev: &BankDevice, pin: u8, high: bool) -> Result<(), Error> {
        dev.validate_pin(pin)?;
        let bit = 1u32 << pin;
        if high {
            self.port_set_bits_raw(dev.bank, bit);
        } else {
            self.port_clear_bits_raw(dev.bank, bit);
        }
        Ok(())
    }

    pub fn configure_pin(
        &self,
        dev: &BankDevice,
        pin: u8,
        cfg: SgpiomPinConfig,
    ) -> Result<(), Error> {
        dev.validate_pin(pin)?;

        if cfg.pull_up || cfg.pull_down {
            return Err(Error::UnsupportedFlags);
        }

        if cfg.direction == Direction::Output {
            if let Some(initial) = cfg.initial {
                self.pin_set_raw(dev, pin, initial == InitialLevel::High)?;
            }
        }

        // SGPIOM direction is hardware managed in this design; no extra register write needed.
        Ok(())
    }

    /// Map a (`mode`, `trig`) pair to the 3-bit SGPIOM sensitivity-type code.
    ///
    /// Returns `Ok(None)` for [`InterruptMode::Disabled`] (no sensitivity to
    /// program) and `Err(UnsupportedFlags)` for invalid combinations.
    fn interrupt_sens_type(
        mode: InterruptMode,
        trig: InterruptTrigger,
    ) -> Result<Option<u8>, Error> {
        let int_type = match mode {
            InterruptMode::Disabled => return Ok(None),
            InterruptMode::Level => match trig {
                InterruptTrigger::Low => 2,
                InterruptTrigger::High => 3,
                InterruptTrigger::Both => return Err(Error::UnsupportedFlags),
            },
            InterruptMode::Edge => match trig {
                InterruptTrigger::Low => 0,
                InterruptTrigger::High => 1,
                InterruptTrigger::Both => 4,
            },
        };
        Ok(Some(int_type))
    }

    /// Write the 3-bit sensitivity code for every set bit in `mask` into the
    /// bank's `int_sens_type[0..2]` registers, leaving other pins untouched.
    ///
    /// The three sensitivity registers can only be rewritten one at a time, so
    /// a pin whose interrupt is enabled would pass through transient codes
    /// while they are updated (e.g. level-high `011` -> falling-edge `000`
    /// passes through level-low `010` after the first write) and could latch a
    /// spurious interrupt. To prevent that, any currently enabled pins in
    /// `mask` are disabled for the duration of the update; status latched
    /// under the old or transient sensitivity is cleared before their enable
    /// bits are restored.
    fn write_sens_bits(&self, bank: Bank, mask: u32, int_type: u8) {
        let en = self.int_en_read(bank);
        let enabled = en & mask;
        if enabled != 0 {
            self.int_en_write(bank, en & !mask);
        }

        let mut s0 = self.int_sens_read(bank, 0) & !mask;
        let mut s1 = self.int_sens_read(bank, 1) & !mask;
        let mut s2 = self.int_sens_read(bank, 2) & !mask;

        if (int_type & 0x1) != 0 {
            s0 |= mask;
        }
        if (int_type & 0x2) != 0 {
            s1 |= mask;
        }
        if (int_type & 0x4) != 0 {
            s2 |= mask;
        }

        self.int_sens_write(bank, 0, s0);
        self.int_sens_write(bank, 1, s1);
        self.int_sens_write(bank, 2, s2);

        if enabled != 0 {
            self.clear_interrupt_status(bank, enabled);
            self.int_en_write(bank, self.int_en_read(bank) | enabled);
        }
    }

    /// Configure a pin's interrupt sensitivity *and* enable/disable bit.
    ///
    /// For finer control (e.g. the HAL `GpioInterrupt` split of configure vs.
    /// enable), see [`Self::configure_interrupt_sensitivity`] and
    /// [`Self::set_interrupt_enable`].
    pub fn configure_interrupt(
        &self,
        dev: &BankDevice,
        pin: u8,
        mode: InterruptMode,
        trig: InterruptTrigger,
    ) -> Result<(), Error> {
        dev.validate_pin(pin)?;

        let bit = 1u32 << pin;
        match Self::interrupt_sens_type(mode, trig)? {
            None => {
                let en = self.int_en_read(dev.bank) & !bit;
                self.int_en_write(dev.bank, en);
            }
            Some(int_type) => {
                // Program sensitivity while the pin is (still) disabled, then
                // clear any status latched under the previous sensitivity so a
                // stale event cannot fire the moment the enable bit is set.
                self.write_sens_bits(dev.bank, bit, int_type);
                self.clear_interrupt_status(dev.bank, bit);
                let en = self.int_en_read(dev.bank) | bit;
                self.int_en_write(dev.bank, en);
            }
        }

        Ok(())
    }

    /// Program a pin's interrupt sensitivity only, without touching the enable
    /// bit. Mirrors the EarlGrey `irq_configure` semantics where sensitivity
    /// and enable are controlled independently.
    pub fn configure_interrupt_sensitivity(
        &self,
        dev: &BankDevice,
        pin: u8,
        mode: InterruptMode,
        trig: InterruptTrigger,
    ) -> Result<(), Error> {
        dev.validate_pin(pin)?;
        // Disabled has no sensitivity to program; treat as code 0 (falling edge)
        // which is inert while the enable bit stays clear.
        let int_type = Self::interrupt_sens_type(mode, trig)?.unwrap_or(0);
        self.write_sens_bits(dev.bank, 1u32 << pin, int_type);
        Ok(())
    }

    /// Program interrupt sensitivity for every pin in `mask` in one pass
    /// (three register read-modify-writes total, instead of three per pin).
    /// Like [`Self::configure_interrupt_sensitivity`], the enable bits are not
    /// changed.
    pub fn configure_interrupt_sensitivity_masked(
        &self,
        dev: &BankDevice,
        mask: u32,
        mode: InterruptMode,
        trig: InterruptTrigger,
    ) -> Result<(), Error> {
        dev.validate_mask(mask)?;
        let int_type = Self::interrupt_sens_type(mode, trig)?.unwrap_or(0);
        if mask != 0 {
            self.write_sens_bits(dev.bank, mask, int_type);
        }
        Ok(())
    }

    /// Set the interrupt-enable bits for `mask` on a bank (OR into `int_en`).
    pub fn set_interrupt_enable(&self, bank: Bank, mask: u32) {
        let en = self.int_en_read(bank) | mask;
        self.int_en_write(bank, en);
    }

    /// Clear the interrupt-enable bits for `mask` on a bank.
    pub fn clear_interrupt_enable(&self, bank: Bank, mask: u32) {
        let en = self.int_en_read(bank) & !mask;
        self.int_en_write(bank, en);
    }

    /// Read the latched interrupt status register for a bank.
    #[must_use]
    pub fn interrupt_status(&self, bank: Bank) -> u32 {
        match bank {
            Bank::Ad => self.regs().gpio514().read().bits(),
            Bank::Eh => self.regs().gpio530().read().bits(),
            Bank::Il => self.regs().gpio54c().read().bits(),
            Bank::Mp => self.regs().gpio5a4().read().bits(),
        }
    }

    /// Acknowledge (clear) interrupt status bits for a bank.
    pub fn clear_interrupt_status(&self, bank: Bank, mask: u32) {
        match bank {
            Bank::Ad => self.regs().gpio514().write(|w| unsafe { w.bits(mask) }),
            Bank::Eh => self.regs().gpio530().write(|w| unsafe { w.bits(mask) }),
            Bank::Il => self.regs().gpio54c().write(|w| unsafe { w.bits(mask) }),
            Bank::Mp => self.regs().gpio5a4().write(|w| unsafe { w.bits(mask) }),
        };
    }

    pub fn passthrough_masked(&self, bank: Bank, mask: u32) {
        let sampled = self.port_get_raw(bank);
        self.port_set_masked_raw(bank, mask, sampled);
    }

    fn port_write_raw(&self, bank: Bank, value: u32) {
        match bank {
            Bank::Ad => self.regs().gpio500().write(|w| unsafe { w.bits(value) }),
            Bank::Eh => self.regs().gpio51c().write(|w| unsafe { w.bits(value) }),
            Bank::Il => self.regs().gpio538().write(|w| unsafe { w.bits(value) }),
            Bank::Mp => self.regs().gpio590().write(|w| unsafe { w.bits(value) }),
        };
    }

    fn int_en_read(&self, bank: Bank) -> u32 {
        match bank {
            Bank::Ad => self.regs().gpio504().read().bits(),
            Bank::Eh => self.regs().gpio520().read().bits(),
            Bank::Il => self.regs().gpio53c().read().bits(),
            Bank::Mp => self.regs().gpio594().read().bits(),
        }
    }

    fn int_en_write(&self, bank: Bank, value: u32) {
        match bank {
            Bank::Ad => self.regs().gpio504().write(|w| unsafe { w.bits(value) }),
            Bank::Eh => self.regs().gpio520().write(|w| unsafe { w.bits(value) }),
            Bank::Il => self.regs().gpio53c().write(|w| unsafe { w.bits(value) }),
            Bank::Mp => self.regs().gpio594().write(|w| unsafe { w.bits(value) }),
        };
    }

    fn int_sens_read(&self, bank: Bank, index: u8) -> u32 {
        match (bank, index) {
            (Bank::Ad, 0) => self.regs().gpio508().read().bits(),
            (Bank::Ad, 1) => self.regs().gpio50c().read().bits(),
            (Bank::Ad, 2) => self.regs().gpio510().read().bits(),
            (Bank::Eh, 0) => self.regs().gpio524().read().bits(),
            (Bank::Eh, 1) => self.regs().gpio528().read().bits(),
            (Bank::Eh, 2) => self.regs().gpio52c().read().bits(),
            (Bank::Il, 0) => self.regs().gpio540().read().bits(),
            (Bank::Il, 1) => self.regs().gpio544().read().bits(),
            (Bank::Il, 2) => self.regs().gpio548().read().bits(),
            (Bank::Mp, 0) => self.regs().gpio598().read().bits(),
            (Bank::Mp, 1) => self.regs().gpio59c().read().bits(),
            (Bank::Mp, 2) => self.regs().gpio5a0().read().bits(),
            _ => 0,
        }
    }

    fn int_sens_write(&self, bank: Bank, index: u8, value: u32) {
        match (bank, index) {
            (Bank::Ad, 0) => {
                self.regs().gpio508().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Ad, 1) => {
                self.regs().gpio50c().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Ad, 2) => {
                self.regs().gpio510().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Eh, 0) => {
                self.regs().gpio524().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Eh, 1) => {
                self.regs().gpio528().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Eh, 2) => {
                self.regs().gpio52c().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Il, 0) => {
                self.regs().gpio540().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Il, 1) => {
                self.regs().gpio544().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Il, 2) => {
                self.regs().gpio548().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Mp, 0) => {
                self.regs().gpio598().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Mp, 1) => {
                self.regs().gpio59c().write(|w| unsafe { w.bits(value) });
            }
            (Bank::Mp, 2) => {
                self.regs().gpio5a0().write(|w| unsafe { w.bits(value) });
            }
            _ => {}
        };
    }
}
