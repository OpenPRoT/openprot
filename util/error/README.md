# util_error

Structured error handling for OpenProt.

This crate provides a mechanism for defining and using structured 32-bit error codes (`ErrorCode`) partitioned by 16-bit modules (`ErrorModule`).

## Key Concepts

### ErrorModule

An `ErrorModule` is a 16-bit identifier that categorizes a set of error codes. It is recommended to use ASCII characters for the module ID to aid in debugging.

```rust
use util_error::ErrorModule;

// Define a module with ID 'MY' (0x4d59)
pub const MY_MODULE: ErrorModule = ErrorModule::new(0x4d59);
```

### ErrorCode

An `ErrorCode` is a 32-bit value composed of:
*   Upper 16 bits: The `ErrorModule` ID.
*   Lower 16 bits: A module-specific error value.

`ErrorCode` implements `core::error::Error`, `Display`, and `Debug`. It formats as a hex representation of the 32-bit value (e.g., `0x4b450001`).

```rust
use util_error::ErrorCode;

// Create an error code under MY_MODULE
pub const MY_ERROR: ErrorCode = MY_MODULE.error(1);
```

### Pigweed Integration

`ErrorCode` supports integration with `pw_status::Error`. You can embed a Pigweed status into the lower 16 bits of the error code using `from_pw`.

The lower 16 bits are partitioned as:
*   Bits 5-15: Module-specific error code.
*   Bits 0-4: Pigweed `pw_status::Error` (which is 5 bits).

```rust
use util_error::ErrorModule;
use pw_status::Error;

pub const MY_MODULE: ErrorModule = ErrorModule::new(0x4d59);

// Create an error code that embeds pw_status::Error::InvalidArgument
pub const MY_INVALID_ARG_ERROR: ErrorCode = MY_MODULE.from_pw(1, Error::InvalidArgument);
```

## Defined Modules

The following modules are defined in this crate:

| Module | ID (Hex) | ASCII | Description |
| :--- | :--- | :--- | :--- |
| `KERNEL_ERROR` | `0x4b45` | `KE` | Kernel-specific error codes (see [kernel.rs](kernel.rs)). |
| `FLASH_GENERIC` | `0x464c` | `FL` | Generic flash and SFDP errors (see [flash.rs](flash.rs)). |
| `FLASH_OPENTITAN`| `0x464f` | `FO` | OpenTitan-specific flash errors (see [flash.rs](flash.rs)). |
| `IPC_ERROR` | `0x4943` | `IC` | IPC-specific error codes (see [ipc.rs](ipc.rs)). |
