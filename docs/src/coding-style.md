# Coding Style

This document describes the coding style used by this project.
All of the formatters can be invoked by the `pw` utility script in the root of
the project:

```
$ ./pw format
```

## Rust

This project follows the standard [Rust Style
Guide](https://doc.rust-lang.org/style-guide/) for Rust code.  Formatting is
done with `rustfmt` using the
[`rustfmt.toml`](https://pigweed.googlesource.com/pigweed/pigweed/+/refs/heads/main/rustfmt.toml)
from the upstream [Pigweed](https://pigweed.googlesource.com/pigweed/pigweed) repository.

Beyond formatting, openprot is embedded, security-critical firmware and review
enforces the following constraints. See `.github/copilot-instructions.md` for
the full list.

- **`no_std` only.** No heap types (`Vec`, `String`, `HashMap`, `Box`). Use
  fixed-size arrays and `heapless` collections.
- **Panic-free.** No `unwrap`, `expect`, `panic!`, or direct `[]` indexing in
  production paths. Prefer `get()`, pattern matching, or returning
  `Result` / `Option`.
- **Checked arithmetic.** Use `checked_*` / `saturating_*` / `wrapping_*` rather
  than bare `+`, `-`, `*` on integers where overflow is possible.
- **Volatile MMIO.** Register access uses `read_volatile` / `write_volatile`
  and must go through HAL traits rather than touching registers directly.
- **`unsafe` blocks require `// SAFETY:` comments** that explain why the
  invariants hold.
- **Constant-time crypto.** Use [`subtle`] for comparing secrets, and
  [`zeroize`] to bound the lifetime of secret material in memory.

[`subtle`]: https://docs.rs/subtle
[`zeroize`]: https://docs.rs/zeroize

## Python

This project follows [PEP8](https://peps.python.org/pep-0008/) for Python code.
Formatting is done with the `black` formatter.

## Starlark (bazel build system)

This project follows the standard [bzl style
guide](https://bazel.build/rules/bzl-style) for Starlark code.
Formatting is done with the `buildifier` tool.

## C / C++

This project follows the [Google C++ Style Guide] for C and C++ code, with the
following exceptions:
- Indent is 4 spaces instead of 2.
- Function names should use `snake_case` instead of `PascalCase`.
- In pointer declarations, the asterisk is attached to the variable name (`int
  *foo`) instead of the type name (`int* foo`).

Formatting is done with `clang-format`.
