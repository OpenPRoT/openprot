// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for the Caliptra mailbox, which the MCU uses to send commands to
//! the Caliptra core.
//!
//! This is a port of the mailbox code in caliptra-mcu-sw
//! `romtime/src/soc_manager.rs` (`CaliptraSoC::start_mailbox_req` and
//! `finish_mailbox_resp`). Like that code, it handles requests and responses
//! as 32-bit words and polls for the response. It does not support
//! interrupts, or the external mailbox command, which passes a large request
//! by address.
//!
//! A request is the checksum word followed by the request body. A response is
//! a checksum word, a FIPS status word and the response body. Both are
//! described in the Caliptra runtime README and defined in caliptra-sw
//! `api/src/mailbox.rs`.

#![no_std]

use core::ptr::with_exposed_provenance;

use caliptra_ss_registers::mbox::bits::{MboxExecute, MboxLock, MboxStatus};
use caliptra_ss_registers::mbox::regs::Mbox;
use caliptra_ss_registers::soc::regs::Soc;
use tock_registers::interfaces::{Readable, Writeable};

/// Size of the mailbox in bytes when Caliptra is part of a subsystem
/// (`MBOX_SIZE_SUBSYSTEM` in caliptra-sw).
pub const MAILBOX_SIZE: usize = 16 * 1024;

/// Maximum number of status polls for one command. caliptra-mcu-sw uses the
/// same limit (`CaliptraSoC::MAX_WAIT_CYCLES`).
const POLL_LIMIT: u32 = 400_000;

/// Errors from the mailbox.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MailboxError {
    /// The request does not fit in the mailbox.
    RequestTooLarge,
    /// The mailbox lock is held, by another user of the mailbox or by a
    /// command that was sent and whose response was not received.
    Busy,
    /// The lock register read 0 again after the read that takes the lock.
    /// The mailbox registers are probably not readable, for example because
    /// Caliptra does not accept the AXI user of the MCU.
    Unreadable,
    /// Caliptra did not complete the command within the poll limit.
    Timeout,
    /// Caliptra reported that the command failed. The value is Caliptra's
    /// fatal firmware error register, or its non-fatal one if the fatal one
    /// is zero.
    CommandFailed(u32),
    /// The command completed without a response.
    NoResponse,
    /// The mailbox status was not `DataReady`, `CmdComplete` or
    /// `CmdFailure`. The value is the status register.
    UnknownStatus(u32),
    /// The checksum of the response is wrong.
    InvalidChecksum,
}

/// The Caliptra mailbox.
pub struct Mailbox {
    mbox: &'static Mbox,
    soc: &'static Soc,
}

impl Mailbox {
    /// `mbox_base` and `soc_ifc_base` are the addresses of the Caliptra
    /// mailbox registers and SoC interface registers (`mbox_offset` and
    /// `soc_offset` in the caliptra-mcu-sw memory map).
    ///
    /// # Safety
    ///
    /// Both register blocks must be at the given addresses and stay mapped
    /// for the caller, and no other code on the MCU may use the mailbox.
    pub unsafe fn new(mbox_base: usize, soc_ifc_base: usize) -> Self {
        // SAFETY: the caller guarantees that the register blocks are at these
        // addresses and stay mapped. Their fields are only accessed with
        // volatile reads and writes.
        unsafe {
            Self {
                mbox: &*with_exposed_provenance(mbox_base),
                soc: &*with_exposed_provenance(soc_ifc_base),
            }
        }
    }

    /// Sends command `cmd` and waits for its response. See [`send`](Self::send)
    /// and [`receive`](Self::receive).
    pub fn execute(
        &mut self,
        cmd: u32,
        body: impl Iterator<Item = u32> + Clone,
        response: &mut [u32],
    ) -> Result<usize, MailboxError> {
        self.send(cmd, body)?;
        self.receive(response)
    }

    /// Sends command `cmd` without waiting for its response.
    ///
    /// `body` is the request without its first word. The first word is the
    /// checksum, which this function computes and sends.
    ///
    /// The length of the request is always a multiple of 4 bytes. A request
    /// whose last field has a variable length, such as `CmShaUpdateReq`,
    /// gives that length in an earlier field, so Caliptra does not count the
    /// zeros that pad the last word. A command that takes its data length
    /// from the length of the request cannot be sent with this function
    /// unless that length is a multiple of 4.
    ///
    /// On success the mailbox stays locked until [`receive`](Self::receive)
    /// is called.
    pub fn send(
        &mut self,
        cmd: u32,
        body: impl Iterator<Item = u32> + Clone,
    ) -> Result<(), MailboxError> {
        // The checksum is 0 minus the sum of the bytes of `cmd` and `body`.
        let mut sum = byte_sum(cmd);
        let mut words: usize = 1;
        for word in body.clone() {
            sum = sum.wrapping_add(byte_sum(word));
            words = words.saturating_add(1);
        }
        if words > MAILBOX_SIZE / 4 {
            return Err(MailboxError::RequestTooLarge);
        }

        // Reading 0 from the lock register takes the lock.
        if self.mbox.mbox_lock.is_set(MboxLock::Lock) {
            return Err(MailboxError::Busy);
        }
        // The lock is now held, so the register reads 1.
        if !self.mbox.mbox_lock.is_set(MboxLock::Lock) {
            return Err(MailboxError::Unreadable);
        }
        self.mbox.mbox_cmd.set(cmd);
        self.mbox.mbox_dlen.set((words * 4) as u32);
        self.mbox.mbox_datain.set(0u32.wrapping_sub(sum));
        for word in body {
            self.mbox.mbox_datain.set(word);
        }
        self.mbox.mbox_execute.write(MboxExecute::Execute::SET);
        Ok(())
    }

    /// Waits for the response to the command that was sent, then releases the
    /// mailbox.
    ///
    /// The response, including its checksum and FIPS status words, is stored
    /// in `response`. Words that do not fit are discarded.
    ///
    /// Returns the length of the response in bytes, which can be more than
    /// `response` holds.
    pub fn receive(&mut self, response: &mut [u32]) -> Result<usize, MailboxError> {
        let result = self.read_response(response);
        // Clearing the execute register ends the command and releases the
        // lock. This is also done after an error so that the next command can
        // take the lock.
        self.mbox.mbox_execute.write(MboxExecute::Execute::CLEAR);
        result
    }

    /// Waits for Caliptra to finish the command, then reads the response.
    fn read_response(&mut self, response: &mut [u32]) -> Result<usize, MailboxError> {
        use MboxStatus::Status::Value;
        match self.wait()? {
            Some(Value::DataReady) => {}
            Some(Value::CmdComplete) => return Err(MailboxError::NoResponse),
            Some(Value::CmdFailure) => {
                let fatal = self.soc.cptra_fw_error_fatal.get();
                let code = if fatal != 0 {
                    fatal
                } else {
                    self.soc.cptra_fw_error_non_fatal.get()
                };
                return Err(MailboxError::CommandFailed(code));
            }
            _ => return Err(MailboxError::UnknownStatus(self.mbox.mbox_status.get())),
        }

        let len = self.mbox.mbox_dlen.get() as usize;
        let mut checksum = 0;
        let mut sum: u32 = 0;
        for i in 0..len.div_ceil(4) {
            let mut word = self.mbox.mbox_dataout.get();
            if i == 0 {
                checksum = word;
            } else {
                // Only the first `len` bytes of the response are valid.
                let mut bytes = word.to_le_bytes();
                let valid = len.saturating_sub(i * 4);
                bytes.iter_mut().skip(valid).for_each(|byte| *byte = 0);
                word = u32::from_le_bytes(bytes);
                sum = sum.wrapping_add(byte_sum(word));
            }
            if let Some(slot) = response.get_mut(i) {
                *slot = word;
            }
        }
        if checksum != 0u32.wrapping_sub(sum) {
            return Err(MailboxError::InvalidChecksum);
        }
        Ok(len)
    }

    /// Polls until the mailbox status is no longer busy and returns the
    /// status.
    fn wait(&self) -> Result<Option<MboxStatus::Status::Value>, MailboxError> {
        use MboxStatus::Status::Value;
        for _ in 0..POLL_LIMIT {
            let status = self.mbox.mbox_status.read_as_enum(MboxStatus::Status);
            if status != Some(Value::CmdBusy) {
                return Ok(status);
            }
            core::hint::spin_loop();
        }
        Err(MailboxError::Timeout)
    }
}

/// Returns the sum of the bytes of `word`.
fn byte_sum(word: u32) -> u32 {
    word.to_le_bytes().iter().map(|&byte| byte as u32).sum()
}
