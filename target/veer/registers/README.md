# target/veer/registers

Umbrella crate (`caliptra_ss_registers`) that re-exports the generated
register definitions for every Caliptra Subsystem peripheral.

## Source

Registers are generated from the Caliptra-SS SystemRDL sources by
`caliptra_mcu_registers_generator` and live in the pinned
`caliptra-mcu-sw` third-party dependency at
`registers/generated-firmware/src/`. Each peripheral module exposes a
`bits` sub-module of `tock_registers::register_bitfields!` types plus a
base address constant (e.g. `I3C_CSR_ADDR = 0x2000_4000`).

## Usage

Add `//target/veer/registers` to your `deps` and import the peripheral
module you need:

```rust
use caliptra_ss_registers::i3c;
// Bitfield types for register reads/writes
use i3c::bits::Control;

// Base address for MMIO pointer construction
const BASE: u32 = i3c::I3C_CSR_ADDR;

```
