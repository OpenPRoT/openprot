# Design

This section contains design documents that provide a detailed overview of the
design and implementation of the OpenProt project. These documents are intended
to provide guidance to developers and anyone interested in the internal workings
of the project.

## Documents

-   [**Generic Digest Server Design Document**](./driver-hubris-hash.md): This
    document describes the design and architecture of a generic digest server
    for Hubris OS that supports both SPDM and PLDM protocol implementations.

-   [**Hubris I2C Subsystem Design**](./i2c/subsystem.md): A complete
    reference for the Hubris I2C subsystem, covering master/slave modes,
    architecture, and MCTP integration.

-   [**Converting Rust HAL Traits to Idol Interfaces**](./rust-trait-to-idl-conversion.md):
    A practical guide that explains how to transform Rust Hardware Abstraction
    Layer (HAL) traits into Idol interface definitions for use in Hubris-based
    systems.
