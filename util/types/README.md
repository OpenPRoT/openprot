# util_types

A collection of common utility types. This crate is `#![no_std]` and suitable for embedded development.

## Types

### [`Blocking`](lib.rs)

A trait for blocking on notifications. Typically implemented by mechanisms that need to wait for an event or notification (e.g., an interrupt).

```rust
pub trait Blocking {
    fn wait_for_notification(&self);
}
```

### [`Opcode`](opcode.rs)

A 32-bit IPC opcode, typically represented as a 4-character ASCII string. It wraps a `u32` and implements `zerocopy` traits (`FromBytes`, `IntoBytes`, `Immutable`) for safe serialization/deserialization.

```rust
pub struct Opcode(u32);

impl Opcode {
    pub const fn new(val: [u8; 4]) -> Self;
}
```

### [`PowerOf2Usize`](power_of_2.rs)

A wrapper around `usize` that is guaranteed to be a power of two. This guarantee allows the compiler to optimize operations (e.g., replacing division with bitwise shifts).

```rust
pub struct PowerOf2Usize(usize);

impl PowerOf2Usize {
    pub const fn new(val: usize) -> Option<Self>;
    pub const fn get(self) -> usize;
}
```
