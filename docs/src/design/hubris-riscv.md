# Porting Hubris OS to RISC-V

## Executive Summary

Porting Hubris OS to RISC-V would be **trivial** compared to porting most operating systems because Hubris was explicitly designed with architecture portability in mind. The documentation already includes RISC-V specifications, and the architecture-specific code is minimal and well-isolated.

## 🏗️ Architecture-Agnostic Design Philosophy

### Minimal Kernel Surface Area
Hubris follows a **microkernel philosophy** where the kernel does as little as possible:

- **Small syscall set**: Only 14 syscalls total.
- **Preemptive scheduling**: Simple priority-based scheduler.
- **Statically allocated**: All resources determined at compile time
- **Task isolation**: Memory protection via region-based MPU/PMP, not page tables

### Clean Architecture Abstraction

All architecture-specific code is isolated in a single module:

```rust
// sys/kern/src/arch.rs - Current structure
cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        pub mod arm_m;
        pub use arm_m::*;
    } else {
        compile_error!("support for this architecture not implemented");
    }
}
```

Adding RISC-V support requires only:

```rust
// Proposed addition
    } else if #[cfg(target_arch = "riscv32")] {
        pub mod riscv;
        pub use riscv::*;
    } else {
```

## What Already Exists for RISC-V

### Documentation References

The Hubris documentation **already includes RISC-V specifications**:

#### 1. **Syscall Interface** (syscalls.adoc:74-76)
```
=== RISC-V

Syscalls are invoked using the `ECALL` instruction. The rest is TBD.
```

#### 2. **Timer System** (timers.adoc:6)
```
silicon vendors -- the `SysTick` on ARM, the `mtimer` on RISC-V. Hubris provides
a multiplexer for this timer, so that each task appears to have its own.
```

#### 3. **Interrupt Handling** (interrupts.adoc:5-11)
```
Hubris port, but these ideas are intended to translate to RISC-V systems using
controllers like the PLIC.
```

### Architecture Requirements Already Defined

The documentation specifies what any architecture port needs:

1. **32-bit registers**: ✅ RISC-V32 matches
2. **Supervisor call instruction**: ✅ `ECALL` equivalent to ARM's `SVC`
3. **Memory protection**: ✅ RISC-V PMP equivalent to ARM MPU
4. **Standard timer**: ✅ `mtimer` equivalent to ARM `SysTick`
5. **Interrupt controller**: ✅ PLIC equivalent to ARM NVIC

## 🔧 Implementation Requirements (Minimal)

### Architecture Module (~2000 lines total)

Based on the existing ARM implementation (`sys/kern/src/arch/arm_m.rs` - 1901 lines):

| Component | Estimated Lines | Complexity | ARM Equivalent |
|-----------|----------------|------------|----------------|
| **Context switching** | ~300 | Medium | Save/restore `x1-x31` vs `r0-r15` |
| **Syscall entry** | ~200 | Low | `ECALL` handler vs `SVC` handler |
| **Timer integration** | ~100 | Low | `mtimer` vs `SysTick` |
| **Memory protection** | ~200 | Medium | PMP setup vs MPU setup |
| **Interrupt routing** | ~200 | Low | PLIC vs NVIC |
| **Task state management** | ~500 | Medium | TCB save/restore |
| **Boot sequence** | ~100 | Low | Reset handler |
| **Utilities/macros** | ~300 | Low | Architecture helpers |
| **Total** | **~1900** | **Low-Medium** | **Direct translation** |

### Register Mapping (Trivial)

**Current ARM Syscall Convention:**
- Arguments: `r4` through `r10` (7 args)
- Syscall number: `r11`
- Returns: `r4` through `r11` (8 returns)

**Proposed RISC-V Convention:**
- Arguments: `x10-x16` (`a0-a6`) (7 args)
- Syscall number: `x17` (`a7`)
- Returns: `x10-x17` (`a0-a7`) (8 returns)

### Core Functions to Implement

```rust
// Required architecture interface (based on ARM module)
pub fn apply_memory_protection(task: &Task) -> Result<(), FaultInfo>;
pub fn start_task(task: &Task) -> !;
pub fn save_task_state(task: &mut Task);
pub fn restore_task_state(task: &Task);
pub fn current_task_ptr() -> *const Task;
pub fn set_current_task_ptr(task: *const Task);
pub fn usermode_entry_point() -> u32;
pub fn get_task_dump_area() -> &'static mut [u8];
```

### ✅ What Makes Hubris Easy to Port

1. **🎯 Narrow target scope**: Only 32-bit microcontrollers
2. **📦 Rust ecosystem**: RISC-V already well-supported
3. **🔒 Memory safety**: Rust prevents most porting bugs
4. **⚡ Simple execution model**: Privileged kernel, unprivileged tasks
5. **🛡️ Minimal assembly**: Most code is portable Rust
6. **📚 Clear documentation**: Architecture requirements already specified

