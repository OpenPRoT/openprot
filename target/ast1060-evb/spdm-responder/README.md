# SPDM Responder Application for AST1060-EVB

This is a complete SPDM (Security Protocol and Data Model) responder application for the AST1060-EVB platform. It integrates all platform-specific implementations to provide device attestation and secure communication capabilities.

## Overview

The SPDM responder application demonstrates the complete SPDM platform abstraction layer:

- **Certificate Management**: Uses `Ast1060CertStore` for device certificates
- **Evidence Collection**: Uses `Ast1060Evidence` for device measurements
- **Cryptographic Operations**: Uses `SpdmCryptoHash` and `SpdmCryptoRng` via crypto service
- **Transport Layer**: Ready for MCTP transport integration
- **Protocol Handling**: Uses `SpdmResponder` service for message processing

## Architecture

```text
┌────────────────────────────────────────────────┐
│  SPDM Requester (External Device)             │
└────────────────┬───────────────────────────────┘
                 │ SPDM Protocol
                 │ (over MCTP - TODO)
                 ▼
┌────────────────────────────────────────────────┐
│  SPDM Responder Application                   │
│  ┌──────────────────────────────────────────┐ │
│  │  SpdmResponder Service                   │ │
│  │  - Message processing                    │ │
│  │  - Protocol state machine                │ │
│  └──────────────┬───────────────────────────┘ │
│                 │                              │
│  ┌──────────────┴───────────────────────────┐ │
│  │  Platform Implementations                │ │
│  │  - Ast1060CertStore (certificates)       │ │
│  │  - Ast1060Evidence (measurements)        │ │
│  │  - SpdmCryptoHash (SHA-384 via IPC)      │ │
│  │  - SpdmCryptoRng (RNG via IPC)           │ │
│  │  - [MCTP Transport - TODO]               │ │
│  └──────────────────────────────────────────┘ │
└────────────────┬───────────────────────────────┘
                 │ IPC
                 ▼
┌────────────────────────────────────────────────┐
│  Crypto Service                                │
│  - ECDSA P-384 signing                         │
│  - SHA-384 hashing                             │
│  - Random number generation                    │
└────────────────────────────────────────────────┘
```

## System Configuration

The application is configured in `system.json5`:

- **SPDM Responder App**:
  - Flash: 192KB (SPDM library is large)
  - RAM: 64KB (for SPDM contexts and buffers)
  - Stack: 16KB per thread
  - Objects: CRYPTO (channel to crypto service)

- **Crypto Server**:
  - Flash: 128KB
  - RAM: 64KB
  - Stack: 8KB per thread
  - Objects: CRYPTO (channel handler)

Total memory usage: ~704KB (fits in AST1060's 768KB SRAM)

## Building

Build the SPDM responder system image:

```bash
bazel build --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-responder:spdm_responder
```

This produces a complete system image with:
- Kernel
- SPDM responder application
- Crypto service
- All platform implementations

## Running

### In QEMU

```bash
bazel run --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-responder:spdm_responder_test
```

### On Hardware

Flash the system image to AST1060-EVB and connect via UART to see logs.

## SPDM Protocol Support

The responder supports the following SPDM commands:

1. **GET_VERSION** - Returns supported SPDM versions (1.2, 1.1)
2. **GET_CAPABILITIES** - Returns device capabilities
3. **NEGOTIATE_ALGORITHMS** - Negotiates cryptographic algorithms
4. **GET_DIGESTS** - Returns certificate chain digests
5. **GET_CERTIFICATE** - Returns certificate chains (with chunking support)
6. **CHALLENGE** - Challenge-response authentication
7. **GET_MEASUREMENTS** - Returns device measurements

## Capabilities

The responder advertises:

- **CERT_CAP**: Certificate provisioning
- **CHAL_CAP**: Challenge-response authentication
- **MEAS_CAP**: Measurements with signatures
- **MEAS_FRESH_CAP**: Fresh measurements
- **CHUNK_CAP**: Large message chunking

## Algorithms

Supported cryptographic algorithms:

- **Hash**: SHA-384 (TPM_ALG_SHA_384)
- **Asymmetric**: ECDSA with NIST P-384 (TPM_ALG_ECDSA_ECC_NIST_P384)
- **Measurement**: DMTF Measurement Specification

## Current Status

✅ **Implemented:**
- Complete platform abstraction layer
- All SPDM traits implemented
- Integration with crypto service
- Application structure and build configuration

⚠️ **TODO:**
- MCTP transport integration (currently commented out)
- End-to-end testing with SPDM requester
- Real certificate provisioning (currently using placeholder data)

## Testing

Run the system image test:

```bash
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-responder:spdm_responder_test
```

Check for panics:

```bash
bazel test --platforms=//target/ast1060-evb:ast1060-evb \
    //target/ast1060-evb/spdm-responder:no_panics_test
```

## Logs

The application uses pw_log for logging:

```
[INFO] SPDM Responder App starting
[INFO] SPDM Responder starting...
[INFO] Platform implementations initialized
[INFO] SPDM responder configuration: ResponderConfig { ... }
[INFO] SPDM Responder initialized (waiting for MCTP transport)
[INFO] SPDM Responder thread spawned
```

## Next Steps

To make this fully functional:

1. **Add MCTP Transport**:
   - Implement MCTP service
   - Create `MctpSpdmTransport` wrapper
   - Add MCTP object to system.json5
   - Uncomment transport code in spdm_responder_app.rs

2. **Provision Real Certificates**:
   - Replace placeholder data in cert-store
   - Generate device-specific keys
   - Include real X.509 certificate chain

3. **Add Real Measurements**:
   - Integrate with boot measurements
   - Add measurement event log
   - Include platform-specific measurements

4. **End-to-End Testing**:
   - Set up SPDM requester
   - Test full protocol flow
   - Verify attestation and authentication

## Integration with Other Components

The SPDM responder integrates with:

- **Crypto Service** (`services/crypto/server`): Provides cryptographic operations
- **Cert Store** (`target/ast1060-evb/cert-store`): Device certificates and signing
- **Evidence** (`target/ast1060-evb/evidence`): Device measurements
- **Hash Service** (`services/spdm/hash`): SHA-384 operations
- **RNG Service** (`services/spdm/rng`): Random number generation

## Memory Map

```
0x00000000 - 0x000004A0: Vector table (1184 bytes)
0x000004A0 - 0x00020000: Kernel (~126KB)
0x00020000 - 0x00050000: SPDM responder app (192KB)
0x00050000 - 0x00070000: Crypto server (128KB)
0x00070000 - 0x00090000: Kernel RAM (128KB)
0x00090000 - 0x000B0000: App RAM (128KB)

Total: ~704KB / 768KB SRAM available
```

## License

Licensed under the Apache-2.0 license.
