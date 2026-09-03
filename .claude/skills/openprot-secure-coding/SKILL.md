---
name: openprot-secure-coding
description: Use when writing, editing, or reviewing Rust code that touches secrets, crypto, or hardware registers in this repo (target/*/peripherals, target/*/drivers, target/*/registers, hal/, platform/impls) — enforces the security-specific guidelines from .github/copilot-instructions.md
---

# Secrets, Crypto, and Register Access

## Secrets and crypto

- Constant-time comparison for secrets — use the `subtle` crate, never `==`/`!=` on key/MAC/password bytes.
- Zeroize sensitive data after use — use the `zeroize` crate on keys, passwords, intermediate crypto buffers.
- Never include sensitive data (keys, plaintext secrets, raw credentials) in error messages, logs, or `Debug` output.

## Hardware register access

Confirmed pattern across targets — do not regress to bare pointer access:

- Register blocks are wrapped in a driver struct holding a typed register-block handle
  (a generated PAC/`ureg::RegisterBlock` type) — never a bare untyped address.
- The struct is `!Send + !Sync` (e.g. `PhantomData<*const ()>`) when it represents exclusive/singleton
  hardware access.
- Constructors that take a raw address/pointer are `unsafe fn new(...)` with a `// SAFETY:` doc comment
  stating the pointer-validity and exclusive-ownership contract.
- Field reads/writes go through the register block's typed accessors, not raw
  `core::ptr::read_volatile`/`write_volatile`. Raw volatile pointer ops are reserved for bulk MMIO
  buffer copies with no register field to target — not for normal register poking.
- Standard peripheral kinds (GPIO, I2C, crypto/HACE, etc.) implement the matching
  `openprot_hal_blocking::*` trait (see `hal/blocking/src/`), usually in a dedicated
  `hal_impl.rs`/`hal_slave_impl.rs` file. Target-specific capabilities without a core HAL trait yet
  are allowed as documented driver-only extensions (mark with a comment, e.g. "target-specific
  extension not yet in the core HAL").

## Checklist before finishing

- [ ] no `==`/`!=` on secret bytes — uses `subtle`
- [ ] sensitive buffers are zeroized after use
- [ ] no secret/key material in `log`/`pw_log`/error strings or derived `Debug` impls
- [ ] new register access goes through a typed register-block struct with a safety-documented unsafe constructor, not a bare pointer
- [ ] new peripheral driver implements the relevant `openprot_hal_blocking` trait if one exists
- [ ] invoke `ponytail:ponytail-review` on the diff to catch verbosity/over-engineering introduced while adding safety/security handling

## Reference

Full checklist: `.github/copilot-instructions.md`
