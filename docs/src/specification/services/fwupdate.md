# Firmware Update

Status: Draft

## Overview

This section details the OpenPRoT firmware update mechanism, incorporating the
DMTF standards for PLDM and SPDM, while emphasizing the security and resilience
principles of the project.

Goals

*   To provide a secure and reliable method for updating OpenPRoT firmware.
*   To ensure that firmware updates are authenticated and authorized.
*   To provide a recovery mechanism in the event of a failed update.
*   To align with industry standards for firmware updates (PLDM, SPDM).

Use Cases

*   Updating the OpenPRoT firmware itself.
*   Updating the firmware of downstream devices managed by OpenPRoT.
*   Applying critical security updates and bug fixes.
*   Updating firmware to enable new features.

## PLDM for Firmware Update

OpenPRoT devices will support **PLDM Type 5 version 1.3.0** for Firmware
Updates. This will be the primary mechanism for transferring firmware images and
metadata to the device. PLDM provides a standardized method for managing
firmware updates and is particularly well-suited for out-of-band management
scenarios.

### PLDM Firmware Update Package

The firmware update package is essential for conveying the information required
for the PLDM Firmware Update commands.

*Package Header*

The package will contain a header that describes the contents of the firmware
update package, including:

*   Overall packaging version and date.
*   Device identifier records to specify the target OpenPRoT devices.
*   Downstream device identifier records to describe target downstream devices.
*   Component image information, including classification, offset, size, and
    version.
*   A checksum for integrity verification.
*   Package Payload: Contains the actual firmware component images to be updated

#### Package Header Information

Field                       | Size (bytes) | Definition
:-------------------------- | :----------- | :---------
PackageHeaderIdentifier     | 16           | Set to 0x7B291C996DB64208801B0202E6463C78 (v1.3.0 UUID) (big endian)
PackageHeaderFormatRevision | 1            | Set to 0x04 (v1.3.0 header format revision)
PackageHeaderSize           | 2            | The total byte count of this header structure, including fields within the Package Header Information, Firmware Device Identification Area, Downstream Device Identification Area, Component Image Information Area, and Checksum sections.
PackageReleaseDateTime      | 13           | The date and time when this package was released in timestamp104 formatting. Refer to the PLDM Base Specification for field format definition.
ComponentBitmapBitLength    | 2            | Number of bits used to represent the bitmap in the ApplicableComponents field for a matching device. This value is a multiple of 8 and is large enough to contain a bit for each component in the package.
PackageVersionStringType    | 1            | The type of string used in the PackageVersionString field. Refer to [DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf) Table 33 for values.
PackageVersionStringLength  | 1            | Length, in bytes, of the PackageVersionString field.
PackageVersionString        | Variable     | Package version information, up to 255 bytes. Contains a variable type string describing the version of this firmware update package.
DeviceIDRecordCount         | uint8        | The count of firmware device ID records that are defined within this package.
FirmwareDeviceIDRecords     | Variable     | Contains a record, a set of descriptors, and optional package data for each firmware device within the count provided from the DeviceIDRecordCount field.

#### Firmware Device ID Descriptor

Field                                | Size (bytes) | Definition
:----------------------------------- | :----------- | :---------
RecordLength                         | 2            | The total length in bytes for this record. The length includes the RecordLength, DescriptorCount, DeviceUpdateOptionFlags, ComponentImageSetVersionStringType, ComponentSetVersionStringLength, FirmwareDevicePackageDataLength, ApplicableComponents, ComponentImageSetVersionString, and RecordDescriptors, and FirmwareDevicePackageData fields.
DescriptorCount                      | 1            | The number of descriptors included within the RecordDescriptors field for this record.
DeviceUpdateOptionFlags              | 4            | 32-bit field where each bit represents an update option. bit 0 is set to 1 (Continue component updates after failure).
ComponentImageSetVersionStringType   | 1            | The type of string used in the ComponentImageSetVersionString field. Refer to [DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf) Table 33 for values.
ComponentImageSetVersionStringLength | 1            | Length, in bytes, of the ComponentImageSetVersionString.
FirmwareDevicePackageDataLength      | 2            | Length in bytes of the FirmwareDevicePackageData field. If no data is provided, set to 0x0000.
ReferenceManifestLength              | 4            | Length in bytes of the ReferenceManifestData field. If no data is provided, set to 0x00000000.
ApplicableComponents                 | Variable     | Bitmap indicating which firmware components apply to devices matching this Device Identifier record. A set bit indicates the Nth component in the payload is applicable to this device. bit 0: OpenPRoT RT Image bit 1: Downstream SoC Manifest bit 2 : Downstream SoC Firmware bit 3:: Downstream EEPROM
ComponentImageSetVersionString       | Variable     | Component Image Set version information, up to 255 bytes. Describes the version of component images applicable to the firmware device indicated in this record.
RecordDescriptors                    | Variable     | These descriptors are defined by the vendor. Refer to [DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf) Table 7 for details of these fields and the values that can be selected.
FirmwareDevicePackageData            | Variable     | Optional data provided within the firmware update package for the FD during the update process. If FirmwareDevicePackageDataLength is 0x0000, this field contains no data.
ReferenceManifestData                | Variable     | Optional data field containing a Reference Manifest for the firmware update. If present, it describes the firmware update provided by this package. If ReferenceManifestLength is 0x00000000, this field contains no data.

#### Downstream Device ID Descriptor

Field                         | Size | Definition
:---------------------------- | :--- | :---------
DownstreamDeviceIDRecordCount | 1    | 0

#### Component Image Information

Field                         | Size | Definition
:---------------------------- | :--- | :---------
ComponentClassification            | 2        | 0x000A: Downstream EEPROM, Downstream SoC Firmware, and OpenPRoT RT Image (Firmware), 0x0001: Downstream SoC Manifest (Other)
ComponentIdentifier                | 2        | Unique value selected by the FD vendor to distinguish between component images. 0x0001: OpenPRoT RT Image, 0x0002: Downstream SoC Manifest, 0x0003: 0x0003: Downstream EEPROM 0x1000\-0xFFFF \- Reserved for other vendor-defined SoC images
ComponentComparisonStamp           | 4        | Value used as a comparison in determining if a firmware component is down-level or up-level. When ComponentOptions bit 1 is set, this field should use a comparison stamp format (e.g., MajorMinorRevisionPatch). If not set, use 0xFFFFFFFF.
ComponentOptions                   | 2        | Refer to ComponentOptions definition in [DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf)
RequestedComponentActivationMethod | 2        | Refer to RequestedComponentActivationMethoddefinition in[DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf)
ComponentLocationOffset            | 4        | Offset in bytes from byte 0 of the package header to where the component image begins.
ComponentSize                      | 4        | Size in bytes of the Component image.
ComponentVersionStringType         | 1        | Type of string used in the ComponentVersionString field. Refer to[DMTF Firmware Update Specification v.1.3.0](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.3.0.pdf) Table 33 for values.
ComponentVersionStringLength       | 1        | Length, in bytes, of the ComponentVersionString.
ComponentVersionString             | Variable | Component version information, up to 255 bytes. Contains a variable type string describing the component version.
ComponentOpaqueDataLength          | 4        | Length in bytes of the ComponentOpaqueData field. If no data is provided, set to 0x00000000.
ComponentOpaqueData                | Variable | Optional data transferred to the FD/FDP during the firmware update

## Component Identifiers

| Component Image | Name                      | Description                    |
| :-------------- | :------------------------ | :----------------------------- |
| 0x0             | OpenPRoT RT Image         | OpenPRoT manifest and firmware images (e.g. BL0, RT firmware).                     |
| 0x1             | Downstream SoC Manifest   | SoC manifest covering firmware images. Used to stage verification of the firmware payload. |
| 0x2             | Downstream SoC Firmware   | SoC firmware payload.          |
| 0x3             | Downstream EEPROM         | Bulk update of downstream EEPROM |
| \>= 0x1000      | Vendor defined components |                                |

## PLDM Firmware Update Process

The update process will involve the following steps:

1.  **RequestUpdate**: The Update Agent (UA) initiates the firmware update by
    sending the `RequestUpdate` command to the OpenPRoT device. We refer to
    OpenPRoT as the Firmware Device (FD).
2.  **GetPackageData**: If there is optional package data for the Firmware
    Device (FD), the UA will transfer it to the FD prior to transferring
    component images.
3.  **GetDeviceMetaData**: The UA may also optionally retrieve FD metadata that
    will be saved and restored after all components are updated.
4.  **PassComponentTable**: The UA will send the `PassComponentTable` command
    with information about the component images to be updated. This includes the
    identifier, component comparison stamp, classification, and version
    information for each component image.
5.  **UpdateComponent**: The UA will send the `UpdateComponent` command for each
    component, which includes: component classification, component version,
    component size, and update options. The UA will subsequently transfer
    component images using the `RequestFirmwareData` command..
6.  **TransferComplete**: After successfully transferring component data, the FD
    will send a `TransferComplete` command.
7.  **VerifyComplete**: Once a component transfer is complete the FD will
    perform a verification of the image.
8.  **ApplyComplete**: The FD will use the `ApplyComplete` command to signal
    that the component image has been successfully applied.
9.  **ActivateFirmware**: After all components are transferred, the UA sends the
    `ActivateFirmware` command. If self-contained activation is supported, the
    FD should immediately enable the new component images. Otherwise, the
    component enters a "pending activation" state which will require a reset to
    complete the activation.
10. **GetStatus**: The UA will periodically use the `GetStatus` command to
    detect when the activation process has completed.

For downstream device updates, the UA will use `RequestDownstreamDeviceUpdate`
to initiate the update sequence on the FDP. The rest of the process is similar,
with the FDP acting as a proxy for the downstream device.

##### PLDM Firmware Update Error Handling and Recovery

*   The PLDM specification defines a set of completion codes for error
    conditions.
*   OpenPRoT will adhere to the timing specifications defined in the PLDM
    specification (DSP0240 and DSP0267) for command timeouts and retries.
*   The `CancelUpdateComponent` command is available to cancel the update of a
    component image, and the `CancelUpdate` command can be used to exit from
    update mode. The UA should attempt to complete the update and avoid
    cancelling if possible.
*   OpenPRoT devices **will implement a dual-bank approach for firmware**
    components. This will allow for a fallback to a known-good firmware image in
    case of a failed update. If a power loss occurs prior to the
    `ActivateFirmware` command, the FD will continue to use the currently active
    image, and the UA can restart the firmware update process.
