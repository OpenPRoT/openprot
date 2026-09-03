---
name: openprot-no-panic
description: Use when writing, editing, or reviewing Rust code in this repo's firmware paths (target/, services/, hal/, platform/, drivers/) — enforces panic-free, explicit-error-handling rules from .github/copilot-instructions.md
---

# Panic-Free / Error Handling

Applies to firmware code. Does not apply to `#[cfg(test)]` or host-only tooling code.

## Forbidden patterns

| Forbidden | Required alternative |
|---|---|
| `value.unwrap()` | `match value { Some(v) => v, None => return Err(...) }` |
| `result.expect("msg")` | `match result { Ok(v) => v, Err(e) => return Err(e.into()) }` |
| `collection[index]` | `collection.get(index).ok_or(Error::OutOfBounds)?` |
| `a + b` (integers) | `a.checked_add(b).ok_or(Error::Overflow)?` (or `saturating_*`/`wrapping_*` if overflow is expected/benign) |
| `panic!(...)` | return a typed `Result` error |

## Checklist before finishing

- [ ] grep the diff for `.unwrap(`, `.expect(`, `panic!`, bare `[i]`/`[idx]` indexing outside `#[cfg(test)]`
- [ ] all fallible operations return `Result` or `Option`
- [ ] every `unsafe` block has a `// SAFETY:` comment explaining why it's sound
- [ ] tests cover error paths, not just the happy path
- [ ] error messages don't leak sensitive data (see `openprot-secure-coding` if touching secrets)
- [ ] invoke `ponytail:ponytail-review` on the diff to catch verbosity/over-engineering introduced while adding error handling

## Reference

Full checklist: `.github/copilot-instructions.md`
