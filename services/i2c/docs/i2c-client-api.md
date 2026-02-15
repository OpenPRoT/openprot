# I2C Client API Design

**Version:** 0.1.0  
**Date:** February 15, 2026  
**Status:** Draft  

---

## Overview

This document defines the I2C client API for OpenPRoT Pigweed. The API provides type-safe, ergonomic access to I2C devices through an IPC-based server architecture.

### Design Goals

1. **Type Safety:** Validated addresses, explicit error handling
2. **Ergonomic:** Builder patterns, reduced API surface
3. **Portable:** Transport-agnostic core, IPC details at boundary
4. **Efficient:** Zero-copy where possible, minimal allocations
5. **Testable:** Trait-based design enables mocking

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Code                           │
│            (sensor drivers, MCTP handlers, etc.)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ uses I2cClient trait
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        i2c-api crate                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ I2cAddress  │  │ I2cError    │  │ I2cClient trait         │  │
│  │ BusIndex    │  │ ResponseCode│  │ I2cClientBlocking trait │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ IPC (transport-specific)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      I2C Server Task                            │
│                   (hardware access)                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Types

### I2cAddress

A validated 7-bit I2C address.

```rust
/// A validated 7-bit I2C address.
///
/// I2C addresses are 7 bits (0x00-0x7F), with reserved ranges:
/// - 0x00-0x07: Reserved (general call, CBUS, etc.)
/// - 0x78-0x7F: Reserved (10-bit addressing, device ID)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct I2cAddress(u8);

impl I2cAddress {
    /// Creates a validated address, rejecting reserved ranges.
    pub const fn new(addr: u8) -> Result<Self, AddressError>;
    
    /// Creates an address without validation (for reserved addresses).
    pub const fn new_unchecked(addr: u8) -> Self;
    
    /// Returns the raw 7-bit address.
    pub const fn value(self) -> u8;
    
    /// Returns address formatted for wire (shifted left, R/W bit space).
    pub const fn write_address(self) -> u8;
    pub const fn read_address(self) -> u8;
}

impl TryFrom<u8> for I2cAddress {
    type Error = AddressError;
}
```

### BusIndex

Identifies an I2C bus/controller.

```rust
/// I2C bus identifier.
///
/// Each bus represents a physical I2C controller or a logical bus
/// behind a multiplexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusIndex(u8);

impl BusIndex {
    /// Creates a new bus index.
    pub const fn new(index: u8) -> Self;
    
    /// Returns the raw index value.
    pub const fn value(self) -> u8;
}

// Common bus indices as constants
impl BusIndex {
    pub const BUS_0: BusIndex = BusIndex(0);
    pub const BUS_1: BusIndex = BusIndex(1);
    pub const BUS_2: BusIndex = BusIndex(2);
}
```

### I2cError

Transport-agnostic error type.

```rust
/// I2C operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cError {
    /// High-level error classification.
    pub code: ResponseCode,
    /// Hardware-level error kind (for diagnostics).
    pub kind: Option<I2cErrorKind>,
}

/// Response codes from I2C operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResponseCode {
    Success = 0,
    NoDevice = 1,        // NACK on address
    NackData = 2,        // NACK during data
    ArbitrationLost = 3, // Lost bus arbitration
    BusStuck = 4,        // SDA/SCL stuck low
    Timeout = 5,         // Operation timed out
    InvalidBus = 6,      // Bad bus index
    InvalidAddress = 7,  // Bad address
    BufferTooSmall = 8,  // Read buffer insufficient
    BufferTooLarge = 9,  // Write exceeds limit
    NotInitialized = 10, // Controller not ready
    Busy = 11,           // Controller busy
    Unauthorized = 12,   // Permission denied
    IoError = 13,        // General I/O error
    ServerError = 14,    // Internal server error
}

/// Low-level error classification (compatible with embedded-hal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cErrorKind {
    Bus,
    ArbitrationLoss,
    NoAcknowledge(NoAcknowledgeSource),
    Overrun,
    Other,
}

/// Implement embedded-hal Error trait for compatibility.
impl embedded_hal::i2c::Error for I2cError {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self.kind {
            Some(I2cErrorKind::Bus) => ErrorKind::Bus,
            Some(I2cErrorKind::ArbitrationLoss) => ErrorKind::ArbitrationLoss,
            Some(I2cErrorKind::NoAcknowledge(src)) => ErrorKind::NoAcknowledge(src),
            Some(I2cErrorKind::Overrun) => ErrorKind::Overrun,
            _ => ErrorKind::Other,
        }
    }
}
```

---

## Client Traits

Traits follow the `embedded_hal::i2c::ErrorType` pattern, separating error type
definition from behavior traits for maximum composability.

### ErrorType Pattern

```rust
use embedded_hal::i2c::ErrorType;

/// Alias for I2C error type constraint.
/// Error types must implement embedded-hal's Error trait.
pub trait I2cErrorType {
    /// Error type returned by I2C operations.
    type Error: embedded_hal::i2c::Error + core::fmt::Debug;
}
```

### I2cClient (Core)

The fundamental I2C client trait providing basic operations.

```rust
/// Core I2C client operations.
///
/// This trait defines the fundamental I2C operations without
/// transport-specific details. Implementations handle IPC,
/// direct hardware access, or mocking.
///
/// Uses the `ErrorType` supertrait pattern from embedded-hal.
pub trait I2cClient: I2cErrorType {
    /// Write data to a device, then read response.
    ///
    /// This is the fundamental I2C operation:
    /// - If `write` is non-empty and `read` is empty: write-only
    /// - If `write` is empty and `read` is non-empty: read-only
    /// - If both non-empty: write-then-read (repeated start)
    ///
    /// # Arguments
    ///
    /// * `bus` - I2C bus to use
    /// * `address` - Device address
    /// * `write` - Data to write (can be empty)
    /// * `read` - Buffer for read data (can be empty)
    ///
    /// # Returns
    ///
    /// Number of bytes read on success.
    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error>;

    /// Execute multiple operations as a single transaction.
    ///
    /// All operations execute atomically without releasing the bus.
    fn transaction(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error>;
}

/// I2C operation for transaction sequences.
#[derive(Debug)]
pub enum Operation<'a> {
    /// Write data to device.
    Write(&'a [u8]),
    /// Read data from device.
    Read(&'a mut [u8]),
}
```

### I2cClientBlocking (Convenience)

Extended trait with blocking convenience methods.

```rust
/// Blocking I2C client with convenience methods.
///
/// Extends `I2cClient` with higher-level operations commonly
/// used with register-based I2C devices.
pub trait I2cClientBlocking: I2cClient {
    /// Write data to a device.
    fn write(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        self.write_read(bus, address, data, &mut [])?;
        Ok(())
    }

    /// Read data from a device.
    fn read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        self.write_read(bus, address, &[], buffer)
    }

    /// Read a register value.
    ///
    /// Writes the register address, then reads the value.
    fn read_register<R: AsRef<[u8]>, V: AsMut<[u8]> + Default>(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        register: R,
    ) -> Result<V, Self::Error> {
        let mut value = V::default();
        self.write_read(bus, address, register.as_ref(), value.as_mut())?;
        Ok(value)
    }

    /// Write a register value.
    ///
    /// Writes register address followed by value in single transaction.
    fn write_register<R: AsRef<[u8]>, V: AsRef<[u8]>>(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        register: R,
        value: V,
    ) -> Result<(), Self::Error> {
        // Combine register and value into single write
        self.transaction(bus, address, &mut [
            Operation::Write(register.as_ref()),
            Operation::Write(value.as_ref()),
        ])
    }

    /// Check if a device is present at the given address.
    ///
    /// Performs a zero-length write to probe for ACK.
    fn probe(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<bool, Self::Error> {
        use embedded_hal::i2c::{Error, ErrorKind};
        
        match self.write(bus, address, &[]) {
            Ok(()) => Ok(true),
            Err(e) => {
                // Use embedded-hal Error trait to check error kind
                if matches!(e.kind(), ErrorKind::NoAcknowledge(_)) {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }
}

// Blanket implementation for all I2cClient implementors
impl<T: I2cClient> I2cClientBlocking for T {}
```

---

## Target Mode API

For protocols like MCTP that require responding to incoming transactions.

### TargetClient Trait

```rust
/// I2C target mode operations.
///
/// Allows the device to respond to I2C transactions initiated
/// by other controllers on the bus. Uses notification-based message
/// delivery rather than polling.
///
/// Uses the `ErrorType` supertrait pattern from embedded-hal.
pub trait I2cTargetClient: I2cErrorType {
    /// Configure this controller to respond at the given address.
    fn configure_target_address(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
    ) -> Result<(), Self::Error>;

    /// Enable target receive mode.
    ///
    /// After this call, incoming transactions to the configured
    /// address will trigger notifications.
    fn enable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    /// Disable target receive mode.
    fn disable_receive(&mut self, bus: BusIndex) -> Result<(), Self::Error>;

    /// Wait for incoming target messages.
    ///
    /// Blocks until one or more messages are available, or timeout expires.
    /// Returns the number of messages retrieved.
    fn wait_for_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
        timeout: Option<Duration>,
    ) -> Result<usize, Self::Error>;

    /// Register a notification callback for incoming messages.
    ///
    /// When a target message arrives, the kernel will post a notification
    /// to the calling task. The task can then call `get_pending_messages`
    /// to retrieve the buffered data.
    fn register_notification(
        &mut self,
        bus: BusIndex,
        notification_mask: u32,
    ) -> Result<(), Self::Error>;

    /// Retrieve pending messages after receiving a notification.
    ///
    /// Call this after receiving a target message notification.
    /// Returns the number of messages retrieved.
    fn get_pending_messages(
        &mut self,
        bus: BusIndex,
        messages: &mut [TargetMessage],
    ) -> Result<usize, Self::Error>;
}

/// A message received in target mode.
#[derive(Debug, Clone)]
pub struct TargetMessage {
    /// Address of the controller that sent this message.
    pub source_address: I2cAddress,
    /// Message data (up to 255 bytes).
    data: [u8; 255],
    /// Actual length of data.
    len: u8,
}

impl TargetMessage {
    /// Returns the message data.
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
    
    /// Returns the message length.
    pub fn len(&self) -> usize {
        self.len as usize
    }
    
    /// Returns true if message is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for TargetMessage {
    fn default() -> Self {
        Self {
            source_address: I2cAddress::new_unchecked(0),
            data: [0u8; 255],
            len: 0,
        }
    }
}
```

### Notification-Based Flow

Target mode uses hardware interrupts delivered as task notifications:

```
┌────────────────────────────────────────────────────────────────┐
│  External I2C controller sends data to our target address     │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│  I2C Hardware generates IRQ                                    │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│  Kernel delivers notification to I2C server task               │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│  Server buffers message, posts notification to client task     │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│  Client calls get_pending_messages() to retrieve data          │
└────────────────────────────────────────────────────────────────┘
```

---

## Usage Examples

### Basic Device Access

```rust
use i2c_api::{I2cClient, I2cClientBlocking, I2cAddress, BusIndex};

fn read_temperature<C: I2cClient>(
    client: &mut C,
) -> Result<f32, C::Error> {
    let bus = BusIndex::BUS_0;
    let address = I2cAddress::new(0x48).expect("valid address");
    
    // Read 2-byte temperature register at offset 0x00
    let mut buffer = [0u8; 2];
    client.write_read(bus, address, &[0x00], &mut buffer)?;
    
    // Convert to temperature (device-specific)
    let raw = i16::from_be_bytes(buffer);
    Ok(raw as f32 * 0.0625)
}
```

### Register-Based Device

```rust
use i2c_api::{I2cClientBlocking, I2cAddress, BusIndex};

fn configure_sensor<C: I2cClientBlocking>(
    client: &mut C,
) -> Result<(), C::Error> {
    let bus = BusIndex::BUS_1;
    let address = I2cAddress::new(0x76)?;
    
    // Write configuration register
    client.write_register(bus, address, [0xF4], [0x27])?;
    
    // Read status register
    let status: [u8; 1] = client.read_register(bus, address, [0xF3])?;
    
    Ok(())
}
```

### Multi-Operation Transaction

```rust
use i2c_api::{I2cClient, Operation, I2cAddress, BusIndex};

fn atomic_read_write<C: I2cClient>(
    client: &mut C,
) -> Result<[u8; 4], C::Error> {
    let bus = BusIndex::BUS_0;
    let address = I2cAddress::new(0x50)?;
    
    let command = [0x10, 0x20];
    let mut response = [0u8; 4];
    
    // Execute as single atomic transaction
    client.transaction(bus, address, &mut [
        Operation::Write(&command),
        Operation::Read(&mut response),
    ])?;
    
    Ok(response)
}
```

### Target Mode (MCTP)

```rust
use i2c_api::{I2cTargetClient, I2cAddress, BusIndex, TargetMessage};
use core::time::Duration;

const TARGET_MSG_NOTIFICATION: u32 = 1 << 4;

fn mctp_handler<C: I2cTargetClient>(
    client: &mut C,
) -> Result<(), C::Error> {
    let bus = BusIndex::BUS_2;
    let our_address = I2cAddress::new(0x1D)?;
    
    // Configure and enable target mode with notifications
    client.configure_target_address(bus, our_address)?;
    client.register_notification(bus, TARGET_MSG_NOTIFICATION)?;
    client.enable_receive(bus)?;
    
    let mut messages = [TargetMessage::default(); 4];
    
    loop {
        // Wait for notification from kernel (blocks until message arrives)
        sys_recv_notification(TARGET_MSG_NOTIFICATION);
        
        // Retrieve all pending messages
        let count = client.get_pending_messages(bus, &mut messages)?;
        
        for msg in &messages[..count] {
            process_mctp_message(msg.source_address, msg.data())?;
        }
    }
}

// Alternative: blocking wait with timeout
fn mctp_handler_blocking<C: I2cTargetClient>(
    client: &mut C,
) -> Result<(), C::Error> {
    let bus = BusIndex::BUS_2;
    let our_address = I2cAddress::new(0x1D)?;
    
    client.configure_target_address(bus, our_address)?;
    client.enable_receive(bus)?;
    
    let mut messages = [TargetMessage::default(); 4];
    
    loop {
        // Block waiting for messages (up to 1 second)
        let count = client.wait_for_messages(
            bus,
            &mut messages,
            Some(Duration::from_secs(1)),
        )?;
        
        for msg in &messages[..count] {
            process_mctp_message(msg.source_address, msg.data())?;
        }
    }
}
```

### Device Probing

```rust
use i2c_api::{I2cClientBlocking, I2cAddress, BusIndex};

fn scan_bus<C: I2cClientBlocking>(
    client: &mut C,
    bus: BusIndex,
) -> Result<Vec<I2cAddress>, C::Error> {
    let mut found = Vec::new();
    
    for addr in 0x08..0x78 {
        if let Ok(address) = I2cAddress::new(addr) {
            if client.probe(bus, address)? {
                found.push(address);
            }
        }
    }
    
    Ok(found)
}
```

---

## Error Handling

### Patterns

```rust
use i2c_api::{I2cClient, I2cErrorType};
use embedded_hal::i2c::{Error, ErrorKind};

fn handle_errors<C: I2cClient>(client: &mut C, bus: BusIndex, addr: I2cAddress) {
    let data = [0x00];
    let mut buf = [0u8; 4];
    
    match client.write_read(bus, addr, &data, &mut buf) {
        Ok(n) => println!("Read {} bytes", n),
        
        Err(e) => {
            // Use embedded-hal Error trait for portable error handling
            match e.kind() {
                ErrorKind::NoAcknowledge(_) => {
                    println!("Device not present");
                }
                ErrorKind::ArbitrationLoss => {
                    println!("Lost bus arbitration");
                }
                ErrorKind::Bus => {
                    // Attempt recovery
                    // client.recover_bus(bus);
                }
                _ => {
                    println!("Error: {:?}", e);
                }
            }
        }
    }
}
```

### Error Type with ResponseCode

For application-specific error handling, downcast to `I2cError`:

```rust
use i2c_api::{I2cError, ResponseCode};

fn handle_response_code(err: &I2cError) {
    match err.code {
        ResponseCode::NoDevice => { /* ... */ }
        ResponseCode::Timeout => { /* ... */ }
        ResponseCode::BusStuck => { /* ... */ }
        ResponseCode::Unauthorized => { /* permission error */ }
        _ => { /* other */ }
    }
}
```

---

## Implementation Notes

### IPC Integration

The client traits are transport-agnostic. The IPC implementation depends on the kernel:

```rust
/// Pigweed kernel channel-based I2C client.
/// 
/// Uses pw_channel for communication with the I2C server task.
pub struct I2cChannelClient {
    /// Channel endpoint for sending requests to I2C server
    server_channel: pw_channel::Channel,
}

/// Implement the error type trait (required by I2cClient supertrait).
impl I2cErrorType for I2cChannelClient {
    type Error = I2cError;
}

impl I2cClient for I2cChannelClient {

    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // Encode request into channel message
        // Format: [op, bus, addr, write_len, write_data..., read_len]
        let mut request = [0u8; 256];
        request[0] = OP_WRITE_READ;
        request[1] = bus.value();
        request[2] = address.value();
        request[3] = write.len() as u8;
        request[4..4 + write.len()].copy_from_slice(write);
        request[4 + write.len()] = read.len() as u8;
        
        let request_len = 5 + write.len();
        
        // Send request and wait for response
        self.server_channel.write(&request[..request_len])?;
        
        // Read response: [status, data...]
        let mut response = [0u8; 256];
        let response_len = self.server_channel.read(&mut response)?;
        
        // Check status
        let status = ResponseCode::from_u8(response[0])
            .ok_or(I2cError::from_code(ResponseCode::ServerError))?;
        
        if status != ResponseCode::Success {
            return Err(I2cError::from_code(status));
        }
        
        // Copy response data
        let data_len = (response_len - 1).min(read.len());
        read[..data_len].copy_from_slice(&response[1..1 + data_len]);
        
        Ok(data_len)
    }
    
    fn transaction(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Encode transaction as sequence of operations
        // ... implementation details
        todo!()
    }
}
```

**Note:** The actual IPC mechanism will use Pigweed's Rust channel primitives
(`pw_channel`) or the kernel's native message passing, not `pw_rpc` which is C++.

### Testing

```rust
/// Mock client for testing.
pub struct MockI2cClient {
    expected_calls: Vec<ExpectedCall>,
    call_index: usize,
}

/// Implement the error type trait.
impl I2cErrorType for MockI2cClient {
    type Error = I2cError;
}

impl I2cClient for MockI2cClient {
    fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let expected = &self.expected_calls[self.call_index];
        self.call_index += 1;
        
        assert_eq!(address, expected.address);
        assert_eq!(write, expected.write_data);
        
        let len = expected.response.len().min(read.len());
        read[..len].copy_from_slice(&expected.response[..len]);
        
        expected.result.clone()
    }
    
    fn transaction(
        &mut self,
        _bus: BusIndex,
        _address: I2cAddress,
        _operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Transaction mock implementation
        Ok(())
    }
}
```

---

## Comparison with Hubris API

| Aspect | Hubris | Pigweed (This Design) |
|--------|--------|----------------------|
| Address handling | Raw `u8` | `I2cAddress` newtype |
| Bus routing | Embedded in `I2cDevice` | Separate `BusIndex` parameter |
| Method count | 11+ variants | 3 core + convenience |
| Error type | `ResponseCode` (30 variants) | `I2cError` (15 codes) |
| Target mode | Runtime state checks | Same (consider type-state later) |
| Generics | `IntoBytes + FromBytes` | `AsRef<[u8]>` / concrete |

---

## Future Considerations

### Type-State for Target Mode

```rust
// Potential future API with compile-time state tracking
let configured = client.configure_target(bus, address)?;
let receiving = configured.enable_receive()?;
let messages = receiving.wait()?;
```

### Async Support

```rust
#[async_trait]
pub trait I2cClientAsync {
    async fn write_read(
        &mut self,
        bus: BusIndex,
        address: I2cAddress,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<usize, Self::Error>;
}
```

### Bus Multiplexer Support

```rust
pub struct MuxedBus {
    bus: BusIndex,
    mux_address: I2cAddress,
    channel: u8,
}

impl MuxedBus {
    fn select(&self, client: &mut impl I2cClient) -> Result<(), I2cError>;
}
```

---

## File Structure

```
services/i2c/api/
├── Cargo.toml
├── BUILD.bazel
└── src/
    ├── lib.rs          # Re-exports
    ├── address.rs      # I2cAddress, AddressError
    ├── error.rs        # I2cError, ResponseCode, I2cErrorKind
    ├── client.rs       # I2cClient, I2cClientBlocking traits
    ├── target.rs       # I2cTargetClient, TargetMessage
    └── operation.rs    # Operation enum
```

---

## References

- [Hubris I2C Architecture Review](./hubris-i2c-architecture-review.md)
- [embedded-hal I2C traits](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/)
- [Pigweed pw_i2c](https://pigweed.dev/pw_i2c/)
