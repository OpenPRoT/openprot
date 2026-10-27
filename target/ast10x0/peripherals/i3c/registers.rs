// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Confined-`unsafe` MMIO façade over the per-bus I3C register blocks.
//!
//! One driver manages multiple bus instances: the bus is selected at
//! **runtime** by index (no per-instance type parameter), mirroring the
//! reference `aspeed-rust` driver. Following the `SmcRegisters` precedent,
//! **all register operations go through this single point**: the rest of the
//! driver (`hardware.rs` upward) never touches PAC types or MMIO `unsafe` —
//! it calls the intent-named methods below.
//!
//! Method naming follows the AST1060 PAC convention where an operation maps
//! to one register (`read_reset_ctrl` ↔ `i3cd034`), and the datasheet's
//! vocabulary where an operation is a multi-field sequence
//! (`enable_controller_primary`, `assert_all_queue_resets`).

use core::marker::PhantomData;

use super::constants::MAX_BUSES;

// -----------------------------------------------------------------------------
// Per-bus register dispatch (private)
// -----------------------------------------------------------------------------
//
// The I3C-global block packs one reg0/reg1 pair per bus, and the DAT is eight
// identically-shaped registers; the PAC gives each its own accessor, so these
// macros do the bus/position match once, here, inside the façade. The `_`
// arms are unreachable: `I3cRegisters::new` validates `bus`, and DAT
// positions are bounded by `dat_depth()` — kept panic-free regardless.

macro_rules! i3cg_reg0 {
    ($self:expr, $($ops:tt)*) => {{
        match $self.bus {
            0 => $self.i3cg().i3c010().$($ops)*,
            1 => $self.i3cg().i3c020().$($ops)*,
            2 => $self.i3cg().i3c030().$($ops)*,
            _ => $self.i3cg().i3c040().$($ops)*,
        }
    }};
}

macro_rules! i3cg_reg1 {
    ($self:expr, $($ops:tt)*) => {{
        match $self.bus {
            0 => $self.i3cg().i3c014().$($ops)*,
            1 => $self.i3cg().i3c024().$($ops)*,
            2 => $self.i3cg().i3c034().$($ops)*,
            _ => $self.i3cg().i3c044().$($ops)*,
        }
    }};
}

macro_rules! dat_reg {
    ($self:expr, $pos:expr, $($ops:tt)*) => {{
        match $pos {
            0 => $self.i3c().i3cd280().$($ops)*,
            1 => $self.i3c().i3cd284().$($ops)*,
            2 => $self.i3c().i3cd288().$($ops)*,
            3 => $self.i3c().i3cd28c().$($ops)*,
            4 => $self.i3c().i3cd290().$($ops)*,
            5 => $self.i3c().i3cd294().$($ops)*,
            6 => $self.i3c().i3cd298().$($ops)*,
            _ => $self.i3c().i3cd29c().$($ops)*,
        }
    }};
}

/// Safe wrapper around the I3C / I3C-global / SCU hardware registers of one bus.
///
/// This struct consolidates all unsafe I3C MMIO access. All register
/// operations go through this single point, making it easy to audit safety
/// invariants — the same shape as `SmcRegisters` in `smc/registers.rs`.
///
/// Not `Copy`/`Clone`: an `I3cRegisters` represents exclusive ownership of
/// one bus's register blocks.
pub struct I3cRegisters {
    i3c: *const ast1060_pac::i3c::RegisterBlock,
    i3cg: *const ast1060_pac::i3cglobal::RegisterBlock,
    scu: *const ast1060_pac::scu::RegisterBlock,
    bus: u8,
    // `*const ()` marker keeps the handle `!Send` and `!Sync`. An
    // `I3cRegisters` represents exclusive ownership of one bus's register
    // blocks; it must not be shared between threads or moved into another
    // execution context where it could alias the controller it owns.
    _not_send_sync: PhantomData<*const ()>,
}

impl I3cRegisters {
    /// Create the register façade for `bus` (0..[`MAX_BUSES`]).
    ///
    /// Returns `None` if `bus` is out of range — every accessor below is
    /// therefore panic-free: a constructed façade always holds valid pointers
    /// and an in-range bus index.
    ///
    /// `pub(crate)`: the single *public* unsafe gate is
    /// [`Ast1060I3c::new`](super::hardware::Ast1060I3c::new), which forwards
    /// this contract; external callers cannot construct a second façade for
    /// a bus behind the driver's back.
    ///
    /// # Safety
    ///
    /// This is the entire `unsafe` perimeter for I3C MMIO (Delta D3):
    /// - The AST1060 PAC singleton pointers (`I3c*::ptr()`,
    ///   `I3cglobal::ptr()`, `Scu::ptr()`) must point to valid register
    ///   blocks for the program's lifetime (they do on AST1060 hardware).
    /// - Access through the returned façade must be serialized by the caller
    ///   (the type is `!Sync`); only one owner per physical bus may be
    ///   active at a time.
    #[must_use]
    pub(crate) const unsafe fn new(bus: u8) -> Option<Self> {
        let i3c = match bus {
            0 => ast1060_pac::I3c::ptr(),
            1 => ast1060_pac::I3c1::ptr(),
            2 => ast1060_pac::I3c2::ptr(),
            3 => ast1060_pac::I3c3::ptr(),
            _ => return None,
        };
        // Redundant with the match above, but keeps the invariant explicit if
        // MAX_BUSES and the match ever diverge.
        if bus as usize >= MAX_BUSES {
            return None;
        }
        Some(Self {
            i3c,
            i3cg: ast1060_pac::I3cglobal::ptr(),
            scu: ast1060_pac::Scu::ptr(),
            bus,
            _not_send_sync: PhantomData,
        })
    }

    /// Bus index this façade was constructed for (always `< MAX_BUSES`).
    #[inline]
    #[must_use]
    pub fn bus(&self) -> u8 {
        self.bus
    }

    /// Second handle over the same bus for the **ISR side**.
    ///
    /// This is the one sanctioned exception to "one `I3cRegisters` per bus":
    /// the interrupt service routine cannot borrow the thread-owned handle,
    /// so it gets its own. No Rust memory is aliased (the pointers target
    /// MMIO, not Rust objects).
    ///
    /// # Safety
    ///
    /// Device access through the alias must remain serialized with the
    /// owning handle — on this single-core target that holds because the ISR
    /// runs atomically with respect to the thread.
    pub(crate) unsafe fn isr_alias(&self) -> Self {
        Self {
            i3c: self.i3c,
            i3cg: self.i3cg,
            scu: self.scu,
            bus: self.bus,
            _not_send_sync: PhantomData,
        }
    }

    // -------------------------------------------------------------------------
    // Interior deref helpers — the only repeated `unsafe`
    // -------------------------------------------------------------------------

    /// The only repeated interior `unsafe` for the I3C block.
    ///
    /// Returns a `'static` reference: the constructor's contract guarantees
    /// the pointer is valid for the program lifetime.
    #[inline]
    fn i3c(&self) -> &'static ast1060_pac::i3c::RegisterBlock {
        // SAFETY: `new` guarantees a valid pointer for the program lifetime;
        // access is serialized by the caller (the type is `!Sync`).
        unsafe { &*self.i3c }
    }

    /// The only repeated interior `unsafe` for the I3C-global block. See
    /// [`i3c`](Self::i3c).
    #[inline]
    fn i3cg(&self) -> &'static ast1060_pac::i3cglobal::RegisterBlock {
        // SAFETY: see `i3c`.
        unsafe { &*self.i3cg }
    }

    /// The only repeated interior `unsafe` for the SCU block. See
    /// [`i3c`](Self::i3c).
    #[inline]
    fn scu(&self) -> &'static ast1060_pac::scu::RegisterBlock {
        // SAFETY: see `i3c`.
        unsafe { &*self.scu }
    }

    // -------------------------------------------------------------------------
    // SCU: per-bus reset and clock
    // -------------------------------------------------------------------------

    /// SCU050: assert this bus's controller reset.
    pub(crate) fn core_reset_assert(&self) {
        match self.bus {
            0 => self
                .scu()
                .scu050()
                .modify(|_, w| w.rst_i3c0ctrl().set_bit()),
            1 => self
                .scu()
                .scu050()
                .modify(|_, w| w.rst_i3c1ctrl().set_bit()),
            2 => self
                .scu()
                .scu050()
                .modify(|_, w| w.rst_i3c2ctrl().set_bit()),
            _ => self
                .scu()
                .scu050()
                .modify(|_, w| w.rst_i3c3ctrl().set_bit()),
        };
    }

    /// SCU054: deassert this bus's controller reset (write-1-to-clear).
    pub(crate) fn core_reset_deassert(&self) {
        let mask = 1u32 << (8 + u32::from(self.bus));
        self.scu()
            .scu054()
            .modify(|_, w| unsafe { w.scu050sys_rst_ctrl_clear_reg2().bits(mask) });
    }

    /// SCU050: assert the shared I3C register/DMA reset.
    #[allow(dead_code)]
    pub(crate) fn global_reset_assert(&self) {
        self.scu()
            .scu050()
            .modify(|_, w| w.rst_i3cregdmactrl().set_bit());
    }

    /// SCU054: deassert the shared I3C register/DMA reset (write-1-to-clear).
    pub(crate) fn global_reset_deassert(&self, mask: u32) {
        self.scu()
            .scu054()
            .modify(|_, w| unsafe { w.scu050sys_rst_ctrl_clear_reg2().bits(mask) });
    }

    /// SCU094: ungate this bus's clock (write-1-to-clear stop bit).
    pub(crate) fn clock_on(&self) {
        let mask = 1u32 << (8 + u32::from(self.bus));
        self.scu()
            .scu094()
            .modify(|_, w| unsafe { w.scu090clk_stop_ctrl_clear_reg_set2().bits(mask) });
    }

    /// SCU004: hardware revision ID.
    pub(crate) fn hw_rev_id(&self) -> u32 {
        self.scu().scu004().read().hw_rev_id().bits().into()
    }

    // -------------------------------------------------------------------------
    // I3C-global: this bus's reg0/reg1 pair
    // -------------------------------------------------------------------------

    /// I3CG reg0: raw read.
    pub(crate) fn i3cg_read_reg0(&self) -> u32 {
        i3cg_reg0!(self, read().bits())
    }

    /// I3CG reg0: raw write.
    pub(crate) fn i3cg_write_reg0(&self, val: u32) {
        i3cg_reg0!(self, write(|w| unsafe { w.bits(val) }));
    }

    /// I3CG reg1: raw read.
    pub(crate) fn i3cg_read_reg1(&self) -> u32 {
        i3cg_reg1!(self, read().bits())
    }

    /// I3CG reg1: program act-mode 1, instance id = bus, and the bring-up
    /// static address.
    pub(crate) fn i3cg_program_reg1(&self, static_addr: u8) {
        let bus = self.bus;
        i3cg_reg1!(
            self,
            write(|w| unsafe {
                w.actmode()
                    .bits(1)
                    .instid()
                    .bits(bus)
                    .staticaddr()
                    .bits(static_addr)
            })
        );
    }

    /// I3CG reg1: read-modify-write setting `mask` bits.
    pub(crate) fn i3cg_reg1_set_bits(&self, mask: u32) {
        i3cg_reg1!(self, modify(|r, w| unsafe { w.bits(r.bits() | mask) }));
    }

    /// I3CG reg1: read-modify-write clearing `mask` bits.
    pub(crate) fn i3cg_reg1_clear_bits(&self, mask: u32) {
        i3cg_reg1!(self, modify(|r, w| unsafe { w.bits(r.bits() & !mask) }));
    }

    /// I3CG reg1: absolute write via read-modify-write (preserves the
    /// reference's RMW bus access pattern).
    pub(crate) fn i3cg_reg1_overwrite(&self, val: u32) {
        i3cg_reg1!(self, modify(|_r, w| unsafe { w.bits(val) }));
    }

    // -------------------------------------------------------------------------
    // I3CD000: device control
    // -------------------------------------------------------------------------

    /// I3CD000: set/clear automatic hot-join NACK.
    pub(crate) fn set_hot_join_nack(&self, on: bool) {
        self.i3c().i3cd000().modify(|_, w| {
            if on {
                w.hot_join_ack_nack_ctrl().set_bit()
            } else {
                w.hot_join_ack_nack_ctrl().clear_bit()
            }
        });
    }

    /// I3CD000: is the controller enable bit set?
    pub(crate) fn controller_enabled(&self) -> bool {
        self.i3c().i3cd000().read().enbl_i3cctrl().bit_is_set()
    }

    /// I3CD000: clear the controller enable bit.
    pub(crate) fn disable_controller(&self) {
        self.i3c()
            .i3cd000()
            .modify(|_, w| w.enbl_i3cctrl().clear_bit());
    }

    /// I3CD000: enable in primary (master) role — include broadcast address.
    pub(crate) fn enable_controller_primary(&self) {
        self.i3c().i3cd000().modify(|_, w| {
            w.i3cbroadcast_addr_include()
                .set_bit()
                .enbl_i3cctrl()
                .set_bit()
        });
    }

    /// I3CD000: enable in secondary (target) role — IBI payload on, I2C/I3C
    /// mode adaption off.
    pub(crate) fn enable_controller_secondary(&self) {
        self.i3c().i3cd000().modify(|_, w| {
            w.enbl_adaption_of_i2ci3cmode()
                .clear_bit()
                .ibipayloaden()
                .set_bit()
                .enbl_i3cctrl()
                .set_bit()
        });
    }

    /// I3CD000: resume from a halted transfer state.
    pub(crate) fn resume(&self) {
        self.i3c().i3cd000().modify(|_, w| w.i3cresume().set_bit());
    }

    /// I3CD000: abort the current transfer (software halt request).
    pub(crate) fn abort(&self) {
        self.i3c().i3cd000().modify(|_, w| w.i3cabort().set_bit());
    }

    /// I3CD000: program the IBI mandatory data byte.
    pub(crate) fn set_ibi_mdb(&self, mdb: u8) {
        self.i3c()
            .i3cd000()
            .modify(|_, w| unsafe { w.mdb().bits(mdb) });
    }

    // -------------------------------------------------------------------------
    // I3CD004 / I3CD008: device address & capability
    // -------------------------------------------------------------------------

    /// I3CD004: program the secondary-role static address (and mark valid).
    pub(crate) fn program_secondary_static_addr(&self, addr: u8) {
        self.i3c()
            .i3cd004()
            .write(|w| unsafe { w.dev_static_addr().bits(addr).static_addr_valid().set_bit() });
    }

    /// I3CD004: program the primary-role dynamic address (and mark valid).
    pub(crate) fn program_primary_dynamic_addr(&self, addr: u8) {
        self.i3c().i3cd004().write(|w| unsafe {
            w.dev_dynamic_addr()
                .bits(addr)
                .dynamic_addr_valid()
                .set_bit()
        });
    }

    /// I3CD004: currently assigned dynamic address.
    pub(crate) fn dynamic_addr(&self) -> u8 {
        self.i3c().i3cd004().read().dev_dynamic_addr().bits()
    }

    /// I3CD004: is the dynamic address valid?
    pub(crate) fn dynamic_addr_valid(&self) -> bool {
        self.i3c().i3cd004().read().dynamic_addr_valid().bit()
    }

    /// I3CD008: does the device advertise hot-join capability?
    pub(crate) fn hj_capable(&self) -> bool {
        self.i3c().i3cd008().read().slvhjcap().bit()
    }

    // -------------------------------------------------------------------------
    // Command / response / data ports
    // -------------------------------------------------------------------------

    /// I3CD00C: push one word into the command queue.
    pub(crate) fn push_cmd(&self, val: u32) {
        self.i3c().i3cd00c().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD010: pop one word from the response queue.
    pub(crate) fn pop_response(&self) -> u32 {
        self.i3c().i3cd010().read().bits()
    }

    /// I3CD014: write `bytes` into the TX FIFO (LE words; tail zero-padded).
    pub(crate) fn tx_fifo_write(&self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(4);
        for chunk in &mut chunks {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            self.i3c()
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }

        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut tmp = [0u8; 4];
            tmp[..rem.len()].copy_from_slice(rem);
            let word = u32::from_le_bytes(tmp);
            self.i3c()
                .i3cd014()
                .write(|w| unsafe { w.tx_data_port().bits(word) });
        }
    }

    /// I3CD014: read `out.len()` bytes from the RX FIFO (LE words).
    pub(crate) fn rx_fifo_read(&self, out: &mut [u8]) {
        Self::fifo_read(|| self.i3c().i3cd014().read().rx_data_port().bits(), out);
    }

    /// I3CD014: discard `len` bytes from the RX FIFO.
    pub(crate) fn rx_fifo_drain(&self, len: usize) {
        Self::fifo_drain(|| self.i3c().i3cd014().read().rx_data_port().bits(), len);
    }

    /// I3CD018: pop one word from the IBI queue.
    pub(crate) fn ibi_fifo_pop(&self) -> u32 {
        self.i3c().i3cd018().read().bits()
    }

    /// I3CD018: read `out.len()` bytes from the IBI queue (LE words).
    pub(crate) fn ibi_fifo_read(&self, out: &mut [u8]) {
        Self::fifo_read(|| self.i3c().i3cd018().read().bits(), out);
    }

    /// I3CD018: discard `len` bytes from the IBI queue.
    pub(crate) fn ibi_fifo_drain(&self, len: usize) {
        Self::fifo_drain(|| self.i3c().i3cd018().read().bits(), len);
    }

    /// Word-at-a-time FIFO scatter helper (shared by RX/IBI reads).
    fn fifo_read<F: FnMut() -> u32>(mut read_word: F, out: &mut [u8]) {
        let mut chunks = out.chunks_exact_mut(4);
        for chunk in &mut chunks {
            let val = read_word();
            chunk.copy_from_slice(&val.to_le_bytes());
        }

        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let val = read_word();
            let bytes = val.to_le_bytes();
            rem.copy_from_slice(&bytes[..rem.len()]);
        }
    }

    /// Word-at-a-time FIFO discard helper (shared by RX/IBI drains).
    fn fifo_drain<F: FnMut() -> u32>(mut read_word: F, len: usize) {
        let nwords = (len + 3) >> 2;
        for _ in 0..nwords {
            read_word();
        }
    }

    // -------------------------------------------------------------------------
    // Queue thresholds
    // -------------------------------------------------------------------------

    /// I3CD01C: program the IBI data threshold.
    pub(crate) fn set_ibi_data_threshold(&self, val: u8) {
        self.i3c()
            .i3cd01c()
            .write(|w| unsafe { w.ibidata_threshold_value().bits(val) });
    }

    /// I3CD01C: program the response-buffer threshold.
    pub(crate) fn set_resp_buf_threshold(&self, val: u8) {
        self.i3c()
            .i3cd01c()
            .modify(|_, w| unsafe { w.response_buffer_threshold_value().bits(val) });
    }

    /// I3CD020: program the RX-buffer threshold.
    pub(crate) fn set_rx_buf_threshold(&self, val: u8) {
        self.i3c()
            .i3cd020()
            .modify(|_, w| unsafe { w.rx_buffer_threshold_value().bits(val) });
    }

    // -------------------------------------------------------------------------
    // IBI MR/SIR reject masks
    // -------------------------------------------------------------------------

    /// I3CD02C: write the master-request reject mask.
    pub(crate) fn write_mr_reject(&self, val: u32) {
        self.i3c().i3cd02c().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD030: read the SIR reject mask.
    pub(crate) fn read_sir_reject(&self) -> u32 {
        self.i3c().i3cd030().read().bits()
    }

    /// I3CD030: write the SIR reject mask.
    pub(crate) fn write_sir_reject(&self, val: u32) {
        self.i3c().i3cd030().write(|w| unsafe { w.bits(val) });
    }

    // -------------------------------------------------------------------------
    // I3CD034: reset control
    // -------------------------------------------------------------------------

    /// I3CD034: assert every queue/buffer/core software reset at once.
    pub(crate) fn assert_all_queue_resets(&self) {
        self.i3c().i3cd034().write(|w| {
            w.ibiqueue_sw_rst()
                .set_bit()
                .rx_buffer_sw_rst()
                .set_bit()
                .tx_buffer_sw_rst()
                .set_bit()
                .response_queue_sw_rst()
                .set_bit()
                .cmd_queue_sw_rst()
                .set_bit()
                .core_sw_rst()
                .set_bit()
        });
    }

    /// I3CD034: raw write (selected reset bits).
    pub(crate) fn write_reset_ctrl(&self, val: u32) {
        self.i3c().i3cd034().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD034: raw read (0 once all resets have self-cleared).
    pub(crate) fn read_reset_ctrl(&self) -> u32 {
        self.i3c().i3cd034().read().bits()
    }

    // -------------------------------------------------------------------------
    // I3CD038: slave event control
    // -------------------------------------------------------------------------

    /// I3CD038: raw read.
    pub(crate) fn read_slv_event_ctrl(&self) -> u32 {
        self.i3c().i3cd038().read().bits()
    }

    /// I3CD038: raw write.
    pub(crate) fn write_slv_event_ctrl(&self, val: u32) {
        self.i3c().i3cd038().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD038: request a hot-join (secondary role, no dynamic address yet).
    pub(crate) fn raise_hot_join_request(&self) {
        self.write_slv_event_ctrl(super::constants::SLV_EVENT_CTRL_HJ_REQ);
    }

    // -------------------------------------------------------------------------
    // Interrupt status / enables
    // -------------------------------------------------------------------------

    /// I3CD03C: read the interrupt status.
    pub(crate) fn read_intr_status(&self) -> u32 {
        self.i3c().i3cd03c().read().bits()
    }

    /// I3CD03C: clear the given interrupt-status bits (write-1-to-clear).
    pub(crate) fn clear_intr_status(&self, mask: u32) {
        self.i3c().i3cd03c().write(|w| unsafe { w.bits(mask) });
    }

    /// I3CD040/I3CD044: enable the primary-role interrupt set
    /// (transfer error + response ready), status and signal.
    pub(crate) fn enable_master_irqs(&self) {
        self.i3c().i3cd040().write(|w| {
            w.transfererrstaten()
                .set_bit()
                .respreadystatintren()
                .set_bit()
        });

        self.i3c().i3cd044().write(|w| {
            w.transfererrsignalen()
                .set_bit()
                .respreadysignalintren()
                .set_bit()
        });
    }

    /// I3CD040/I3CD044: enable the secondary-role interrupt set
    /// (transfer error, response ready, CCC update, DA assignment, IBI
    /// update, read request), status and signal.
    pub(crate) fn enable_target_irqs(&self) {
        self.i3c().i3cd040().write(|w| {
            w.transfererrstaten()
                .set_bit()
                .respreadystatintren()
                .set_bit()
                .cccupdatedstaten()
                .set_bit()
                .dynaddrassgnstaten()
                .set_bit()
                .ibiupdatedstaten()
                .set_bit()
                .readreqrecvstaten()
                .set_bit()
        });

        self.i3c().i3cd044().write(|w| {
            w.transfererrsignalen()
                .set_bit()
                .respreadysignalintren()
                .set_bit()
                .cccupdatedsignalen()
                .set_bit()
                .dynaddrassgnsignalen()
                .set_bit()
                .ibiupdatedsignalen()
                .set_bit()
                .readreqrecvsignalen()
                .set_bit()
        });
    }

    /// I3CD040/I3CD044: additionally enable the IBI threshold interrupt,
    /// status and signal.
    pub(crate) fn enable_ibi_thld_irq(&self) {
        self.i3c()
            .i3cd040()
            .modify(|_, w| w.ibithldstaten().set_bit());
        self.i3c()
            .i3cd044()
            .modify(|_, w| w.ibithldsignalen().set_bit());
    }

    /// I3CD040/I3CD044: mask the master transfer-completion interrupt sources
    /// (response ready + transfer error), status and signal.
    ///
    /// Called by the ISR (flag-and-defer): the response queue stays non-empty
    /// until the polling thread drains it, so the level-style status would
    /// refire the line forever if left enabled. The thread re-enables via
    /// [`unmask_master_xfer_irqs`](Self::unmask_master_xfer_irqs) after
    /// draining. `modify` (not `write`) so the IBI-threshold enables are
    /// untouched.
    pub(crate) fn mask_master_xfer_irqs(&self) {
        self.i3c().i3cd040().modify(|_, w| {
            w.transfererrstaten()
                .clear_bit()
                .respreadystatintren()
                .clear_bit()
        });
        self.i3c().i3cd044().modify(|_, w| {
            w.transfererrsignalen()
                .clear_bit()
                .respreadysignalintren()
                .clear_bit()
        });
    }

    /// I3CD040/I3CD044: re-enable the master transfer-completion interrupt
    /// sources after the thread has drained the response queue. Counterpart
    /// of [`mask_master_xfer_irqs`](Self::mask_master_xfer_irqs).
    pub(crate) fn unmask_master_xfer_irqs(&self) {
        self.i3c().i3cd040().modify(|_, w| {
            w.transfererrstaten()
                .set_bit()
                .respreadystatintren()
                .set_bit()
        });
        self.i3c().i3cd044().modify(|_, w| {
            w.transfererrsignalen()
                .set_bit()
                .respreadysignalintren()
                .set_bit()
        });
    }

    /// I3CD040: read the interrupt status-enable mask (debug).
    #[allow(dead_code)]
    pub(crate) fn read_intr_status_en(&self) -> u32 {
        self.i3c().i3cd040().read().bits()
    }

    /// I3CD044: read the interrupt signal-enable mask (debug).
    #[allow(dead_code)]
    pub(crate) fn read_intr_signal_en(&self) -> u32 {
        self.i3c().i3cd044().read().bits()
    }

    // -------------------------------------------------------------------------
    // Queue / transfer status
    // -------------------------------------------------------------------------

    /// I3CD04C: number of entries in the response buffer.
    pub(crate) fn resp_buf_level(&self) -> usize {
        self.i3c().i3cd04c().read().respbufblr().bits() as usize
    }

    /// I3CD04C: number of pending IBI status entries.
    pub(crate) fn ibi_status_count(&self) -> u8 {
        self.i3c().i3cd04c().read().ibistatuscnt().bits()
    }

    /// I3CD054: current transfer state machine status.
    pub(crate) fn xfer_status(&self) -> u8 {
        self.i3c().i3cd054().read().cmtfrstatus().bits()
    }

    /// I3CD05C: device address table depth.
    pub(crate) fn dat_depth(&self) -> u16 {
        self.i3c().i3cd05c().read().devaddrtabledepth().bits()
    }

    // -------------------------------------------------------------------------
    // PID / characteristics
    // -------------------------------------------------------------------------

    /// I3CD070: program the MIPI manufacturer ID (and select PID[31:0] as
    /// the instance value, not DCR).
    pub(crate) fn set_pid_mfg_id(&self, id: u16) {
        self.i3c()
            .i3cd070()
            .write(|w| unsafe { w.slvmipimfgid().bits(id).slvpiddcr().clear_bit() });
    }

    /// I3CD074: program the PID instance value word.
    pub(crate) fn write_slv_pid_value(&self, val: u32) {
        self.i3c().i3cd074().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD078: read the slave characteristics register.
    pub(crate) fn read_slv_char_ctrl(&self) -> u32 {
        self.i3c().i3cd078().read().bits()
    }

    /// I3CD07C: read the max write/read length register (MRL in bits 31:16,
    /// MWL in bits 15:0; updated by the bus master via SETMRL/SETMWL).
    pub(crate) fn read_slv_max_len(&self) -> u32 {
        self.i3c().i3cd07c().read().bits()
    }

    /// I3CD078: write the slave characteristics register.
    pub(crate) fn write_slv_char_ctrl(&self, val: u32) {
        self.i3c().i3cd078().write(|w| unsafe { w.bits(val) });
    }

    // -------------------------------------------------------------------------
    // Misc control
    // -------------------------------------------------------------------------

    /// I3CD08C: raise a slave interrupt request (SIR).
    pub(crate) fn raise_sir(&self) {
        self.i3c().i3cd08c().write(|w| w.sir().set_bit());
    }

    /// I3CD0B0: program the device operation mode (0 = master, 1 = slave).
    pub(crate) fn set_dev_op_mode(&self, mode: u8) {
        self.i3c()
            .i3cd0b0()
            .modify(|_, w| unsafe { w.dev_op_mode().bits(mode) });
    }

    // -------------------------------------------------------------------------
    // Clock / timing
    // -------------------------------------------------------------------------

    /// I3CD0BC: program the I2C Fast-mode SCL high/low counts.
    pub(crate) fn set_i2c_fm_timing(&self, hi: u16, lo: u16) {
        self.i3c()
            .i3cd0bc()
            .write(|w| unsafe { w.i2cfmhcnt().bits(hi).i2cfmlcnt().bits(lo) });
    }

    /// I3CD0C0: program the I2C Fast-mode-Plus SCL high/low counts.
    pub(crate) fn set_i2c_fmp_timing(&self, hi: u8, lo: u16) {
        self.i3c()
            .i3cd0c0()
            .write(|w| unsafe { w.i2cfmphcnt().bits(hi).i2cfmplcnt().bits(lo) });
    }

    /// I3CD0B4: program the I3C open-drain SCL high/low counts.
    pub(crate) fn set_od_timing(&self, hi: u8, lo: u8) {
        self.i3c()
            .i3cd0b4()
            .write(|w| unsafe { w.i3codhcnt().bits(hi).i3codlcnt().bits(lo) });
    }

    /// I3CD0B8: program the I3C push-pull SCL high/low counts.
    pub(crate) fn set_pp_timing(&self, hi: u8, lo: u8) {
        self.i3c()
            .i3cd0b8()
            .write(|w| unsafe { w.i3cpphcnt().bits(hi).i3cpplcnt().bits(lo) });
    }

    /// I3CD0D0: read the SDA hold/debounce register.
    pub(crate) fn read_sda_hold(&self) -> u32 {
        self.i3c().i3cd0d0().read().bits()
    }

    /// I3CD0D0: write the SDA hold/debounce register.
    pub(crate) fn write_sda_hold(&self, val: u32) {
        self.i3c().i3cd0d0().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD0D4: program the bus-free timing register.
    pub(crate) fn write_bus_free_timing(&self, val: u32) {
        self.i3c().i3cd0d4().write(|w| unsafe { w.bits(val) });
    }

    /// I3CD0D4: IBI-free wait window in core-clock cycles.
    pub(crate) fn ibi_free_cycles(&self) -> u32 {
        self.i3c().i3cd0d4().read().i3cibifree().bits().into()
    }

    // -------------------------------------------------------------------------
    // Device address table (I3CD280..I3CD29C)
    // -------------------------------------------------------------------------

    /// DAT[pos]: raw read.
    pub(crate) fn dat_read(&self, pos: usize) -> u32 {
        dat_reg!(self, pos, read().bits())
    }

    /// DAT[pos]: raw write.
    pub(crate) fn dat_write_raw(&self, pos: usize, val: u32) {
        dat_reg!(self, pos, write(|w| unsafe { w.bits(val) }));
    }

    /// DAT[pos]: reject SIR and master requests (detached/idle slot).
    pub(crate) fn dat_set_reject(&self, pos: usize) {
        dat_reg!(
            self,
            pos,
            write(|w| w.sirreject().set_bit().mrreject().set_bit())
        );
    }

    /// DAT[pos]: program a device's dynamic address (with parity bit) while
    /// keeping SIR/MR rejected until IBIs are explicitly enabled.
    pub(crate) fn dat_program_addr(&self, pos: usize, addr_with_parity: u8) {
        dat_reg!(
            self,
            pos,
            write(|w| unsafe {
                w.sirreject()
                    .set_bit()
                    .mrreject()
                    .set_bit()
                    .devdynamicaddr()
                    .bits(addr_with_parity)
            })
        );
    }
}
