# Attestation

## 1. Introduction

### 1.1 OpenPRoT Attestation Components

The OpenPRoT firmware stack provides the following attestation capabilities:

**SPDM Responder**: Enables external relying parties to establish trust in OpenPRoT by:
- Responding to attestation requests over SPDM protocol
- Providing cryptographically signed evidence about platform state
- Supporting both initial trust establishment and periodic re-attestation
- Enabling secure session establishment with authenticated endpoints

**SPDM Requester**: Enables OpenPRoT to establish trust in other platform components by:
- Requesting attestation evidence from downstream devices
- Verifying device identities and configurations
- Establishing secure sessions with attested devices
- Supporting platform composition attestation

**Local Verifier**: Enables on-platform verification of attestation evidence by:
- Appraising evidence from platform components without external connectivity
- Supporting air-gapped and latency-sensitive deployments
- Enforcing platform-specific security policies
- Making local trust decisions for platform operations

**Note**: While OpenPRoT includes a local verifier component, verification can also be performed remotely by external verifiers. The choice between local and remote verification depends on deployment requirements, connectivity constraints, and security policies.

## 2. Scope and Purpose

### 2.1 Scope

This specification covers the attestation capabilities provided by the OpenPRoT firmware stack:

**In Scope:**

- **SPDM Responder Implementation**: How OpenPRoT responds to external attestation requests
- **SPDM Requester Implementation**: How OpenPRoT requests attestation from platform devices
- **Local Verifier Architecture**: On-platform evidence appraisal capabilities
- **Evidence Generation**: How OpenPRoT firmware collects and reports platform measurements
- **Evidence Formats**: Standardized structures for conveying attestation claims (OCP RATS EAT, Concise Evidence)
- **Protocol Bindings**: SPDM protocol integration and message flows
- **Device Identity Provisioning**: Owner identity provisioning workflows
- **Reference Value Integration**: How OpenPRoT uses CoRIM for verification
- **Plugin Architecture**: Extensibility for non-OCP evidence formats

### 2.2 Out of Scope

This specification does not cover:

**Hardware-Specific Details:**

- **PRoT Hardware Implementations**: Specific hardware designs, architectures, and capabilities
- **Manufacturing Provisioning**: Secret provisioning into hardware (vendor-specific)
- **Hardware Root of Trust Mechanisms**: Boot ROM implementation, key derivation, measurement collection at hardware level
- **Attester Composition**: Layered measurement and key derivation (hardware-dependent)
- **HAL Trait Implementations**: Specific implementations of HAL traits for particular hardware platforms (integrator responsibility)

**Note on Hardware Variance**: OpenPRoT is a software stack that operates on top of PRoT hardware. The security strength and attestation capabilities of an OpenPRoT-based system depend significantly on the underlying hardware implementation. Hardware vendors must document their specific:
- Root of trust initialization and measurement mechanisms
- Key derivation and protection approaches
- Certificate chain structures
- Cryptographic capabilities and algorithms
- Isolation and protection boundaries

**Other Out of Scope Items:**

- **OpenPRoT Firmware Implementation Details**: Internal firmware architecture (covered in OpenPRoT project documentation)
- **Application-level Attestation Policies**: Use-case specific verification policies
- **Cryptographic Algorithm Specifications**: Defers to NIST and industry standards
- **Remote Verifier Implementation**: External verifier systems (though evidence format is specified)
- **Reference Value Provider Services**: CoRIM generation and distribution infrastructure
- **Transport Layer Details**: Physical/link layer protocols (I2C, I3C, MCTP, etc.)

## 3. Normative References

The following standards are normatively referenced in this specification:

### 3.1 IETF Specifications

- **RFC 9334**: Remote ATtestation procedureS (RATS) Architecture
- **RFC 9711**: Entity Attestation Token (EAT)
- **CoRIM**: Concise Reference Integrity Manifest (IETF Draft)
- **RFC 8949**: Concise Binary Object Representation (CBOR)
- **RFC 9052**: CBOR Object Signing and Encryption (COSE)
- **RFC 5280**: Internet X.509 Public Key Infrastructure Certificate and CRL Profile

### 3.2 TCG Specifications

- **DICE Layering Architecture**: Device Identity Composition Engine
- **DICE Attestation Architecture**: Certificate-based attestation
- **DICE Protection Environment (DPE)**: Runtime attestation service
- **TCG DICE Concise Evidence Binding for SPDM**: Evidence format specification

### 3.3 DMTF Specifications

- **DSP0274**: Security Protocol and Data Model (SPDM) Version 1.3 or later
- **DSP0277**: Secured Messages using SPDM over MCTP Binding
- **DSP0236**: Management Component Transport Protocol (MCTP) Base Specification

### 3.4 Other Standards

- **NIST FIPS 186-5**: Digital Signature Standard (DSS)
- **NIST SP 800-90A**: Recommendation for Random Number Generation
- **NIST SP 800-108**: Recommendation for Key Derivation Functions

---

## 4. Terminology and Definitions

### 4.1 Attestation Roles

Following IETF RATS RFC 9334, the OpenPRoT attestation architecture defines the following roles:

**Attester**: An entity (OpenPRoT firmware and associated platform components) that produces attestation evidence about its state and configuration.

**Relying Party**: An entity that depends on the validity of attestation evidence to make operational decisions. In OpenPRoT deployments, this is typically:
- External platform owner or management system (for initial trust establishment)
- Platform management controller (for periodic verification)
- Cloud service provider infrastructure (for fleet management)

**Verifier**: An entity that appraises attestation evidence against reference values and policies to produce attestation results. OpenPRoT supports:
- **Local Verifier**: Running within OpenPRoT firmware for on-platform verification
- **Remote Verifier**: External system performing verification (implementation not specified here)

**Endorser**: An entity that vouches for the authenticity and properties of attestation components. For OpenPRoT:

**PRoT Hardware**: The underlying hardware platform that provides the root of trust capabilities (secure boot, cryptographic acceleration, isolated execution, OTP storage).

**SPDM Responder Role**: OpenPRoT acting as an SPDM responder to provide attestation evidence to external requesters.

**SPDM Requester Role**: OpenPRoT acting as an SPDM requester to obtain attestation evidence from platform devices.

**Local Verifier**: The verification component within OpenPRoT that appraises evidence from platform devices without requiring external connectivity.

**Platform Composition**: The complete set of attested components including OpenPRoT and downstream devices.

### 4.3 Key Attestation Terms

**Root of Trust (RoT)**: The foundational hardware and immutable firmware that serves as the trust anchor for the platform. In OpenPRoT context, this is the PRoT hardware's boot ROM.

**Compound Device Identifier (CDI)**: A cryptographic secret derived from measurements and used as the basis for key derivation in DICE.

**Target Environment**: A uniquely identifiable component or configuration that is measured and attested. In OpenPRoT:
- OpenPRoT firmware components (bootloader, runtime firmware)
- Hardware configurations (fuse settings, security configurations)
- Platform devices (when acting as SPDM requester)

**TCB (Trusted Computing Base)**: The set of components that must be trusted for the security properties of a system to hold.

**Evidence**: Authenticated claims about platform state produced by the Attester. OpenPRoT generates evidence in multiple formats:
- DICE certificates with TCBInfo extensions
- TCG Concise Evidence
- RATS Entity Attestation Token (EAT)

**Reference Values**: Known-good measurements provided by the Reference Value Provider for comparison during verification. Typically distributed as CoRIM (Concise Reference Integrity Manifest).

**Endorsement**: Authenticated statements about device properties or certifications.

**Appraisal Policy**: Rules used by the Verifier to evaluate evidence against reference values.

**Freshness**: Property ensuring that evidence represents current platform state, typically achieved through nonces or timestamps.

### 4.4 DICE/DPE Terms

**UDS (Unique Device Secret)**: A hardware-unique secret provisioned during manufacturing, stored in OTP/fuses, used as the root secret for DICE key derivation.

**IDevID (Initial Device Identity)**: The manufacturer-provisioned device identity derived from UDS.

**LDevID (Local Device Identity)**: An operator-provisioned device identity that can be used in place of IDevID.

**Alias Key**: A DICE-derived key that represents a specific layer in the boot chain.

**DPE (DICE Protection Environment)**: A service that extends DICE principles to runtime, allowing dynamic context creation and key derivation.

**DPE Context**: A chain of measurements representing a specific execution path through the system.

**DPE Handle**: An identifier for a specific DPE context, used to extend measurements or derive keys.

### 4.5 SPDM Terms

**SPDM Session**: An authenticated and optionally encrypted communication channel between SPDM requester and responder.

**Measurement Block**: A collection of measurements representing a specific component or configuration.

**Slot**: An SPDM certificate chain storage location (Slot 0-7).

**GET_MEASUREMENTS**: SPDM command to retrieve attestation measurements.

**GET_CERTIFICATE**: SPDM command to retrieve certificate chains.

**CHALLENGE**: SPDM command to request signed evidence with freshness.

---

## 5. Attestation Architecture Overview

### 5.1 RATS Architecture Mapping

OpenPRoT implements the IETF RATS architecture with specific role assignments:

**High-Level Flow:**

1. Relying Party (Platform Owner, CSP, Management System) needs to establish trust in the platform
2. Relying Party requests attestation evidence from OpenPRoT via SPDM
3. OpenPRoT (Attester) generates evidence containing measurements and claims
4. Verifier (Remote or Local) receives evidence and appraises it
5. Verifier retrieves reference values and endorsements from Reference Value Provider
6. Verifier applies appraisal policy and generates attestation result
7. Attestation result is conveyed to Relying Party
8. Relying Party makes trust decision based on attestation result

**Components:**

- **Attester**: OpenPRoT Firmware Stack + PRoT Hardware
- **Verifier**: Remote verifier system OR OpenPRoT Local Verifier (for device attestation)
- **Relying Party**: External management system OR OpenPRoT (when verifying devices)
- **Reference Value Provider**: OpenPRoT project, hardware vendors, platform integrators
- **Endorser**: Hardware vendors, OpenPRoT project, platform integrators

### 5.2 Evidence Format Strategy

OpenPRoT adopts a standardized approach to evidence generation and verification:

#### 5.2.1 OpenPRoT Evidence Generation (SPDM Responder)

When acting as an SPDM Responder, OpenPRoT produces attestation evidence in the following formats:

**Primary Evidence Format: RATS EAT with OCP Profile**

OpenPRoT generates Entity Attestation Tokens (EAT) following the OCP RATS EAT Attestation Profile. This format provides:

- Standardized container for attestation claims
- CBOR-encoded for efficiency
- COSE-signed for authenticity
- Nonce-based freshness
- TCG Concise Evidence embedded in measurements claim

**EAT Structure:**

The OpenPRoT EAT follows the OCP RATS EAT Attestation Profile specification. For complete details on the EAT structure, claims, and encoding, see:

https://opencomputeproject.github.io/Security/ietf-eat-profile/HEAD/

**Supporting Evidence Formats:**

- **DICE Certificates with TCBInfo**: Certificate chain establishing device identity and boot measurements
- **TCG Concise Evidence**: Standalone format containing reference-triples for measurements
- **SPDM Measurement Blocks**: Native SPDM measurement format for basic compatibility

#### 5.2.2 OpenPRoT Evidence Verification (Local Verifier)

When acting as a Local Verifier, OpenPRoT supports multiple evidence formats:

**Native Support: OCP RATS EAT Profile**

The OpenPRoT Local Verifier natively supports appraisal of evidence in OCP RATS EAT Attestation Profile format. This enables:

- Standardized verification logic for OCP-compliant devices
- Consistent appraisal policy across vendors
- Interoperability with OCP ecosystem devices
- Direct comparison against CoRIM reference values

**Verification Process for OCP EAT:**

1. Validate EAT signature using device certificate chain
2. Verify nonce freshness
3. Extract Concise Evidence from measurements claim
4. Retrieve CoRIM reference values using corim-locator
5. Compare evidence reference-triples against CoRIM reference-triples
6. Apply appraisal policy
7. Generate attestation result

**Extended Support: Evidence Format Plugins**

To accommodate diverse platform ecosystems, OpenPRoT includes an extensibility mechanism for non-OCP-compliant evidence formats:

**Plugin Architecture:**

- **Evidence Parser Plugins**: Parse vendor-specific evidence formats
- **Claim Extractor Plugins**: Extract measurements and claims from proprietary formats
- **Policy Adapter Plugins**: Map vendor-specific claims to OpenPRoT appraisal policies

**Use Cases for Plugins:**

- Legacy devices with proprietary attestation formats
- Vendor-specific evidence structures not yet migrated to OCP profile
- Specialized evidence formats for specific device classes
- Transitional support during ecosystem migration to OCP standards

**Plugin Interface Requirements:**

Plugins must implement the following interfaces:

- **parse_evidence()**: Convert vendor format to internal representation
- **extract_claims()**: Extract target environments and measurements
- **validate_signature()**: Verify evidence authenticity
- **get_reference_values()**: Retrieve or map to reference values
- **apply_policy()**: Execute appraisal logic

**Plugin Integration:**

Platform integrators can add custom plugins to OpenPRoT's local verifier to support their specific device ecosystem while maintaining the core OCP-compliant verification path for standard devices.

### 5.3 OpenPRoT Dual Role Architecture

OpenPRoT operates in two distinct attestation roles depending on the interaction:

#### 5.3.1 OpenPRoT as Attester (SPDM Responder)

When external relying parties need to establish trust in OpenPRoT:

**Flow:**

1. External Relying Party (SPDM Requester) initiates SPDM session
2. SPDM version negotiation and capability exchange
3. Algorithm negotiation
4. Certificate chain retrieval (GET_CERTIFICATE)
5. Measurement request (GET_MEASUREMENTS) with nonce
6. OpenPRoT (SPDM Responder) generates EAT with OCP profile
7. OpenPRoT returns signed EAT containing Concise Evidence
8. Verifier (remote) appraises evidence against reference values
9. Attestation result returned to Relying Party

**Evidence Provided by OpenPRoT:**

- Certificate chain (structure determined by underlying PRoT hardware implementation)
- RATS EAT with OCP Profile containing:
  - TCG Concise Evidence with reference-triples
  - Freshness nonce
  - CoRIM locator URI
- COSE signature using attestation key provided by underlying hardware

**Note on Hardware Dependencies:**

The certificate chain structure and attestation key derivation mechanisms are determined by the underlying PRoT hardware implementation and are outside the scope of OpenPRoT firmware. OpenPRoT leverages the attestation capabilities provided by the hardware platform. Hardware vendors should document their specific:

- Certificate chain structure and hierarchy
- Key derivation mechanisms
- Supported cryptographic algorithms
- Identity provisioning approach

**Use Cases:**

- Initial platform deployment and trust establishment
- Periodic re-attestation for fleet management
- Pre-workload-deployment verification
- Compliance auditing

#### 5.3.2 OpenPRoT as Verifier (SPDM Requester + Local Verifier)

When OpenPRoT needs to establish trust in platform devices:

**Flow:**

1. OpenPRoT (SPDM Requester) initiates SPDM session with platform device
2. SPDM version negotiation and capability exchange
3. Algorithm negotiation
4. Certificate chain retrieval from device (GET_CERTIFICATE)
5. Measurement request (GET_MEASUREMENTS) with nonce
6. Platform Device (SPDM Responder) returns evidence
7. OpenPRoT Local Verifier receives evidence
8. If OCP EAT format: Native verification path
9. If non-OCP format: Plugin-based verification path
10. Local Verifier appraises evidence against reference values
11. Local trust decision made by OpenPRoT
12. Result used for platform composition decisions

**Standard Measurement Report:**

OpenPRoT follows the SPDM Standard Measurement Report format for evidence collection from devices. This standardized approach ensures consistent evidence structure across different device types and vendors.

For complete details on the Standard Measurement Report format, see:

https://github.com/steven-bellock/libspdm/blob/96d08a730ecbe3f05fa3a2cdbf0b7c2613b24a2f/doc/standard_measurement_report.md

**Evidence Received by OpenPRoT:**

- Device certificate chain (structure varies by device implementation)
- Device evidence (OCP EAT preferred, plugin-supported formats allowed)
- Device measurements and claims in Standard Measurement Report format

**Verification Paths:**

- **OCP-Compliant Devices**: Direct verification using native OCP EAT verifier
- **Non-OCP Devices**: Plugin-based parsing and verification
- **Hybrid Platforms**: Mix of OCP and non-OCP devices verified appropriately

**Use Cases:**

- Verifying network cards, storage controllers, accelerators, soc
- Establishing trust in platform composition
- Air-gapped deployments without external verifier access
- Real-time device trust decisions

### 5.4 Attestation Flow

The basic attestation flow follows these steps:

**Phase 1: Measurement Collection (Boot Time)**

1. PRoT Hardware Boot ROM (immutable) starts execution
2. Boot ROM measures OpenPRoT bootloader (First Mutable Code)
3. Hardware-specific key derivation and certificate generation occurs
4. Control transfers to OpenPRoT bootloader
5. Bootloader measures OpenPRoT runtime firmware
6. Hardware-specific measurement chain continues
7. Control transfers to OpenPRoT runtime firmware
8. Runtime firmware initializes attestation services
9. Runtime firmware measures platform components (optional)

**Note:** The specific measurement and key derivation mechanisms in steps 3 and 6 are hardware-dependent and outside the scope of OpenPRoT firmware.

**Phase 2: Evidence Generation (On Request)**

1. External requester initiates SPDM session with OpenPRoT
2. OpenPRoT SPDM Responder receives attestation request (GET_MEASUREMENTS)
3. OpenPRoT collects current measurements from platform state
4. OpenPRoT formats measurements as TCG Concise Evidence (reference-triples)
5. OpenPRoT constructs RATS EAT with OCP Profile:
   - Sets issuer to OpenPRoT identifier
   - Includes requester-provided nonce
   - Embeds Concise Evidence in measurements claim
   - Adds CoRIM locator URI
6. OpenPRoT signs EAT using hardware-provided attestation key (COSE signature)
7. OpenPRoT returns EAT and certificate chain to requester

**Phase 3: Evidence Conveyance**

1. SPDM Responder transmits evidence via SPDM protocol
2. Evidence includes:
   - Certificate chain (for signature verification)
   - Signed EAT (containing measurements)
   - Optional: Additional endorsements
3. Transport layer delivers evidence to verifier

**Phase 4: Reference Value Retrieval**

1. Verifier extracts CoRIM locator from EAT
2. Verifier retrieves reference values CoRIM from repository
3. Verifier retrieves endorsements (device identity, certifications)
4. Verifier validates CoRIM signatures
5. Verifier loads appraisal policy

**Phase 5: Appraisal**

1. Verifier validates EAT signature using certificate chain
2. Verifier checks certificate chain to trusted root
3. Verifier verifies nonce freshness
4. Verifier extracts Concise Evidence from EAT measurements claim
5. Verifier parses reference-triples from Concise Evidence
6. For each target environment in evidence:
   - Compare against CoRIM reference values
   - Check measurements match expected values
   - Verify SVN meets minimum requirements
   - Apply policy rules
7. Verifier generates attestation result

**Phase 6: Trust Decision**

1. Attestation result conveyed to Relying Party
2. Relying Party evaluates result against requirements
3. Relying Party makes operational decision:
   - Accept platform for use
   - Reject platform
   - Request additional evidence
   - Apply restricted usage policy

### 5.5 Trust Model

OpenPRoT's attestation architecture relies on the following trust assumptions:

#### 5.5.1 Hardware Trust Anchor

**Trusted Components:**

- PRoT Hardware Boot ROM (immutable code)
- Hardware cryptographic accelerators
- OTP/Fuse storage for device secrets
- Hardware isolation mechanisms

**Assumptions:**

- Boot ROM is free from vulnerabilities
- Hardware random number generation is cryptographically secure
- Secrets in OTP/fuses cannot be extracted
- Hardware isolation prevents unauthorized access to secrets

**Hardware-Specific Trust:**

The specific trust properties and security guarantees are determined by the underlying PRoT hardware implementation. Hardware vendors must document:

- Root of trust initialization process
- Secret storage mechanisms
- Key derivation approach
- Isolation boundaries
- Cryptographic capabilities

#### 5.5.2 Firmware Trust Chain

**Trust Establishment:**

- Boot ROM measures and authenticates OpenPRoT bootloader
- Bootloader measures and authenticates OpenPRoT runtime
- Each layer's measurements are recorded
- Compromise of any layer results in detectable measurement changes

**Properties:**

- Measurements cannot be forged without detection
- Certificate chain provides cryptographic proof of boot integrity
- Hardware-specific key binding ensures authenticity

**OpenPRoT Scope:**

OpenPRoT firmware operates within the trust chain established by the underlying hardware. The firmware:

- Collects and reports measurements
- Generates evidence in standardized formats
- Implements SPDM responder and requester roles
- Provides local verification capabilities

The underlying measurement and key derivation mechanisms are hardware-dependent.

#### 5.5.3 Cryptographic Trust

**Cryptographic Assumptions:**

- Digital signatures cannot be forged without private key
- Hash collisions are computationally infeasible
- Key derivation functions provide one-way security
- COSE signature scheme provides authenticity and integrity

**Key Protection:**
**Threat:** Attacker provides false reference values to verifier

**Mitigation:**

- CoRIM signed by trusted authority
- Verifier validates CoRIM signature before use
- Secure distribution channels for reference values
- Verifier configured with trusted root certificates

#### 5.6.7 Man-in-the-Middle Attacks

**Threat:** Attacker intercepts and modifies attestation messages

**Mitigation:**

- SPDM secure sessions provide encryption and authentication
- Evidence signed by device prevents modification
- Certificate-based mutual authentication
- Integrity protection on all messages

#### 5.6.8 Plugin Exploitation

**Threat:** Attacker provides false reference values to verifier

**Mitigation:**

- CoRIM signed by trusted authority
- Verifier validates CoRIM signature before use
- Secure distribution channels for reference values
- Verifier configured with trusted root certificates

#### 5.6.7 Man-in-the-Middle Attacks

**Threat:** Attacker intercepts and modifies attestation messages

**Mitigation:**

- SPDM secure sessions provide encryption and authentication
- Evidence signed by device prevents modification
- Certificate-based mutual authentication
- Integrity protection on all messages

### 5.7 Device Identity Provisioning

OpenPRoT supports flexible device identity provisioning to accommodate different deployment models and ownership scenarios.

#### 5.7.1 Identity Types

**Manufacturer-Provisioned Identity:**

- Provisioned by hardware manufacturer during production
- Rooted in hardware-unique secrets
- Provides vendor attestation anchor
- Permanent identity tied to hardware

**Owner-Provisioned Identity:**

- Provisioned by platform owner during deployment
- Enables owner-controlled attestation anchor
- Supports organizational PKI integration
- Can be updated by authorized owner

#### 5.7.2 Owner Identity Provisioning with OpenPRoT

OpenPRoT implements the OCP Device Identity Provisioning specification to enable platform owners to provision owner-controlled identities to devices under their control. OpenPRoT acts as the intermediary between the owner and the device, facilitating secure identity provisioning.

**Provisioning Process:**

The owner identity provisioning follows the standardized flow defined in the OCP specification:

1. **Owner Initiates Provisioning**: Owner uses OpenPRoT to begin owner identity provisioning process

2. **CSR Collection**: OpenPRoT collects Certificate Signing Request (CSR) from the target device
   - Device generates identity key pair internally
   - Device creates CSR containing public key
   - OpenPRoT retrieves CSR from device

3. **Trust Establishment**: OpenPRoT establishes trust in the device's identity key
   - Verifies device's manufacturer-provisioned identity certificate chain
   - Validates that CSR is signed by device
   - Confirms key is hardware-protected
   - Provides attestation evidence to owner

4. **Endorsement Generation**: Owner generates identity endorsement
   - Owner reviews device attestation evidence
   - Owner verifies device trustworthiness
   - Owner signs CSR with owner CA
   - Owner creates identity certificate

5. **Endorsement Provisioning**: OpenPRoT provisions the endorsement to the device
   - OpenPRoT receives signed identity certificate from owner
   - OpenPRoT provisions identity certificate to device
   - Device validates and stores identity certificate

6. **Verification**: OpenPRoT verifies successful provisioning
   - Requests device to use owner-provisioned identity for attestation
   - Validates identity certificate chain
   - Confirms device can sign with identity key

**OpenPRoT's Role:**

- **CSR Broker**: Retrieves CSRs from devices
- **Trust Validator**: Verifies device identity and key protection before owner endorsement
- **Provisioning Agent**: Delivers owner-signed certificates to devices
- **Verification Service**: Confirms successful identity provisioning

**Benefits:**

- **Owner Control**: Platform owners control their attestation trust anchor
- **PKI Integration**: Enables integration with organizational PKI infrastructure
- **Privacy**: Owner-provisioned identity can provide privacy from manufacturer tracking
- **Flexibility**: Supports diverse deployment and ownership models
- **Automated Workflow**: OpenPRoT automates the provisioning process

**Specification Reference:**

For complete details on the device identity provisioning process, see:

https://opencomputeproject.github.io/Security/device-identity-provisioning/HEAD/

## 6. Claims and Target Environments

**TODO**: Define OpenPRoT-specific claims and target environment structures

---

## 7. Evidence Formats

**TODO**: Detail evidence format specifications for OpenPRoT

---

## 8. Reference Values and Endorsements

**TODO**: Describe reference value and endorsement mechanisms

---

## 9. SPDM Protocol Integration

**TODO**: Specify SPDM protocol bindings and requirements

---

## 10. Local Verifier

**TODO**: Define local verifier architecture and capabilities

---

## 11. Attestation Use Cases

**TODO**: Document common attestation scenarios and workflows

---

## 12. Security Considerations

**TODO**: Additional security considerations beyond threat model

---

## 13. Implementation Guidelines

**TODO**: Guidance for implementers of OpenPRoT attestation

---
