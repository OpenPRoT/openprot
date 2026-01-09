# I2C API

This section documents the complete client API for interacting with I2C devices.

## Device Identification

Each I2C device is identified by a 5-tuple:

```rust
pub struct I2cDevice {
    pub task: TaskId,              // I2C driver task
    pub controller: Controller,     // I2C peripheral (I2C0-I2C7)
    pub port: PortIndex,           // Pin configuration
    pub segment: Option<(Mux, Segment)>, // Optional multiplexer
    pub address: u8,               // 7-bit device address
}
```

## Data Types

### Controller

```rust
pub enum Controller {
    I2C0 = 0,
    I2C1 = 1,
    I2C2 = 2,
    I2C3 = 3,
    I2C4 = 4,
    I2C5 = 5,
    I2C6 = 6,
    I2C7 = 7,
}
// Note: AST1060 supports I2C0-I2C13
```

### SlaveMessage

```rust
pub struct SlaveMessage {
    /// The 7-bit I2C address of the master that sent this message
    pub source_address: u8,
    /// Length of the message data in bytes (0-255)
    pub data_length: u8,
    /// The message data (only first data_length bytes are valid)
    pub data: [u8; 255],
}

impl SlaveMessage {
    pub fn data(&self) -> &[u8] {
        &self.data[..self.data_length as usize]
    }
}
```

### SlaveConfig

```rust
pub struct SlaveConfig {
    pub controller: Controller,  // Which I2C controller
    pub port: PortIndex,         // Port/pin configuration
    pub address: u8,             // 7-bit slave address (0x08-0x77)
}
```

## Master Mode Methods

```rust
impl I2cDevice {
    /// Write data then read response
    pub fn write_read(&self, wbuf: &[u8], rbuf: &mut [u8]) -> Result<usize, ResponseCode>;

    /// Write register address, read value
    pub fn read_reg<T: AsBytes + FromBytes>(&self, reg: u8) -> Result<T, ResponseCode>;

    /// Write data only
    pub fn write(&self, buf: &[u8]) -> Result<(), ResponseCode>;

    /// Read data only
    pub fn read_into(&self, buf: &mut [u8]) -> Result<(), ResponseCode>;

    /// SMBus block read (length byte first)
    pub fn read_block(&self, reg: u8, buf: &mut [u8]) -> Result<usize, ResponseCode>;
}
```

## Slave Mode Methods

```rust
impl I2cDevice {
    /// Configure this controller as a slave with the given address
    pub fn configure_slave_address(&self, addr: u8) -> Result<(), ResponseCode>;

    /// Start listening for incoming messages
    pub fn enable_slave_receive(&self) -> Result<(), ResponseCode>;

    /// Stop listening
    pub fn disable_slave_receive(&self) -> Result<(), ResponseCode>;

    /// Register for async notifications when messages arrive
    pub fn enable_slave_notification(&self, mask: u32) -> Result<(), ResponseCode>;

    /// Unregister from notifications
    pub fn disable_slave_notification(&self) -> Result<(), ResponseCode>;

    /// Retrieve a received message (call after notification)
    pub fn get_slave_message(&self) -> Result<SlaveMessage, ResponseCode>;
}
```

## Operation Types

### Master Mode Operations

| Operation | Op Code | Description |
|-----------|---------|-------------|
| WriteRead | 1 | Standard I2C write-then-read operation |
| WriteReadBlock | 2 | SMBus block read with length byte |

### Slave Mode Operations

| Operation | Op Code | Description |
|-----------|---------|-------------|
| ConfigureSlaveAddress | 3 | Set slave address |
| EnableSlaveReceive | 4 | Start listening for messages |
| DisableSlaveReceive | 5 | Stop listening |
| EnableSlaveNotification | 6 | Register for notifications |
| DisableSlaveNotification | 7 | Unregister notifications |
| GetSlaveMessage | 8 | Retrieve received message |

## Reserved I2C Addresses

Per I2C specification, certain addresses are reserved:

| Address (7-bit) | Binary | Purpose |
|-----------------|--------|---------|
| 0x00 | 0000000 | General Call |
| 0x01 | 0000001 | CBUS Address |
| 0x02 | 0000010 | Future Bus Reserved |
| 0x03 | 0000011 | Future Purposes |
| 0x04-0x07 | 000010x | High Speed Reserved |
| 0x78-0x7B | 111110x | 10-bit Addressing |
| 0x7C-0x7F | 111111x | Reserved |
