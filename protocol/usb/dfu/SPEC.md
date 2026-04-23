# USB Device Firmware Upgrade (DFU) 1.1 Specification

This document describes the USB DFU 1.1 implementation for this project.

## 1. Overview
The USB DFU 1.1 protocol allows for firmware upgrades over USB. This implementation is designed for limited-memory devices and supports multiple alternate settings to address different memory regions (e.g., internal flash, external SPI flash, bootloader).

## 2. USB Descriptors

### 2.1 DFU Functional Descriptor
The DFU functional descriptor follows the USB DFU 1.1 specification.

| Offset | Field | Size | Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| 0 | bLength | 1 | 0x09 | Size of this descriptor in bytes. |
| 1 | bDescriptorType | 1 | 0x21 | DFU FUNCTIONAL descriptor type. |
| 2 | bmAttributes | 1 | 0x07 | bitCanDnload, bitCanUpload, bitManifestationTolerant. |
| 3 | wDetachTimeOut | 2 | 0x0000 | Time in ms to wait after DFU_DETACH. |
| 5 | wTransferSize | 2 | Configurable | Maximum number of bytes the device can accept per control-write. |
| 7 | bcdDFUVersion | 2 | 0x0110 | DFU specification release number in BCD. |

### 2.2 Interface Descriptors
Multiple alternate settings are supported. Each altsetting typically represents a different memory partition.

| Offset | Field | Size | Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| 0 | bLength | 1 | 0x09 | Size of this descriptor in bytes. |
| 1 | bDescriptorType | 1 | 0x04 | INTERFACE descriptor type. |
| 2 | bInterfaceNumber | 1 | Variable | Number of this interface. |
| 3 | bAlternateSetting | 1 | Variable | Value used to select this alternate setting. |
| 4 | bNumEndpoints | 1 | 0x00 | No endpoints used (control only). |
| 5 | bInterfaceClass | 1 | 0xFE | Application Specific Class. |
| 6 | bInterfaceSubClass | 1 | 0x01 | Device Firmware Upgrade. |
| 7 | bInterfaceProtocol | 1 | 0x02 | DFU Mode. |
| 8 | iInterface | 1 | Variable | Index of string descriptor describing this partition. |

## 3. DFU Requests

The following DFU-specific requests are supported on the control endpoint (0):

| Request | Code | Direction | Description |
| :--- | :--- | :--- | :--- |
| DFU_DETACH | 0 | Host to Device | Requests the device to leave its current mode and enter DFU mode. |
| DFU_DNLOAD | 1 | Host to Device | Data packets sent from the host to the device. |
| DFU_UPLOAD | 2 | Device to Host | Data packets sent from the device to the host. |
| DFU_GETSTATUS | 3 | Device to Host | Returns the current state and status of the device. |
| DFU_CLRSTATUS | 4 | Host to Device | Clears error status and returns to dfuIDLE. |
| DFU_GETSTATE | 5 | Device to Host | Returns the current state of the device. |
| DFU_ABORT | 6 | Host to Device | Aborts the current operation and returns to dfuIDLE. |

## 4. State Machine
This implementation follows the DFU 1.1 state machine. For simplicity, all operations (flash erase/write) are performed synchronously within the `DFU_DNLOAD` and `DFU_UPLOAD` request handling, or immediately transition from SYNC to IDLE.

## 5. Application Interface (`DfuHandler`)
The application must provide an implementation of the `DfuHandler` trait.

```rust
pub trait DfuHandler {
    /// Handle a DNLOAD block.
    fn dnload(&mut self, alt: u8, block_num: u16, data: &[u8]) -> DfuResult;
    /// Handle an UPLOAD block.
    fn upload(&mut self, alt: u8, block_num: u16, data: &mut [u8]) -> Result<usize, DfuStatus>;
    /// Perform manifestation.
    fn manifest(&mut self) -> DfuResult;
    /// Abort current operation.
    fn abort(&mut self);
}
```
