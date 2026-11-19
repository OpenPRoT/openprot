# MCU mailbox

The MCI has two MCU mailboxes. Through a mailbox, an agent of the SoC sends a
command to the MCU and gets a response. A mailbox is part of the register
space of the MCI, which is reached only over the AXI bus of the subsystem. An
agent can use a mailbox only if it is on that bus with the MCU.

This page describes how a mailbox works and what the driver does. The driver
is
[`target/veer/peripherals/mcu_mbox.rs`](https://github.com/OpenPRoT/openprot/blob/main/target/veer/peripherals/mcu_mbox.rs),
`veer_mcu_mbox`. The OpenPRoT runtime image does not use the mailboxes. It
does not take the step that opens a mailbox to agents after a reset (see
[A command](#a-command)), so no agent can send it a command.

The page describes hardware revision 2.1.0 of the MCI, which is what the MCI
of the FPGA product reports. The behavior comes from the RTL of that
revision: `src/mci/rtl/mcu_mbox.sv`, `src/mci/rtl/mcu_mbox_csr.rdl`,
`src/mci/rtl/mci_reg_top.sv`, `src/mci/rtl/mci_top.sv` and
`src/libs/rtl/sram_zeroization_gadget.sv` in the revision of
[caliptra-ss](https://github.com/chipsalliance/caliptra-ss) that
[caliptra-mcu-sw](https://github.com/chipsalliance/caliptra-mcu-sw) includes
as `hw/caliptra-ss`. [Revision 2.2.0](#revision-220-of-the-hardware) changes
who can write a mailbox.

## Parts of a mailbox

A mailbox is an SRAM and a set of registers. Mailbox 0 is `0x40_0000` above
the base of the MCI block and mailbox 1 is `0x80_0000` above it. The SRAM is
at the start of a mailbox, and the registers are 2 MiB above it.

The SRAM holds the request data and then the response data. Its size is a
parameter of the hardware for each mailbox, up to 2 MiB. A mailbox with a
size of 0 does not exist. The FPGA product has 16 KiB for each mailbox, and
the emulator has 2 MiB. The SRAM is read and written in 32-bit words.

Every access on the bus of the subsystem carries a number that identifies
where it comes from, its AXI user.

| Register | Use |
|---|---|
| `mbox_lock` | A read returns 0 and takes the lock if the lock is free. It returns 1 if the lock is held. |
| `mbox_user` | The AXI user that holds the lock. |
| `mbox_cmd` | The command. |
| `mbox_dlen` | The length in bytes of the request data, and then of the response data. |
| `mbox_execute` | The holder of the lock sets it to start the command and clears it to release the mailbox. |
| `mbox_cmd_status` | The status of the command: `CMD_BUSY` (0), `DATA_READY`, `CMD_COMPLETE` or `CMD_FAILURE`. Only the MCU can write it. |
| `mbox_hw_status` | Whether a read of the SRAM had an ECC error since the lock was taken. |
| `mbox_target_user`, `mbox_target_user_valid`, `mbox_target_status` | Let the MCU give a third AXI user, the target user, access to a request. The driver does not use them. |

The MCI accepts the MCU, the default user `0xFFFF_FFFF`, and up to five more
AXI users at a mailbox. Each of the five is a parameter of the hardware or
the value of a register of the MCI that has been locked. The caliptra-mcu-sw
ROM writes and locks these registers. Any other user gets a bus error. The
registers and the SRAM can only be written while the mailbox is locked. The
MCU can write them whoever holds the lock.

## A command

The agent that sends a command is the requester.

1. The requester reads `mbox_lock` until it reads 0. It now holds the lock.
2. The requester writes the command, the length and the request data, and
   sets `mbox_execute`. Unless the requester has the AXI user of the MCU,
   the MCI raises the notification `MBOX0_COMMAND_AVAILABLE` or
   `MBOX1_COMMAND_AVAILABLE` (see
   [MCI events and errors](./mci-events.md#notifications)).
3. The MCU reads the request. It writes the response data over the request
   in the SRAM, writes the length of the response, and writes the status.
4. The requester reads `mbox_cmd_status` until it is not `CMD_BUSY`, reads
   the response, and clears `mbox_execute`. This releases the mailbox.
5. The MCI clears the registers. It fills the SRAM with zeros, from its start
   to the largest length that was written while the mailbox was locked, or
   all of it if that length is 0. Then it frees the lock. Until then
   `mbox_lock` reads 1 and writes to the mailbox have no effect.

In step 5 the MCI uses only as many bits of the length as the SRAM has
address bits. If a length that is larger than the SRAM was written, the MCI
clears less than the data that the SRAM holds.

When the MCI comes out of reset, the MCU holds the lock. The MCU writes the
size of the SRAM as the length and clears `mbox_execute`, so that the MCI
clears the whole SRAM and frees the lock.

The emulator differs in step 5. It clears the SRAM at once, and when the
largest length is 0 it clears nothing.

## The driver

The driver gives the MCU the receiving side of a command.

| Function | What it does |
|---|---|
| `McuMailbox::clear` | Releases the mailbox with the size of the SRAM as the length, as the MCU does after a reset of the MCI. If a requester holds the lock, its command ends without an answer. |
| `McuMailbox::request` | Returns the command that is waiting for an answer, if there is one, as a `Request`. |
| `Request::command`, `user`, `len` | The command, the AXI user of the requester, and the length of the request data. |
| `Request::read` | Copies a range of the request data. It fails with `Uncorrectable` if `mbox_hw_status` reports an ECC error that the hardware did not correct. |
| `Request::write` | Writes response data at an offset that is a multiple of 4. |
| `Request::finish` | Writes the length of the response and the status. |
| `Request::respond` | Writes the response data, its length and the status. |

A command fails, with `CMD_FAILURE` and a length of 0, in three cases: its
length is larger than the SRAM, its `Request` is dropped without a call of
`finish` or `respond`, or the response is larger than the SRAM.

Before `write` puts data past the largest length of the command so far, it
writes the end of that data to `mbox_dlen`. The MCI then clears the data on
release even if the response that is sent is shorter.

The driver does not use the interrupt of the MCI. Its user can poll
`request`, or wait for the notification with the `veer_mci` driver.

The driver takes the size of the SRAM from its caller. The sizes of the two
mailboxes of each product are in
[`target/veer/tooling/render_system_config.rs`](https://github.com/OpenPRoT/openprot/blob/main/target/veer/tooling/render_system_config.rs).
`veer_system_config` renders them into the system config of an image as
`@MCU_MBOX0_SRAM_SIZE@` and `@MCU_MBOX1_SRAM_SIZE@`. The image uses a size
for the memory mapping of the SRAM and gives it to its app as a constant.
The MCI reports the sizes in `HW_CONFIG0`, except on the emulator, which
reports 0.

The driver does not send commands from the MCU, and it does not use the
registers for a target user.

### A requester that releases the mailbox early

In revision 2.1.0 a requester can clear `mbox_execute` at any time, and the
MCI then clears the registers. The driver reads `mbox_execute` to check that
the command is still there:

- `request` reads it before and after it reads the other registers, and
  returns nothing if either read is 0.
- `read` reads it after it has read the data, and `write` before it writes.
  Both fail with `Released` if it is 0.
- `finish` and `respond` read it before they write the answer, and fail with
  `Released` if it is 0.

These checks do not cover every case, because a mailbox has no identifier for
a command and the MCU can write a mailbox that another agent has locked:

- If the requester releases the mailbox between a check and the write that
  follows it, the write is dropped, or it goes into the mailbox of a
  requester that has taken the lock since.
- If the requester releases the mailbox and a requester starts a command
  before the next check, the driver takes the two commands for one. It can
  return data of the second command for the first, and its answer goes to
  the second.

## Revision 2.2.0 of the hardware

The RTL of caliptra-ss for hardware revision 2.2.0 gives the SRAM and the
registers `mbox_cmd`, `mbox_dlen` and `mbox_execute` to one owner at a time.
No other AXI user can write them or read the SRAM, and that includes the MCU.

- The holder of the lock is the owner until it sets `mbox_execute`.
- The MCU is the owner from then until it writes a status other than
  `CMD_BUSY`, unless it passes the request to a target user.
- The holder of the lock is the owner again after that.

The requester cannot release the mailbox while the MCU is the owner, so the
cases of [an early release](#a-requester-that-releases-the-mailbox-early) do
not occur. A length that is larger than the SRAM clears the whole SRAM.

The driver writes the length before the status, which this revision needs
because the MCU cannot write the length afterwards. `McuMailbox::clear`
releases a mailbox only while the MCU is its owner: when the MCU holds the
lock, as it does after a reset of the MCI, or while a command waits for an
answer.

Neither product has this revision, and the driver is not tested on it.
