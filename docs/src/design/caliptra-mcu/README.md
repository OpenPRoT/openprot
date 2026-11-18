# Caliptra MCU

This section covers the Caliptra MCU product class (see
[Terminology](../../architecture.md#terminology)): the products whose firmware
runs on the VeeR EL2 MCU of a Caliptra Subsystem. The code is under
[`target/veer/`](https://github.com/OpenPRoT/openprot/tree/main/target/veer).

## Products

| Product | Hardware | Test targets |
|---|---|---|
| Caliptra MCU Emulator | The emulator of [caliptra-mcu-sw](https://github.com/chipsalliance/caliptra-mcu-sw) | `<test>_emulator` |
| Caliptra MCU Test FPGA | A Caliptra Subsystem bitstream on a VCK190 board | `<test>_fpga` |

A test that boots an MCU image is defined once and has a target for each
product. [Usage](../../usage.md) says how to run tests, and
[`target/veer/tooling/fpga/README.md`](https://github.com/OpenPRoT/openprot/blob/main/target/veer/tooling/fpga/README.md)
says how the tests reach the FPGA board.

## Boot

OpenPRoT has no ROM for the MCU. The MCU boots a ROM built from
caliptra-mcu-sw, which starts the OpenPRoT runtime image from MCU SRAM. The
runtime image is
[`target/veer/runtime/`](https://github.com/OpenPRoT/openprot/tree/main/target/veer/runtime),
a system image for `pw_kernel` (see
[Pigweed Integration Overview](../pigweed-overview.md)). Before any userspace
service starts, its kernel disables the watchdog that the ROM started, reads
the table of boot information that the ROM hands off, and sets up the
interrupt of the MCI (Manufacturer Control Interface), the block that holds
the MCU's control and status registers.

## Documents

- [**MCI events and errors**](./mci-events.md): the events and errors that
  the MCI reports, what causes each one, and what the runtime image does
  about it.
