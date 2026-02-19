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
[`rustfmt.toml`](https://github.com/pigweed-project/pigweed/blob/main/rustfmt.toml)
from the upstream [Pigweed](https://github.com/pigweed-project/pigweed/) repository.

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
