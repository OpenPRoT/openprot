# Converting Rust HAL Traits to Idol Interfaces: A Practical Guide

## Overview

This guide explains how to transform Rust Hardware Abstraction Layer (HAL) traits into Idol interface definitions for use in Hubris-based systems. Based on practical experience converting the digest traits, this guide covers the key patterns, challenges, and solutions.

## Table of Contents

1. [Understanding the Transformation](#understanding-the-transformation)
2. [Core Design Patterns](#core-design-patterns)
3. [Step-by-Step Conversion Process](#step-by-step-conversion-process)
4. [Common Challenges and Solutions](#common-challenges-and-solutions)
5. [Type System Considerations](#type-system-considerations)
6. [Error Handling Patterns](#error-handling-patterns)
7. [Performance Considerations](#performance-considerations)
8. [Testing and Validation](#testing-and-validation)

## Understanding the Transformation

### From Trait-Based to IPC-Based

**Rust HAL traits** provide compile-time polymorphism with:
- Associated types
- Lifetime parameters
- Generic parameters
- Zero-cost abstractions
- Direct memory access

**Idol interfaces** provide runtime communication with:
- Concrete types
- Message passing
- Serialization boundaries
- Process isolation
- Memory leases for data transfer

### Key Conceptual Shifts

| Rust Trait Concept | Idol Equivalent | Transformation Strategy |
|-------------------|-----------------|------------------------|
| `&mut self` methods | Session-based operations | Use session IDs |
| Associated types | Concrete types | Define enums/structs |
| Lifetimes | Ownership transfer | Memory leases |
| Generic parameters | Multiple operations | One operation per type |
| Zero-cost abstractions | IPC overhead | Optimize message structure |

## Core Design Patterns

### 1. Session-Based State Management

**Problem**: Rust traits use `&mut self` for stateful operations.
**Solution**: Use session IDs to track state across IPC boundaries.

```rust
// Original Trait
pub trait DigestOp: ErrorType {
    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}
```

```ron
// Idol Interface
Interface(
    name: "Digest",
    ops: {
        "init_sha256": (
            reply: Result(ok: "u32", err: CLike("DigestError")), // Returns session ID
        ),
        "update": (
            args: { "session_id": "u32", "len": "u32" },
            leases: { "data": (type: "[u8]", read: true, max_len: Some(1024)) },
            reply: Result(ok: "()", err: CLike("DigestError")),
        ),
        "finalize_sha256": (
            args: { "session_id": "u32" },
            leases: { "digest_out": (type: "[u32; 8]", write: true) },
            reply: Result(ok: "()", err: CLike("DigestError")),
        ),
    },
)
```

### 2. Generic Type Expansion

**Problem**: Rust traits use generics to support multiple types.
**Solution**: Create separate operations for each concrete type.

```rust
// Original Generic Trait
pub trait DigestInit<T: DigestAlgorithm>: ErrorType {
    fn init(&mut self, params: T) -> Result<Self::OpContext<'_>, Self::Error>;
}
```

```ron
// Idol Interface - Expanded Operations
"init_sha256": (/* ... */),
"init_sha384": (/* ... */),
"init_sha512": (/* ... */),
"init_sha3_256": (/* ... */),
// etc.
```

### 3. Memory Lease Patterns

**Problem**: Rust uses references and slices for zero-copy operations.
**Solution**: Use Idol memory leases for efficient data transfer.

| Rust Pattern | Idol Lease Pattern | Use Case |
|-------------|-------------------|----------|
| `&[u8]` | `read: true` | Input data |
| `&mut [u8]` | `write: true` | Output buffers |
| `&T` | `read: true` | Configuration structs |
| `&mut T` | `write: true` | Result structs |

### 4. Error Type Consolidation

**Problem**: Traits use associated error types and generic error handling.
**Solution**: Define comprehensive concrete error enums.

```rust
// Original - Generic Error
pub trait ErrorType {
    type Error: Error;
}

pub trait Error: core::fmt::Debug {
    fn kind(&self) -> ErrorKind;
}
```

```rust
// Idol - Concrete Error Enum
#[derive(Copy, Clone, Debug, FromPrimitive, Eq, PartialEq, IdolError, counters::Count)]
#[repr(u32)]
pub enum DigestError {
    InvalidInputLength = 1,
    UnsupportedAlgorithm = 2,
    // ... comprehensive error cases
    #[idol(server_death)]
    ServerRestarted = 100,
}
```

## Step-by-Step Conversion Process

### Step 1: Analyze the Original Trait

1. **Identify State Management Patterns**
   - Methods that take `&mut self` → Need session management
   - Methods that consume `self` → Need session cleanup
   - Associated types → Need concrete type definitions

2. **Map Data Flow**
   - Input parameters → Idol args + read leases
   - Output parameters → Idol return values + write leases
   - Mutable references → Write leases

3. **Catalog Error Cases**
   - Collect all possible error conditions
   - Map generic `ErrorKind` to specific error variants

### Step 2: Design the Idol Interface

1. **Create the IDL File**
   ```bash
   mkdir -p hubris/idl/
   touch hubris/idl/my_trait.idol
   ```

2. **Define Operations Structure**
   ```ron
   Interface(
       name: "MyTrait",
       ops: {
           // Initialization operations
           "init_*": (/* ... */),
           
           // State manipulation operations  
           "operation_*": (/* ... */),
           
           // Cleanup operations
           "reset": (/* ... */),
           
           // Convenience operations
           "oneshot_*": (/* ... */),
       },
   )
   ```

3. **Design Session Management**
   - Use `u32` session IDs
   - Return session ID from init operations
   - Pass session ID to subsequent operations

### Step 3: Create the API Package

1. **Directory Structure**
   ```
   hubris/drv/my-trait-api/
   ├── Cargo.toml
   ├── build.rs
   └── src/
       └── lib.rs
   ```

2. **Configure Cargo.toml**
   ```toml
   [package]
   name = "drv-my-trait-api"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   idol-runtime.workspace = true
   num-traits.workspace = true
   zerocopy.workspace = true
   zerocopy-derive.workspace = true
   counters = { path = "../../lib/counters" }
   derive-idol-err = { path = "../../lib/derive-idol-err" }
   userlib = { path = "../../sys/userlib" }

   [build-dependencies]
   idol.workspace = true

   [lib]
   test = false
   doctest = false
   bench = false

   [lints]
   workspace = true
   ```

3. **Create build.rs**
   ```rust
   fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
       idol::client::build_client_stub("../../idl/my_trait.idol", "client_stub.rs")?;
       Ok(())
   }
   ```

### Step 4: Implement Type Definitions

1. **Create Zerocopy-Compatible Types**
   ```rust
   #[derive(
       Copy, Clone, Debug, PartialEq, Eq,
       zerocopy::IntoBytes,
       zerocopy::FromBytes,
       zerocopy::Immutable,
       zerocopy::KnownLayout,
   )]
   #[repr(C, packed)] // Use packed for complex structs
   pub struct MyConfig {
       pub field1: u32,
       pub field2: u8,
       // Avoid bool - use u8 instead
       pub enabled: u8,
   }
   ```

2. **Define Error Types**
   ```rust
   #[derive(
       Copy, Clone, Debug, FromPrimitive, Eq, PartialEq, 
       IdolError, counters::Count,
   )]
   #[repr(u32)]
   pub enum MyTraitError {
       // Map from original ErrorKind
       InvalidInput = 1,
       HardwareFailure = 2,
       // ... 
       #[idol(server_death)]
       ServerRestarted = 100,
   }
   ```

3. **Create Enum Types for IPC**
   ```rust
   #[derive(
       Copy, Clone, Debug, PartialEq, Eq,
       zerocopy::IntoBytes,
       zerocopy::Immutable,
       zerocopy::KnownLayout,
       FromPrimitive,
   )]
   #[repr(u32)] // Use u32 for enums
   pub enum MyAlgorithm {
       Algorithm1 = 0,
       Algorithm2 = 1,
   }
   ```

### Step 5: Handle Memory Management

1. **Input Data Patterns**
   ```ron
   "process_data": (
       args: { "len": "u32" },
       leases: {
           "input_data": (type: "[u8]", read: true, max_len: Some(4096)),
       },
   ),
   ```

2. **Output Data Patterns**
   ```ron
   "get_result": (
       args: { "session_id": "u32" },
       leases: {
           "output_buffer": (type: "[u8]", write: true, max_len: Some(1024)),
       },
   ),
   ```

3. **Configuration Patterns**
   ```ron
   "configure": (
       args: { "session_id": "u32" },
       leases: {
           "config": (type: "MyConfig", read: true),
       },
   ),
   ```

## Common Challenges and Solutions

### Challenge 1: Associated Types

**Problem**: Rust traits use associated types for flexibility.
```rust
pub trait DigestAlgorithm {
    const OUTPUT_BITS: usize;
    type Digest;
}
```

**Solution**: Define concrete types and use constants.
```rust
pub const SHA256_WORDS: usize = 8;
pub type Sha256Digest = DigestOutput<SHA256_WORDS>;

#[repr(C)]
pub struct DigestOutput<const N: usize> {
    pub value: [u32; N],
}
```

### Challenge 2: Lifetime Parameters

**Problem**: Rust contexts have lifetime dependencies.
```rust
pub trait DigestInit<T>: ErrorType {
    type OpContext<'a>: DigestOp where Self: 'a;
    fn init<'a>(&'a mut self, params: T) -> Result<Self::OpContext<'a>, Self::Error>;
}
```

**Solution**: Replace with session-based state management.
```rust
// Server maintains context mapping
struct DigestServer {
    contexts: HashMap<u32, DigestContext>,
    next_session_id: u32,
}
```

### Challenge 3: Generic Methods

**Problem**: Single generic method supports multiple types.
```rust
fn process<T: Algorithm>(&mut self, data: &[u8], algo: T) -> Result<T::Output, Error>;
```

**Solution**: Create type-specific operations.
```ron
"process_sha256": (/* ... */),
"process_sha384": (/* ... */),
"process_aes": (/* ... */),
```

### Challenge 4: Complex Return Types

**Problem**: Rust can return complex generic types.
```rust
fn finalize(self) -> Result<Self::Output, Self::Error>;
```

**Solution**: Use output leases for complex types.
```ron
"finalize": (
    args: { "session_id": "u32" },
    leases: { "result": (type: "MyResult", write: true) },
    reply: Result(ok: "()", err: CLike("MyError")),
),
```

## Type System Considerations

### Zerocopy Compatibility

All types used in Idol interfaces must be zerocopy-compatible:

```rust
// ✅ Good - Zerocopy compatible
#[derive(zerocopy::IntoBytes, zerocopy::FromBytes, zerocopy::Immutable)]
#[repr(C)]
pub struct GoodConfig {
    pub value: u32,
    pub enabled: u8, // Not bool!
    pub _padding: [u8; 3], // Explicit padding
}

// ❌ Bad - Not zerocopy compatible  
pub struct BadConfig {
    pub value: u32,
    pub enabled: bool, // bool doesn't implement FromBytes
    pub data: Vec<u8>, // Dynamic allocation
}
```

### Enum Representations

```rust
// ✅ Good - Use u32 for enums
#[derive(FromPrimitive)]
#[repr(u32)]
pub enum MyEnum {
    Variant1 = 0,
    Variant2 = 1,
}

// ❌ Bad - u8 enums with FromBytes need 256 variants
#[repr(u8)]
pub enum SmallEnum {
    A = 0,
    B = 1, // Only 2 variants - FromBytes won't work
}
```

### Padding and Alignment

```rust
// ✅ Good - Use packed for complex layouts
#[repr(C, packed)]
pub struct PackedStruct {
    pub field1: u8,
    pub field2: u32, // No padding issues
}

// ✅ Good - Manual padding control
#[repr(C)]
pub struct PaddedStruct {
    pub field1: u8,
    pub _pad: [u8; 3], // Explicit padding
    pub field2: u32,
}
```

## Error Handling Patterns

### Comprehensive Error Mapping

Map all possible error conditions from the original trait:

```rust
// Original trait error kinds
pub enum ErrorKind {
    InvalidInputLength,
    UnsupportedAlgorithm,
    HardwareFailure,
    // ...
}

// Idol error enum - comprehensive mapping
#[derive(Copy, Clone, Debug, FromPrimitive, IdolError, counters::Count)]
#[repr(u32)]
pub enum MyTraitError {
    // Map each ErrorKind to a specific variant
    InvalidInputLength = 1,
    UnsupportedAlgorithm = 2,
    HardwareFailure = 3,
    
    // Add IPC-specific errors
    InvalidSession = 10,
    TooManySessions = 11,
    
    // Required for Hubris
    #[idol(server_death)]
    ServerRestarted = 100,
}
```

### Error Context Preservation

```rust
// Add context-specific error variants
pub enum MyTraitError {
    // Operation-specific errors
    InitializationFailed = 20,
    UpdateFailed = 21,
    FinalizationFailed = 22,
    
    // Resource-specific errors
    OutOfMemory = 30,
    BufferTooSmall = 31,
    InvalidConfiguration = 32,
}
```

## Performance Considerations

### Minimize Message Overhead

1. **Batch Operations**: Combine related parameters into single calls
   ```ron
   // ✅ Good - Single call with all parameters
   "configure_and_start": (
       args: {
           "algorithm": "MyAlgorithm",
           "buffer_size": "u32",
           "timeout_ms": "u32",
       },
   ),
   
   // ❌ Bad - Multiple round trips
   "set_algorithm": (args: {"algo": "MyAlgorithm"}),
   "set_buffer_size": (args: {"size": "u32"}),
   "set_timeout": (args: {"timeout": "u32"}),
   "start": (),
   ```

2. **Efficient Data Transfer**: Use appropriate lease sizes
   ```ron
   leases: {
       // Size limits based on expected usage
       "small_data": (type: "[u8]", read: true, max_len: Some(256)),
       "large_data": (type: "[u8]", read: true, max_len: Some(4096)),
   }
   ```

### Memory Lease Optimization

1. **Right-size Buffers**: Don't over-allocate
2. **Reuse Sessions**: Avoid constant init/cleanup
3. **Batch Updates**: Process multiple chunks in one call when possible

## Testing and Validation

### Build Verification

1. **ARM Target Build**:
   ```bash
   cargo build -p drv-my-trait-api --target thumbv7em-none-eabihf
   ```

2. **Generated Code Inspection**:
   ```bash
   ls target/thumbv7em-none-eabihf/debug/build/drv-my-trait-api*/out/
   head -50 target/thumbv7em-none-eabihf/debug/build/drv-my-trait-api*/out/client_stub.rs
   ```

### API Surface Validation

1. **Check Generated Operations**: Verify all expected operations are present
2. **Type Safety**: Ensure all types compile correctly
3. **Error Handling**: Verify error propagation works

### Integration Testing

1. **Mock Server**: Create a simple server implementation
2. **Client Testing**: Test all operation patterns
3. **Error Scenarios**: Test error handling paths

## Example: Complete Conversion

Here's a complete example showing the transformation of a simple trait:

### Original Rust Trait

```rust
pub trait Crypto: ErrorType {
    type Algorithm: CryptoAlgorithm;
    type Context<'a>: CryptoOp where Self: 'a;
    
    fn init<'a>(&'a mut self, algo: Self::Algorithm) -> Result<Self::Context<'a>, Self::Error>;
}

pub trait CryptoOp: ErrorType {
    type Output;
    fn process(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}
```

### Converted Idol Interface

```ron
Interface(
    name: "Crypto",
    ops: {
        "init_aes": (
            reply: Result(ok: "u32", err: CLike("CryptoError")),
        ),
        "init_chacha": (
            reply: Result(ok: "u32", err: CLike("CryptoError")),
        ),
        "process": (
            args: { "session_id": "u32", "len": "u32" },
            leases: { "data": (type: "[u8]", read: true, max_len: Some(1024)) },
            reply: Result(ok: "()", err: CLike("CryptoError")),
        ),
        "finalize_aes": (
            args: { "session_id": "u32" },
            leases: { "output": (type: "[u8; 16]", write: true) },
            reply: Result(ok: "()", err: CLike("CryptoError")),
        ),
        "finalize_chacha": (
            args: { "session_id": "u32" },
            leases: { "output": (type: "[u8; 32]", write: true) },
            reply: Result(ok: "()", err: CLike("CryptoError")),
        ),
    },
)
```

### API Package Implementation

```rust
// drv/crypto-api/src/lib.rs
#![no_std]

use derive_idol_err::IdolError;
use userlib::{sys_send, FromPrimitive};

#[derive(Copy, Clone, Debug, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable, FromPrimitive)]
#[repr(u32)]
pub enum CryptoAlgorithm {
    Aes = 0,
    ChaCha = 1,
}

#[derive(Copy, Clone, Debug, FromPrimitive, Eq, PartialEq, IdolError, counters::Count)]
#[repr(u32)]
pub enum CryptoError {
    InvalidInput = 1,
    InvalidSession = 2,
    HardwareFailure = 3,
    #[idol(server_death)]
    ServerRestarted = 100,
}

include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));
```

## Conclusion

Converting Rust HAL traits to Idol interfaces requires careful consideration of:

1. **State Management**: Sessions instead of lifetimes
2. **Type Systems**: Concrete types instead of generics  
3. **Memory Management**: Leases instead of references
4. **Error Handling**: Comprehensive concrete error enums
5. **Performance**: Efficient message design

The key is to preserve the semantic meaning and safety guarantees of the original trait while adapting to the constraints and patterns of the Hubris IPC system.

By following these patterns and guidelines, you can successfully transform complex Rust HAL traits into efficient, type-safe Idol interfaces that maintain the robustness and performance characteristics expected in embedded systems.
