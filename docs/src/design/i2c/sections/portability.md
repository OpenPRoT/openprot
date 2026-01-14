# Portability Considerations

This section documents the Hubris-specific aspects of the I2C subsystem design and provides guidance for adapting to other microkernel platforms.

## Overview

The I2C subsystem was designed specifically for Hubris, a Rust-based microkernel with capability-based memory isolation. Several design decisions were driven by Hubris's lease-based memory sharing model, which affects portability to other systems.

## I2cHardware Trait Design

### Hubris-Specific Aspects

The `I2cHardware` trait uses closure-based buffer access instead of direct slice references:

```rust
fn write_read(
    &mut self,
    device: &I2cDevice,
    wbuf: impl Fn(usize) -> Option<u8>,         // Read from write buffer
    wlen: usize,
    rbuf: impl FnMut(usize, u8) -> Option<()>,  // Write to read buffer
    rlen: usize,
) -> Result<usize, ResponseCode>;
```

This design wraps Hubris's `sys_borrow_read` and `sys_borrow_write` syscalls, which access client memory through capability-based leases. The driver cannot directly access client buffers due to MPU isolation.

**Benefits:**
- Zero-copy operation under memory isolation
- Type-safe capability-based access (compile-time enforced)
- Byte-by-byte access integrates cleanly with hardware polling loops
- Prevents buffer overruns at the syscall level

**Drawbacks:**
- Non-portable API (specific to capability-based systems)
- Higher abstraction overhead than direct slice access
- Incompatible with standard Rust embedded ecosystem (`embedded-hal`)

### Portable Alternative Design

A more portable trait would use standard slice references:

```rust
fn write_read(
    &mut self,
    device: &I2cDevice,
    write_buf: &[u8],
    read_buf: &mut [u8],
) -> Result<(), I2cError>;
```

**Benefits:**
- Compatible with `embedded-hal` and other standard traits
- Lower abstraction overhead
- Direct hardware access (single DMA setup)
- Familiar API for Rust embedded developers

**Drawbacks:**
- Requires shared memory or kernel-mediated copy for isolated tasks
- No compile-time capability enforcement
- Potential buffer overrun vulnerabilities if bounds checking fails

## Porting to Other Microkernels

### pw_kernel (Pigweed)

Google's experimental `pw_kernel` shares similar design philosophy with Hubris: Rust, memory isolation, syscall-based IPC, and static configuration. However, it uses kernel-mediated buffer copy instead of capability-based leases.

**Required Changes:**

1. **Replace closure-based access with slice-based API:**
   ```rust
   // pw_kernel/drivers/i2c/src/lib.rs
   pub trait I2cHardware {
       fn write_read(
           &mut self,
           device: &I2cDevice,
           write_buf: &[u8],
           read_buf: &mut [u8],
       ) -> Result<()>;
   }
   ```

2. **IPC using channel_transact instead of sys_send:**
   ```rust
   use pw_kernel_userspace::syscall;
   
   let bytes_received = syscall::channel_transact(
       handle::I2C_DRIVER,
       &request,
       &mut response,
       Instant::infinite_future(),
   )?;
   ```

3. **Signal-based notifications instead of sys_post:**
   ```rust
   // Wait for channel readable or interrupt
   let signals = syscall::object_wait(
       handle::IPC,
       Signals::READABLE | Signals::INTERRUPT_A,
       Instant::infinite_future(),
   )?;
   ```

4. **Configuration via system.json5 instead of app.toml:**
   ```json5
   {
     apps: [
       {
         name: "i2c_driver",
         process: {
           objects: [
             { name: "IPC", type: "channel_handler" },
             { name: "I2C_IRQ", type: "interrupt", irq: 76 }
           ]
         }
       }
     ]
   }
   ```

See [Porting to Pigweed pw_kernel](porting_pw_kernel.md) for detailed mapping of syscalls, IPC patterns, and configuration.

## Memory Isolation Tradeoffs

Different microkernels make different tradeoffs between security and performance:

| System | Memory Sharing | Performance | Security |
|--------|----------------|-------------|----------|
| Hubris | Capability-based leases | Zero-copy | Compile-time enforced |
| pw_kernel | Kernel-mediated copy | Single copy overhead | Runtime validated |
| seL4 | Capability-based endpoints | Zero-copy | Formally verified |

**Hubris's Choice:**
- Trades API portability for memory safety
- Closure-based access prevents invalid pointer dereferences at compile time
- Lease system enables zero-copy under MPU isolation

**Alternative Approach:**
- Use slice-based API in platform-independent layer
- Implement Hubris-specific adapter that wraps `sys_borrow_*` syscalls
- Better separation between generic logic and platform-specific memory access

## Abstraction Layer Options

### Option 1: Keep Hubris-Specific Trait (Current Design)

```
┌─────────────────┐
│  Client API     │  (Portable - operation codes, device addressing)
├─────────────────┤
│  I2C Server     │  (Portable - state machine, error handling)
├─────────────────┤
│  I2cHardware    │  (Hubris-specific - closure-based buffer access)
├─────────────────┤
│  HW Driver      │  (Platform-specific - register manipulation)
└─────────────────┘
```

**Pros:** Optimized for Hubris's lease system  
**Cons:** Porting requires rewriting server buffer access logic

### Option 2: Adapter Pattern for Portability

```
┌─────────────────────────┐
│  Client API             │  (Portable)
├─────────────────────────┤
│  I2C Server             │  (Portable - uses slice-based API)
├─────────────────────────┤
│  I2cHardwareSlices      │  (Portable trait - slice-based)
├─────────────────────────┤
│  Hubris Adapter         │  (sys_borrow_* → slice access)
├─────────────────────────┤
│  HW Driver              │  (Platform-specific)
└─────────────────────────┘
```

**Pros:** Server logic is portable, only adapter needs porting  
**Cons:** Extra abstraction layer, potential performance overhead

### Option 3: embedded-hal Compatibility Layer

```
┌─────────────────────────┐
│  Client API             │  (Portable)
├─────────────────────────┤
│  I2C Server             │  (Portable)
├─────────────────────────┤
│  embedded-hal traits    │  (Standard Rust embedded ecosystem)
├─────────────────────────┤
│  Platform Adapter       │  (OS-specific: Hubris/pw_kernel)
├─────────────────────────┤
│  HW Driver              │  (Platform-specific)
└─────────────────────────┘
```

**Pros:** Maximal portability, ecosystem compatibility  
**Cons:** embedded-hal doesn't support target mode or complex mux traversal

## Recommendations for Future Ports

1. **Identify Memory Isolation Model Early**
   - Does the target OS use shared memory, capability-based access, or kernel-mediated copy?
   - How does it affect the buffer access pattern in the driver?

2. **Map IPC Primitives**
   - Synchronous request-reply (Hubris, pw_kernel, seL4)
   - Asynchronous callback (FreeRTOS)
   - Message passing (QNX, MINIX)

3. **Understand Notification Mechanisms**
   - Direct task notification (Hubris `sys_post`)
   - Signal-based waiting (pw_kernel `object_wait`)
   - Callback invocation (FreeRTOS, other RTOS)

4. **Review Configuration System**
   - Static TOML/JSON (Hubris, pw_kernel)
   - Dynamic runtime (Linux, Zephyr)
   - Build-time code generation (all static systems)

5. **Consider Adapter Pattern**
   - Keep server logic generic (slice-based API)
   - Platform-specific adapters handle OS syscalls
   - Easier to maintain multiple platform ports

## Related Documentation

- [OS Dependencies](os_dependencies.md) - Hubris syscall usage
- [Architecture](architecture.md) - Three-layer design
