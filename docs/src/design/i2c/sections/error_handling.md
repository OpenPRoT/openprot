# Error Handling

This section covers error codes, handling patterns, and recovery strategies.

## Error Codes

| Error Code | Description | Common Cause |
|------------|-------------|--------------|
| `BadResponse` | Invalid response from server | Protocol mismatch |
| `BadArg` | Invalid argument in request | Bad address format |
| `NoDevice` | I2C device doesn't exist | Device not connected or wrong address |
| `BadController` | Invalid controller index | Controller not configured |
| `ReservedAddress` | Address is reserved by I2C spec | Using 0x00-0x07 or 0x78-0x7F |
| `BadPort` | Invalid port index | Port not configured |
| `NoRegister` | Device doesn't have requested register | Wrong register address |
| `BusReset` | I2C bus was reset during operation | Bus contention |
| `BusLocked` | I2C bus locked up and was reset | Device holding SDA low |
| `ControllerBusy` | Controller appeared busy and was reset | Incomplete previous transaction |
| `BusError` | General I2C bus error | Electrical issues |
| `OperationNotSupported` | Operation not supported on this hardware | Hardware limitation |
| `TooMuchData` | Data exceeds buffer capacity | Message too large |
| `SlaveAddressInUse` | Target address already configured | Duplicate configuration |
| `SlaveNotSupported` | Target mode not supported on controller | Hardware limitation |
| `SlaveNotEnabled` | Target receive not enabled | Forgot to call enable_slave_receive() |
| `SlaveBufferFull` | Hardware buffer full, messages dropped | Too much traffic |
| `BadSlaveAddress` | Invalid target address | Reserved or out of range |
| `SlaveConfigurationFailed` | Hardware failed to configure target mode | Hardware error |
| `NoSlaveMessage` | No target message available to retrieve | Spurious notification |
| `NotificationFailed` | Failed to register notification | System resource issue |

## Configuration Error Handling

```rust
match device.configure_slave_address(0x1D) {
    Ok(()) => {}
    Err(ResponseCode::BadSlaveAddress) => {
        // Address is reserved (0x00-0x07, 0x78-0x7F) or > 0x7F
    }
    Err(ResponseCode::SlaveAddressInUse) => {
        // Another task already configured this address
    }
    Err(ResponseCode::SlaveNotSupported) => {
        // This controller doesn't support target mode
    }
    Err(e) => {
        // Other configuration error
    }
}
```

## Runtime Error Handling

```rust
match device.get_slave_message() {
    Ok(slave_msg) => {
        process_message(slave_msg.source_address, slave_msg.data());
    }
    Err(ResponseCode::NoSlaveMessage) => {
        // Normal: notification but message already retrieved
    }
    Err(ResponseCode::SlaveNotEnabled) => {
        // Target mode was disabled, re-enable if needed
        device.enable_slave_receive()?;
    }
    Err(e) => {
        // Log and handle other errors
    }
}
```

## Reconfiguration After Error

```rust
// If communication becomes unreliable
device.disable_slave_notification()?;
device.disable_slave_receive()?;

// Re-initialize
device.configure_slave_address(addr)?;
device.enable_slave_receive()?;
device.enable_slave_notification(mask)?;
```

## Error Conversion (AST1060)

```rust
fn convert_error(err: I2cError) -> ResponseCode {
    match err {
        I2cError::NoAcknowledge => ResponseCode::NoDevice,
        I2cError::Timeout => ResponseCode::ControllerBusy,
        I2cError::Invalid => ResponseCode::BadArg,
        I2cError::InvalidAddress => ResponseCode::BadSlaveAddress,
        I2cError::BusError => ResponseCode::BusError,
        I2cError::ArbitrationLost => ResponseCode::BusError,
        I2cError::SlaveError => ResponseCode::SlaveConfigurationFailed,
        I2cError::InvalidController => ResponseCode::BadController,
        I2cError::BufferFull => ResponseCode::SlaveBufferFull,
    }
}
```
