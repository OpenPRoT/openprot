# Design

This section contains design documents that provide a detailed overview of the
design and implementation of the OpenProt project. These documents are intended
to provide guidance to developers and anyone interested in the internal workings
of the project.

## Documents

-   [**Pigweed Integration Overview**](./pigweed-overview.md): What openprot
    consumes from Pigweed (`pw_kernel`, `pw_log`, `pw_status`, toolchains,
    crate universe, the `./pw` launcher) and where each piece is pinned.
-   [**pw_kernel IPC**](./pw-kernel-ipc.md): How to declare and use channel
    objects to communicate between two `pw_kernel` userspace processes.
    Worked example lives at `target/veer/ipc/`.
