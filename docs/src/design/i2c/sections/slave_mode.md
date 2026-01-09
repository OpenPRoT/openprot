# Slave Mode Deep Dive

This section provides comprehensive coverage of I2C slave mode operation, which is the primary focus for MCTP-based RoT applications.

## Interrupt-Driven Receive Flow

![Slave Receive Process](../images/slave_receive_process.png)

## Single Subscriber Pattern

Each `openprot-i2c-server` instance has `notification_client: Option<(TaskId, u32)>`. Only one task can receive slave mode notifications from a given driver instance.

**Rationale:**
1. **Simplicity**: Single `Option<(TaskId, u32)>` per driver minimizes state
2. **Clear Ownership**: One task owns slave traffic for that driver instance
3. **Typical Usage**: Most systems have a single protocol handler per bus

## Complete Slave Mode Example

```rust
use drv_i2c_api::*;
use userlib::*;

task_slot!(I2C, i2c_driver);

const I2C_RX_NOTIF: u32 = 0x0001;

fn main() -> ! {
    // Setup slave mode
    let device = I2cDevice::new(
        I2C.get_task_id(),
        Controller::I2C1,
        PortIndex(0),
        None,
        0x1D,  // Our slave address
    );

    device.configure_slave_address(0x1D).unwrap_lite();
    device.enable_slave_receive().unwrap_lite();
    device.enable_slave_notification(I2C_RX_NOTIF).unwrap_lite();

    let mut msg_buf = [0u8; 256];

    loop {
        let msg = sys_recv_open(&mut msg_buf, I2C_RX_NOTIF);

        if msg.sender == TaskId::KERNEL
            && (msg.operation & I2C_RX_NOTIF) != 0
        {
            match device.get_slave_message() {
                Ok(slave_msg) => {
                    let source = slave_msg.source_address;
                    let data = slave_msg.data();
                    handle_message(source, data);
                }
                Err(ResponseCode::NoSlaveMessage) => {
                    // Spurious notification, continue
                }
                Err(e) => {
                    // Handle error
                }
            }
        }
    }
}
```

## Multiple I2C Controllers

Different tasks can use slave mode on different I2C controllers independently:

![Multiple Controllers Routing](../images/multiple_controllers_routing.png)

## Performance Characteristics

| Metric | Target | Notes |
|--------|--------|-------|
| Interrupt to buffer | < 5µs | Critical path, hardware dependent |
| Buffer to notification | < 1µs | sys_post() overhead |
| Notification to client | < 10µs | Kernel scheduling latency |
| IPC get_message | < 5µs | Buffer pop + IPC |
| **Total latency** | **< 25µs** | Interrupt to message in client |

## Buffer Management

### Master Mode

Master mode operations are **synchronous** and use caller-provided buffers:

```rust
// Caller provides buffers - no driver allocation needed
let write_data = [0x42];
let mut read_data = [0u8; 16];
device.write_read(0x50, &write_data, &mut read_data)?;
```

### Slave Mode: Single Message Buffer

The current implementation uses a **single message buffer** per driver instance:

```rust
// In openprot-i2c-server
let mut pending_slave_msg: Option<(u8, SlaveMessage)> = None;
let mut notification_client: Option<(TaskId, u32)> = None;
```

**Design Rationale:**
- Minimizes memory usage on resource-constrained platforms
- Matches the interrupt-driven model where each interrupt corresponds to one message
- Simplifies implementation and reduces stack usage
- Client must retrieve messages promptly before next interrupt

**Message Flow:**

![Slave Buffer Flow](../images/slave_buffer_flow.png)

**Important Constraint:** If a second message arrives before the client retrieves the first, the first message will be overwritten.

### Memory Budget

```
Master Mode (synchronous - no buffering):
- 0 bytes (uses caller buffers)

Slave Mode (1 message buffer):
- SlaveMessage: 257 bytes (1 addr + 1 len + 255 data)
- State overhead: ~16 bytes
- Total: ~273 bytes per controller

Total driver state: ~270 bytes
```

### Hardware Buffer Interaction

**Two-Level Buffering:**

1. **Hardware Buffer**: Temporary storage
   - AST1060: 32-byte packet buffer (NOT a FIFO)
   - STM32H7: 32-byte FIFO
   - LPC55: 16-byte FIFO
   - Must be read quickly in interrupt

2. **Software Buffer**: Our message queue
   - Holds complete message
   - Survives across interrupts
   - Required for all architectures
