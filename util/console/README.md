# `util_console`

This subdirectory contains the module `util_console`, which is a generic
output-only console.

The `pw_log` implementation is nice, but is fundamentally tied to
`printf`-like format strings.  In contrast, most Rust code that prints
text is structured around the `Display` and `Debug` traits.  As these
traits are rather heavyweight and require dynamic dispatch, there are
replacement crates (like `ufmt`) which provide a `Debug`/`Display`-like
interface, but are designed for embedded firmware.

The `util_console` crate implements a `ufmt`-based console that can be
used by firmware or host code (e.g. tests) that sends output to the same
output device as the `pw_log` macros.

In addition to supplying rust `print` and `trace` macros, `util_console`
includes a very minimal C `printf` implementation.  The `printf`
implementation is meant to be used in C-based firmware code.
