# MCTP

MCTP OpenPRoT devices shall support MCTP as the transport for all DMTF
protocols.

## Versions

The *minimum required* MCTP version is 1.3.1 (DSP0236.) Support for MCTP 2.0.0
(DSP0256) may be introduced in a future version of this spec.

## Required Bindings

Currently only one binding is *mandatory* in the OpenPRoT specification, though
this will change in future versions.

1.  MCTP over SMBus (DSP0237, 1.2.0)

## Recommended Bindings

1.  MCTP over I3C (DSP0233, 1.0.1)
2.  MCTP over PCIe-VDM (DSP0238, 1.2.1)
    *   Only on platforms utilizing PCIe 6 and up.
3.  MCTP over USB (DSP0283, 1.0.0)

## Required Commands

1.  Set Endpoint ID
2.  Get Endpoint ID
3.  Get MCTP Version Support
4.  Get Message Type Support
5.  Get Vendor Defined Message Support
6.  All commands in the range 0xF0 \- 0xFF

## Optional Commands

1.  All other commands are optional, but may become required in future
    revisions.

## Development TCP Binding

1.  OpenPRoT will provide a TCP binding for developmental purposes.
