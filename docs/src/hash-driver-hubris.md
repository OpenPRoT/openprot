# Generic Hash Accelerator Driver Architecture

This document proposes a generic driver model for hash accelerator hardware in Hubris, based on the  OpenPRoT HAL.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                  Generic Digest Driver Architecture                         │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────────────┐
│   Client Tasks  │    │  IDL Generated   │    │     Digest Server Task      │
│  (Applications) │    │  IPC Interface   │    │   (/hubris/task/digest)     │
│                 │    │                  │    │                             │
│ ┌─────────────┐ │    │ ┌──────────────┐ │    │ ┌─────────────────────────┐ │
│ │ SPDM/TLS    │ │◄──►│ │ digest.idol  │ │◄──►│ │ InOrderDigestImpl       │ │
│ │ Tasks       │ │    │ │              │ │    │ │ - init_sha256/384/512   │ │
│ └─────────────┘ │    │ │ Operations:  │ │    │ │ - update(session, data) │ │
│                 │    │ │ • Session-   │ │    │ │ - finalize_sha256/384/  │ │
│ ┌─────────────┐ │    │ │   based APIs │ │    │ │   512(session)          │ │
│ │ Boot/       │ │    │ │ • One-shot   │ │    │ │ - digest_oneshot_*      │ │
│ │ Verification│ │    │ │   operations │ │    │ │ - reset(session)        │ │
│ │ Tasks       │ │    │ │ • Leased     │ │    │ └─────────────────────────┘ │
│ └─────────────┘ │    │ │   memory I/O │ │    │                             │
│                 │    │ └──────────────┘ │    │ ┌─────────────────────────┐ │
│ ┌─────────────┐ │    │                  │    │ │ Session Management      │ │
│ │ Other       │ │    │ ┌──────────────┐ │    │ │ - FnvIndexMap<u32,      │ │
│ │ Crypto      │ │    │ │ Error        │ │    │ │   DigestSession, 8>     │ │
│ │ Tasks       │ │    │ │ Handling:    │ │    │ │ - Session lifecycle     │ │
│ └─────────────┘ │    │ │ DigestError  │ │    │ │ - Resource limits       │ │
└─────────────────┘    │ │ enum (20+    │ │    │ └─────────────────────────┘ │
                       │ │ variants)    │ │    └─────────────────────────────┘
                       │ └──────────────┘ │            ▲
                       │                  │            │ HAL Integration
┌─────────────────┐    │ ┌──────────────┐ │            │ 
│ Generated       │    │ │ Memory       │ │    ┌───────┴───────┐
│ Client API      │    │ │ Management:  │ │    │ ┌───────────┐ │
│ (Future)        │    │ │ • Leased<R,  │ │    │ │ OpenPRoT  │ │
│                 │    │ │   [u8]> for  │ │    │ │ HAL       │ │
│ ┌─────────────┐ │    │ │   input      │ │    │ │ Blocking  │ │
│ │ Type-Safe   │ │    │ │ • Leased<W,  │ │    │ │ Traits    │ │
│ │ Algorithm   │ │    │ │   [u32]> for │ │    │ └───────────┘ │
│ │ Wrappers    │ │    │ │   output     │ │    └───────────────┘
│ │             │ │    │ │ • LenLimit   │ │            ▲
│ └─────────────┘ │    │ │   bounds     │ │            │ Feature Selection
└─────────────────┘    │ └──────────────┘ │    ┌───────┴───────┐
                       └──────────────────┘    │ ┌───────────┐ │
                                               │ │#[cfg(     │ │
                                               │ │ feature = │ │
                                               │ │"opentitan"│ │
                                               │ │)]         │ │
                                               │ │HashDevice │ │
                                               │ └───────────┘ │
                                               └───────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                     OpenPRoT HAL Integration                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │                    Implementation Pattern                               │ │
│ │                                                                         │ │
│ │  // Server implementation using OpenPRoT HAL traits                     │ │
│ │  struct ServerImpl {                                                    │ │
│ │      sessions: FnvIndexMap<u32, DigestSession, MAX_SESSIONS>,           │ │
│ │      next_session_id: u32,                                              │ │
│ │      hardware: HardwareBackend, // Feature-gated type alias             │ │
│ │  }                                                                      │ │
│ │                                                                         │ │
│ │  // Feature-gated hardware backend selection                            │ │
│ │  #[cfg(feature = "opentitan")]                                          │ │
│ │  type HardwareBackend = OtHashDevice;                                   │ │
│ │  #[cfg(feature = "ast1060")]                                            │ │
│ │  type HardwareBackend = AsHashDevice;                                   │ │
│ │  // OpenPRoT HAL trait usage                                            │ │
│ │  impl InOrderDigestImpl for ServerImpl {                                │ │
│ │      fn init_sha256(&mut self, _: &RecvMessage) -> Result<u32, ...> {   │ │
│ │          let session_id = self.allocate_session_id()?;                  │ │
│ │          let context = self.hardware.init(Sha2_256)?; // HAL trait      │ │
│ │          // Store context and return session ID                         │ │
│ │      }                                                                  │ │
│ │                                                                         │ │
│ │      fn update(&mut self, session_id: u32, data: Leased<R,[u8]>) {      │ │
│ │          let session = self.get_session_mut(session_id)?;               │ │
│ │          // Read from leased memory and update hardware context         │ │
│ │          session.context.update(&buffer)?; // HAL trait                 │ │
│ │      }                                                                  │ │
│ │  }                                                                      │ │
│ │                                                                         │ │
│ │  // Session lifecycle management                                        │ │
│ │  struct DigestSession {                                                 │ │
│ │      algorithm: DigestAlgorithm,                                        │ │
│ │      context: DigestContext,      // Hardware context wrapper           │ │
│ │      initialized: bool,                                                 │ │
│ │  }                                                                      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           Hardware Layer                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │                    OpenPRoT HAL Trait Integration                       │ │
│ │                                                                         │ │
│ │  trait DigestInit<T> {                                                  │ │
│ │      type Context;                                                      │ │
│ │      type Error;                                                        │ │
│ │      fn init(&mut self) -> Result<Self::Context, Self::Error>;          │ │
│ │  }                                                                      │ │
│ │                                                                         │ │
│ │  trait DigestOp {                                                       │ │
│ │      type Error;                                                        │ │
│ │      fn update(&mut self, data: &[u8]) -> Result<(), Self::Error>;      │ │
│ │      fn finalize(self) -> Result<&[u8], Self::Error>;                   │ │
│ │  }                                                                      │ │
│ │                                                                         │ │
│ │  trait DigestCtrlReset {                                                │ │
│ │      type Error;                                                        │ │
│ │      fn reset(&mut self) -> Result<(), Self::Error>;                    │ │
│ │  }                                                                      │ │
│ │                                                                         │ │
│ │  // Algorithm marker types for compile-time dispatch                    │ │
│ │  struct Sha2_256;                                                       │ │
│ │  struct Sha2_384;                                                       │ │
│ │  struct Sha2_512;                                                       │ │
│ │                                                                         │ │
│ │  // Actual hardware backend (feature-gated)                             │ │
│ │  #[cfg(feature = "open")]                                          │ │
│ │  type HardwareBackend = HashDevice;                                     │ │
│ │                                                                         │ │
│ │  // Context wrapper for session state                                   │ │
│ │  enum DigestContext {                                                   │ │
│ │      Sha256(hash::Context),                                             │ │
│ │      Sha384(hash::Context),                                             │ │
│ │      Sha512(hash::Context),                                             │ │
│ │  }                                                                      │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Details

### In-Order Server Architecture

The digest driver implements an **in-order server** pattern, which is fundamental to understanding how the system processes cryptographic operations.

#### What is an In-Order Server?

An in-order server in Hubris is a server task that processes IPC requests sequentially, completing each request fully before starting the next one. This is the simplest and most predictable server pattern.

**Key Characteristics:**
- **Sequential Processing**: Requests are handled one at a time in arrival order
- **Complete Before Continue**: Each IPC call must finish entirely before the next begins
- **Single-Threaded**: No concurrency complexity within the server task
- **Predictable Resource Usage**: Only one operation accesses hardware at a time

#### Implementation in Digest Driver

```rust
impl InOrderDigestImpl for ServerImpl {
    // Each method call completes atomically
    fn init_sha256(&mut self, _: &RecvMessage) -> Result<u32, DigestError> {
        // This entire function executes before any other IPC call
        let session_id = self.allocate_session_id()?;
        let context = self.hardware.init::<Sha2_256>()?;
        // ... complete initialization
        Ok(session_id)
    }
    
    fn update(&mut self, session_id: u32, data: Leased<R, [u8]>) -> Result<(), DigestError> {
        // This processes ALL data in the lease before returning
        // No other client can make requests during this operation
        // ...
    }
}
```

#### Benefits for Cryptographic Operations

1. **Hardware Safety**: Prevents concurrent access to shared crypto hardware
2. **Session Integrity**: No interleaving of operations between different sessions  
3. **Memory Safety**: Leased memory access is atomic per operation
4. **Error Recovery**: Simplified error handling without partial state issues
5. **Resource Management**: Clear resource ownership during each operation

#### Trade-offs

- **Latency**: High-priority clients must wait for current operation to complete
- **Throughput**: Cannot overlap I/O operations with computation
- **Simplicity**: Much easier to implement and debug than concurrent alternatives

#### Interaction with Session Management

While the server processes requests in-order, it can manage multiple concurrent **sessions**:

```rust
// Multiple clients can have active sessions simultaneously
let session_a = server.init_sha256()?;  // Client A gets session
let session_b = server.init_sha256()?;  // Client B gets different session

// But updates are processed sequentially
server.update(session_a, data_a)?;     // Completes fully
server.update(session_b, data_b)?;     // Then this executes
```

This design balances simplicity with the ability to support multiple concurrent cryptographic contexts.

### Code Organization
```
/hubris/task/digest/
├── Cargo.toml           # Dependencies and features
├── build.rs             # IDL code generation
├── src/main.rs          # Server implementation
└── ../idl/digest.idol   # Interface definition
```

### Session State Management
```rust
const MAX_SESSIONS: usize = 8;

struct DigestSession {
    algorithm: DigestAlgorithm,
    context: DigestContext,
    initialized: bool,
}

enum DigestAlgorithm {
    Sha256,
    Sha384, 
    Sha512,
}

enum DigestContext {
    Sha256(hash::Context),
    Sha384(hash::Context), 
    Sha512(hash::Context),
}

// Session storage with bounded capacity
sessions: FnvIndexMap<u32, DigestSession, MAX_SESSIONS>
```

### IDL Interface (digest.idol)
Algorithm-specific operations with leased memory:
- `init_sha256()` → `Result<u32, DigestError>` (session ID)
- `init_sha384()` → `Result<u32, DigestError>`
- `init_sha512()` → `Result<u32, DigestError>`
- `update(session_id, data: Leased<R, [u8]>)` → `Result<(), DigestError>`
- `finalize_sha256(session_id, digest: Leased<W, [u32], 8>)` → `Result<(), DigestError>`
- `digest_oneshot_sha256(data, digest)` → `Result<(), DigestError>`
- `reset(session_id)` → `Result<(), DigestError>`

### OpenPRoT HAL Integration
```rust
use openprot_hal_blocking::{DigestInit, DigestOp, DigestCtrlReset};
use openprot_hal_blocking::{Sha2_256, Sha2_384, Sha2_512};

// Feature-gated hardware backend selection
#[cfg(feature = "open")]
type HardwareBackend = openprot_platform_open::HashDevice;

impl InOrderDigestImpl for ServerImpl {
    fn init_sha256(&mut self, _: &RecvMessage) -> Result<u32, DigestError> {
        let session_id = self.allocate_session_id()?;
        let context = self.hardware.init::<Sha2_256>()?; // HAL trait
        // Store context and return session ID
        Ok(session_id)
    }
    
    fn update(&mut self, session_id: u32, data: Leased<R, [u8]>) 
        -> Result<(), DigestError> {
        let session = self.sessions.get_mut(&session_id)?;
        let buffer = data.read_range(0..data.len())?;
        session.context.update(&buffer)?; // HAL trait
        Ok(())
    }
}
```

### Build Integration
```toml
# Cargo.toml  
[dependencies]
openprot-hal-blocking = { path = "../../hal/blocking" }
openprot-platform-open = { path = "../../platform/impls/open", features = ["hmac"] }

[features]
default = []
open = ["dep:openprot-platform-open"]
```

```rust
// build.rs - IDL code generation
use idol::code_gen::generate_server_code;
fn main() {
    generate_server_code("../idl/digest.idol", "src/generated.rs");
}
```

## Architecture Benefits

### 1. **Type Safety at Compile Time**
- Algorithm marker types (`Sha2_256`, etc.) prevent runtime errors
- Feature gates ensure only valid hardware backends are compiled
- IDL generation provides validated IPC interfaces

### 2. **Zero-Copy Performance**  
- Leased memory eliminates data copying for large inputs
- Direct hardware register access via OpenPRoT HAL
- Session-based state management minimizes allocations

### 3. **Resource Management**
- Bounded session capacity prevents resource exhaustion  
- Automatic cleanup on session drop
- Feature-gated dependencies reduce binary size

### 4. **Platform Extensibility**
- New platforms only need OpenPRoT HAL implementation
- Feature flags enable platform-specific optimizations
- Standard trait interface ensures consistent behavior

This implementation demonstrates the successful transformation from OpenPRoT traits to working IDL-based drivers, providing a foundation for generic driver development across the Hubris ecosystem.

## Architecture Benefits

### 1. **Type Safety at Compile Time**
- Algorithm marker types (`Sha2_256`, etc.) prevent runtime errors
- Feature gates ensure only valid hardware backends are compiled
- IDL generation provides validated IPC interfaces

### 2. **Zero-Copy Performance**  
- Leased memory eliminates data copying for large inputs
- Direct hardware register access via OpenPRoT HAL
- Session-based state management minimizes allocations

### 3. **Resource Management**
- Bounded session capacity prevents resource exhaustion  
- Automatic cleanup on session drop
- Feature-gated dependencies reduce binary size

### 4. **Platform Extensibility**
- New platforms only need OpenPRoT HAL implementation
- Feature flags enable platform-specific optimizations
- Standard trait interface ensures consistent behavior

## OpenPRot Implementation Analysis

Based on the OpenPRoT HAL , the digest driver follows the established **Device + Context** pattern:

### **Device + Context Pattern**

The OpenTitan implementation separates concerns between:

1. **Device Types** (`HashDevice`):
   - Represent the hardware accelerator instance
   - Handle device initialization and configuration
   - Implement `DigestInit<Algorithm>` traits for each supported algorithm
   - Manage hardware lifecycle (clocks, resets, etc.)

2. **Context Types** (`HashContext`):
   - Represent active digest computation sessions
   - Implement `DigestOp` trait for update/finalize operations
   - Hold references to device and algorithm-specific state
   - Handle streaming data input and digest output

### **Key Design Elements**:

```rust
// Device implements initialization for each algorithm
impl DigestInit<Sha2_256> for HashDevice {
    type Context = HashContext;
    type Error = HashError;
    
    fn init(&mut self) -> Result<Self::Context, Self::Error> {
        // Configure hardware for SHA-256
        // Return context for this session
    }
}

// Context implements the digest operations  
impl DigestOp for HashContext {
    type Error = HashError;
    
    fn update(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        // Write to hardware FIFO
    }
    
    fn finalize(self) -> Result<&[u8], Self::Error> {
        // Read digest from hardware registers
    }
}
```

### **Hubris Integration**

The digest server extends this pattern for multi-session IPC environments:

- **Session Management**: Maps multiple contexts to session IDs
- **Feature Gates**: Compile-time hardware backend selection
- **IDL Interface**: Algorithm-specific operations generated from `digest.idol`
- **Leased Memory**: Zero-copy data transfer for large inputs

This approach leverages proven OpenPRoT patterns while adding the multi-session capabilities needed for Hubris IPC-based drivers.

This implementation demonstrates the successful transformation from OpenPRoT traits to working IDL-based drivers, providing a foundation for generic driver development across the Hubris ecosystem.
