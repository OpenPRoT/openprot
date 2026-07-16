# System-Level Firmware Update Demo (Host-Initiated)

## 1. Overview

This document describes the **system-level firmware update demo** proposed by Dwarka Partani, focused on demonstrating **host-initiated firmware updates across a platform** using OpenPRoT architecture.

The goal is to evolve beyond device-level demonstrations and showcase a **realistic, end-to-end platform workflow** aligned with Open Compute Project (OCP) expectations.

The proposed demo emphasizes:
- Host-driven orchestration (CPU as initiator)
- Platform-wide firmware update pathways
- Standardized management protocols (PLDM)
- Integration across system components

This demo direction reflects internal discussions around elevating update functionality into a **system-level capability**, rather than a localized firmware feature. 【1-6860eb】

---

## 2. Objectives

### Primary Objectives
- Demonstrate **firmware updates initiated from the host CPU**
- Show **system-level coordination** across multiple components
- Validate **PLDM-based control flows** over hardware transports (I2C/I3C)
- Align demo with **OCP Global Summit expectations**

### Secondary Objectives
- Highlight OpenPRoT’s role in **platform orchestration**
- Demonstrate integration of:
  - RoT firmware
  - Device firmware
  - Update and recovery workflows
- Provide a **forward-looking architecture demo** beyond MVP functionality

---

## 3. Demo Scope

### Core Scenario
The demo centers around a **host-initiated firmware update flow**, where:
- The host CPU initiates an update request
- Control is transferred over a standardized protocol (PLDM)
- The update propagates to one or more downstream components

### Key Characteristics
- Initiation point: **Host CPU (not device-local)**
- Protocol: **PLDM over I2C / I3C**
- Target: Platform components (e.g., BMC, peripherals, firmware-managed devices)
- Context: Designed for **OCP Global Summit timeframe** 【1-6860eb】

---

## 4. Architectural Model

### 4.1 High-Level Components

| Component        | Role |
|------------------|------|
| Host CPU         | Initiates firmware update requests |
| OpenPRoT Stack   | Coordinates update orchestration |
| Transport Layer  | PLDM over I2C / I3C |
| RoT Firmware     | Enforces secure update and validation |
| Target Devices   | Receive and apply updates |

---

### 4.2 Logical Flow

1. Host CPU issues firmware update command
2. Command is encoded using PLDM
3. Transport layer delivers message over I2C / I3C
4. OpenPRoT stack processes the update request
5. RoT firmware validates and authorizes the update
6. Firmware is transferred and applied to target devices
7. System reports status back to host

This reflects the **system-oriented update pattern** discussed in the workgroup, including host-driven orchestration and PLDM-based control. 【2-0cad87】

---

## 5. Relationship to Current Demo Direction

### Current Demo Focus (APAC / MVP)
- Secure boot
- Firmware update
- Recovery flows 【3-4dcd71】

### Gap Identified
Current demos treat update as:
- A **localized capability** (device or RoT-level)

### Proposed Enhancement
This demo reframes update as:
- A **system-level workflow**, where:
  - Updates are initiated externally (host)
  - Multiple components participate
  - Protocol-level interoperability is demonstrated

---

## 6. Key Differentiators

### 6.1 System-Level Orchestration
- Moves from **device-centric → platform-centric**
- Demonstrates coordination across components

### 6.2 Host-Initiated Control
- Aligns with **real deployment models**
- Shows integration with CPU-side software stacks

### 6.3 Protocol Alignment
- Emphasizes **PLDM as control plane**
- Enables standardized update mechanisms

### 6.4 OCP-Relevant Use Case
- Demonstrates a **practical platform capability**
- Aligns with expectations for system-level manageability

---

## 7. Risks and Considerations

### 7.1 Integration Complexity
- Requires coordination across:
  - Host stack
  - Transport layer
  - Firmware layers

### 7.2 Protocol Readiness
- PLDM implementation must be sufficiently mature
- Transport bindings (I2C/I3C) must be stable

### 7.3 Hardware Dependencies
- Requires platform capable of:
  - CPU ↔ device communication paths
  - I2C/I3C connectivity
- Discussions indicate use of **AST1060-based platforms** for current demos 【2-0cad87】

---

## 8. Suggested Demo Narrative

> “A system-level firmware update initiated from the host CPU, coordinated through OpenPRoT using industry-standard PLDM messaging over I2C/I3C, demonstrating end-to-end platform update capability.”

---

## 9. Next Steps

- Define **concrete demo topology**
  - Number and type of target devices
- Identify **host-side software components**
- Align on **PLDM message flows**
- Validate **transport readiness (I2C/I3C)**
- Prototype **end-to-end update path**
- Integrate into OCP demo plan

---

## 10. Summary

The demo proposed by Dwarka represents a **step change in scope**:

- From:
  → *Localized firmware capabilities*

- To:
  → *Full system-level orchestration*

This directly supports OpenPRoT’s positioning as:
- A **platform solution**, not just a firmware stack
- A **standardized control plane** for system management
``