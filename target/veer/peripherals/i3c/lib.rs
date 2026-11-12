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

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::i3c;
use i3c::bits::{InterruptEnable, InterruptStatus, Status, StbyCrDeviceAddr};
use tock_registers::fields::FieldValue;
use tock_registers::LocalRegisterCopy;
// I3C register block base for the selected target type, from upstream
// caliptra-mcu-sw's memory maps (same selection as target/veer's kernel config).
#[cfg(not(feature = "fpga"))]
use caliptra_mcu_config_emulator::EMULATOR_MEMORY_MAP as MCU_MEMORY_MAP;
#[cfg(feature = "fpga")]
use caliptra_mcu_config_fpga::FPGA_MEMORY_MAP as MCU_MEMORY_MAP;
use openprot_hal_blocking::i3c_hardware::{
    DynamicAddress, I3cTarget, I3cTargetConfig, TargetEvent,
};

/// Largest private read or write the TTI carries: its TX and RX data FIFOs are
/// 64 DWORDs. Matches caliptra-mcu-sw's `MAX_READ_WRITE_SIZE`.
pub const MAX_READ_WRITE_SIZE: usize = 256;

/// MIPI DCR value for an MCTP endpoint.
pub const MCTP_DCR: u8 = 0xCC;

/// Mandatory Data Byte the target raises with a pending-read IBI, matching the
/// upstream caliptra-mcu-sw runtime driver's MCTP pending-read notification.
pub const MDB_PENDING_READ: u8 = 0xAE;

/// How many times a failed IBI is raised again before giving up. The
/// caliptra-mcu-sw driver retries without a bound; a bound keeps a controller
/// that never acknowledges the IBI from making this driver spin forever.
const IBI_MAX_RETRIES: u8 = 16;

/// The TTI and standby-controller registers this driver uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reg {
    /// `stdby_ctrl_mode_stby_cr_device_addr`: static and dynamic address.
    DeviceAddr,
    /// `tti_status`: last IBI status.
    TtiStatus,
    /// `tti_desc_queue_depth`: RX descriptor depth in the low byte.
    TtiDescQueueDepth,
    /// `tti_interrupt_status`.
    TtiInterruptStatus,
    /// `tti_interrupt_enable`.
    TtiInterruptEnable,
    /// `tti_rx_desc_queue_port`: pop one RX descriptor.
    TtiRxDescPort,
    /// `tti_rx_data_port`: pop one RX data word.
    TtiRxDataPort,
    /// `tti_tx_desc_queue_port`: push a TX descriptor.
    TtiTxDescPort,
    /// `tti_tx_data_port`: push one TX data word.
    TtiTxDataPort,
    /// `tti_tti_ibi_port`: push an IBI descriptor, then payload words.
    TtiIbiPort,
}

impl Reg {
    /// Byte offset of the register from the I3C block base.
    pub const fn offset(self) -> usize {
        match self {
            Reg::DeviceAddr => offset_of!(i3c::regs::I3c, stdby_ctrl_mode_stby_cr_device_addr),
            Reg::TtiStatus => offset_of!(i3c::regs::I3c, tti_status),
            Reg::TtiDescQueueDepth => offset_of!(i3c::regs::I3c, tti_desc_queue_depth),
            Reg::TtiInterruptStatus => offset_of!(i3c::regs::I3c, tti_interrupt_status),
            Reg::TtiInterruptEnable => offset_of!(i3c::regs::I3c, tti_interrupt_enable),
            Reg::TtiRxDescPort => offset_of!(i3c::regs::I3c, tti_rx_desc_queue_port),
            Reg::TtiRxDataPort => offset_of!(i3c::regs::I3c, tti_rx_data_port),
            Reg::TtiTxDescPort => offset_of!(i3c::regs::I3c, tti_tx_desc_queue_port),
            Reg::TtiTxDataPort => offset_of!(i3c::regs::I3c, tti_tx_data_port),
            Reg::TtiIbiPort => offset_of!(i3c::regs::I3c, tti_tti_ibi_port),
        }
    }
}

/// Access to the I3C registers. The driver is generic over it so that host
/// tests can stand in for the hardware.
pub trait Mmio {
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// The I3C controller's memory-mapped registers.
pub struct MmioBlock {
    base: usize,
}

impl MmioBlock {
    /// # Safety
    ///
    /// `base` must be the address of the Caliptra-SS I3C register block. It
    /// must be mapped for the caller, and no other code may access it.
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

/// Call `f` with each little-endian 32-bit word of `bytes`; a 1 to 3 byte tail
/// is zero padded.
fn for_each_word_le(bytes: &[u8], mut f: impl FnMut(u32)) {
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        f(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    let rem = chunks.remainder();
    if !rem.is_empty() {
        let mut tmp = [0u8; 4];
        tmp[..rem.len()].copy_from_slice(rem);
        f(u32::from_le_bytes(tmp));
    }
}

/// Store word number `index` into `buf` as little-endian bytes; bytes past the
/// end of `buf` are dropped.
fn store_word_le(buf: &mut [u8], index: usize, word: u32) {
    for (k, b) in word.to_le_bytes().into_iter().enumerate() {
        if let Some(slot) = buf.get_mut(index * 4 + k) {
            *slot = b;
        }
    }
}

/// Driver for the Caliptra-SS i3c-core in target (secondary) mode.
pub struct CaliptraI3cTarget<M: Mmio = MmioBlock> {
    mmio: M,
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

impl CaliptraI3cTarget<MmioBlock> {
    /// Construct the driver over the Caliptra-SS I3C peripheral.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access to the I3C peripheral for the
    /// lifetime of this value.
    pub const unsafe fn new() -> Self {
        // SAFETY: the caller guarantees exclusive access; the base is the I3C
        // block of the selected memory map.
        Self::with_mmio(unsafe { MmioBlock::new(MCU_MEMORY_MAP.i3c_offset as usize) })
    }
}

impl<M: Mmio> CaliptraI3cTarget<M> {
    /// Construct the driver over `mmio`.
    pub const fn with_mmio(mmio: M) -> Self {
        Self {
            mmio,
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

    fn interrupt_status(&self) -> LocalRegisterCopy<u32, InterruptStatus::Register> {
        LocalRegisterCopy::new(self.mmio.read(Reg::TtiInterruptStatus))
    }

    /// Read-modify-write of the interrupt enable register.
    fn update_interrupt_enable(&mut self, change: FieldValue<u32, InterruptEnable::Register>) {
        let current = self.mmio.read(Reg::TtiInterruptEnable);
        self.mmio
            .write(Reg::TtiInterruptEnable, change.modify(current));
    }

    fn device_addr(&self) -> LocalRegisterCopy<u32, StbyCrDeviceAddr::Register> {
        LocalRegisterCopy::new(self.mmio.read(Reg::DeviceAddr))
    }

    // -------------------------------------------------------------------------
    // Interrupts
    // -------------------------------------------------------------------------

    /// Enable the RX descriptor threshold interrupt.
    pub fn enable_rx_interrupt(&mut self) {
        self.update_interrupt_enable(InterruptEnable::RxDescStatEn::SET);
    }

    /// Disable the RX descriptor threshold interrupt.
    pub fn disable_rx_interrupt(&mut self) {
        self.update_interrupt_enable(InterruptEnable::RxDescStatEn::CLEAR);
    }

    // -------------------------------------------------------------------------
    // TTI receive (controller → target private write)
    // -------------------------------------------------------------------------

    /// Return `true` if an RX descriptor is waiting in the TTI queue.
    pub fn rx_pending(&self) -> bool {
        self.interrupt_status().is_set(InterruptStatus::RxDescStat)
    }

    /// Drain one incoming write into `buf`.  Returns the number of bytes read,
    /// or `None` if no descriptor was present.
    pub fn rx_read(&mut self, buf: &mut [u8]) -> Option<usize> {
        // `RxDescStat` is not a reliable "queue not empty" flag: on the FPGA it
        // deasserts while one descriptor is still queued (seen after a burst of
        // four frames: depth 1, status 0), which strands the last frame because
        // no further interrupt arrives for it. So also consult the RX descriptor
        // queue depth (low byte of `tti_desc_queue_depth`). The status bit stays
        // as a trigger so a model that does not implement the depth register
        // keeps working.
        let status_set = self.interrupt_status().is_set(InterruptStatus::RxDescStat);
        let queued = self.mmio.read(Reg::TtiDescQueueDepth) & 0xff != 0;
        if !status_set && !queued {
            return None;
        }

        let desc = self.mmio.read(Reg::TtiRxDescPort);
        // Lower 16 bits of descriptor carry the data length in bytes.
        let len = (desc & 0xffff) as usize;
        if len == 0 {
            // An empty descriptor means the queue has drained.
            return None;
        }
        let nwords = (len + 3) / 4;

        for index in 0..nwords {
            let word = self.mmio.read(Reg::TtiRxDataPort);
            store_word_le(buf, index, word);
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
        // Descriptor: data_length in lower 16 bits; saturate rather than truncate.
        let desc = u32::try_from(data.len()).unwrap_or(u16::MAX as u32);
        self.mmio.write(Reg::TtiTxDescPort, desc);
        for_each_word_le(data, |word| self.mmio.write(Reg::TtiTxDataPort, word));
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
        // IBI descriptor: MDB in bits [31:24], payload length in bits [7:0].
        let desc = ((mdb as u32) << 24) | (payload.len() as u32 & 0xff);
        self.mmio.write(Reg::TtiIbiPort, desc);
        for_each_word_le(payload, |word| self.mmio.write(Reg::TtiIbiPort, word));
    }
}

/// The server-side facade ([`I3cTarget`]) over the Caliptra-SS TTI data path.
///
/// The deep register mechanics live in the inherent methods above; this impl is
/// the narrow surface the pigweed server-runtime hooks to. The Caliptra ROM has
/// already run dynamic-address assignment, so no `AddressAssigned` event is
/// synthesized here — `dynamic_address` reads the assignment the ROM programmed.
impl<M: Mmio> I3cTarget for CaliptraI3cTarget<M> {
    // Register access on this peripheral cannot fail.
    type Error = core::convert::Infallible;

    fn enable(&mut self) -> Result<(), Self::Error> {
        // An inbound write is queued, and an IBI finished. The third interrupt
        // the upstream runtime driver arms, `TxDescComplete` (the controller
        // consumed a staged response), is left off here: it costs an interrupt per
        // outbound message and only a runtime that gates on `ResponseRead` needs
        // it (see `enable_response_read_events`).
        self.update_interrupt_enable(
            InterruptEnable::RxDescStatEn::SET + InterruptEnable::IbiDoneEn::SET,
        );
        Ok(())
    }

    fn enable_response_read_events(&mut self) -> Result<(), Self::Error> {
        self.update_interrupt_enable(InterruptEnable::TxDescCompleteEn::SET);
        Ok(())
    }

    fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
        // Each status bit is handled on its own, as the upstream driver does;
        // one interrupt can carry several of them.
        let status = self.interrupt_status();

        if status.is_set(InterruptStatus::IbiDone) {
            // Reading the last-IBI status clears `IbiDone`. A non-zero status
            // means the IBI was not accepted by the controller: raise it again
            // (the staged TX data is still queued), up to a bound.
            let ibi_status =
                LocalRegisterCopy::<u32, Status::Register>::new(self.mmio.read(Reg::TtiStatus))
                    .read(Status::LastIbiStatus);
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
            self.mmio.write(
                Reg::TtiInterruptStatus,
                InterruptStatus::TxDescComplete::SET.into(),
            );
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
        let addr = self.device_addr();
        if addr.read(StbyCrDeviceAddr::DynamicAddrValid) == 1 {
            DynamicAddress::try_from(addr.read(StbyCrDeviceAddr::DynamicAddr) as u8).ok()
        } else {
            None
        }
    }

    fn static_address(&self) -> Option<DynamicAddress> {
        let addr = self.device_addr();
        if addr.read(StbyCrDeviceAddr::StaticAddrValid) == 1 {
            DynamicAddress::try_from(addr.read(StbyCrDeviceAddr::StaticAddr) as u8).ok()
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
        self.mmio.write(Reg::TtiInterruptEnable, 0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::collections::HashMap;
    use std::vec::Vec;

    use super::*;

    /// Stands in for the controller: registers read back what was last stored
    /// (or what a test scripted), and every write is recorded in order.
    #[derive(Default)]
    struct FakeMmio {
        regs: HashMap<usize, u32>,
        writes: Vec<(Reg, u32)>,
    }

    impl FakeMmio {
        fn set(&mut self, reg: Reg, value: u32) {
            self.regs.insert(reg.offset(), value);
        }
    }

    impl Mmio for FakeMmio {
        fn read(&self, reg: Reg) -> u32 {
            self.regs.get(&reg.offset()).copied().unwrap_or(0)
        }

        fn write(&mut self, reg: Reg, value: u32) {
            self.writes.push((reg, value));
            // The status register is write-one-to-clear; the rest store.
            if reg != Reg::TtiInterruptStatus {
                self.regs.insert(reg.offset(), value);
            }
        }
    }

    fn driver() -> CaliptraI3cTarget<FakeMmio> {
        CaliptraI3cTarget::with_mmio(FakeMmio::default())
    }

    fn words(bytes: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        for_each_word_le(bytes, |w| out.push(w));
        out
    }

    #[test]
    fn register_offsets_match_the_hardware_map() {
        assert_eq!(Reg::DeviceAddr.offset(), 0x188);
        assert_eq!(Reg::TtiStatus.offset(), 0x208);
        assert_eq!(Reg::TtiDescQueueDepth.offset(), 0x214);
        assert_eq!(Reg::TtiInterruptStatus.offset(), 0x220);
        assert_eq!(Reg::TtiInterruptEnable.offset(), 0x224);
        assert_eq!(Reg::TtiRxDescPort.offset(), 0x270);
        assert_eq!(Reg::TtiRxDataPort.offset(), 0x274);
        assert_eq!(Reg::TtiTxDescPort.offset(), 0x278);
        assert_eq!(Reg::TtiTxDataPort.offset(), 0x27c);
        assert_eq!(Reg::TtiIbiPort.offset(), 0x280);
    }

    #[test]
    fn word_packing_pads_the_tail() {
        assert_eq!(words(&[]), Vec::<u32>::new());
        assert_eq!(words(&[1, 2, 3, 4]), [0x0403_0201]);
        assert_eq!(words(&[1]), [0x0000_0001]);
        assert_eq!(words(&[1, 2]), [0x0000_0201]);
        assert_eq!(words(&[1, 2, 3]), [0x0003_0201]);
        assert_eq!(words(&[1, 2, 3, 4, 5]), [0x0403_0201, 0x0000_0005]);
    }

    #[test]
    fn word_unpacking_drops_bytes_past_the_buffer() {
        let mut buf = [0u8; 5];
        store_word_le(&mut buf, 0, 0x0403_0201);
        store_word_le(&mut buf, 1, 0x0807_0605);
        assert_eq!(buf, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn tx_write_pushes_the_descriptor_before_the_data() {
        let mut d = driver();
        d.tx_write(&[1, 2, 3, 4, 5]);
        assert_eq!(
            d.mmio.writes,
            [
                (Reg::TtiTxDescPort, 5),
                (Reg::TtiTxDataPort, 0x0403_0201),
                (Reg::TtiTxDataPort, 0x0000_0005),
            ]
        );
    }

    #[test]
    fn ibi_raise_pushes_mdb_and_length_before_the_payload() {
        let mut d = driver();
        d.ibi_raise(0xAE, &[0x01, 0x23]);
        assert_eq!(
            d.mmio.writes,
            [
                (Reg::TtiIbiPort, 0xAE00_0002),
                (Reg::TtiIbiPort, 0x0000_2301),
            ]
        );
    }

    #[test]
    fn send_stages_the_response_then_raises_the_length_ibi() {
        let mut d = driver();
        d.send(&[9; 6]).unwrap();
        assert_eq!(
            d.mmio.writes,
            [
                (Reg::TtiTxDescPort, 6),
                (Reg::TtiTxDataPort, 0x0909_0909),
                (Reg::TtiTxDataPort, 0x0000_0909),
                // MDB 0xAE, 2-byte payload: the length, big-endian.
                (Reg::TtiIbiPort, 0xAE00_0002),
                (Reg::TtiIbiPort, 0x0000_0600),
            ]
        );
    }

    #[test]
    fn enable_arms_rx_and_ibi_done_and_disable_clears_all() {
        let mut d = driver();
        d.enable().unwrap();
        let armed = d.mmio.read(Reg::TtiInterruptEnable);
        let rx = u32::from(InterruptEnable::RxDescStatEn::SET);
        let ibi = u32::from(InterruptEnable::IbiDoneEn::SET);
        assert_eq!(armed, rx | ibi);

        d.enable_response_read_events().unwrap();
        let tx = u32::from(InterruptEnable::TxDescCompleteEn::SET);
        assert_eq!(d.mmio.read(Reg::TtiInterruptEnable), rx | ibi | tx);

        d.disable().unwrap();
        assert_eq!(d.mmio.read(Reg::TtiInterruptEnable), 0);
    }

    #[test]
    fn response_read_is_reported_before_inbound_and_cleared() {
        let mut d = driver();
        let both = u32::from(InterruptStatus::TxDescComplete::SET)
            | u32::from(InterruptStatus::RxDescStat::SET);
        d.mmio.set(Reg::TtiInterruptStatus, both);

        assert_eq!(d.on_interrupt().unwrap(), TargetEvent::ResponseRead);
        // Only the clear-on-read bit is written back.
        assert_eq!(
            d.mmio.writes,
            [(
                Reg::TtiInterruptStatus,
                u32::from(InterruptStatus::TxDescComplete::SET)
            )]
        );

        // Once the hardware drops it, the level bit is reported next.
        d.mmio.set(
            Reg::TtiInterruptStatus,
            u32::from(InterruptStatus::RxDescStat::SET),
        );
        assert_eq!(d.on_interrupt().unwrap(), TargetEvent::InboundReady);

        d.mmio.set(Reg::TtiInterruptStatus, 0);
        assert_eq!(d.on_interrupt().unwrap(), TargetEvent::None);
    }

    #[test]
    fn a_failed_ibi_is_raised_again_a_bounded_number_of_times() {
        let mut d = driver();
        d.send(&[1, 2]).unwrap();
        d.mmio.writes.clear();
        d.mmio.set(
            Reg::TtiInterruptStatus,
            u32::from(InterruptStatus::IbiDone::SET),
        );
        // A non-zero last-IBI status means the controller did not accept it.
        d.mmio
            .set(Reg::TtiStatus, u32::from(Status::LastIbiStatus.val(1)));

        for _ in 0..IBI_MAX_RETRIES {
            d.on_interrupt().unwrap();
        }
        let raised = d
            .mmio
            .writes
            .iter()
            .filter(|(r, _)| *r == Reg::TtiIbiPort)
            .count();
        // Descriptor plus one payload word, per retry.
        assert_eq!(raised, 2 * IBI_MAX_RETRIES as usize);

        // The next failure gives up instead of raising it again.
        d.mmio.writes.clear();
        d.on_interrupt().unwrap();
        assert!(d.mmio.writes.is_empty());
        assert_eq!(d.pending_ibi, None);
    }

    #[test]
    fn rx_read_unpacks_a_frame_and_truncates_to_the_buffer() {
        let mut fake = FakeMmio::default();
        fake.set(Reg::TtiInterruptStatus, 1);
        fake.set(Reg::TtiRxDescPort, 6);
        // Both data words are the same register value in this fake.
        fake.set(Reg::TtiRxDataPort, 0x0403_0201);
        let mut d = CaliptraI3cTarget::with_mmio(fake);
        let mut buf = [0u8; 8];
        assert_eq!(d.rx_read(&mut buf), Some(6));
        assert_eq!(buf[..6], [1, 2, 3, 4, 1, 2]);

        let mut small = [0u8; 3];
        assert_eq!(d.rx_read(&mut small), Some(3));
        assert_eq!(small, [1, 2, 3]);
    }

    #[test]
    fn rx_read_is_empty_when_nothing_is_queued() {
        let mut d = driver();
        assert_eq!(d.rx_read(&mut [0u8; 4]), None);
        // Depth says a descriptor is queued although the status bit dropped.
        d.mmio.set(Reg::TtiDescQueueDepth, 1);
        d.mmio.set(Reg::TtiRxDescPort, 0);
        assert_eq!(d.rx_read(&mut [0u8; 4]), None);
    }

    #[test]
    fn addresses_come_from_the_device_address_register() {
        let mut d = driver();
        assert_eq!(d.dynamic_address(), None);
        assert_eq!(d.static_address(), None);

        let dynamic =
            StbyCrDeviceAddr::DynamicAddrValid::SET + StbyCrDeviceAddr::DynamicAddr.val(0x08);
        let stat = StbyCrDeviceAddr::StaticAddrValid::SET + StbyCrDeviceAddr::StaticAddr.val(0x3d);
        d.mmio.set(Reg::DeviceAddr, u32::from(dynamic + stat));
        assert_eq!(d.dynamic_address(), DynamicAddress::try_from(0x08).ok());
        assert_eq!(d.static_address(), DynamicAddress::try_from(0x3d).ok());
    }
}
