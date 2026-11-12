// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Caliptra-SS I3C target peripheral driver.
//!
//! Drives the i3c-core HCI TTI (Target Transaction Interface) on behalf of
//! firmware running on the VeeR RISC-V core.  The i3c-core is always a
//! *target* (secondary) from the VeeR perspective — a BMC controller on the
//! I3C bus initiates all transfers.
//!
//! The Caliptra ROM initializes the I3C core before handing off to application
//! firmware.  This driver assumes the ROM has already run and only manages the
//! data path (interrupt enable/disable, RX drain, TX queue, IBI).
//!
//! # TTI data path
//!
//! - **Incoming write** (controller → target): hardware pushes a descriptor
//!   into `tti_rx_desc_queue_port` then `data_length` words into
//!   `tti_rx_data_port`.  Poll `rx_pending()` or enable the RX interrupt.
//! - **Outgoing read** (target → controller): firmware writes a descriptor
//!   to `tti_tx_desc_queue_port` then `data_length` words to
//!   `tti_tx_data_port`.
//! - **IBI**: write the IBI descriptor (MDB + payload length) to
//!   `tti_tti_ibi_port` then the payload words; hardware raises the IBI on
//!   the bus.

#![no_std]

use caliptra_ss_registers::i3c;
use i3c::bits::{InterruptEnable, InterruptStatus, Status, StbyCrDeviceAddr};

use core::marker::PhantomData;
// I3C register block base for the selected target type, from upstream
// caliptra-mcu-sw's memory maps (same selection as target/veer's kernel config).
#[cfg(not(feature = "fpga"))]
use caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP as MCU_MEMORY_MAP;
#[cfg(feature = "fpga")]
use caliptra_mcu_config_fpga::FPGA_MEMORY_MAP as MCU_MEMORY_MAP;
use openprot_hal_blocking::i3c_hardware::{
    DynamicAddress, I3cTarget, I3cTargetConfig, TargetEvent,
};
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

/// Largest private read or write the TTI carries: its TX and RX data FIFOs are
/// 64 DWORDs. Matches caliptra-mcu-sw's `MAX_READ_WRITE_SIZE`.
pub const MAX_READ_WRITE_SIZE: usize = 256;

/// MIPI DCR value for an MCTP endpoint.
pub const MCTP_DCR: u8 = 0xCC;

/// Mandatory Data Byte the target raises with a pending-read IBI, matching the
/// upstream caliptra-mcu-sw runtime driver's MCTP pending-read notification.
pub const MDB_PENDING_READ: u8 = 0xAE;

/// Driver for the Caliptra-SS i3c-core in target (secondary) mode.
/// How many times a failed IBI is raised again before giving up. The
/// caliptra-mcu-sw driver retries without a bound; a bound keeps a controller
/// that never acknowledges the IBI from making this driver spin forever.
const IBI_MAX_RETRIES: u8 = 16;

pub struct CaliptraI3cTarget {
    regs: *const i3c::regs::I3c,
    // Mandatory Data Byte raised with each pending-read IBI.
    ibi_mdb: u8,
    // The IBI payload (the 2-byte big-endian response length) of the IBI raised
    // for the staged response, kept until the controller acknowledges it, so it
    // can be raised again if the hardware reports a failure.
    pending_ibi: Option<[u8; 2]>,
    // Retries of `pending_ibi` so far.
    ibi_retries: u8,
    // !Send + !Sync: exclusive ownership of one physical peripheral
    _not_send_sync: PhantomData<*mut ()>,
}

impl CaliptraI3cTarget {
    /// Construct the driver over the Caliptra-SS I3C peripheral.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the I3C peripheral for the
    /// lifetime of this value.
    pub const unsafe fn new() -> Self {
        Self {
            regs: MCU_MEMORY_MAP.i3c_offset as *const i3c::regs::I3c,
            ibi_mdb: MDB_PENDING_READ,
            pending_ibi: None,
            ibi_retries: 0,
            _not_send_sync: PhantomData,
        }
    }

    /// Apply pre-kernel target configuration.
    ///
    /// Only the IBI Mandatory Data Byte is consumed here; the static and
    /// dynamic addresses are programmed by the Caliptra ROM before this driver
    /// runs, so `config.static_address` is informational.
    pub fn set_config(&mut self, config: I3cTargetConfig) {
        self.ibi_mdb = config.ibi_mdb;
    }

    #[inline]
    fn regs(&self) -> &i3c::regs::I3c {
        // SAFETY: `new` guarantees a valid address and exclusive access.
        unsafe { &*self.regs }
    }

    // -------------------------------------------------------------------------
    // Interrupts
    // -------------------------------------------------------------------------

    /// Enable the RX descriptor threshold interrupt.
    pub fn enable_rx_interrupt(&mut self) {
        self.regs()
            .tti_interrupt_enable
            .modify(i3c::bits::InterruptEnable::RxDescStatEn::SET);
    }

    /// Disable the RX descriptor threshold interrupt.
    pub fn disable_rx_interrupt(&mut self) {
        self.regs()
            .tti_interrupt_enable
            .modify(i3c::bits::InterruptEnable::RxDescStatEn::CLEAR);
    }

    // -------------------------------------------------------------------------
    // TTI receive (controller → target private write)
    // -------------------------------------------------------------------------

    /// Return `true` if an RX descriptor is waiting in the TTI queue.
    pub fn rx_pending(&self) -> bool {
        self.regs()
            .tti_interrupt_status
            .is_set(i3c::bits::InterruptStatus::RxDescStat)
    }

    /// Drain one incoming write into `buf`.  Returns the number of bytes read,
    /// or `None` if no descriptor was present.
    pub fn rx_read(&mut self, buf: &mut [u8]) -> Option<usize> {
        let regs = self.regs();
        // `RxDescStat` is not a reliable "queue not empty" flag: on the FPGA it
        // deasserts while one descriptor is still queued (seen after a burst of
        // four frames: depth 1, status 0), which strands the last frame because
        // no further interrupt arrives for it. So also consult the RX descriptor
        // queue depth (low byte of `tti_desc_queue_depth`). The status bit stays
        // as a trigger so a model that does not implement the depth register
        // keeps working.
        let status_set = regs
            .tti_interrupt_status
            .is_set(i3c::bits::InterruptStatus::RxDescStat);
        let queued = regs.tti_desc_queue_depth.get() & 0xff != 0;
        if !status_set && !queued {
            return None;
        }

        let desc = regs.tti_rx_desc_queue_port.get();
        // Lower 16 bits of descriptor carry the data length in bytes.
        let len = (desc & 0xffff) as usize;
        if len == 0 {
            // An empty descriptor means the queue has drained.
            return None;
        }
        let nwords = (len + 3) / 4;

        let mut i = 0usize;
        for _ in 0..nwords {
            let word = regs.tti_rx_data_port.get();
            for &b in &word.to_le_bytes() {
                if let Some(slot) = buf.get_mut(i) {
                    *slot = b;
                }
                i += 1;
            }
        }

        // `RxDescStat` is level-triggered: the hardware keeps it set while
        // descriptors remain queued and drops it when the queue empties, so it
        // is not cleared here. (Clearing it by hand would strand any frames
        // still queued behind this one: no new interrupt arrives for them.) The
        // upstream caliptra-mcu-sw driver likewise never writes this bit and
        // reads descriptors until it deasserts.

        Some(len.min(buf.len()))
    }

    // -------------------------------------------------------------------------
    // TTI transmit (target → controller private read response)
    // -------------------------------------------------------------------------

    /// Queue `data` as the response to the next private-read from the controller.
    ///
    /// The descriptor must be written before the data words: the hardware
    /// (and the emulator model) opens a new TX buffer on the descriptor
    /// write and appends subsequent data-port writes to it, matching the
    /// upstream caliptra-mcu-sw runtime driver.
    pub fn tx_write(&mut self, data: &[u8]) {
        let regs = self.regs();
        // Descriptor: data_length in lower 16 bits; saturate rather than truncate.
        regs.tti_tx_desc_queue_port
            .set(u32::try_from(data.len()).unwrap_or(u16::MAX as u32));
        let mut chunks = data.chunks_exact(4);
        for chunk in &mut chunks {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            regs.tti_tx_data_port.set(word);
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut tmp = [0u8; 4];
            tmp[..rem.len()].copy_from_slice(rem);
            regs.tti_tx_data_port.set(u32::from_le_bytes(tmp));
        }
    }

    // -------------------------------------------------------------------------
    // IBI (In-Band Interrupt — target → controller unsolicited notification)
    // -------------------------------------------------------------------------

    /// Raise an IBI with the given Mandatory Data Byte and optional payload.
    /// Payload must be ≤255 bytes; excess bytes are silently dropped.
    ///
    /// The descriptor word must be written before the payload words: the
    /// hardware (and the emulator model) parses the first word written to
    /// the IBI port as the descriptor and takes the payload length from it.
    pub fn ibi_raise(&mut self, mdb: u8, payload: &[u8]) {
        let payload = &payload[..payload.len().min(255)];
        let regs = self.regs();
        // IBI descriptor: MDB in bits [31:24], payload length in bits [7:0].
        let desc = ((mdb as u32) << 24) | (payload.len() as u32 & 0xff);
        regs.tti_tti_ibi_port.set(desc);
        let mut chunks = payload.chunks_exact(4);
        for chunk in &mut chunks {
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            regs.tti_tti_ibi_port.set(word);
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            let mut tmp = [0u8; 4];
            tmp[..rem.len()].copy_from_slice(rem);
            regs.tti_tti_ibi_port.set(u32::from_le_bytes(tmp));
        }
    }
}

/// The server-side facade ([`I3cTarget`]) over the Caliptra-SS TTI data path.
///
/// The deep register mechanics live in the inherent methods above; this impl is
/// the narrow surface the pigweed server-runtime hooks to. The Caliptra ROM has
/// already run dynamic-address assignment, so no `AddressAssigned` event is
/// synthesized here — `dynamic_address` reads the assignment the ROM programmed.
impl I3cTarget for CaliptraI3cTarget {
    // Register access on this peripheral cannot fail.
    type Error = core::convert::Infallible;

    fn enable(&mut self) -> Result<(), Self::Error> {
        // An inbound write is queued, and an IBI finished. The third interrupt
        // the upstream runtime driver arms, `TxDescComplete` (the controller
        // consumed a staged response), is left off here: it costs an interrupt per
        // outbound message and only a runtime that gates on `ResponseRead` needs
        // it (see `enable_response_read_events`).
        self.regs()
            .tti_interrupt_enable
            .modify(InterruptEnable::RxDescStatEn::SET + InterruptEnable::IbiDoneEn::SET);
        Ok(())
    }

    fn enable_response_read_events(&mut self) -> Result<(), Self::Error> {
        self.regs()
            .tti_interrupt_enable
            .modify(InterruptEnable::TxDescCompleteEn::SET);
        Ok(())
    }

    fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
        // Each status bit is handled on its own, as the upstream driver does;
        // one interrupt can carry several of them.
        let status = self.regs().tti_interrupt_status.extract();

        if status.is_set(InterruptStatus::IbiDone) {
            // Reading the last-IBI status clears `IbiDone`. A non-zero status
            // means the IBI was not accepted by the controller: raise it again
            // (the staged TX data is still queued), up to a bound.
            let ibi_status = self.regs().tti_status.read(Status::LastIbiStatus);
            if ibi_status == 0 {
                self.pending_ibi = None;
                self.ibi_retries = 0;
            } else if let Some(payload) = self.pending_ibi {
                if self.ibi_retries < IBI_MAX_RETRIES {
                    self.ibi_retries += 1;
                    self.ibi_raise(self.ibi_mdb, &payload);
                } else {
                    self.pending_ibi = None;
                    self.ibi_retries = 0;
                }
            }
        }

        // The controller has read the staged response out of the TX queue.
        // `TxDescComplete` is the signal for that (only raised if
        // `enable_response_read_events` armed it); `IbiDone` only says the IBI
        // was delivered, which can happen well before the private read.
        let response_read = status.is_set(InterruptStatus::TxDescComplete);
        if response_read {
            self.regs()
                .tti_interrupt_status
                .write(InterruptStatus::TxDescComplete::SET);
        }

        // One interrupt can carry both a response read and an inbound write, and
        // only one event is returned. `TxDescComplete` was just cleared above, so
        // it cannot be reported on a later call, and the runtime releases the
        // next outbound `Send` only on `ResponseRead`: report it first.
        // `RxDescStat` is level-triggered and is not cleared here, so the inbound
        // write is reported by the interrupt that fires again once the runtime
        // acknowledges this one.
        if response_read {
            return Ok(TargetEvent::ResponseRead);
        }
        if status.is_set(InterruptStatus::RxDescStat) {
            // An inbound write is queued; the runtime drains it with
            // `read_frame`.
            return Ok(TargetEvent::InboundReady);
        }
        Ok(TargetEvent::None)
    }

    fn read_frame(&mut self, buf: &mut [u8]) -> Result<Option<usize>, Self::Error> {
        Ok(self.rx_read(buf))
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        // Stage the private-read response, then raise the IBI whose 2-byte
        // big-endian payload tells the controller how many bytes to read.
        self.tx_write(data);
        let len = u16::try_from(data.len()).unwrap_or(u16::MAX);
        let payload = len.to_be_bytes();
        self.pending_ibi = Some(payload);
        self.ibi_retries = 0;
        self.ibi_raise(self.ibi_mdb, &payload);
        Ok(())
    }

    fn dynamic_address(&self) -> Option<DynamicAddress> {
        let regs = self.regs();
        if regs
            .stdby_ctrl_mode_stby_cr_device_addr
            .read(StbyCrDeviceAddr::DynamicAddrValid)
            == 1
        {
            let raw = regs
                .stdby_ctrl_mode_stby_cr_device_addr
                .read(StbyCrDeviceAddr::DynamicAddr) as u8;
            DynamicAddress::try_from(raw).ok()
        } else {
            None
        }
    }

    fn static_address(&self) -> Option<DynamicAddress> {
        let regs = self.regs();
        if regs
            .stdby_ctrl_mode_stby_cr_device_addr
            .read(StbyCrDeviceAddr::StaticAddrValid)
            == 1
        {
            let raw = regs
                .stdby_ctrl_mode_stby_cr_device_addr
                .read(StbyCrDeviceAddr::StaticAddr) as u8;
            DynamicAddress::try_from(raw).ok()
        } else {
            None
        }
    }

    fn max_send_len(&self) -> usize {
        MAX_READ_WRITE_SIZE
    }

    fn max_recv_len(&self) -> usize {
        MAX_READ_WRITE_SIZE
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        // Stop every TTI interrupt: no more inbound, IBI-done or response-read
        // events. `enable` arms them again.
        self.regs().tti_interrupt_enable.set(0);
        Ok(())
    }
}
