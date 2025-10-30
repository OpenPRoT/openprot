# OpenPRoT Operating System Selection

Platform root of trust (PRoT) implementations require an operating system that provides hardware-enforced memory isolation, deterministic behavior, and fault recovery without compromising system integrity.

OpenPRoT addresses these requirements as an open-source, Rust-based platform that provides a secure foundation for platform security. The project offers a Hardware Abstraction Layer (HAL) and suite of services for device attestation, secure firmware updates, and modern security protocols (SPDM, MCTP, PLDM) [5].

The OpenPRoT workgroup (hereafter "the workgroup") evaluated best-in-class Rust embedded OSes to identify the optimal operating system for this security-critical embedded platform.

Rust-based operating systems were a non-negotiable requirement for OpenPRoT due to Rust's fundamental memory safety guarantees.

This whitepaper documents the workgroup's evaluation process and technical rationale for selecting Hubris [2] over Tock OS [3]. Both operating systems implement memory safety through Rust [6], but employ different architectural approaches to isolation, task management, and system composition.

Our evaluation framework assessed:

1. **Memory protection and isolation mechanisms** - Critical for security boundaries
2. **Fault tolerance and recovery capabilities** - Essential for system reliability
3. **Static vs. dynamic system composition** - Impacts predictability and security
4. **System complexity and attack surface** - Affects long-term maintainability and security
5. **Preemptive scheduling and determinism** - Important for responsive system behavior
6. **Debuggability and system observability** - Critical for development, testing, and production monitoring

## Evaluation Criteria Details

**Memory Protection and Isolation Mechanisms**
PRoT requires strict separation between trusted and untrusted components. We evaluated how each OS enforces memory boundaries, prevents unauthorized access between tasks, and isolates drivers from the kernel. Hardware-enforced isolation (Memory Protection Unit - MPU) provides stronger guarantees than software-based partitioning.

**Fault Tolerance and Recovery Capabilities**
Critical infrastructure cannot tolerate cascading failures. We assessed each system's ability to contain faults, restart failed components without affecting others, and maintain system integrity during partial failures. The ability to predict and bound failure modes is essential. Key requirements include in-place component reinitialization capabilities, supervisor-mediated fault recovery, and memory isolation to limit the "blast radius" of failures without requiring system-wide reboots.

**Static vs. Dynamic System Composition**
Runtime flexibility introduces uncertainty in security-critical systems. We compared compile-time system definition (where all components and dependencies are known) against runtime component loading. Static composition enables better security analysis and eliminates entire classes of runtime failures. Key evaluation criteria included compile-time validation capabilities, build-time configuration verification, and the ability to detect resource conflicts and communication path errors before deployment.

**System Complexity and Attack Surface**
PRoT systems have focused requirements that differ from general-purpose embedded applications. We evaluated how each OS architecture aligns with these specific security-critical needs. For platform root of trust implementations, features like dynamic application loading and runtime resource allocation provide valuable capabilities for flexible system deployment, though they introduce considerations around predictability and attack surface analysis.

**Preemptive Scheduling and Determinism**
Platform root of trust implementations require predictable response times for security-critical operations like cryptographic processing and attestation responses. We assessed each system's scheduling guarantees, priority handling, and ability to ensure high-priority security tasks can always preempt lower-priority work within bounded time.

**Debuggability and System Observability**
Complex embedded systems require robust debugging and monitoring capabilities throughout development and deployment. We evaluated each system's approach to runtime inspection, system state visibility, and debugging infrastructure. Traditional console-based debugging introduces security vulnerabilities and code bloat, making kernel-aware debugging tools essential for production systems. The ability to observe system behavior without modifying application code or introducing runtime overhead is critical for security-sensitive platforms.

## Detailed Technical Analysis

### Core Design Decisions

| Feature | Hubris (Oxide) | Tock | Why it matters |
|---------|----------------|------|----------------|
| **System Composition** | **Static**: All tasks defined at compile-time in app.toml configuration, cannot be created/destroyed at runtime. Build system validates all configurations with static assertions. Supports in-place task reinitialization for fault recovery - supervisor task can restart crashed tasks without system reboot. Design philosophy prioritizes eliminating functionality not essential for server management and platform security, resulting in a smaller, more focused codebase to audit and validate. | **Dynamic**: Tasks can be dynamically loaded and assigned. Offers flexibility for diverse application scenarios and runtime adaptation. | Static model with compile-time validation prevents entire classes of runtime failures. In-place restart capability enables component-level recovery, avoiding system-wide reboots for isolated faults. Dynamic models provide flexibility for applications requiring runtime component loading or updates. |
| **Communication** | **Strictly Synchronous**: IPC blocks sender until reply received. Uses rendezvous mechanism inspired by L4 microkernel - kernel performs direct memory copy between tasks, extending Rust's ownership model across task boundaries through leasing. | **Asynchronous**: Callback-based notifications for applications. | Synchronous communication eliminates race conditions, enables precise fault isolation (REPLY_FAULT at error point), and simplifies kernel design by avoiding complex message queue management. |
| **Fault Isolation** | **Disjoint Protection Domains**: Drivers and kernel in separate, MPU-enforced memory spaces. Failing driver cannot corrupt kernel. | **Shared Protection Domain**: Drivers run in same domain as kernel but are partitioned by Rust's type system and capsule architecture. Capsules are kernel modules that rely on Rust's memory safety (borrowing rules, lifetime management) and trait-based interfaces for isolation rather than hardware memory protection. | Hardware-enforced isolation provides robust defense against faults. Memory-safe languages alone don't prevent all failures in critical systems. |
| **Embedded CPU Architecture Support** | **ARM Cortex-M:** Official native support included.<br> **RISC-V** Designed with RISC-V in mind, but currently only has unnofficial support from outside developers including OpenPRoT partners. | **ARM Cortex-M:** Official native support included.<br> **RISC-V** Official native support included.<br> **x86 (32bit):**  Official native support included. |  |
| **Licensing** | **Mozilla Public License Version 2.0**: Commercial use allowed, May be combined with proprietary code, Modified MPL files must be shared and remain MPL, Explicit patent grant included, Must retain copyright notices | **Apache License 2.0**: Commercial use allowed without restrictions, May be combined with proprietary code, Must state significant changes but not required to share, Explicit patent grant included, Must retain copyright notices | Both licenses allow for commercial use and mixing files with other licenses (including proprietary code). The primary difference is that any MPL licensed files must remain under the MPL license, and any changes to those files must be shared publicly. |

### Resource & Memory Management

| Feature | Hubris (Oxide) | Tock | Why it matters |
|---------|----------------|------|----------------|
| **Resource Allocation** | **Fixed**: Memory, hardware, and IRQ allocation determined at build time. Static assertions verify total resource requirements don't exceed physical limits before compilation. Compile-time memory layout with predetermined regions that never change. | **Dynamic**: Resources allocated as applications load. Grant-based dynamic allocation with deterministic memory reclamation through Rust's ownership system and immediate cleanup on process termination. | Build-time allocation with static validation eliminates runtime resource exhaustion. Static allocation provides deterministic usage patterns, critical for long-running server infrastructure. |
| **SRAM Efficiency** | **Maximum**: All code executes in-place from suitable memory regions. SRAM consumption limited to data, heap, and stack only. Static allocation enables compile-time SRAM usage validation. Build system customizes kernel to application, eliminating unused features and achieving optimal resource utilization. | **Optimal**: Kernel and applications execute in-place from suitable memory regions. SRAM consumed by runtime data only (stack, heap, process state). Tock's general-purpose kernel includes features for diverse application scenarios. | Both systems achieve excellent SRAM efficiency through XIP execution. Hubris achieves maximum efficiency through application-specific kernel customization and compile-time optimization, while Tock provides optimal efficiency with runtime flexibility and general-purpose kernel design. |
| **Scheduling** | **Priority-based Preemptive**: Deterministic scheduling with strict priority ordering, higher priority tasks always preempt lower ones. | **Cooperative**: Kernel space cooperation with round-robin userspace scheduling. | Preemptive scheduling ensures critical security operations (cryptographic processing, attestation responses) can respond promptly and predictably, essential for platform trust establishment. |
| **Debuggability** | **Kernel-aware Debugger**: Humility debugger co-developed with Hubris kernel provides deep system inspection through Debug Binary Interface (DBI). No in-application console interfaces or printf formatting. External debugger handles all formatting and command parsing, eliminating security vulnerabilities and code bloat. | **Traditional Console**: UART/USB-based console interfaces with in-application command parsing and printf-style formatting. Provides runtime system inspection and control capabilities. | Kernel-aware debugging eliminates security vulnerabilities from console interfaces while providing superior system observability. Traditional consoles introduce attack surface and code complexity but offer familiar debugging workflows. |

### System Architecture & Philosophy

| Feature | Hubris (Oxide) | Tock | Why it matters |
|---------|----------------|------|----------------|
| **Hardware Abstraction** | **Direct Register Access**: Tasks directly manipulate hardware registers with no abstraction layer. Each task gets explicit hardware permissions defined at compile time. | **Capsule-based**: Higher-level interfaces to hardware resources through Tock's capsule abstraction layer. | Direct register access makes system behavior predictable and easier to audit for security compliance. |
| **Design Philosophy** | **Reliability-focused**: Emphasizes static validation, correctness and predictability over flexibility. Avoids unsolved problems and unnecessary complexity, prioritizing correctness and reliability by construction for high-stakes server management. | **Application-flexible**: Designed for general-purpose embedded systems with dynamic application loading. Targets applications beyond Rust that can be dynamically loaded/replaced/removed separately from kernel, similar to traditional desktop/server OS but for resource-constrained settings. Platform supporting diverse embedded applications including security-critical systems. | Production systems require proven, stable interfaces while flexible platforms enable diverse application scenarios. Different philosophies serve different use cases and constraints. |
| **System Composition** | **Static**: System composition fixed at build time with all dependencies resolved statically. Boot sequence is predictable and repeatable. | **Dynamic**: Runtime component loading and initialization. | Predictable system composition critical for server infrastructure where remote recovery from boot failures is expensive or impossible. |

## Key Findings & Differentiators

The analysis revealed that Hubris's microkernel architecture with MPU-enforced isolation and static task assignment better aligns with PRoT requirements than Tock's dynamic application model.

**Hubris's "Aggressively Static" Philosophy**
Hubris employs comprehensive compile-time validation through static assertions, moving error detection from runtime to build time [1]. All system configuration is declared in app.toml files, with the build system performing extensive checks on task priorities, resource requirements, and communication paths [2]. This approach makes entire classes of runtime failures impossible by construction - if a configuration would lead to resource exhaustion or invalid task communication, the build simply fails with a clear error message.

**Synchronous IPC Design for Robustness**
Hubris implements synchronous, message-based Inter-Process Communication inspired by L4 microkernel design [1]. The rendezvous mechanism operates like cross-task function calls: the sender blocks until the receiver processes the message and replies. This enables direct memory copying between tasks without intermediate queues, extends Rust's ownership model across task boundaries through memory leasing [6], and provides precise fault isolation - a buggy task can be terminated with REPLY_FAULT at the exact error point, preventing fault propagation.

**Component-Level Fault Recovery**
Hubris enables recursive component-level restarts without system reboots through in-place task reinitialization [1]. When a task experiences a kernel-visible fault (memory access violation, panic), the kernel notifies a designated supervisor task, which can restart the failed task by resetting its registers, stack, and resource connections. Memory isolation limits the "blast radius" - corrupt state in one task cannot affect others. This allows individual driver crashes to be handled by restarting just the affected components rather than the entire system, critical for continuous operation in server infrastructure.

**Kernel-Aware Debugging Architecture**
Hubris takes a unique approach to system debugging through its co-developed Humility debugger and Debug Binary Interface (DBI). Rather than implementing traditional console interfaces within applications, Hubris applications contain no printf-level formatting code, command parsing, or console interfaces. Instead, the external Humility debugger provides comprehensive system inspection capabilities through kernel-aware debugging protocols.

This architecture eliminates common security vulnerabilities associated with console interfaces - buffer overflows, format string vulnerabilities, and command injection attacks - while reducing application code size by removing formatting and parsing logic. The DBI allows applications to declare variables and types that the debugger can automatically discover and manipulate, providing superior observability without runtime overhead or security compromise.

Hubris includes comprehensive core dump support, enabling the capture of complete system snapshots into files for post-mortem analysis. These dumps can be loaded into Humility for offline debugging, allowing detailed investigation of system failures without requiring access to the live hardware. This capability proves particularly valuable for security-critical systems where traditional debugging interfaces would introduce unacceptable attack surface, enabling thorough failure analysis while maintaining production system security.

**Critical Architectural Differences**
Key differentiators include Hubris's hardware-enforced memory boundaries, user-space driver architecture, and compile-time system composition versus Tock's software-based isolation for kernel drivers (capsules) [4] and runtime application loading. In Tock, capsules are kernel modules that share the same privilege level and address space as the kernel core, with isolation achieved through Rust's type system, borrowing checker, and carefully designed trait boundaries rather than hardware memory protection. Hubris eliminates dynamic memory allocation, task creation/destruction, and runtime resource management [2], while Tock maintains flexibility through grant-based dynamic allocation and runtime component loading [3,4].

These architectural differences have direct implications for security guarantees, system predictability, and fault containment in PRoT applications.

## Conclusion & Recommendation

For OpenPRoT platform root of trust implementation, **Hubris is the recommended operating system choice**. This decision reflects a deliberate preference for architectural simplicity and predictability over flexibility in the context of security-critical infrastructure.

Both Hubris and Tock represent sophisticated, well-engineered approaches to embedded operating systems, each with distinct technical merits. Tock's dynamic application model, innovative capsule architecture, and flexible resource management demonstrate significant technical innovation and provide valuable capabilities for many embedded applications. Its production deployments in security-critical systems demonstrate the maturity of the platform.

However, for platform root of trust applications, OpenPRoT prioritizes avoiding complexity over gaining flexibility. Hubris's static task model, hardware-enforced isolation, and deterministic behavior align with the fundamental requirement that PRoT systems "cannot fail." The choice reflects a conscious trade-off: accepting the constraints of static system composition in exchange for eliminating entire classes of runtime uncertainties and potential failure modes.

This decision is specific to the security-critical, infrastructure-focused requirements of platform root of trust implementations. Different embedded applications with varying requirements for flexibility, multi-tenancy, or dynamic component loading might reasonably reach different conclusions in their OS selection process.

## References

1. Biffle, C. L. (2024). *On Hubris and Humility*. https://cliffle.com/blog/on-hubris-and-humility/
2. Hubris Operating System Documentation. *Hubris Kernel Design and Implementation*. https://github.com/oxidecomputer/hubris
3. Tock Operating System. *Tock OS Documentation and Design Principles*. https://www.tockos.org/
4. Levy, A., et al. (2017). *Multiprogramming a 64kB Computer Safely and Efficiently*. Proceedings of the 26th Symposium on Operating Systems Principles (SOSP '17). https://doi.org/10.1145/3132747.3132786
5. OpenPRoT Workgroup. *Platform Root of Trust Architecture and Requirements*. https://github.com/OpenPRoT/openprot
6. Klabnik, S. & Nichols, C. *The Rust Programming Language: Memory Safety and Zero-Cost Abstractions*. https://doc.rust-lang.org/book/
