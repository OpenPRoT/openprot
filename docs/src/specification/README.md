# OpenPRoT Specification

Version: v0.5 - Work in Progress

## Introduction

The concept of a Platform Root of Trust (PRoT) is central to establishing a
secure computing environment. A PRoT is a trusted component within a system that
serves as the foundation for all security operations. It is responsible for
ensuring that the system boots securely, verifying the integrity of the firmware
and software, and performing critical cryptographic functions. By acting as a
trust anchor, the PRoT provides a secure starting point from which the rest of
the system's security measures can be built. This is particularly important in
an era where cyber threats are becoming increasingly sophisticated, targeting
the lower layers of the computing stack, such as firmware, to gain persistent
access to systems.

OpenPRoT is a project intended to enhance the security and transparency of PRoTs
by defining and building an open source firmware stack that can be run on a
variety of hardware implementations. Open source firmware offers several
benefits that can enhance the effectiveness and trustworthiness of a PRoT.
Firstly, open source firmware allows for greater transparency, as the source
code is publicly available for review and audit. This transparency helps
identify and mitigate vulnerabilities more quickly, as a global community of
developers and security experts can scrutinize the code. It also reduces the
risk of hidden backdoors or malicious code, which can be a concern with
proprietary firmware.

Moreover, an open source firmware stack can foster innovation and collaboration
within the industry. By providing a common platform that is accessible to all,
developers can contribute improvements, share best practices, and develop new
security features that benefit the entire ecosystem. This collaborative approach
can lead to more robust and resilient firmware solutions, as it leverages the
collective expertise of a diverse community. Additionally, open source firmware
can enhance interoperability and reduce vendor lock-in, giving organizations
more flexibility in choosing hardware and software components that best meet
their security needs.

Incorporating an open source firmware stack into a PRoT not only strengthens the
security posture of a system but also aligns with broader industry trends
towards openness and collaboration. As organizations increasingly recognize the
importance of securing the foundational layers of their computing environments,
the combination of a PRoT with open source firmware represents a powerful
strategy for building trust and resilience in the face of evolving cyber
threats.

## Background

TBD

### Goals

TBD

### Use cases

TBD

## Industry standards and specifications

TBD

## Threat Model

TBD

## High Level Architecture

TBD

### Block Diagram

TBD

## Middleware

OpenPRoT middleware consists of support libraries necessary to implement Root of
Trust functionality, telemetry, and firmware management. Support for DMTF
protocols such as MCTP, SPDM, and PLDM are provided.

*   [MCTP](middleware/mctp.md)
*   [SPDM](middleware/spdm.md)
*   [PLDM](middleware/pldm.md)

## Firmware Resiliency

FW Resiliency Firmware resiliency is a critical concept in modern cybersecurity,
particularly as outlined in the NIST SP 800-193 specification. As computing
devices become more integral to both personal and organizational operations, the
security of their underlying firmware has become paramount. Firmware is often a
target for sophisticated cyberattacks because it operates below the operating
system, making it a potential vector for persistent threats that can evade
traditional security measures. NIST SP 800-193 addresses these concerns by
providing a comprehensive framework for enhancing the security and resiliency of
platform firmware, ensuring that systems can withstand, detect, and recover from
attacks.

The NIST SP 800-193 guidelines focus on three main pillars: protection,
detection, and recovery. Protection involves implementing measures to prevent
unauthorized modifications to firmware, such as using cryptographic techniques
to authenticate updates. Detection is about ensuring that any unauthorized
changes to the firmware are quickly identified, which can be achieved through
integrity checks and monitoring mechanisms. Recovery is the ability to restore
firmware to a known good state after an attack or corruption, ensuring that the
system can continue to operate securely. By addressing these areas, the
guidelines aim to create a robust defense against firmware-level threats, which
are increasingly being exploited by attackers seeking to gain deep access to
systems.

In the context of NIST SP 800-193, firmware resiliency is not just about
preventing attacks but also about ensuring continuity and trust in the system.
The specification recognizes that while it is impossible to eliminate all risks,
having a resilient firmware infrastructure can significantly mitigate the impact
of potential breaches. This approach is particularly important for critical
infrastructure and enterprise environments, where the integrity and availability
of systems are crucial. By adopting the principles of NIST SP 800-193, we can
enhance our security posture, protect sensitive data, and maintain operational
stability in the face of evolving cyber threats.

### PRoT Resiliency

TBD

### Connected Device Resiliency

TBD

## Services

*   [Firmware Update](services/fwupdate.md)
*   Firmware Recovery (TBD)
*   Secure Boot (TBD)
*   Policy Management (TBD)

## Device Abstraction

* [Device Abstraction](device_abstraction/README.md)

## Terminology

* [Terminology](terminology.md)

