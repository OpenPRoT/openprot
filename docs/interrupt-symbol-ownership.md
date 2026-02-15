# Library Design Recommendation: Interrupt Handler Pattern

## Background: How ARM Cortex-M Interrupts Work

On ARM Cortex-M processors, interrupt handling is driven by a **vector table** - an array of function pointers located at a fixed memory address (typically 0x00000000). When an interrupt fires, the hardware:

1. Looks up the corresponding entry in the vector table
2. Jumps directly to that address
3. Executes the Interrupt Service Routine (ISR)

The vector table is typically defined by the PAC (Peripheral Access Crate) using a static array with `extern "C"` function declarations:

```rust
// From a typical PAC (Peripheral Access Crate)
#[link_section = ".vector_table.interrupts"]
#[no_mangle]
pub static __INTERRUPTS: [Vector; N] = [
    Vector { handler: uart0 },
    Vector { handler: uart1 },
    Vector { handler: timer0 },
    Vector { handler: spi },
    // ... N interrupt vectors
];

extern "C" {
    fn uart0();
    fn uart1();
    fn timer0();
    fn spi();
    // ... declared but NOT defined
}
```

The PAC **declares** these symbols but does **not define** them. Someone must provide the actual function implementations.

## The Problem: Symbol Ownership Conflict

### Scenario 1: Standalone Driver Application

When building a standalone application with just the driver library:

```
[PAC] --declares--> uart0()
[Driver] --defines---> #[no_mangle] pub extern "C" fn uart0() { ... }
```

This works perfectly. The driver provides the ISR, the linker resolves the symbol, interrupts work.

### Scenario 2: Driver + Kernel/RTOS

When integrating with a kernel that manages its own vector table:

```
[PAC] --declares--> uart0()
[Driver] --defines---> #[no_mangle] pub extern "C" fn uart0() { ... }  ❌ CONFLICT
[Kernel Entry] --defines--> #[no_mangle] pub extern "C" fn uart0() { ... }  ❌ CONFLICT
```

**Result:** `error: symbol 'uart0' multiply defined`

### Why Kernels Need ISR Ownership

A kernel/RTOS needs to own ISR entry points for several reasons:

1. **Context Saving**: The kernel may need to save thread context before handling the interrupt
2. **Priority Management**: The kernel scheduler may need to run after the ISR
3. **Statistics**: Track interrupt counts, latency, etc.
4. **Safety Boundaries**: Validate that the interrupt is expected before dispatching

The kernel's ISR wrapper typically looks like:

```rust
#[no_mangle]
pub extern "C" fn uart0() {
    // Kernel housekeeping
    kernel::enter_interrupt();
    
    // Dispatch to actual handler
    my_driver::uart::uart0_irq_handler();
    
    // Kernel housekeeping  
    kernel::exit_interrupt();
}
```

### The Fundamental Issue

**A `#[no_mangle]` symbol is a global, unique identifier in the final binary.**

When a library unconditionally exports `#[no_mangle]` symbols, it:
- Claims exclusive ownership of those symbol names
- Prevents any other code from defining them
- Forces a specific interrupt handling strategy
- Makes the library unusable in any context that needs ISR control

## Solution: Separate Logic from Export

### 1. Provide callable handler functions (always available)

```rust
/// UART0 interrupt handler - call this from your ISR
#[inline]
pub fn uart0_irq_handler() {
    // Actual interrupt handling logic
}
```

### 2. Conditionally export ISR symbols via feature flag

```rust
#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn uart0() {
    uart0_irq_handler();
}
```

### 3. Default the feature to off

```toml
[features]
default = []
isr-handlers = []  # Export ISR symbols - disable for kernel integration
```

## Benefits

| Standalone Use | Kernel Integration |
|----------------|-------------------|
| Enable `isr-handlers` feature | Leave feature disabled |
| Library provides vector table entries | Kernel provides ISR stubs |
| Zero integration work | Kernel calls handler functions |

## Integration Example

Kernel entry point calls library handlers:

```rust
#[no_mangle]
pub extern "C" fn uart0() {
    my_driver::uart::uart0_irq_handler();
}

#[no_mangle]
pub extern "C" fn timer0() {
    my_driver::timer::timer0_irq_handler();
}
```

## Guidelines

1. **Never export `#[no_mangle]` symbols unconditionally** in libraries intended for embedded use
2. **Provide public handler functions** that encapsulate interrupt logic
3. **Use feature flags** to control symbol export
4. **Document the pattern** so integrators know how to wire up handlers
5. **Apply consistently** to all ISR exports (timers, peripherals, DMA, etc.)

## Example Implementation

A driver crate implementing this pattern for UART handlers:

```rust
// src/uart.rs

/// UART0 interrupt handler - call this from your ISR
#[inline]
pub fn uart0_irq_handler() {
    dispatch_irq(0);
}

/// UART1 interrupt handler - call this from your ISR
#[inline]
pub fn uart1_irq_handler() {
    dispatch_irq(1);
}

// ISR exports - only when isr-handlers feature is enabled
#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn uart0() {
    uart0_irq_handler();
}

#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn uart1() {
    uart1_irq_handler();
}
```

Cargo.toml:
```toml
[features]
default = []
isr-handlers = []  # Export ISR handlers with #[no_mangle] - disable for kernel integration
```

Kernel integration (entry.rs):
```rust
#[unsafe(no_mangle)]
pub extern "C" fn uart0() {
    my_driver::uart::uart0_irq_handler();
}

#[unsafe(no_mangle)]
pub extern "C" fn uart1() {
    my_driver::uart::uart1_irq_handler();
}
```

## Why Not Use Weak Symbols?

An alternative approach is to use weak linkage:

```rust
#[linkage = "weak"]
#[no_mangle]
pub extern "C" fn uart0() {
    default_handler();
}
```

This allows the symbol to be overridden. However, this approach has drawbacks:

1. **Implicit behavior**: It's not obvious that the symbol can/should be overridden
2. **Toolchain support**: Weak linkage behavior varies across linkers
3. **No compile-time feedback**: You won't know if override worked until runtime
4. **Still pollutes symbol namespace**: The library still exports the symbol

The feature flag approach is explicit, portable, and provides clear compile-time control.

## Repository Structure: Separate Test Crates

Test code that defines ISRs should live in a **separate crate**, not inside the library. This keeps the library clean and avoids leaking test infrastructure to consumers.

### Visual Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         aspeed-rust (workspace)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────┐     ┌─────────────────────────────────┐   │
│  │   aspeed-ddk (lib crate)    │     │  aspeed-ddk-tests (bin crate)   │   │
│  │                             │     │                                 │   │
│  │  src/                       │     │  src/                           │   │
│  │  ├── lib.rs                 │     │  ├── main.rs  ◄── single entry  │   │
│  │  ├── uart.rs                │     │  │   │                          │   │
│  │  │   └── uart0_irq_handler()│◄────┼───────┤ calls test_uart()       │   │
│  │  ├── timer.rs               │     │  │   │ calls test_timer()       │   │
│  │  │   └── timer0_irq_handler()◄────┼───────┤ calls test_i2c()        │   │
│  │  ├── i3c/                   │     │  │   │ calls test_i3c()         │   │
│  │  │   └── i3c_irq_handler()  │◄────┼───────┤ ...                     │   │
│  │  └── i2c/                   │     │  │   └── loop {}                │   │
│  │      └── i2c0_irq_handler() │◄────┼───────┤                         │   │
│  │                             │     │  │                              │   │
│  │  (NO test code here)        │     │  ├── tests/                     │   │
│  │  (NO #[no_mangle] ISRs)     │     │  │   ├── mod.rs                 │   │
│  │                             │     │  │   ├── uart_test.rs           │   │
│  └─────────────────────────────┘     │  │   ├── timer_test.rs          │   │
│                                      │  │   ├── i2c_test.rs            │   │
│                                      │  │   ├── i3c_test.rs            │   │
│                                      │  │   └── gpio_test.rs           │   │
│                                      │  │                              │   │
│                                      │  └── isr.rs ◄── all ISRs here   │   │
│                                      │      ├── #[no_mangle] uart0()   │   │
│                                      │      ├── #[no_mangle] timer()   │   │
│                                      │      ├── #[no_mangle] i3c()     │   │
│                                      │      └── #[no_mangle] i2c()     │   │
│                                      │                                 │   │
│                                      └─────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
aspeed-rust/
├── Cargo.toml                  # workspace definition
├── aspeed-ddk/                 # lib crate - clean, no test ISRs
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # handler functions + conditional exports
│       ├── uart.rs
│       ├── timer.rs
│       ├── i3c/
│       └── i2c/
├── aspeed-ddk-tests/           # bin crate - owns all ISRs
│   ├── Cargo.toml              # depends on aspeed-ddk
│   └── src/
│       ├── main.rs             # single test binary entry point
│       ├── isr.rs              # all #[no_mangle] ISR definitions
│       └── tests/
│           ├── mod.rs
│           ├── uart_test.rs
│           ├── timer_test.rs
│           ├── i2c_test.rs
│           ├── i3c_test.rs
│           └── gpio_test.rs
└── xtask/                      # optional build tooling
```

### Why Separate Crates?

| Inside Library | Separate Crate |
|----------------|----------------|
| Test code ships to consumers | Clean library, no test leakage |
| Feature flags guard test modules | No feature complexity for tests |
| Test deps pollute library deps | Isolated dependency trees |
| `pub mod tests` in lib.rs | Tests are a normal bin crate |

### Test Crate Cargo.toml

```toml
[package]
name = "aspeed-ddk-tests"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "functional-tests"
path = "src/main.rs"

[dependencies]
aspeed-ddk = { path = "../aspeed-ddk" }
```

### Test Crate main.rs

```rust
#![no_std]
#![no_main]

use aspeed_ddk::uart;
use aspeed_ddk::i3c::ast1060_i3c;

mod tests;
mod isr;

#[cortex_m_rt::entry]
fn main() -> ! {
    // Initialize hardware...
    
    // Run tests
    tests::uart_test::run_uart_tests();
    tests::timer_test::run_timer_tests();
    tests::i3c_test::run_i3c_tests();
    
    loop {}
}
```

### Test Crate isr.rs

```rust
use aspeed_ddk::uart;
use aspeed_ddk::i3c::ast1060_i3c;

// Test crate owns all ISRs
#[no_mangle]
pub extern "C" fn uart0() {
    uart::uart0_irq_handler();
}

#[no_mangle]
pub extern "C" fn i3c() {
    ast1060_i3c::i3c_irq_handler();
}

#[no_mangle]
pub extern "C" fn timer() {
    // Test-specific timer handling
}
```

This mirrors how kernel integration works - the integrating crate (kernel or test) depends on the driver library and wires up its own ISRs.

## Common Mistakes to Avoid

### 1. Embedding test code in the library

```rust
// BAD: Test code with ISRs inside the library
// src/lib.rs
pub mod tests;  // Contains #[no_mangle] ISRs!
```

**Fix:** Move tests to a separate crate in the workspace.

### 2. Forgetting peripheral ISRs beyond the obvious ones

Libraries often have ISRs for:
- Timers (timer, timer1, timer2, ...)
- Communication (i2c, spi, uart, ...)
- DMA channels
- Error handlers

**Fix:** Audit all `#[no_mangle]` exports with:
```bash
grep -rn "#\[no_mangle\]" src/
```

### 3. Mixing ISR logic with ISR export

```rust
// BAD: Logic embedded in no_mangle function
#[no_mangle]
pub extern "C" fn uart0() {
    let status = read_status_register();
    if status & IRQ_PENDING != 0 {
        // 50 lines of handling logic
    }
}
```

**Fix:** Put logic in a callable function, export just calls it:
```rust
pub fn uart0_irq_handler() {
    // All the logic here
}

#[cfg(feature = "isr-handlers")]
#[no_mangle]
pub extern "C" fn uart0() {
    uart0_irq_handler();
}
```

## Checklist for Library Contributors

- [ ] Identify all `#[no_mangle]` ISR exports in your library
- [ ] Create a feature flag (e.g., `isr-handlers`) that defaults to off
- [ ] Refactor each ISR into: handler function + conditional export
- [ ] Move test code with ISRs to a separate crate in the workspace
- [ ] Document the integration pattern in your README
- [ ] Provide example code showing kernel integration

## Related Patterns

This pattern applies beyond interrupt handlers to any `#[no_mangle]` symbol:
- Panic handlers (`#[panic_handler]`)
- Exception handlers (HardFault, MemManage, etc.)
- Entry points (`#[entry]`, `main`)
- Allocator hooks (`#[global_allocator]`)

The principle is the same: **libraries should provide functionality, not claim global symbols**.
