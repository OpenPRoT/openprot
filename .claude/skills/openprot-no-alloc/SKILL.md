---
name: openprot-no-alloc
description: Use when writing, editing, or reviewing Rust code in this repo's firmware paths (target/, services/, hal/, platform/, drivers/) — enforces no_std / no-dynamic-allocation rules from .github/copilot-instructions.md
---

# No_std / No Dynamic Allocation

Applies to firmware code. Does not apply to `#[cfg(test)]` or host-only tooling code —
those may use `std::vec::Vec` etc. (confirmed existing pattern: `services/orchestrator/sm/src/tests.rs`,
`services/mctp/transport-i2c/src/sender.rs`).

There is no `#[global_allocator]` anywhere in this repo — firmware crates must stay `#![no_std]`
with no `extern crate alloc`.

## Forbidden patterns

| Forbidden | Required alternative |
|---|---|
| `Vec<T>` | `heapless::Vec<T, N>` or a fixed-size array `[T; N]` |
| `HashMap<K, V>` | `heapless::FnvIndexMap<K, V, N>` or fixed-size lookup table |
| `String` | `heapless::String<N>` or `&str` |
| `Box<T>` | stack allocation or `&mut T` reference |

## Checklist before finishing

- [ ] grep the diff for `Vec<`, `HashMap<`, `Box<`, `std::string::String`, `extern crate alloc` outside `#[cfg(test)]`
- [ ] new fixed-capacity collections (`heapless::*`, `[T; N]`) have a sane, justified `N`
- [ ] stack usage is bounded — no unbounded recursion, no large stack-allocated buffers sized by untrusted input
- [ ] crate has `#![no_std]` at the top unless it's a host-only/test crate
- [ ] invoke `ponytail:ponytail-review` on the diff to catch verbosity/over-engineering introduced while adding fixed-capacity types

## Reference

Full checklist: `.github/copilot-instructions.md`
