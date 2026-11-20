// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Driver for the Caliptra mailbox, which the MCU uses to send commands to
//! the Caliptra core.
//!
//! This is a port of the mailbox code in caliptra-mcu-sw
//! `romtime/src/soc_manager.rs` (`CaliptraSoC::start_mailbox_req` and
//! `finish_mailbox_resp`). Like that code, it handles requests and responses
//! as 32-bit words and polls for the response. It does not support
//! interrupts.
//!
//! A request is the checksum word followed by the request body. A response is
//! a checksum word, a FIPS status word and the response body. Both are
//! described in the Caliptra runtime README and defined in caliptra-sw
//! `api/src/mailbox.rs`.
//!
//! A request that does not fit in the mailbox goes through the external
//! mailbox command ([`Mailbox::execute_external`]): the request is in memory
//! that Caliptra reads over AXI, and the mailbox carries its address.
//!
//! The driver also has the commands that replace the running MCU image. A
//! SoC manifest does not fit in the mailbox, so
//! [`Mailbox::auth_manifest`] takes the address of the request, and
//! [`auth_manifest_header`] returns the header that the caller stores in
//! front of the manifest. [`Mailbox::start_mcu_activation`] sends
//! `ACTIVATE_FIRMWARE` for the MCU image.

#![no_std]

use core::ptr::with_exposed_provenance;

use caliptra_api::mailbox::{ActivateFirmwareReq, CommandId, SetAuthManifestReq};
use caliptra_ss_registers::mbox::bits::{MboxExecute, MboxLock, MboxStatus};
use caliptra_ss_registers::mbox::regs::Mbox;
use caliptra_ss_registers::soc::regs::Soc;
use tock_registers::interfaces::{Readable, Writeable};

/// Size of the mailbox in bytes when Caliptra is part of a subsystem
/// (`MBOX_SIZE_SUBSYSTEM` in caliptra-sw).
pub const MAILBOX_SIZE: usize = 16 * 1024;

/// Size in bytes of the header of a `VERIFY_AUTH_MANIFEST` or
/// `SET_AUTH_MANIFEST` request: the checksum and the size of the manifest.
/// The manifest follows the header.
pub const AUTH_MANIFEST_HEADER_SIZE: usize = 8;

/// The largest SoC manifest that Caliptra accepts.
pub const MAX_AUTH_MANIFEST_SIZE: usize = SetAuthManifestReq::MAX_MAN_SIZE;

/// Number of manifest bytes that [`auth_manifest_header`] reads at once.
const CHECKSUM_PART: usize = 64;

/// Maximum number of status polls for one command. caliptra-mcu-sw uses the
/// same limit (`CaliptraSoC::MAX_WAIT_CYCLES`).
const POLL_LIMIT: u32 = 400_000;

/// Errors from the mailbox.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MailboxError {
    /// The request does not fit in the mailbox, or a manifest is larger
    /// than [`MAX_AUTH_MANIFEST_SIZE`].
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

/// An address on Caliptra's AXI bus.
///
/// The MCU cannot use it as a pointer: the MCU has another address for the
/// same memory, or none.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AxiAddr(u64);

impl AxiAddr {
    pub const fn new(address: u64) -> Self {
        Self(address)
    }

    /// The low 32 bits of the address.
    pub const fn lo(self) -> u32 {
        self.0 as u32
    }

    /// The high 32 bits of the address.
    pub const fn hi(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// The address `bytes` after this one, or `None` if it does not fit in
    /// 64 bits.
    pub const fn checked_add(self, bytes: u64) -> Option<Self> {
        match self.0.checked_add(bytes) {
            Some(address) => Some(Self(address)),
            None => None,
        }
    }
}

/// A command that takes a SoC manifest.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AuthManifestCommand {
    /// `VERIFY_AUTH_MANIFEST`: Caliptra checks the signatures of the
    /// manifest. The manifest that Caliptra uses stays the same.
    Verify,
    /// `SET_AUTH_MANIFEST`: Caliptra checks the signatures of the manifest
    /// and then authorizes images against it.
    Set,
}

impl AuthManifestCommand {
    const fn id(self) -> u32 {
        match self {
            Self::Verify => CommandId::VERIFY_AUTH_MANIFEST.0,
            Self::Set => CommandId::SET_AUTH_MANIFEST.0,
        }
    }
}

/// Returns the header of the request of `command` for a manifest of
/// `manifest_size` bytes.
///
/// `read(offset, buf)` fills `buf` with the bytes of the manifest at
/// `offset` and returns false if it cannot. The checksum in the header covers
/// the command and the manifest, so the two commands have different headers
/// for one manifest.
///
/// The request that [`Mailbox::auth_manifest`] sends is this header and then
/// the manifest. Caliptra reads the request as words: if the size of the
/// manifest is not a multiple of 4, the bytes after the manifest up to the
/// next multiple are part of the request and must be 0.
///
/// Returns `None` if the manifest is larger than [`MAX_AUTH_MANIFEST_SIZE`]
/// or `read` fails.
pub fn auth_manifest_header(
    command: AuthManifestCommand,
    manifest_size: usize,
    mut read: impl FnMut(usize, &mut [u8]) -> bool,
) -> Option<[u8; AUTH_MANIFEST_HEADER_SIZE]> {
    if manifest_size > MAX_AUTH_MANIFEST_SIZE {
        return None;
    }
    let size = manifest_size as u32;
    // The checksum is 0 minus the sum of the bytes of the command and of
    // the request after its first word.
    let mut sum = byte_sum(command.id()).wrapping_add(byte_sum(size));
    let mut buf = [0u8; CHECKSUM_PART];
    let mut offset = 0;
    while offset < manifest_size {
        let part = buf.get_mut(..(manifest_size - offset).min(CHECKSUM_PART))?;
        if !read(offset, part) {
            return None;
        }
        sum = part
            .iter()
            .fold(sum, |sum, &byte| sum.wrapping_add(byte as u32));
        offset += part.len();
    }
    let [c0, c1, c2, c3] = 0u32.wrapping_sub(sum).to_le_bytes();
    let [s0, s1, s2, s3] = size.to_le_bytes();
    Some([c0, c1, c2, c3, s0, s1, s2, s3])
}

/// The Caliptra mailbox.
///
/// The methods take `&self`, so several drivers can hold a reference to one
/// `Mailbox`. Only one command can be in progress: [`send`](Self::send)
/// returns [`MailboxError::Busy`] until the response to the previous command
/// has been received.
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
        &self,
        cmd: u32,
        body: impl Iterator<Item = u32> + Clone,
        response: &mut [u32],
    ) -> Result<usize, MailboxError> {
        self.send(cmd, body)?;
        self.receive(response)
    }

    /// Sends command `cmd` and waits for its response. See [`send`](Self::send)
    /// and [`receive_with`](Self::receive_with).
    pub fn execute_with(
        &self,
        cmd: u32,
        body: impl Iterator<Item = u32> + Clone,
        sink: impl FnMut(usize, u32),
    ) -> Result<usize, MailboxError> {
        self.send(cmd, body)?;
        self.receive_with(sink)
    }

    /// Sends command `cmd`, whose request is in memory that Caliptra reads
    /// over AXI, and waits for its response. See [`receive`](Self::receive).
    ///
    /// `address` is the address of the request, and `size` is its size in
    /// bytes, a multiple of 4. The request starts with
    /// its checksum word, as a request in the mailbox does.
    pub fn execute_external(
        &self,
        cmd: u32,
        address: AxiAddr,
        size: u32,
        response: &mut [u32],
    ) -> Result<usize, MailboxError> {
        // The body of ExternalMailboxCmdReq: the command, the size of its
        // request and the address of its request.
        let body = [cmd, size, address.lo(), address.hi()];
        self.execute(
            CommandId::EXTERNAL_MAILBOX_CMD.0,
            body.into_iter(),
            response,
        )
    }

    /// Sends `command`, whose request is at `address`, and waits until
    /// Caliptra has completed it.
    ///
    /// The request is the header that [`auth_manifest_header`] returns for
    /// `command`, then the manifest of `manifest_size` bytes.
    pub fn auth_manifest(
        &self,
        command: AuthManifestCommand,
        address: AxiAddr,
        manifest_size: usize,
    ) -> Result<(), MailboxError> {
        if manifest_size > MAX_AUTH_MANIFEST_SIZE {
            return Err(MailboxError::RequestTooLarge);
        }
        let size = (AUTH_MANIFEST_HEADER_SIZE + manifest_size).next_multiple_of(4);
        let mut response = [0u32; 2];
        match self.execute_external(command.id(), address, size as u32, &mut response) {
            // The commands return no data.
            Ok(_) | Err(MailboxError::NoResponse) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Sends `ACTIVATE_FIRMWARE` for an MCU image of `image_size` bytes,
    /// without waiting for the response. The image is at the staging
    /// address that Caliptra's SoC manifest gives for the MCU image.
    ///
    /// Caliptra does not complete the command while the MCU runs. It raises
    /// the reset request of the MCI and waits for the MCU to acknowledge it
    /// and to ask the MCI for the MCU reset. It then copies the image to MCU
    /// SRAM, authorizes it against the SoC manifest and releases the MCU.
    /// The mailbox stays locked through the reset, and the caliptra-mcu-sw
    /// ROM receives the response.
    pub fn start_mcu_activation(&self, image_size: u32) -> Result<(), MailboxError> {
        // The body of ActivateFirmwareReq: the number of image identifiers,
        // the identifiers, the size of the MCU image and the flags.
        let ids = core::iter::once(ActivateFirmwareReq::MCU_IMAGE_ID).chain(core::iter::repeat_n(
            0,
            ActivateFirmwareReq::MAX_FW_ID_COUNT - 1,
        ));
        let body = core::iter::once(1).chain(ids).chain([image_size, 0]);
        self.send(CommandId::ACTIVATE_FIRMWARE.0, body)
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
    /// or [`receive_with`](Self::receive_with) is called.
    pub fn send(
        &self,
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
    pub fn receive(&self, response: &mut [u32]) -> Result<usize, MailboxError> {
        self.receive_with(|index, word| {
            if let Some(slot) = response.get_mut(index) {
                *slot = word;
            }
        })
    }

    /// Waits for the response to the command that was sent, then releases the
    /// mailbox.
    ///
    /// Calls `sink` with the index and the value of each word of the
    /// response, in order. The words include the checksum and FIPS status
    /// words. A caller can use `sink` to store a long response in more than
    /// one buffer.
    ///
    /// `sink` receives each word before the checksum is checked, including
    /// the words of a response that fails with
    /// [`MailboxError::InvalidChecksum`].
    ///
    /// Returns the length of the response in bytes.
    pub fn receive_with(&self, sink: impl FnMut(usize, u32)) -> Result<usize, MailboxError> {
        let result = self.read_response(sink);
        // Clearing the execute register ends the command and releases the
        // lock. This is also done after an error so that the next command can
        // take the lock.
        self.mbox.mbox_execute.write(MboxExecute::Execute::CLEAR);
        result
    }

    /// Waits for Caliptra to finish the command, then reads the response.
    fn read_response(&self, mut sink: impl FnMut(usize, u32)) -> Result<usize, MailboxError> {
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
            sink(i, word);
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
