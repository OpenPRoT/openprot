# openprot-spdm-requester

SPDM requester (client) implementation for OpenPRoT.

## Overview

This crate provides the SPDM requester role, which initiates attestation operations with SPDM responders. The requester sends requests to retrieve:
- Version information
- Device capabilities
- Cryptographic algorithms
- Certificates and measurements
- Challenge-response attestation

## Status

This crate is in early development. Basic infrastructure is in place, but SPDM protocol operations are not yet implemented.

## Dependencies

- `spdm-lib` — SPDM protocol library from 9elements
- `heapless` — `no_std` collections

## Future Work

- Implement SPDM 1.2+ request builders
- Add GET_VERSION, GET_CAPABILITIES support
- Implement CHALLENGE and GET_MEASUREMENTS
- Add certificate chain validation
- Integrate with MCTP transport layer
