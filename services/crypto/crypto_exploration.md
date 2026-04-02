# Crypto Service Architecture

## Overview

The crypto service is an IPC-based cryptographic driver with a pluggable backend architecture. It provides hash, MAC, AEAD, and digital signature operations to unprivileged tasks via a Pigweed kernel IPC channel.

## Backend Abstraction

The `api` crate defines two traits parameterized by algorithm marker types:

- **`OneShot<A>`** — single-call operation (compute and return result)
- **`Streaming<A>`** — session-based operation (begin / feed / finish)

Algorithm markers: `Sha256`, `Sha384`, `Sha512`, `HmacSha256`, `HmacSha384`, `HmacSha512`, `Aes256GcmEncrypt`, `Aes256GcmDecrypt`, `EcdsaP256Sign`, `EcdsaP256Verify`, `EcdsaP384Sign`, `EcdsaP384Verify`.

`RustCryptoBackend` is a zero-sized, stateless struct that implements these traits using community RustCrypto crates (`sha2`, `hmac`, `aes-gcm`, `p256`, `p384`). The server never calls RustCrypto directly — it calls `backend.compute()` generically.

## Dispatch via Rust's Type System

Rather than a match-on-opcode dispatch table, the server maps each `CryptoOp` variant to a monomorphized `do_oneshot::<AlgorithmMarker>()` call. Adding a new algorithm means adding a marker type, one `OneShot` impl, and one dispatch line — the server logic stays unchanged.

## Semantic Input Typing

A `CryptoInput<'a>` enum is constructed from the wire format before reaching the backend:

```rust
pub enum CryptoInput<'a> {
    Digest { data: &'a [u8] },
    Mac { key: &'a [u8], data: &'a [u8] },
    Aead { key: &'a [u8], nonce: &'a [u8], data: &'a [u8] },
    Sign { private_key: &'a [u8], message: &'a [u8] },
    Verify { public_key: &'a [u8], message: &'a [u8], signature: &'a [u8] },
}
```

Backends receive structured, named fields — never a flat buffer to interpret.

## Stack Layers

```
Application
  → CryptoClient (ergonomic API)
  → IPC (channel_transact)
  → crypto_server_loop / dispatch_crypto_op
  → do_oneshot::<A> (generic monomorphized dispatch)
  → RustCryptoBackend::compute
  → sha2 / hmac / aes-gcm / p256 crates
```

## Supported Algorithms

| Category | Algorithm | Output | Notes |
|----------|-----------|--------|-------|
| Hash | SHA-256, SHA-384, SHA-512 | 32/48/64 bytes | One-shot and streaming |
| MAC | HMAC-SHA256, HMAC-SHA384, HMAC-SHA512 | 32/48/64 bytes | One-shot |
| AEAD | AES-256-GCM seal/open | 16-byte tag | In-place, 256-bit key, 96-bit nonce |
| Signature | ECDSA P-256, ECDSA P-384 | 64/96 bytes | RFC 6979 deterministic, feature-gated |

## Extensibility

The backend is a pluggable seam. A hardware-accelerated backend (e.g. ASPEED HACE) could replace `RustCryptoBackend` by implementing the same `OneShot<A>` / `Streaming<A>` traits — the server, client, and wire protocol are unaffected.

## File Map

| File | Purpose |
|------|---------|
| `api/src/protocol.rs` | Wire format, `CryptoOp` enum, request/response headers |
| `api/src/backend.rs` | `Algorithm`, `OneShot<A>`, `Streaming<A>`, `CryptoInput<'a>`, `BackendError` |
| `backend-rustcrypto/src/lib.rs` | `RustCryptoBackend` impls for all algorithm combinations |
| `server/src/main.rs` | IPC server loop, `dispatch_crypto_op`, `do_oneshot<A>` |
| `client/src/lib.rs` | `CryptoClient`, `ClientError`, ergonomic free functions |

## Binary Sizes

| Component | Flash | RAM |
|-----------|-------|-----|
| Client | 5.7 KB | 16 KB |
| Server | 42.7 KB | 48 KB |
