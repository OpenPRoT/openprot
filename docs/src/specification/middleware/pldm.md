## PLDM

PLDM OpenPRoT devices will support Platform Level Data Model as a responder for
FW updates and platform monitoring. This means that OpenPRoT will respond to
Type 0, Type 2 and Type 5 as listed in Table 1\.

## PLDM Base Specifications for Supported Types {#pldm-base-specifications-for-supported-types}

### Type 0 - Base Specification

*   Purpose: Base Specification and Initialization
*   Version: 1.2.0
*   [Platform Level Data Model (PLDM) Base Specification](https://www.dmtf.org/sites/default/files/standards/documents/DSP0240_1.2.0.pdf)

All responders shall implement the four (4) *spec mandatory* PLDM commands:

*   `GetTID`
*   `GetPLDMVersion`
*   `GetPLDMTypes`
*   `GetPLDMCommands`

All responders shall implement the following *optional* commands

*   `SetTID`

### Type 2 - Platform Monitoring and Control

*   Purpose: Platform Monitoring and Control
*   Version: 1.3.0
*   [Platform Level Data Model (PLDM) for Platform Monitoring and Control
    Specification](https://www.dmtf.org/sites/default/files/standards/documents/DSP0248_1.3.0.pdf)

OpenPRoT will support PLDM Monitoring and Control by providing a PDR, Platform
Descriptor Record, repository to a prospective PDLM Manageability Access Point
Discovery Agent’s primary PDR. These PDRs will be defined via Json files and
included into OpenPRoT at build time. OpenPRoT will not support any dynamic
adjustments to the PDR repository. These PDRs should be limited to security
features and as such, will only support PLDM sensors and not effectors. PLDM
Monitoring PDRs

*   Terminus Locator PDR
*   Numeric Sensor PDR

### Type 5 - Firmware Update

*   Purpose: Firmware Update
*   Version: 1.3.0
*   [Platform Level Data Model (PLDM) for Firmware Update Specification](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf)

Required Inventory Commands:

*   `QueryDeviceIdentifiers`
*   `GetFirmwareParameters`

Required Update Commands:

*   `RequestFirmwareUpdate`
*   `PassComponentTable`
*   `UpdateComponent`
*   `TransferComplete`
*   `VerifyComplete`
*   `ApplyComplete`
*   `ActivateFirmware`
*   `GetStatus`

All responders shall implement the following optional commands

*   `GetPackageData`
*   `GetPackageMetaData`
