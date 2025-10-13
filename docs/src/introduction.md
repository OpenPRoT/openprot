# Introduction

Welcome to the OpenPRoT documentation!

This documentation provides comprehensive information about the OpenPRoT
project, including user guides, developer documentation, and API references.

## What is OpenPRoT?

OpenPRoT is an open-source, Rust-based project designed to provide a secure and
reliable foundation for platform security. It offers a flexible and extensible
framework for developing firmware and security-related applications for a
variety of platforms.

At its core, OpenPRoT provides a Hardware Abstraction Layer (HAL) that enables
platform-agnostic application development. On top of this HAL, it offers a suite
of services and protocols for platform security, including device attestation,
secure firmware updates, and more. The project is designed with a strong
emphasis on modern security protocols and standards, such as:

-   **SPDM (Security Protocol and Data Model):** For secure communication and
    attestation.
-   **MCTP (Management Component Transport Protocol):** As a transport for
    management and security protocols.
-   **PLDM (Platform Level Data Model):** For modeling platform components and
    their interactions.

## Project Goals

The primary goals of the OpenPRoT project are to:

-   **Promote Security:** Provide a robust and secure foundation for platform
    firmware, leveraging modern, industry-standard security protocols.
-   **Ensure Reliability:** Offer a high-quality, well-tested, and reliable
    codebase for critical platform services.
-   **Enable Extensibility:** Design a modular and extensible architecture that
    can be easily adapted to different hardware platforms and use cases.
-   **Foster Collaboration:** Build an open and collaborative community around
    platform security.

## Documentation Overview

This documentation is structured to help you understand and use OpenPRoT
effectively. Here’s a guide to the different sections:

-   **[Getting Started](./getting-started.md):** A hands-on guide to setting up
    your development environment and building your first OpenPRoT application.
-   **[Specification](./specification/):** The OpenPRoT specification.
-   **[Repo Structure](./architecture.md):** A high-level overview of the
    OpenPRoT repository structure, including its major components and their
    interactions.
-   **[Usage](./usage.md):** Detailed instructions on how to use the OpenPRoT
    framework and its various services.
-   **[Design](./design/):** In-depth design documents and
    specifications for the various components of OpenPRoT.
-   **[Contributing](./contributing.md):** Guidelines for contributing to the
    OpenPRoT project.

## Repositories

-   https://github.com/OpenPRoT/openprot: Main OpenPRoT repository.
-   https://github.com/OpenPRoT/mctp-rs: MCTP protocol support for Linux and
    embedded platforms.

## Contact Us

-   Email us at openprot-wg@lists.chipsalliance.org
-   Join our public mailing list at https://lists.chipsalliance.org/g/openprot-wg

## Quick Start

To get started with OpenPRoT, you can build and test the project using the
following commands:

```bash
cargo xtask build
cargo xtask test
```

For more detailed instructions, please refer to the
[Getting Started](./getting-started.md) guide.

## Governance

The OpenPRoT Technical Charter can be found at
[here](https://github.com/OpenPRoT/.github/blob/main/GOVERNANCE.md).

## Security Policy

Please refer to the [Security Policy](https://github.com/OpenPRoT/openprot/security/policy)
for more information.

## License

Unless otherwise noted, everything in this repository is covered by the [Apache
License, Version 2.0](https://github.com/OpenPRoT/openprot/blob/main/LICENSE).
