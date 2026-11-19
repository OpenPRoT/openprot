# Design

This section contains design documents that provide a detailed overview of the
design and implementation of the OpenPRoT project. These documents are intended
to provide guidance to developers and anyone interested in the internal workings
of the project.

## Documents

-   [**Pigweed Integration Overview**](./pigweed-overview.md): What openprot
    consumes from Pigweed (`pw_kernel`, `pw_log`, `pw_status`, toolchains,
    crate universe, the `./pw` launcher) and where each piece is pinned.
-   [**pw_kernel IPC**](./pw-kernel-ipc.md): How to declare and use channel
    objects to communicate between two `pw_kernel` userspace processes.
    Worked example lives at
    [`target/veer/ipc/`](https://github.com/OpenPRoT/openprot/tree/main/target/veer/ipc).
-   [**Thread priorities**](./thread-priorities.md): How `pw_kernel` schedules
    threads (strict priority, no inheritance) and the scheme servers and
    clients follow, so that a server never ranks below its clients.
-   [**MMIO Policy**](./mmio.md): Where raw register access (`ptr()`, `steal()`,
    volatile) is allowed on AST10x0, the ownership and safety invariants at
    that boundary, prohibited patterns, and how they are enforced.
-   [**Orchestrator**](./orchestrator/orchestrator-overview.md): The eRoT boot-sequence state
    machine (`services/orchestrator/sm`). Covers the two-tier firmware
    verification model (`ComponentAttrs`, eRoT gate, iRoT gate), the
    verification boundary, and the full state/transition table.
-   [**Caliptra MCU**](./caliptra-mcu/README.md): The products that run on the
    MCU of a Caliptra Subsystem (`target/veer/`), with the events and errors
    that the MCI block reports and what the runtime image does about them.
