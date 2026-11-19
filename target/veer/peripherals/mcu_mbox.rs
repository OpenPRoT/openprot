// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for an MCU mailbox of the Caliptra-SS MCI block, with the MCU as
//! the receiver of commands.
//!
//! The MCI has two MCU mailboxes. Each one is an SRAM and a set of
//! registers. A requester, which is an agent of the SoC that the MCI trusts,
//! takes the lock of the mailbox, writes a command, a length and the request
//! data, and sets the execute register. The MCU reads the request, writes
//! the response over it in the same SRAM, and sets the status of the command.
//! The requester reads the response and clears the execute register, which
//! releases the mailbox: the hardware clears the registers, fills the used
//! part of the SRAM with zeros, and then frees the lock.
//!
//! [`McuMailbox::request`] returns the command that is waiting, and the
//! [`Request`] answers it. The driver does not use an interrupt. The MCI
//! reports a command through the notification `MBOX0_COMMAND_AVAILABLE` or
//! `MBOX1_COMMAND_AVAILABLE` of the `veer_mci` driver, and the caller can
//! also poll `request`.
//!
//! When the MCI comes out of reset, the MCU holds the lock, and no requester
//! can use the mailbox until the MCU has released it with
//! [`McuMailbox::clear`].
//!
//! The driver does not send commands from the MCU. It does not use the
//! registers with which the MCU lets another agent, the target user, work on
//! a request.
//!
//! The driver follows revision 2.1.0 of the MCI. That is the revision of the
//! caliptra-ss RTL that caliptra-mcu-sw includes as `hw/caliptra-ss`
//! (`src/mci/rtl/mcu_mbox.sv`, `mcu_mbox_csr.rdl`), and the revision that the
//! MCI of the VCK190 bitstream reports. In that revision a requester can
//! release the mailbox while the MCU works on its command, the MCU can write
//! a mailbox whoever holds its lock, and a command has no identifier. The
//! driver checks that a command is still executing when it reads or writes
//! for it. It cannot tell a command from one that a requester started after
//! a release, so an answer can reach the wrong command.
//!
//! Revision 2.2.0 of caliptra-ss gives the SRAM and the command, length and
//! execute registers to one owner at a time. The MCU is the owner from the
//! time the requester sets the execute register until the MCU writes a
//! status other than busy, and a requester cannot release the mailbox in
//! that time. The driver writes the length before the status, which that
//! revision needs. The driver is not tested on that revision.
//!
//! caliptra-mcu-sw has its driver for the same role in
//! `platforms/emulator/runtime/kernel/drivers/mcu_mbox`. Register offsets and
//! bit fields come from the register definitions that caliptra-mcu-sw
//! generates (`caliptra_ss_registers::mci`).

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr::{with_exposed_provenance, with_exposed_provenance_mut};

use caliptra_ss_registers::mci;
use mci::bits::{MboxCmdStatus, MboxExecute, MboxHwStatus};
use tock_registers::LocalRegisterCopy;

/// Offset of MCU mailbox 0 from the base of the MCI block.
pub const MBOX0_OFFSET: usize = offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_sram);

/// Offset of MCU mailbox 1 from the base of the MCI block.
pub const MBOX1_OFFSET: usize = offset_of!(mci::regs::Mci, mcu_mbox1_csr_mbox_sram);

/// Offset of the registers of a mailbox from its base. The SRAM is at the
/// base, and the hardware reserves this much address space for it.
pub const CSR_OFFSET: usize = csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_lock));

/// The largest SRAM that a mailbox can have, in bytes.
pub const MAX_SRAM_SIZE: usize = CSR_OFFSET;

/// Offset from the base of a mailbox of the register that is at
/// `offset_in_mci` in mailbox 0.
const fn csr_offset(offset_in_mci: usize) -> usize {
    offset_in_mci - MBOX0_OFFSET
}

/// The registers of a mailbox that this driver uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reg {
    /// Word `index` of the SRAM.
    Sram(usize),
    /// `mbox_lock`. A read returns 0 and takes the lock if the lock is free,
    /// and returns 1 if it is held.
    Lock,
    /// `mbox_user`: the AXI user that holds the lock. The AXI user is the
    /// number that the bus carries with each access to identify its origin.
    User,
    /// `mbox_cmd`: the command.
    Command,
    /// `mbox_dlen`: the length in bytes of the request, and then of the
    /// response.
    Length,
    /// `mbox_execute`. The requester sets it to start the command and clears
    /// it to release the mailbox.
    Execute,
    /// `mbox_cmd_status`: the status of the command, which only the MCU can
    /// write.
    CommandStatus,
    /// `mbox_hw_status`: the ECC errors that reads of the SRAM have had
    /// since the lock was taken.
    HwStatus,
}

impl Reg {
    /// Byte offset of the register from the base of the mailbox.
    pub const fn offset(self) -> usize {
        match self {
            Reg::Sram(index) => index.saturating_mul(4),
            Reg::Lock => CSR_OFFSET,
            Reg::User => csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_user)),
            Reg::Command => csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_cmd)),
            Reg::Length => csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_dlen)),
            Reg::Execute => csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_execute)),
            Reg::CommandStatus => {
                csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_cmd_status))
            }
            Reg::HwStatus => csr_offset(offset_of!(mci::regs::Mci, mcu_mbox0_csr_mbox_hw_status)),
        }
    }
}

/// Access to the registers and the SRAM of a mailbox. The driver is generic
/// over this trait so that host tests can use a model in place of the
/// hardware.
pub trait Mmio {
    /// Size of the SRAM in bytes: a multiple of 4 that is at most
    /// [`MAX_SRAM_SIZE`].
    fn sram_size(&self) -> usize;
    fn read(&self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
}

/// The memory-mapped registers and SRAM of a mailbox.
pub struct MmioBlock {
    base: usize,
    sram_size: usize,
}

impl MmioBlock {
    /// `sram_size` is rounded down to a multiple of 4 and limited to
    /// [`MAX_SRAM_SIZE`].
    ///
    /// # Safety
    ///
    /// `base` must be the address of an MCU mailbox. The first `sram_size`
    /// bytes at `base` and the registers at `base + CSR_OFFSET` must be
    /// mapped for the caller, and no other code on the MCU may use the
    /// mailbox.
    pub const unsafe fn new(base: usize, sram_size: usize) -> Self {
        let sram_size = if sram_size > MAX_SRAM_SIZE {
            MAX_SRAM_SIZE
        } else {
            sram_size
        };
        Self {
            base,
            sram_size: sram_size & !3,
        }
    }

    /// The address of `reg`, or `None` for an SRAM word outside the SRAM.
    fn address(&self, reg: Reg) -> Option<usize> {
        if matches!(reg, Reg::Sram(_)) && reg.offset() >= self.sram_size {
            return None;
        }
        Some(self.base.wrapping_add(reg.offset()))
    }
}

impl Mmio for MmioBlock {
    fn sram_size(&self) -> usize {
        self.sram_size
    }

    /// An SRAM word outside the SRAM reads as 0, as it does in the hardware.
    fn read(&self, reg: Reg) -> u32 {
        let Some(addr) = self.address(reg) else {
            return 0;
        };
        // SAFETY: `addr` is a register, or a word inside the SRAM, of the
        // mailbox that the caller of `new` guaranteed is mapped.
        unsafe { with_exposed_provenance::<u32>(addr).read_volatile() }
    }

    /// A write to an SRAM word outside the SRAM has no effect, as in the
    /// hardware.
    fn write(&mut self, reg: Reg, value: u32) {
        let Some(addr) = self.address(reg) else {
            return;
        };
        // SAFETY: as for `read`.
        unsafe { with_exposed_provenance_mut::<u32>(addr).write_volatile(value) }
    }
}

/// The result that the MCU reports for a command.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// The command succeeded and the mailbox holds response data.
    DataReady,
    /// The command succeeded.
    Complete,
    /// The command failed.
    Failure,
}

impl Status {
    /// The value of `mbox_cmd_status` for the status.
    const fn register(self) -> u32 {
        match self {
            Status::DataReady => MboxCmdStatus::Status::DataReady.value,
            Status::Complete => MboxCmdStatus::Status::CmdComplete.value,
            Status::Failure => MboxCmdStatus::Status::CmdFailure.value,
        }
    }
}

/// Errors from the mailbox.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The range is not inside the request, or the response does not fit in
    /// the SRAM.
    OutOfRange,
    /// The offset of response data is not a multiple of 4.
    Unaligned,
    /// A read of the SRAM had an ECC error that the hardware could not
    /// correct, so request data that was read since the requester took the
    /// lock may be wrong.
    Uncorrectable,
    /// The requester released the mailbox before the MCU answered.
    Released,
}

/// An MCU mailbox.
pub struct McuMailbox<M: Mmio = MmioBlock> {
    mmio: M,
    // !Send + !Sync: exclusive ownership of one physical peripheral
    _not_send_sync: PhantomData<*mut ()>,
}

impl McuMailbox<MmioBlock> {
    /// `base` is the address of the mailbox: the base of the MCI block plus
    /// [`MBOX0_OFFSET`] or [`MBOX1_OFFSET`]. `sram_size` is the size of its
    /// SRAM in bytes, which the hardware configuration of the MCI reports
    /// (`Mci::mbox0_sram_size_kib` in `veer_mci`). It is rounded down to a
    /// multiple of 4 and limited to [`MAX_SRAM_SIZE`].
    ///
    /// # Safety
    ///
    /// The mailbox must be at `base`. The first `sram_size` bytes at `base`
    /// and the registers at `base + CSR_OFFSET` must stay mapped for the
    /// caller, and no other code on the MCU may use the mailbox.
    pub const unsafe fn new(base: usize, sram_size: usize) -> Self {
        // SAFETY: the caller guarantees that the mailbox is mapped and that
        // nothing else uses it.
        Self::with_mmio(unsafe { MmioBlock::new(base, sram_size) })
    }
}

impl<M: Mmio> McuMailbox<M> {
    /// Constructs the driver over `mmio`.
    pub const fn with_mmio(mmio: M) -> Self {
        Self {
            mmio,
            _not_send_sync: PhantomData,
        }
    }

    /// Size of the SRAM in bytes, which is the largest request or response.
    pub fn sram_size(&self) -> usize {
        self.mmio.sram_size()
    }

    /// Releases the mailbox with the size of the SRAM as its length, so that
    /// the hardware fills the SRAM with zeros and then frees the lock for a
    /// requester. The hardware does this after the function has returned.
    ///
    /// The MCU calls this once after a reset of the MCI, which leaves the
    /// lock with the MCU.
    ///
    /// In revision 2.1.0 of the MCI the function also releases a mailbox
    /// that a requester has locked, which ends its command without an
    /// answer. If that requester wrote a length that is larger than the
    /// SRAM, the hardware clears only a part of the SRAM. The function has
    /// no effect while the hardware is clearing the SRAM after a release.
    pub fn clear(&mut self) {
        // The MCU can only write the registers while the mailbox is locked.
        // If the lock is free, this read takes it.
        let _ = self.mmio.read(Reg::Lock);
        // The hardware fills the SRAM with zeros up to the largest length
        // that was written while the mailbox was locked.
        let size = u32::try_from(self.mmio.sram_size()).unwrap_or(u32::MAX);
        self.mmio.write(Reg::Length, size);
        self.mmio.write(Reg::Execute, 0);
    }

    /// The command that a requester has started and the MCU has not
    /// answered, if there is one.
    ///
    /// A command whose length is larger than the SRAM fails here and is not
    /// returned.
    pub fn request(&mut self) -> Option<Request<'_, M>> {
        if !self.executing() {
            return None;
        }
        let status = LocalRegisterCopy::<u32, MboxCmdStatus::Register>::new(
            self.mmio.read(Reg::CommandStatus),
        );
        if status.read(MboxCmdStatus::Status) != MboxCmdStatus::Status::CmdBusy.value {
            return None;
        }
        let len = usize::try_from(self.mmio.read(Reg::Length)).unwrap_or(usize::MAX);
        let command = self.mmio.read(Reg::Command);
        let user = self.mmio.read(Reg::User);
        // A release clears the registers. If it came between the reads
        // above, the command is no longer executing.
        if !self.executing() {
            return None;
        }
        if len > self.mmio.sram_size() {
            self.mmio.write(Reg::Length, 0);
            self.mmio
                .write(Reg::CommandStatus, Status::Failure.register());
            return None;
        }
        Some(Request {
            mbox: self,
            command,
            user,
            len,
            covered: len,
            answered: false,
        })
    }

    /// Whether a requester has set the execute register and not cleared it.
    fn executing(&self) -> bool {
        LocalRegisterCopy::<u32, MboxExecute::Register>::new(self.mmio.read(Reg::Execute))
            .is_set(MboxExecute::Execute)
    }
}

/// A command in the mailbox that the MCU has not answered.
///
/// The response goes into the SRAM that holds the request, so the caller
/// reads what it needs of the request before it writes over it.
///
/// A `Request` that is dropped without an answer fails the command.
pub struct Request<'a, M: Mmio = MmioBlock> {
    mbox: &'a mut McuMailbox<M>,
    command: u32,
    user: u32,
    len: usize,
    /// How much of the SRAM the hardware clears when the requester releases
    /// the mailbox: the largest length that the requester or this driver has
    /// written for the command.
    covered: usize,
    answered: bool,
}

impl<M: Mmio> Request<'_, M> {
    /// The command.
    pub fn command(&self) -> u32 {
        self.command
    }

    /// The AXI user of the requester.
    pub fn user(&self) -> u32 {
        self.user
    }

    /// Length of the request data in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the request has no data.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Reads `buf.len()` bytes of the request data, starting at `offset`.
    pub fn read(&self, offset: usize, buf: &mut [u8]) -> Result<(), Error> {
        let end = offset.checked_add(buf.len()).ok_or(Error::OutOfRange)?;
        if end > self.len {
            return Err(Error::OutOfRange);
        }
        // The SRAM can only be read a word at a time.
        let mut word = [0u8; 4];
        for (i, byte) in buf.iter_mut().enumerate() {
            let position = offset.wrapping_add(i);
            if i == 0 || position % 4 == 0 {
                word = self.mbox.mmio.read(Reg::Sram(position / 4)).to_le_bytes();
            }
            *byte = word.get(position % 4).copied().unwrap_or(0);
        }
        let hw_status = LocalRegisterCopy::<u32, MboxHwStatus::Register>::new(
            self.mbox.mmio.read(Reg::HwStatus),
        );
        if hw_status.is_set(MboxHwStatus::EccDoubleError) {
            return Err(Error::Uncorrectable);
        }
        // After a release the SRAM does not hold the request.
        if !self.mbox.executing() {
            return Err(Error::Released);
        }
        Ok(())
    }

    /// Writes `data` as response data, starting at `offset`, which must be a
    /// multiple of 4.
    ///
    /// The SRAM can only be written a word at a time. If the length of
    /// `data` is not a multiple of 4, the rest of its last word is written
    /// as zeros.
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), Error> {
        if offset % 4 != 0 {
            return Err(Error::Unaligned);
        }
        let end = offset.checked_add(data.len()).ok_or(Error::OutOfRange)?;
        if end > self.mbox.mmio.sram_size() {
            return Err(Error::OutOfRange);
        }
        // After a release another requester may hold the lock, and in
        // revision 2.1.0 the MCU can write the SRAM whoever holds it.
        if !self.mbox.executing() {
            return Err(Error::Released);
        }
        // The hardware clears the SRAM up to the largest length that was
        // written, so the length covers the data before the data is written.
        if end > self.covered {
            let length = u32::try_from(end).map_err(|_| Error::OutOfRange)?;
            self.mbox.mmio.write(Reg::Length, length);
            self.covered = end;
        }
        for (i, chunk) in data.chunks(4).enumerate() {
            let mut word = [0u8; 4];
            for (to, from) in word.iter_mut().zip(chunk) {
                *to = *from;
            }
            self.mbox.mmio.write(
                Reg::Sram((offset / 4).wrapping_add(i)),
                u32::from_le_bytes(word),
            );
        }
        Ok(())
    }

    /// Answers the command with `status`. The first `len` bytes of the SRAM
    /// are the response data.
    ///
    /// With [`Error::OutOfRange`] the command fails. With
    /// [`Error::Released`] it has no answer.
    pub fn finish(mut self, status: Status, len: usize) -> Result<(), Error> {
        self.answer(status, len)
    }

    /// Answers the command with `status` and `data` as the response data.
    /// See [`finish`](Self::finish).
    pub fn respond(mut self, status: Status, data: &[u8]) -> Result<(), Error> {
        self.write(0, data)?;
        self.answer(status, data.len())
    }

    fn answer(&mut self, status: Status, len: usize) -> Result<(), Error> {
        if len > self.mbox.mmio.sram_size() {
            return Err(Error::OutOfRange);
        }
        let length = u32::try_from(len).map_err(|_| Error::OutOfRange)?;
        if !self.mbox.executing() {
            return Err(Error::Released);
        }
        // The requester reads the length when it sees the status. In
        // revision 2.2.0 the MCU cannot write the length after the status.
        self.mbox.mmio.write(Reg::Length, length);
        self.mbox.mmio.write(Reg::CommandStatus, status.register());
        self.answered = true;
        Ok(())
    }
}

impl<M: Mmio> Drop for Request<'_, M> {
    fn drop(&mut self) {
        if !self.answered {
            let _ = self.answer(Status::Failure, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;

    /// The AXI user of the MCU in the model.
    const MCU: u32 = 0xffff_ffff;

    /// The AXI user of the requester in the model.
    const SOC: u32 = 0x5;

    /// Size of the model's SRAM in bytes.
    const SRAM_SIZE: usize = 64;

    const BUSY: u32 = 0;
    const DATA_READY: u32 = 1;
    const COMPLETE: u32 = 2;
    const FAILURE: u32 = 3;

    struct State {
        sram: [u32; SRAM_SIZE / 4],
        lock: bool,
        user: u32,
        command: u32,
        length: u32,
        execute: u32,
        command_status: u32,
        hw_status: u32,
        /// The largest length that was written since the lock was taken.
        max_length: u32,
        /// `max_length` at the last release.
        released_length: Option<u32>,
        /// The requester releases the mailbox when the MCU has read this
        /// many registers.
        release_after_reads: Option<usize>,
        /// The writes of the MCU that the mailbox accepted.
        writes: Vec<(Reg, u32)>,
    }

    /// A mailbox as memory, with the rules of `mcu_mbox.sv` for the MCU: a
    /// read of the lock register takes a free lock, writes have no effect
    /// while the lock is free, and a write of 0 to the execute register
    /// clears the registers, fills the SRAM with zeros up to the largest
    /// length that was written, and frees the lock. The model does the
    /// clearing at once.
    struct FakeMbox(RefCell<State>);

    impl FakeMbox {
        /// The mailbox after a reset of the MCI: the MCU holds the lock. The
        /// SRAM holds data from before the reset.
        fn after_reset() -> Self {
            Self(RefCell::new(State {
                sram: [0xdead_beef; SRAM_SIZE / 4],
                lock: true,
                user: MCU,
                command: 0,
                length: 0,
                execute: 0,
                command_status: 0,
                hw_status: 0,
                max_length: 0,
                released_length: None,
                release_after_reads: None,
                writes: Vec::new(),
            }))
        }

        /// A mailbox that the MCU has cleared.
        fn idle() -> Self {
            let mut mbox = Self::after_reset();
            mbox.write(Reg::Execute, 0);
            mbox.0.borrow_mut().writes.clear();
            mbox
        }

        /// A mailbox in which the requester has started `command` with
        /// `data`.
        fn with_command(command: u32, data: &[u8]) -> Self {
            let mbox = Self::idle();
            mbox.send(command, data);
            mbox
        }

        /// The requester takes the lock and starts `command` with `data`.
        fn send(&self, command: u32, data: &[u8]) {
            self.prepare(command, data);
            self.0.borrow_mut().execute = 1;
        }

        /// The requester takes the lock and writes `command` and `data`
        /// without starting the command.
        fn prepare(&self, command: u32, data: &[u8]) {
            let mut state = self.0.borrow_mut();
            assert!(!state.lock, "the mailbox is locked");
            state.lock = true;
            state.user = SOC;
            state.command = command;
            state.length = data.len() as u32;
            state.max_length = data.len() as u32;
            for (word, chunk) in state.sram.iter_mut().zip(data.chunks(4)) {
                let mut bytes = [0u8; 4];
                bytes[..chunk.len()].copy_from_slice(chunk);
                *word = u32::from_le_bytes(bytes);
            }
        }

        /// The accepted writes of the MCU to registers other than the SRAM.
        fn register_writes(&self) -> Vec<(Reg, u32)> {
            let state = self.0.borrow();
            let is_register = |write: &&(Reg, u32)| !matches!(write.0, Reg::Sram(_));
            state.writes.iter().filter(is_register).copied().collect()
        }

        /// The first `len` bytes of the SRAM.
        fn data(&self, len: usize) -> Vec<u8> {
            let state = self.0.borrow();
            let mut bytes: Vec<u8> = state.sram.iter().flat_map(|w| w.to_le_bytes()).collect();
            bytes.truncate(len);
            bytes
        }
    }

    impl State {
        fn release(&mut self) {
            // The hardware clears up to the word of the last byte, and uses
            // only as many bits of the length as the SRAM has address bits.
            let last = (self.max_length.wrapping_sub(1) as usize % SRAM_SIZE) / 4;
            self.sram[..=last].fill(0);
            self.released_length = Some(self.max_length);
            self.lock = false;
            self.user = 0;
            self.command = 0;
            self.length = 0;
            self.execute = 0;
            self.command_status = 0;
            self.hw_status = 0;
            self.max_length = 0;
        }
    }

    impl Mmio for FakeMbox {
        fn sram_size(&self) -> usize {
            SRAM_SIZE
        }

        fn read(&self, reg: Reg) -> u32 {
            let mut state = self.0.borrow_mut();
            match state.release_after_reads {
                Some(0) => {
                    state.release_after_reads = None;
                    state.release();
                }
                Some(reads) => state.release_after_reads = Some(reads - 1),
                None => {}
            }
            match reg {
                Reg::Sram(index) => state.sram.get(index).copied().unwrap_or(0),
                Reg::Lock => {
                    if state.lock {
                        1
                    } else {
                        state.lock = true;
                        state.user = MCU;
                        0
                    }
                }
                Reg::User => state.user,
                Reg::Command => state.command,
                Reg::Length => state.length,
                Reg::Execute => state.execute,
                Reg::CommandStatus => state.command_status,
                Reg::HwStatus => state.hw_status,
            }
        }

        fn write(&mut self, reg: Reg, value: u32) {
            let mut state = self.0.borrow_mut();
            if !state.lock {
                return;
            }
            state.writes.push((reg, value));
            match reg {
                Reg::Sram(index) => {
                    if let Some(word) = state.sram.get_mut(index) {
                        *word = value;
                    }
                }
                Reg::Lock | Reg::User | Reg::HwStatus => {}
                Reg::Command => state.command = value,
                Reg::Length => {
                    state.length = value;
                    state.max_length = state.max_length.max(value);
                }
                Reg::Execute => {
                    state.execute = value & 1;
                    if value & 1 == 0 {
                        state.release();
                    }
                }
                Reg::CommandStatus => state.command_status = value & 0xf,
            }
        }
    }

    fn mailbox(fake: FakeMbox) -> McuMailbox<FakeMbox> {
        McuMailbox::with_mmio(fake)
    }

    fn state(mbox: &McuMailbox<FakeMbox>) -> core::cell::Ref<'_, State> {
        mbox.mmio.0.borrow()
    }

    #[test]
    fn register_offsets_match_the_hardware_map() {
        assert_eq!(MBOX0_OFFSET, 0x40_0000);
        assert_eq!(MBOX1_OFFSET, 0x80_0000);
        assert_eq!(MAX_SRAM_SIZE, 0x20_0000);
        assert_eq!(Reg::Sram(0).offset(), 0);
        assert_eq!(Reg::Sram(3).offset(), 12);
        assert_eq!(Reg::Lock.offset(), 0x20_0000);
        assert_eq!(Reg::User.offset(), 0x20_0004);
        assert_eq!(Reg::Command.offset(), 0x20_0010);
        assert_eq!(Reg::Length.offset(), 0x20_0014);
        assert_eq!(Reg::Execute.offset(), 0x20_0018);
        assert_eq!(Reg::CommandStatus.offset(), 0x20_0020);
        assert_eq!(Reg::HwStatus.offset(), 0x20_0024);
    }

    #[test]
    fn mailbox_1_has_the_layout_of_mailbox_0() {
        use mci::regs::Mci;
        let in_mailbox_1 = |offset_in_mci: usize| offset_in_mci - MBOX1_OFFSET;
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_lock)),
            Reg::Lock.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_user)),
            Reg::User.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_cmd)),
            Reg::Command.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_dlen)),
            Reg::Length.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_execute)),
            Reg::Execute.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_cmd_status)),
            Reg::CommandStatus.offset()
        );
        assert_eq!(
            in_mailbox_1(offset_of!(Mci, mcu_mbox1_csr_mbox_hw_status)),
            Reg::HwStatus.offset()
        );
    }

    #[test]
    fn the_memory_mapped_block_keeps_sram_access_inside_the_sram() {
        // SAFETY: the test only asks for addresses.
        let block = unsafe { MmioBlock::new(0x1000_0000, 18) };
        assert_eq!(block.sram_size(), 16);
        assert_eq!(block.address(Reg::Sram(3)), Some(0x1000_000c));
        assert_eq!(block.address(Reg::Sram(4)), None);
        assert_eq!(block.address(Reg::Sram(usize::MAX)), None);
        assert_eq!(block.address(Reg::Lock), Some(0x1020_0000));

        // SAFETY: as above.
        let block = unsafe { MmioBlock::new(0x1000_0000, usize::MAX) };
        assert_eq!(block.sram_size(), MAX_SRAM_SIZE);
        assert_eq!(block.address(Reg::Sram(MAX_SRAM_SIZE / 4)), None);
    }

    #[test]
    fn clear_releases_the_lock_that_the_mcu_holds_after_reset() {
        let mut mbox = mailbox(FakeMbox::after_reset());
        mbox.clear();
        let state = state(&mbox);
        assert!(!state.lock);
        assert_eq!(state.sram, [0; SRAM_SIZE / 4]);
        assert_eq!(state.released_length, Some(SRAM_SIZE as u32));
    }

    #[test]
    fn clear_fills_the_sram_of_a_free_mailbox_with_zeros() {
        let fake = FakeMbox::idle();
        fake.0.borrow_mut().sram = [0xdead_beef; SRAM_SIZE / 4];
        let mut mbox = mailbox(fake);
        mbox.clear();
        let state = state(&mbox);
        assert!(!state.lock);
        assert_eq!(state.sram, [0; SRAM_SIZE / 4]);
        assert_eq!(state.released_length, Some(SRAM_SIZE as u32));
    }

    #[test]
    fn clear_ends_the_command_of_a_requester() {
        let mut mbox = mailbox(FakeMbox::with_command(7, &[1, 2, 3]));
        mbox.clear();
        assert!(mbox.request().is_none());
        assert!(!state(&mbox).lock);
    }

    #[test]
    fn has_no_request_before_the_requester_executes() {
        let mut mbox = mailbox(FakeMbox::after_reset());
        assert!(mbox.request().is_none());
        mbox.clear();
        assert!(mbox.request().is_none());

        // The requester has the lock and has written its command.
        mbox.mmio.prepare(7, &[1, 2, 3]);
        assert!(mbox.request().is_none());
        assert_eq!(state(&mbox).command_status, BUSY);
    }

    #[test]
    fn returns_the_command_of_the_requester() {
        let mut mbox = mailbox(FakeMbox::with_command(0x1234_5678, &[1, 2, 3, 4, 5]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.command(), 0x1234_5678);
        assert_eq!(request.user(), SOC);
        assert_eq!(request.len(), 5);
        assert!(!request.is_empty());

        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.len(), 0);
        assert!(request.is_empty());
    }

    #[test]
    fn does_not_return_a_command_that_is_released_while_it_is_read() {
        // `request` reads six registers.
        for reads in 0..6 {
            let mut mbox = mailbox(FakeMbox::with_command(0x1234_5678, &[1, 2, 3, 4, 5]));
            mbox.mmio.0.borrow_mut().release_after_reads = Some(reads);
            assert!(mbox.request().is_none(), "release after {reads} reads");
            assert!(mbox.mmio.register_writes().is_empty());
        }

        let mut mbox = mailbox(FakeMbox::with_command(0x1234_5678, &[1, 2, 3, 4, 5]));
        mbox.mmio.0.borrow_mut().release_after_reads = Some(6);
        let request = mbox.request().expect("no request");
        assert_eq!(
            (request.command(), request.user(), request.len()),
            (0x1234_5678, SOC, 5)
        );
    }

    #[test]
    fn does_not_return_request_data_that_was_read_after_a_release() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3, 4, 5]));
        let request = mbox.request().expect("no request");
        request.mbox.mmio.0.borrow_mut().release_after_reads = Some(1);
        assert_eq!(request.read(0, &mut [0; 5]), Err(Error::Released));
    }

    #[test]
    fn reads_any_range_of_the_request() {
        let data: Vec<u8> = (1..=11).collect();
        let mut mbox = mailbox(FakeMbox::with_command(1, &data));
        let request = mbox.request().expect("no request");
        for offset in 0..=data.len() {
            for end in offset..=data.len() {
                let mut buf = vec![0xaa; end - offset];
                assert_eq!(request.read(offset, &mut buf), Ok(()));
                assert_eq!(buf, data[offset..end], "range {offset}..{end}");
            }
        }
    }

    #[test]
    fn does_not_read_outside_the_request() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3, 4, 5]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.read(0, &mut [0; 6]), Err(Error::OutOfRange));
        assert_eq!(request.read(5, &mut [0; 1]), Err(Error::OutOfRange));
        assert_eq!(request.read(6, &mut []), Err(Error::OutOfRange));
        assert_eq!(
            request.read(usize::MAX, &mut [0; 2]),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn reports_an_uncorrectable_error_in_the_request() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3, 4]));
        mbox.mmio.0.borrow_mut().hw_status = 0b01;
        let request = mbox.request().expect("no request");
        assert_eq!(request.read(0, &mut [0; 4]), Ok(()));

        request.mbox.mmio.0.borrow_mut().hw_status = 0b10;
        assert_eq!(request.read(0, &mut [0; 4]), Err(Error::Uncorrectable));
    }

    #[test]
    fn answers_with_data() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[9; 16]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.respond(Status::DataReady, &[1, 2, 3, 4, 5]), Ok(()));
        assert_eq!(state(&mbox).command_status, DATA_READY);
        assert_eq!(state(&mbox).length, 5);
        // The rest of the last word is zero.
        assert_eq!(mbox.mmio.data(8), [1, 2, 3, 4, 5, 0, 0, 0]);
        assert!(state(&mbox).lock);
        // The length is in place when the requester sees the status.
        assert_eq!(
            mbox.mmio.register_writes(),
            [(Reg::Length, 5), (Reg::CommandStatus, DATA_READY)]
        );
    }

    #[test]
    fn answers_with_data_written_in_parts() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        let mut request = mbox.request().expect("no request");
        assert_eq!(request.write(8, &[9, 10]), Ok(()));
        assert_eq!(request.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]), Ok(()));
        assert_eq!(request.finish(Status::Complete, 10), Ok(()));
        assert_eq!(state(&mbox).command_status, COMPLETE);
        assert_eq!(state(&mbox).length, 10);
        assert_eq!(mbox.mmio.data(10), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn answers_without_data() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.finish(Status::Failure, 0), Ok(()));
        assert_eq!(state(&mbox).command_status, FAILURE);
        assert_eq!(state(&mbox).length, 0);
    }

    #[test]
    fn does_not_return_a_command_that_it_answered() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        assert_eq!(request.finish(Status::Complete, 0), Ok(()));
        assert!(mbox.request().is_none());

        // The requester releases the mailbox and sends another command.
        mbox.mmio.write(Reg::Execute, 0);
        assert!(mbox.request().is_none());
        mbox.mmio.send(2, &[4]);
        assert_eq!(mbox.request().expect("no request").command(), 2);
    }

    #[test]
    fn does_not_write_a_response_at_an_unaligned_offset() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        let mut request = mbox.request().expect("no request");
        for offset in [1, 2, 3, 5] {
            assert_eq!(request.write(offset, &[1]), Err(Error::Unaligned));
        }
    }

    #[test]
    fn does_not_write_a_response_that_is_larger_than_the_sram() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        let mut request = mbox.request().expect("no request");
        assert_eq!(request.write(0, &[1; SRAM_SIZE]), Ok(()));
        assert_eq!(
            request.write(0, &[1; SRAM_SIZE + 1]),
            Err(Error::OutOfRange)
        );
        assert_eq!(request.write(SRAM_SIZE, &[1]), Err(Error::OutOfRange));
        assert_eq!(request.write(SRAM_SIZE, &[]), Ok(()));
        assert_eq!(
            request.write(usize::MAX - 3, &[1; 8]),
            Err(Error::OutOfRange)
        );
        assert_eq!(request.finish(Status::DataReady, SRAM_SIZE), Ok(()));
        assert_eq!(state(&mbox).command_status, DATA_READY);
        assert_eq!(state(&mbox).length, SRAM_SIZE as u32);
    }

    #[test]
    fn fails_a_command_whose_answer_is_larger_than_the_sram() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        assert_eq!(
            request.finish(Status::DataReady, SRAM_SIZE + 1),
            Err(Error::OutOfRange)
        );
        assert_eq!(state(&mbox).command_status, FAILURE);
        assert_eq!(state(&mbox).length, 0);
        assert!(mbox.request().is_none());
    }

    #[test]
    fn fails_a_command_whose_request_is_dropped() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        drop(mbox.request().expect("no request"));
        assert_eq!(state(&mbox).command_status, FAILURE);
        assert_eq!(state(&mbox).length, 0);
        assert!(mbox.request().is_none());

        // A response that failed part of the way is not sent.
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        assert_eq!(
            request.respond(Status::DataReady, &[1; SRAM_SIZE + 1]),
            Err(Error::OutOfRange)
        );
        assert_eq!(state(&mbox).command_status, FAILURE);
        assert_eq!(state(&mbox).length, 0);
    }

    #[test]
    fn the_hardware_clears_response_data_that_is_not_sent() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let mut request = mbox.request().expect("no request");
        assert_eq!(request.write(0, &[0xaa; 16]), Ok(()));
        assert_eq!(request.write(16, &[0xbb; 6]), Ok(()));
        assert_eq!(request.write(4, &[0xcc; 4]), Ok(()));
        assert_eq!(request.finish(Status::Failure, 0), Ok(()));
        // The length covers data before the data is written, and a write
        // inside what is covered does not change the length.
        assert_eq!(
            state(&mbox).writes[..2],
            [(Reg::Length, 16), (Reg::Sram(0), 0xaaaa_aaaa)]
        );
        assert_eq!(
            mbox.mmio.register_writes(),
            [
                (Reg::Length, 16),
                (Reg::Length, 22),
                (Reg::Length, 0),
                (Reg::CommandStatus, FAILURE)
            ]
        );

        // The requester releases the mailbox.
        mbox.mmio.write(Reg::Execute, 0);
        assert_eq!(state(&mbox).released_length, Some(22));
        assert_eq!(state(&mbox).sram, [0; SRAM_SIZE / 4]);
    }

    #[test]
    fn does_not_raise_the_length_for_data_inside_the_request() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[9; 16]));
        let mut request = mbox.request().expect("no request");
        assert_eq!(request.write(0, &[1; 16]), Ok(()));
        assert!(request.mbox.mmio.register_writes().is_empty());
    }

    #[test]
    fn does_not_answer_after_the_requester_released_the_mailbox() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        request.mbox.mmio.write(Reg::Execute, 0);
        assert_eq!(request.finish(Status::DataReady, 0), Err(Error::Released));
        assert_eq!(state(&mbox).command_status, BUSY);
    }

    #[test]
    fn does_not_write_into_the_command_of_the_next_requester() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[1, 2, 3]));
        let request = mbox.request().expect("no request");
        // The requester releases the mailbox, and another one takes the lock
        // and writes its request.
        request.mbox.mmio.write(Reg::Execute, 0);
        request.mbox.mmio.0.borrow_mut().writes.clear();
        request.mbox.mmio.prepare(2, &[7; 8]);
        assert_eq!(
            request.respond(Status::DataReady, &[1; 12]),
            Err(Error::Released)
        );
        assert!(mbox.mmio.register_writes().is_empty());
        assert_eq!(mbox.mmio.data(12), [7, 7, 7, 7, 7, 7, 7, 7, 0, 0, 0, 0]);
        assert_eq!(state(&mbox).length, 8);
        assert_eq!(state(&mbox).command_status, BUSY);
    }

    #[test]
    fn fails_a_command_that_is_larger_than_the_sram() {
        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        mbox.mmio.0.borrow_mut().length = SRAM_SIZE as u32 + 1;
        assert!(mbox.request().is_none());
        assert_eq!(
            mbox.mmio.register_writes(),
            [(Reg::Length, 0), (Reg::CommandStatus, FAILURE)]
        );

        let mut mbox = mailbox(FakeMbox::with_command(1, &[]));
        mbox.mmio.0.borrow_mut().length = SRAM_SIZE as u32;
        assert_eq!(mbox.request().expect("no request").len(), SRAM_SIZE);
    }
}
