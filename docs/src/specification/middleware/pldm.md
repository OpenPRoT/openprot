# PLDM

OpenPRoT devices support the Platform Level Data Model (PLDM) as a responder for
firmware updates and platform monitoring. This entails responding to messages of
Type 0 (Base), Type 2 (Platform Monitoring and Control), and Type 5 (Firmware
Update).

## PLDM Base Specifications

### Type 0 - Base Specification

-   **Purpose**: Base Specification and Initialization
-   **Version**: 1.2.0
-   **Specification**:
    [PLDM Base Specification](https://www.dmtf.org/sites/default/files/standards/documents/DSP0240_1.2.0.pdf)

All responders must implement the following mandatory PLDM commands:

-   `GetTID`
-   `GetPLDMVersion`
-   `GetPLDMTypes`
-   `GetPLDMCommands`

All responders must also implement the following optional command:

-   `SetTID`

### Type 2 - Platform Monitoring and Control

-   **Purpose**: Platform Monitoring and Control
-   **Version**: 1.3.0
-   **Specification**:
    [PLDM for Platform Monitoring and Control](https://www.dmtf.org/sites/default/files/standards/documents/DSP0248_1.3.0.pdf)

OpenPRoT supports PLDM Monitoring and Control by providing a Platform Descriptor
Record (PDR) repository to a prospective PLDM manageability access point
discovery agent. These PDRs are defined in JSON files and included in OpenPRoT
at build time, with no support for dynamic adjustments. The PDRs are limited to
security features and will only support PLDM sensors, not effectors.

#### PLDM Monitoring PDRs

-   Terminus Locator PDR
-   Numeric Sensor PDR

### Type 5 - Firmware Update

-   **Purpose**: Firmware Update
-   **Version**: 1.3.0
-   **Specification**:
    [PLDM for Firmware Update](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf)

#### Required Inventory Commands

-   `QueryDeviceIdentifiers`
-   `GetFirmwareParameters`

#### Required Update Commands

-   `RequestFirmwareUpdate`
-   `PassComponentTable`
-   `UpdateComponent`
-   `TransferComplete`
-   `VerifyComplete`
-   `ApplyComplete`
-   `ActivateFirmware`
-   `GetStatus`

All responders must also implement the following optional commands:

-   `GetPackageData`
-   `GetPackageMetaData`
