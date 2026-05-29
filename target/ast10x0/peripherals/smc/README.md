# SMC — Static Memory Controller

Driver for the AST1060 FMC and SPI static memory controllers, supporting
memory-mapped flash reads, DMA transfers, and interrupt handling.

## Module Structure

```
smc/
├── mod.rs              — Re-exports all public types; module declarations
├── types.rs            — Core types: SmcError, ChipSelect, FlashConfig, SmcConfig,
│                         SmcController, TransferMode, AddressWidth, SmcRetryable
├── register_traits.rs  — SmcRegisterBackend trait (shared register contract)
├── fmc_backend.rs      — FmcRegisterBackend: wraps ast1060_pac::fmc::RegisterBlock
├── spi_backend.rs      — SpiRegisterBackend: wraps ast1060_pac::spi::RegisterBlock
├── controller.rs       — Generic Smc<B: SmcRegisterBackend, Mode> state machine;
│                         type aliases UninitSmc / ReadySmc / UninitSpiSmc / ReadySpiSmc
├── fmc.rs              — FmcUninit / FmcReady: FMC-specific facade over UninitSmc/ReadySmc
├── spi.rs              — SpiUninit / SpiReady: SPI-specific facade over UninitSpiSmc/ReadySpiSmc
├── interrupts.rs       — SmcInterrupt, SmcInterruptDecoder, SmcIsrContext
├── helpers.rs          — Internal helpers: segment encoding, DMA validation, frequency divisor
└── device/             — SPI NOR flash device facade: SpiNorFlash, SpiNorFlashDevice
    ├── flash.rs            trait, FlashCommandProfile, FlashAddressingPolicy, JedecId
    └── block_device.rs     SpiNorBlockDevice, BlockDeviceInfo
```

## Architecture

The driver uses a two-level abstraction:

**Register backend** (`SmcRegisterBackend` trait)
Abstracts the PAC-level differences between FMC and SPI register blocks.
The same register offset can have different field layouts in the two PACs:

| Backend              | PAC type                          | Notes                               |
|----------------------|-----------------------------------|-------------------------------------|
| `FmcRegisterBackend` | `ast1060_pac::fmc::RegisterBlock` | Named fields (e.g. `dmaintenbl()`)  |
| `SpiRegisterBackend` | `ast1060_pac::spi::RegisterBlock` | Raw bit access; adds `spi06c/074()` |

**Controller state machine** (`Smc<B, Mode>`)
A type-state generic over backend `B` and mode (`Uninitialized` / `Ready`).
Transitions from `Uninitialized` → `Ready` happen through `init()`, which
configures timing, segments, and transfer mode.

**Facade types** (`FmcUninit/FmcReady`, `SpiUninit/SpiReady`)
Thin wrappers around the concrete `Smc<…>` aliases, intended as the
primary API surface for callers.

## Key Types

| Type                   | Description                                  |
|------------------------|----------------------------------------------|
| `SmcRegisterBackend`   | Trait: register read/write/modify operations |
| `FmcRegisterBackend`   | FMC PAC register backend                    |
| `SpiRegisterBackend`   | SPI PAC register backend                    |
| `Smc<B, Mode>`         | Generic controller state machine             |
| `UninitSmc`/`ReadySmc` | FMC controller type aliases                  |
| `UninitSpiSmc`/`ReadySpiSmc` | SPI controller type aliases            |
| `FmcUninit`/`FmcReady` | FMC facade                                  |
| `SpiUninit`/`SpiReady` | SPI facade                                  |
| `SmcConfig`            | Controller configuration (timing, segments)  |
| `FlashConfig`          | Per-CS flash geometry                       |
| `ChipSelect`           | `Cs0` / `Cs1`                               |
| `TransferMode`         | Single / Dual / Quad SPI                    |
| `SmcInterrupt`         | DMA completion / error interrupt variants    |
