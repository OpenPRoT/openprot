# MCI events and errors

The MCI (Manufacturer Control Interface) is the block of the Caliptra
Subsystem that holds the MCU's control and status registers. It reports
hardware errors and other events to the MCU, and it reports errors to the SoC
that contains the subsystem. This page lists the events and errors, what
causes each one, and what the OpenPRoT runtime image does about it.

The code is in two places:

- [`target/veer/peripherals/mci.rs`](https://github.com/OpenPRoT/openprot/blob/main/target/veer/peripherals/mci.rs)
  is the driver, `veer_mci`. The names in the tables below are its constants.
- [`target/veer/runtime/target.rs`](https://github.com/OpenPRoT/openprot/blob/main/target/veer/runtime/target.rs)
  is the runtime image. Its kernel enables and handles the MCI interrupt.

The causes on this page come from the RTL of the Caliptra Subsystem:
`src/mci/rtl/mci_reg.rdl`, `src/mci/rtl/mci_reg_top.sv` and
`src/integration/rtl/caliptra_ss_top.sv` in the revision of
[caliptra-ss](https://github.com/chipsalliance/caliptra-ss) that
[caliptra-mcu-sw](https://github.com/chipsalliance/caliptra-mcu-sw) includes
as `hw/caliptra-ss`. The MCI of that revision has hardware revision 2.1.0,
which is also what the MCI of the FPGA product reports. The causes are not
verified against the bitstream of the FPGA product.

## Reporting paths

The MCI reports through three paths: its interrupt to the MCU, its error
records, and its two error outputs to the SoC.

Two resets matter for what the MCI keeps. A warm reset (`mci_rst_b`) resets
the MCI while the subsystem stays powered. A power cycle (`mci_pwrgood`)
resets everything.

### The interrupt to the MCU

The MCI has one interrupt, which is IRQ 1 of the MCU's interrupt controller.
Its events are in four groups. Each group has an enable register, a status
register and a trigger register in the MCI's interrupt block:

| Group | Enable | Status | Trigger | Driver type |
|---|---|---|---|---|
| Error events | `error0_intr_en_r` | `error0_internal_intr_r` | `error0_intr_trig_r` | `ErrorEvents` |
| Notifications | `notif0_intr_en_r` | `notif0_internal_intr_r` | `notif0_intr_trig_r` | `Notifications` |
| Aggregated fatal errors | `error1_intr_en_r` | `error1_internal_intr_r` | `error1_intr_trig_r` | `AggregatedErrors` |
| Aggregated non-fatal errors | `notif1_intr_en_r` | `notif1_internal_intr_r` | `notif1_intr_trig_r` | `AggregatedErrors` |

The aggregated errors are errors of the other blocks of the subsystem; see
[Aggregated errors](#aggregated-errors).

An event sets its bit in the status register of its group, and the event is
then pending. The interrupt is asserted while a pending event has its enable
bit set. Firmware acknowledges an event by writing 1 to its status bit.
Writing 1 to a bit of a trigger register makes the event pending as if it had
happened, which is how the tests raise events.

The status bits of the error events and of the aggregated fatal errors keep
their value through a warm reset. A power cycle clears them. A warm reset
clears the status bits of the other two groups.

### The error records

| Register | Holds | Firmware clears a bit by |
|---|---|---|
| `HW_ERROR_FATAL`, `HW_ERROR_NON_FATAL` | Errors of the MCI's own hardware | Writing 1 to it |
| `AGG_ERROR_FATAL`, `AGG_ERROR_NON_FATAL` | The aggregated errors | Writing 1 to it |
| `FW_ERROR_FATAL`, `FW_ERROR_NON_FATAL` | A code that firmware writes | Writing the register |
| `FW_EXTENDED_ERROR_INFO` | Eight words of details that firmware writes | Writing the register |

All of these keep their value through a warm reset. A power cycle clears
them.

### The error outputs to the SoC

A bit that becomes set in `HW_ERROR_FATAL`, `AGG_ERROR_FATAL` or
`FW_ERROR_FATAL` asserts `mci_error_fatal`, unless the mask register of that
record (`internal_hw_error_fatal_mask`, `internal_agg_error_fatal_mask` or
`internal_fw_error_fatal_mask`) masks the bit. The output stays asserted until
a warm reset or a power cycle.

A bit that becomes set in `HW_ERROR_NON_FATAL`, `AGG_ERROR_NON_FATAL` or
`FW_ERROR_NON_FATAL` asserts `mci_error_non_fatal`, unless the matching
non-fatal mask register masks the bit. The output is deasserted when firmware
has cleared or masked every bit of the three non-fatal records.

## Runtime image behavior

The image enables the MCI interrupt for two kinds of events. The first kind
is the events that stop the image, which the tables below mark as *Stops*.
The second is one notification, `CPTRA_MCU_RESET_REQUEST`: Caliptra, the root
of trust of the subsystem, requests an MCU reset.

When the kernel has started, it:

1. Logs the four hardware and aggregated records and the two firmware error
   codes on the console, if any of them is not 0. The image does not clear
   the records, so it logs them at every start until a power cycle.
2. Logs and acknowledges the pending events that stop the image. They may
   come from an earlier boot, because their status bits keep their value
   through a warm reset. An aggregated error whose input is still asserted
   becomes pending again, and it then stops the image as soon as step 3
   enables its interrupt.
3. Enables the MCI interrupt for the two kinds of events.

When an event that stops the image interrupts the MCU, the kernel's handler
for the MCI interrupt:

1. Writes the pending events that stop the image to
   `FW_EXTENDED_ERROR_INFO`.
2. Writes the code `0xf000_0001` to `FW_ERROR_FATAL`. This write asserts
   `mci_error_fatal`, if the hardware has not asserted it already.
3. Spins with interrupts off and does not return.

The handler does not log, because the thread that it interrupted may be
using the console. It does not acknowledge the events. After a warm reset
they are still pending, and the image logs them when it starts.

The SoC can read the code and the details. The image runs again only after
the SoC resets or power-cycles the subsystem.

When Caliptra requests an MCU reset, the handler acknowledges the request
and writes the `RESET_REQUEST` register of the MCI to ask for the reset.
Caliptra makes this request when it is about to replace the image in MCU
SRAM.

### Fatal error codes

| `FW_ERROR_FATAL` | Meaning | `FW_EXTENDED_ERROR_INFO` |
|---|---|---|
| `0xf000_0001` | An event that stops the image interrupted the MCU. | Word 0: the pending error events that stop the image, as bits of `ErrorEvents`. Word 1: the pending aggregated fatal errors that stop the image, as bits of `AggregatedErrors`. Words 2 to 7: 0. |

The code is in the range that caliptra-mcu-sw keeps for vendors
(`McuError::new_vendor`). The ROM writes its own codes to the same register;
they are the `McuError` constants in `error/src/lib.rs` of caliptra-mcu-sw.

## Error events

In the image column, *Stops* means that the image enables the interrupt for
the event and stops when the event happens. *Not enabled* means that the
image leaves the interrupt for the event off.

| Bit | `ErrorEvents` | Cause | Record | Image |
|---|---|---|---|---|
| 0 | `MCU_SRAM_DMI_AXI_COLLISION` | An AXI access to MCU SRAM and a data access to MCU SRAM through the debug module happened in the same cycle. | `HW_ERROR_FATAL` bit 2 | Stops |
| 1 | `INTERNAL` | None. The RTL ties the source of this event to 0, so only the trigger register raises it. The runtime test raises it. | None | Stops |
| 2 | `MBOX0_ECC_UNCORRECTED` | The SRAM of MCU mailbox 0 had an ECC error that was not corrected. | `HW_ERROR_NON_FATAL` bit 0 | Not enabled |
| 3 | `MBOX1_ECC_UNCORRECTED` | The SRAM of MCU mailbox 1 had an ECC error that was not corrected. | `HW_ERROR_NON_FATAL` bit 1 | Not enabled |
| 4 | `WDT_TIMER1_TIMEOUT` | Watchdog timer 1 reached its period. | None | Not enabled |
| 5 | `WDT_TIMER2_TIMEOUT` | Watchdog timer 2 reached its period. | None | Not enabled |

The image stops on `INTERNAL` because the MCI defines it as an error of the
MCI itself, although this revision of the hardware never raises it. The
image does not enable the mailbox errors because it does not use the MCU
mailboxes. It does not enable the watchdog events because it disables the
watchdog that the ROM starts.

## Aggregated errors

The MCI has 32 inputs for fatal errors and 32 for non-fatal errors from the
other blocks of the subsystem. An asserted input sets its bit in
`AGG_ERROR_FATAL` or `AGG_ERROR_NON_FATAL` and makes the aggregated error
pending. `caliptra_ss_top.sv` gives each source a range of inputs, the same
range for fatal and for non-fatal errors, and ties the inputs that the table
does not name to 0. Each `AggregatedErrors` constant is the set of all inputs
in one range.

| Inputs | `AggregatedErrors` | Fatal cause | Non-fatal cause | Image (fatal inputs) |
|---|---|---|---|---|
| 0 to 5 | `CALIPTRA` | Input 0: the fatal error output of the Caliptra core (`cptra_error_fatal`). | Input 0: the non-fatal error output of the Caliptra core (`cptra_error_non_fatal`). | Stops |
| 6 to 11 | `MCU` | Input 6: an ECC error in the MCU's DCCM (its data memory) that was not corrected. | Input 6: an ECC error in the MCU's DCCM that was corrected. | Stops |
| 12 to 17 | `LIFECYCLE_CONTROLLER` | Inputs 12 to 14: the alerts of the lifecycle controller. | The same alerts. | Not enabled |
| 18 to 23 | `FUSE_CONTROLLER` | Inputs 18 to 22: the alerts of the fuse controller. Input 23: the error interrupt of the fuse controller. | The same alerts and interrupt. | Not enabled |
| 24 to 29 | `I3C` | Input 24: the `escalated_reset_o` output of the I3C core. Input 25: its `peripheral_reset_o` output. | The same outputs. | Not enabled |
| 30 and 31 | `SPARE` | None. The RTL ties these inputs to 0. | None. | Not enabled |

The image does not enable the interrupt for any aggregated non-fatal error.

`caliptra_ss_top.sv` connects the alerts of the lifecycle controller and of
the fuse controller, and the two outputs of the I3C core, to the fatal and to
the non-fatal inputs alike. `mci_reg.rdl` names the alerts:

| Input | Lifecycle controller | Input | Fuse controller |
|---|---|---|---|
| 12 | `fatal_prog_error` | 18 | `fatal_macro_error` |
| 13 | `fatal_state_error` | 19 | `fatal_check_error` |
| 14 | `fatal_bus_integ_error` | 20 | `fatal_bus_integ_error` |
| | | 21 | `fatal_prim_otp_alert` |
| | | 22 | `recov_prim_otp_alert` |

The mask registers decide which of these errors reach the SoC. The reset
value of `internal_agg_error_fatal_mask` masks inputs 12 to 29, so only the
errors of the Caliptra core and of the MCU's DCCM assert `mci_error_fatal`.
The reset value of `internal_agg_error_non_fatal_mask` masks no input, so
every aggregated non-fatal error asserts `mci_error_non_fatal`. The ROM of
caliptra-mcu-sw does not write these masks. The descriptions of `AGG_ERROR_FATAL` and
`AGG_ERROR_NON_FATAL` in `mci_reg.rdl` say that the fatal mask masks only the
I3C inputs and that the non-fatal mask masks the inputs of the lifecycle
controller and of the fuse controller. The values on this page are the reset
values of the mask fields in the same file.

The image stops on the same aggregated fatal errors that assert
`mci_error_fatal` with these reset values: those of the Caliptra core and of
the MCU's DCCM. The other aggregated errors are recorded in `AGG_ERROR_FATAL`
and `AGG_ERROR_NON_FATAL`, and the image logs the records when it next
starts.

## Bits of the error records

| Register | Bit | Driver constant | Cause |
|---|---|---|---|
| `HW_ERROR_FATAL` | 0 | `HwFatalErrors::MCU_SRAM_ECC_UNCORRECTED` | MCU SRAM had an ECC error that was not corrected. |
| `HW_ERROR_FATAL` | 1 | `HwFatalErrors::NMI` | The watchdog raised the MCU's non-maskable interrupt. |
| `HW_ERROR_FATAL` | 2 | `HwFatalErrors::MCU_SRAM_DMI_AXI_COLLISION` | The same cause as the error event of the same name. |
| `HW_ERROR_NON_FATAL` | 0 | `HwNonFatalErrors::MBOX0_ECC_UNCORRECTED` | The same cause as the error event of the same name. |
| `HW_ERROR_NON_FATAL` | 1 | `HwNonFatalErrors::MBOX1_ECC_UNCORRECTED` | The same cause as the error event of the same name. |
| `AGG_ERROR_FATAL` | 0 to 31 | `AggregatedErrors` | The aggregated fatal inputs. |
| `AGG_ERROR_NON_FATAL` | 0 to 31 | `AggregatedErrors` | The aggregated non-fatal inputs. |
| `FW_ERROR_FATAL` | 0 to 31 | None | A code from the ROM or from the image; see [Fatal error codes](#fatal-error-codes). |
| `FW_ERROR_NON_FATAL` | 0 to 31 | None | A code from firmware. The image does not write this register. |

An uncorrected ECC error in MCU SRAM and the non-maskable interrupt have a
record but no event, so the MCI does not interrupt the MCU for them. Both
assert `mci_error_fatal`: the reset value of `internal_hw_error_fatal_mask`
masks no bit. The image learns of them only from the record, which it logs
when it next starts.

## Non-maskable interrupt

The watchdog raises the MCU's non-maskable interrupt when timer 2 times out
in cascade mode, in which timer 2 runs after timer 1 has timed out. The
image has no handler for it. The NMI vector points into the ROM, and the ROM
locks the vector before it starts the image. This page does not cover what
the ROM does with the interrupt. The image disables the watchdog when it
starts, so the watchdog does not raise the interrupt while the image runs.

## Notifications

The only notification that the image enables is `CPTRA_MCU_RESET_REQUEST`.

| Bit | `Notifications` | Cause |
|---|---|---|
| 0 | `MCU_SRAM_ECC_CORRECTED` | MCU SRAM had an ECC error that was corrected. |
| 1 | `CPTRA_MCU_RESET_REQUEST` | Caliptra requests an MCU reset. |
| 2 | `GENERIC_INPUT_TOGGLE` | A generic input wire from the SoC changed. |
| 3 | `MBOX0_TARGET_DONE` | The target user of MCU mailbox 0 is done and its status is available. |
| 4 | `MBOX1_TARGET_DONE` | The target user of MCU mailbox 1 is done and its status is available. |
| 5 | `MBOX0_COMMAND_AVAILABLE` | MCU mailbox 0 holds a command for the MCU. |
| 6 | `MBOX1_COMMAND_AVAILABLE` | MCU mailbox 1 holds a command for the MCU. |
| 7 | `CPTRA_MBOX_COMMAND_AVAILABLE` | The Caliptra mailbox holds a command for the MCU. |
| 8 | `MBOX0_ECC_CORRECTED` | The SRAM of MCU mailbox 0 had an ECC error that was corrected. |
| 9 | `MBOX1_ECC_CORRECTED` | The SRAM of MCU mailbox 1 had an ECC error that was corrected. |
| 10 | `DEBUG_LOCKED` | The debug-locked bit of the security state changed. |
| 11 | `SCAN_MODE` | The subsystem entered scan mode. |
| 12 | `MBOX0_SOC_REQUESTED_LOCK` | The SoC asked for MCU mailbox 0 while the MCU held its lock. |
| 13 | `MBOX1_SOC_REQUESTED_LOCK` | The SoC asked for MCU mailbox 1 while the MCU held its lock. |
| 14 | `OTP_OPERATION_DONE` | An operation of the OTP controller is done. |

## Tests

| Test | What it checks |
|---|---|
| `//target/veer/peripherals:mci_test` | The driver against a model of the registers. It runs on the build machine. |
| `//target/veer/tests/mci:mci_test` | The driver on the MCU: an interrupt from each of the four groups, raised through the trigger registers. |
| `//target/veer/runtime:runtime_test` | The runtime image. The test program raises Caliptra's request and checks that the image resets the MCU. It raises an error event and reads the fatal code and the details from the MCI, then checks that the image no longer answers Caliptra's request. |

The test program of the runtime test runs outside the MCU and reaches the
MCI registers through the hardware model of caliptra-mcu-sw. The last two
tests have an `_emulator` and an `_fpga` target each:

```bash
bazel test //target/veer/peripherals:mci_test
bazel test //target/veer/tests/mci:mci_test_emulator //target/veer/runtime:runtime_test_emulator
bazel test --config=fpga //target/veer/tests/mci:mci_test_fpga //target/veer/runtime:runtime_test_fpga
```

The tests have these limits:

- Every event is raised through a trigger register. None of these tests
  makes the hardware raise one.
- The emulator does not model the aggregated errors or the hardware error
  records, so `//target/veer/tests/mci:mci_test` checks the aggregated errors
  only on the FPGA. See
  [caliptra-mcu-sw issue 2378](https://github.com/chipsalliance/caliptra-mcu-sw/issues/2378).
- The runtime test raises the `INTERNAL` error event. None of these tests
  raises an aggregated fatal error against the runtime image.
- None of these tests resets the subsystem after the image has stopped to
  check what the image logs and acknowledges when it starts again.
