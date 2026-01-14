# Porting to Pigweed pw_kernel

This document outlines a strategy for porting the Hubris I2C subsystem to  `pw_kernel` — a Rust-based microkernel in the Pigweed project. Both Hubris and pw_kernel share similar design philosophies: Rust, memory isolation, syscall-based IPC, and static configuration. 


## Architecture Comparison

### Hubris Model

```
┌─────────────────┐     IPC (sys_send)      ┌───────────────────┐
│  Client Task    │───────────────────────▶||  I2C Driver      ||
│  (Rust binary)  │                         │  Task (Server)    │
└─────────────────┘                         └────────┬──────────┘
        ▲                                            │
        └──────── sys_post (notification) ───────────┤
                                            ┌────────▼────────┐
                                            │   Hardware      │
                                            └─────────────────┘
```

**Characteristics:**

- **Isolation:** MPU-enforced task separation
- **Communication:** Synchronous IPC (`sys_send`/`sys_recv_open`/`sys_reply`)
- **Async Events:** `sys_post` notifications with bitmasks
- **Memory Sharing:** Capability-based leases (`sys_borrow_*`)
- **Configuration:** `app.toml` → build.rs code generation

### pw_kernel Model (Rust)

```
┌─────────────────┐   IPC Channel/Syscall  ┌─────────────────┐
│  Userspace App  │ ───────────────────────▶│  Driver Task    │
│  (Rust binary)  │                         │  (Rust binary)  │
└─────────────────┘                         └────────┬────────┘
        ▲                                            │
        └──────── Notification ──────────────────────┤
                                            ┌────────▼────────┐
                                            │   Hardware      │
                                            └─────────────────┘
```

**Characteristics:**

- **Isolation:** Kernelspace/userspace separation (MPU on Cortex-M, PMP on RISC-V)
- **Communication:** IPC channels (recently added via `pw_kernel: Add IPC channels`)
- **Async Events:** Syscall-based notification mechanism
- **Memory Sharing:** Shared memory regions in system manifest
- **Configuration:** `system.json5` → `system_codegen` build tool

### Structural Similarity Score

| Feature | Hubris | pw_kernel | Compatibility |
|---------|--------|-----------|---------------|
| Language | Rust | Rust | ✅ Direct |
| Task Model | Static tasks | Static apps | ✅ Direct |
| IPC | Syscall-based | Syscall-based | ✅ Direct |
| Memory Isolation | MPU regions | MPU/PMP regions | ✅ Direct |
| Config Format | TOML | JSON5 | 🔄 Trivial conversion |
| Build Integration | build.rs | system_codegen | 🔄 Adapt macros |
| Interrupt Handling | Kernel-mediated | Kernel-mediated | ✅ Direct |

## OS Abstraction Mapping

### Key Differences from Hubris

Before diving into the syscall mapping, note these critical differences:

1. **No IRQ enable/disable syscall** - pw_kernel automatically masks IRQs when they fire and unmasks on `interrupt_ack()`. There is no userspace control over IRQ masking like Hubris's `sys_irq_control()`.

2. **Asymmetric channels** - Initiator and handler have distinct APIs (`channel_transact()` vs. `channel_read()`/`channel_respond()`).

3. **Per-object USER signals** - `object_raise_peer_user_signal()` raises a signal on a specific channel object, not a task-wide bitmask like Hubris `sys_post()`.

4. **Interrupt objects handle multiple IRQs** - A single interrupt object can handle up to 16 IRQs, mapped to `INTERRUPT_A` through `INTERRUPT_P` signals.

### Syscall Mapping Table

Based on the OS Dependencies section from the Hubris I2C guide:

| Hubris Syscall | Purpose | pw_kernel Equivalent | Status |
|----------------|---------|----------------------|--------|
| sys_send | Synchronous IPC request | channel_transact() | ✅ Available |
| sys_recv_open | Block on IPC + notifications | object_wait() + channel_read() | ✅ Available |
| sys_reply | IPC response | channel_respond() | ✅ Available |
| sys_post | Async notification to task | object_raise_peer_user_signal() | ✅ Available (USER signal) |
| sys_irq_control | Enable/disable IRQ | **No equivalent** | ❌ Kernel manages IRQ masking |
| sys_borrow_info | Get lease metadata | N/A (kernel-mediated copy) | ❌ Not available |
| sys_borrow_read | Read from lease | channel_read() with offset | ✅ Available |
| sys_borrow_write | Write to lease | channel_respond() data | ✅ Available |
| sys_get_timer | Monotonic time | Instant type | ✅ Available |

### IPC Channel Mapping

**Hubris IPC Pattern:**

```rust
// Client side (Hubris)
let (code, _) = sys_send(
    self.task,           // Target task ID
    Op::WriteRead as u16, // Operation code
    &request_bytes,      // Request payload
    &mut response,       // Response buffer
    &[Lease::from(wbuf), Lease::from(rbuf)],
);
```

**pw_kernel IPC Pattern (Actual API):**

```rust
// Client side (pw_kernel)
// Based on pw_kernel/userspace/syscall.rs
use pw_kernel_userspace::syscall;
use pw_kernel_userspace::time::Instant;

// Object handle comes from generated code (system.json5 → build-time codegen)
// e.g., `handle::I2C_DRIVER` generated from system manifest
let mut request = [0u8; 64];
let mut response = [0u8; 64];

// Marshal request into buffer
request[0..4].copy_from_slice(&device.marshal());
request[4..6].copy_from_slice(&(wbuf.len() as u16).to_le_bytes());
request[6..8].copy_from_slice(&(rbuf.len() as u16).to_le_bytes());

// Synchronous IPC transaction with deadline
let bytes_received = syscall::channel_transact(
    handle::I2C_DRIVER,  // u32 handle from generated code
    &request,
    &mut response,
    Instant::infinite_future(),  // or specific deadline
)?;

// Response contains read data
rbuf.copy_from_slice(&response[..rbuf.len()]);
```

### Notification Mapping

**Hubris Pattern:**

```rust
// Server: notify client of target message arrival
if let Some((client_task, mask)) = notification_client {
    sys_post(client_task, mask);
}

// Client: wait for notification
let msg = sys_recv_open(&mut buffer, I2C_RX_MASK);
if msg.sender == TaskId::KERNEL && (msg.operation & I2C_RX_MASK) != 0 {
    // Handle target message ready
}
```

**pw_kernel Pattern (Actual API):**

```rust
// Based on pw_kernel/userspace/syscall.rs
// pw_kernel uses signal-based waiting on objects, not separate notify/wait calls

use pw_kernel_userspace::syscall;
use syscall_defs::Signals;

// Server: signals are raised automatically when channel state changes
// - READABLE is set when transaction pending (handler side)
// - WRITEABLE is set when ready for new transaction (initiator side)
// - USER signal (bit 15) can be used for custom notifications via channel

// Client: wait for signals on channel object
let signals = syscall::object_wait(
    handle::I2C_CHANNEL,
    Signals::READABLE | Signals::USER,  // Wait for data or user signal
    Instant::infinite_future(),
)?;

if signals.contains(Signals::READABLE) {
    // Data available - read from channel
    let mut buffer = [0u8; 256];
    let len = syscall::channel_read(handle::I2C_CHANNEL, 0, &mut buffer)?;
}
```

**Note:** pw_kernel provides `object_raise_peer_user_signal()` to raise the USER signal on a peer's channel object, similar to Hubris `sys_post()`. Additionally, notifications happen automatically through:

1. Channel state changes (READABLE/WRITEABLE signals)
2. Interrupt objects (INTERRUPT_A through INTERRUPT_P signals - one object can handle up to 16 IRQs)
3. USER signal for application-defined notifications

**Complete Signal Set:**

```rust
use syscall_defs::Signals;

// Channel/general signals (bits 0-15)
Signals::READABLE    // 1 << 0  - Data available to read
Signals::WRITEABLE   // 1 << 1  - Object ready for write
Signals::ERROR       // 1 << 2  - Error condition
Signals::USER        // 1 << 15 - User-defined signal

// Interrupt signals (bits 16-31)
Signals::INTERRUPT_A // 1 << 16
Signals::INTERRUPT_B // 1 << 17
// ... through ...
Signals::INTERRUPT_P // 1 << 31
```

### Memory Sharing (Leases → IPC Buffers)

Hubris leases provide capability-based memory sharing. In pw_kernel, the current approach uses kernel-mediated buffer copy during IPC transactions rather than shared memory regions.

**Hubris Lease Pattern:**

```rust
// Client creates lease
let write_lease = Lease::from(&write_buffer);
let read_lease = Lease::from(&mut read_buffer);

// Server accesses lease
let len = sys_borrow_info(msg.sender, lease_index).len;
for i in 0..len {
    let byte = sys_borrow_read(msg.sender, lease_index, i);
    process_byte(byte);
}
```

**pw_kernel Current Pattern (Kernel-Mediated Copy):**

```rust
// pw_kernel IPC uses direct buffer copy via syscalls - no shared memory regions yet
// Data is copied through kernel during channel_transact/channel_read/channel_respond

// Client side: data is copied INTO kernel during transact
let bytes = syscall::channel_transact(handle, &send_buf, &mut recv_buf, deadline)?;

// Handler side: data is copied FROM kernel during read
let len = syscall::channel_read(handle, offset, &mut buffer)?;
// Process buffer...
// Response is copied INTO kernel during respond
syscall::channel_respond(handle, &response_buf)?;
```

**Future Design Goal (NOT YET IMPLEMENTED):**

```rust
// Shared memory regions are described in pw_kernel design docs as "initial support"
// but are not yet exposed in the userspace API or system.json5 schema.
//
// When implemented, it may look like:
// {
//     "shared_memory": {
//         "i2c_buffers": { "size": 4096, "apps": ["mctp_server", "i2c_driver"] }
//     }
// }
//
// For now, all data passes through kernel-mediated copy which is safe but
// has higher overhead for large transfers.
```

**Porting Strategy:** For the initial port, use the kernel-mediated IPC buffer copy pattern. This matches pw_kernel's zero-allocation design. Shared memory optimization can be added later when pw_kernel exposes the API.

## Module-by-Module Porting Plan

### Phase 1: Userspace I2C API Crate

**Source:** `drv/i2c-api/src/lib.rs`  
**Target:** `pw_kernel/userspace/i2c/src/lib.rs`

```rust
// pw_kernel/userspace/i2c/src/lib.rs

//! I2C userspace API for pw_kernel
//!
//! Provides client-side access to I2C driver tasks.
//! Uses the actual pw_kernel syscall API.

#![no_std]

use pw_kernel_userspace::syscall;
use pw_kernel_userspace::time::Instant;
use pw_status::Result;

/// I2C device address (7-bit or 10-bit)
#[derive(Copy, Clone, Debug)]
pub struct Address(u16);

impl Address {
    pub const fn seven_bit(addr: u8) -> Self {
        assert!(addr <= 0x7F);
        Self(addr as u16)
    }

    pub const fn ten_bit(addr: u16) -> Self {
        assert!(addr <= 0x3FF);
        Self(addr | 0x8000) // Flag for 10-bit
    }
}

/// I2C controller identifier
#[derive(Copy, Clone, Debug)]
pub struct Controller(pub u8);

/// I2C port on a controller
#[derive(Copy, Clone, Debug)]
pub struct Port(pub u8);

/// Mux segment for I2C mux traversal
#[derive(Copy, Clone, Debug)]
pub struct Segment {
    pub mux: u8,
    pub segment: u8,
}

/// Complete I2C device specification
#[derive(Copy, Clone, Debug)]
pub struct I2cDevice {
    pub address: Address,
    pub controller: Controller,
    pub port: Port,
    pub segment: Option<Segment>,
}

/// I2C client handle - wraps an IPC channel handle
pub struct I2c {
    /// Channel initiator handle (from generated code)
    handle: u32,
}

/// I2C operation codes
const OP_WRITE_READ: u8 = 1;
const OP_GET_SLAVE_MESSAGE: u8 = 2;

impl I2c {
    /// Create I2C client from a channel initiator handle
    ///
    /// The handle comes from generated code based on system.json5 configuration.
    /// Example: `I2c::new(handle::I2C_DRIVER)`
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }

    /// Perform write-then-read transaction
    pub fn write_read(
        &self,
        device: I2cDevice,
        write_buf: &[u8],
        read_buf: &mut [u8],
    ) -> Result<()> {
        // Build request: [op, device(4), write_len(2), read_len(2), write_data...]
        let mut request = [0u8; 256];
        request[0] = OP_WRITE_READ;
        request[1..5].copy_from_slice(&device.marshal());
        request[5..7].copy_from_slice(&(write_buf.len() as u16).to_le_bytes());
        request[7..9].copy_from_slice(&(read_buf.len() as u16).to_le_bytes());
        request[9..9 + write_buf.len()].copy_from_slice(write_buf);

        let request_len = 9 + write_buf.len();
        let mut response = [0u8; 256];

        // Perform IPC transaction via syscall
        let resp_len = syscall::channel_transact(
            self.handle,
            &request[..request_len],
            &mut response,
            Instant::infinite_future(),
        )?;

        // Check response status (first byte)
        if resp_len > 0 && response[0] != 0 {
            return Err(pw_status::Status::from_code(response[0] as u32));
        }

        // Copy read data from response (after status byte)
        if read_buf.len() > 0 && resp_len > 1 {
            let data_len = (resp_len - 1).min(read_buf.len());
            read_buf[..data_len].copy_from_slice(&response[1..1 + data_len]);
        }

        Ok(())
    }

    /// Write only (no read phase)
    pub fn write(&self, device: I2cDevice, buf: &[u8]) -> Result<()> {
        self.write_read(device, buf, &mut [])
    }

    /// Read only (no write phase)
    pub fn read(&self, device: I2cDevice, buf: &mut [u8]) -> Result<()> {
        self.write_read(device, &[], buf)
    }
}

/// Marshaled device info for IPC (matches Hubris format)
impl I2cDevice {
    fn marshal(&self) -> [u8; 4] {
        [
            self.address.0 as u8,
            self.controller.0,
            self.port.0,
            match self.segment {
                None => 0,
                Some(seg) => 0x80 | (seg.mux << 4) | seg.segment,
            },
        ]
    }
}
```

**BUILD.bazel:**

```python
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "i2c",
    srcs = ["src/lib.rs"],
    deps = [
        "//pw_kernel/userspace:core",
    ],
    visibility = ["//visibility:public"],
)
```

### Phase 2: I2C Driver Task

**Source:** `drv/openprot-i2c-server/src/main.rs`  
**Target:** `pw_kernel/apps/i2c_driver/main.rs`

```rust
// pw_kernel/apps/i2c_driver/main.rs

//! I2C Driver Task for pw_kernel
//!
//! Handles both controller and target mode I2C operations.
//! This is a handler-side app that receives IPC requests from initiators.

#![no_std]
#![no_main]

use pw_kernel_userspace::syscall;
use pw_kernel_userspace::time::Instant;
use syscall_defs::Signals;
use pw_log::info;

mod hardware;
use hardware::I2cHardware;

// Generated from system.json5 via system_codegen
// Provides: handle::IPC (channel handler), handle::I2C_IRQ (interrupt object)
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

/// I2C operation codes (matching Hubris API)
#[repr(u8)]
enum Op {
    WriteRead = 1,
    GetSlaveMessage = 2,
}

/// Driver state
struct I2cDriver<H: I2cHardware> {
    hw: H,
    slave_buffer: [u8; 256],
    slave_msg_len: usize,
}

impl<H: I2cHardware> I2cDriver<H> {
    fn new(hw: H) -> Self {
        Self {
            hw,
            slave_buffer: [0u8; 256],
            slave_msg_len: 0,
        }
    }

    fn handle_request(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, u8> {
        if request.is_empty() {
            return Err(1); // BadOp
        }

        match request[0] {
            1 => self.handle_write_read(&request[1..], response),
            2 => self.handle_get_slave_message(response),
            _ => Err(1), // BadOp
        }
    }

    fn handle_write_read(&mut self, payload: &[u8], response: &mut [u8]) -> Result<usize, u8> {
        if payload.len() < 8 {
            return Err(2); // BadArg
        }

        let device = I2cDevice::unmarshal(&payload[0..4]);
        let write_len = u16::from_le_bytes([payload[4], payload[5]]) as usize;
        let read_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;

        // Write data follows the header in the request
        let write_buf = &payload[8..8 + write_len];
        let read_buf = &mut response[..read_len];

        match self.hw.write_read(&device, write_buf, read_buf) {
            Ok(()) => Ok(read_len),
            Err(e) => Err(e.into()),
        }
    }

    fn handle_interrupt(&mut self) {
        if self.hw.has_slave_data() {
            self.slave_msg_len = self.hw.read_slave_data(&mut self.slave_buffer);
        }

        // CRITICAL: pw_kernel requires interrupt_ack() to re-enable the IRQ
        // Unlike Hubris sys_irq_control(), there is no way to manually enable/disable IRQs
        // The kernel automatically masks IRQs when they fire, and unmasks on ack
        let _ = syscall::interrupt_ack(handle::I2C_IRQ, Signals::INTERRUPT_A);
    }

    fn handle_get_slave_message(&mut self, response: &mut [u8]) -> Result<usize, u8> {
        if self.slave_msg_len == 0 {
            return Err(3); // NoMessage
        }

        let len = self.slave_msg_len.min(response.len());
        response[..len].copy_from_slice(&self.slave_buffer[..len]);
        self.slave_msg_len = 0;

        Ok(len)
    }
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    info!("I2C driver starting");

    // Initialize hardware (platform-specific)
    let hw = hardware::init();
    let mut driver = I2cDriver::new(hw);

    // Main event loop - wait for IPC requests or interrupts
    let mut request_buf = [0u8; 256];
    let mut response_buf = [0u8; 256];

    loop {
        // Wait for either IPC request (READABLE on channel) or interrupt
        let signals = syscall::object_wait(
            handle::IPC,
            Signals::READABLE | Signals::INTERRUPT_A,
            Instant::infinite_future(),
        ).unwrap_or(Signals::empty());

        // Handle interrupt if signaled
        if signals.contains(Signals::INTERRUPT_A) {
            driver.handle_interrupt();
        }

        // Handle IPC request if pending
        if signals.contains(Signals::READABLE) {
            // Read the request from the channel
            let req_len = match syscall::channel_read(handle::IPC, 0, &mut request_buf) {
                Ok(len) => len,
                Err(_) => continue,
            };

            // Process request and build response
            let resp_len = match driver.handle_request(&request_buf[..req_len], &mut response_buf) {
                Ok(len) => {
                    response_buf[0] = 0; // Success status
                    len + 1
                }
                Err(code) => {
                    response_buf[0] = code; // Error status
                    1
                }
            };

            // Send response back
            let _ = syscall::channel_respond(handle::IPC, &response_buf[..resp_len]);
        }
    }
}
```

### Phase 3: Hardware Abstraction Trait

**Source:** `drv/lib/i2c-driver/src/lib.rs` (`I2cHardware` trait)  
**Target:** `pw_kernel/drivers/i2c/src/lib.rs`

```rust
// pw_kernel/drivers/i2c/src/lib.rs

//! I2C Hardware Abstraction for pw_kernel
//!
//! Platform-specific drivers implement this trait.

#![no_std]

use core::time::Duration;

/// I2C transaction error types
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum I2cError {
    NoDevice,
    Nack,
    BusError,
    ArbitrationLost,
    Timeout,
    InvalidAddress,
}

/// Result type for I2C operations
pub type Result<T> = core::result::Result<T, I2cError>;

/// Hardware abstraction trait for I2C controllers
pub trait I2cHardware {
    /// Perform a write-then-read transaction (controller mode)
    fn write_read(
        &mut self,
        device: &I2cDevice,
        write_buf: &[u8],
        read_buf: &mut [u8],
    ) -> Result<()>;
    
    /// Configure target mode address
    fn configure_slave_address(&mut self, address: u8) -> Result<()>;
    
    /// Enable target receive mode
    fn enable_slave_receive(&mut self) -> Result<()>;
    
    /// Disable target receive mode
    fn disable_slave_receive(&mut self) -> Result<()>;
    
    /// Check if target data is available
    fn has_slave_data(&self) -> bool;
    
    /// Read received target data into buffer
    /// Returns number of bytes read
    fn read_slave_data(&mut self, buf: &mut [u8]) -> usize;
    
    /// Reset the I2C bus (clock stretching recovery)
    fn reset_bus(&mut self) -> Result<()>;
    
    /// Get controller status for diagnostics
    fn status(&self) -> ControllerStatus;
}

/// Controller diagnostic status
#[derive(Clone, Debug, Default)]
pub struct ControllerStatus {
    pub bus_busy: bool,
    pub arbitration_lost: bool,
    pub slave_addressed: bool,
    pub data_ready: bool,
}

/// I2C device specification
#[derive(Copy, Clone, Debug)]
pub struct I2cDevice {
    pub address: u8,
    pub controller: u8,
    pub port: u8,
    pub segment: Option<(u8, u8)>, // (mux, segment)
}

impl I2cDevice {
    /// Unmarshal from 4-byte wire format
    pub fn unmarshal(bytes: &[u8]) -> Self {
        Self {
            address: bytes[0],
            controller: bytes[1],
            port: bytes[2],
            segment: if bytes[3] & 0x80 != 0 {
                Some(((bytes[3] >> 4) & 0x07, bytes[3] & 0x0F))
            } else {
                None
            },
        }
    }
}
```

### Phase 4: AST1060 Hardware Driver

**Source:** `drv/i2c-devices/ast1060-i2c/src/lib.rs`  
**Target:** `pw_kernel/drivers/i2c_ast1060/src/lib.rs`

```rust
// pw_kernel/drivers/i2c_ast1060/src/lib.rs

//! AST1060 I2C Controller Driver for pw_kernel
//!
//! Supports both controller and target mode operations.

#![no_std]

use pw_kernel_drivers_i2c::{I2cHardware, I2cDevice, I2cError, Result, ControllerStatus};
use core::ptr::{read_volatile, write_volatile};

/// AST1060 I2C controller registers
#[repr(C)]
struct Registers {
    fun_ctrl: u32,         // 0x00: Function control
    ac_timing: u32,        // 0x04: AC timing
    intr_ctrl: u32,        // 0x08: Interrupt control
    intr_sts: u32,         // 0x0C: Interrupt status
    cmd_sts: u32,          // 0x10: Command/status
    dev_addr: u32,         // 0x14: Device address
    buf_ctrl: u32,         // 0x18: Buffer control
    byte_buf: u32,         // 0x1C: Byte buffer
    dma_base: u32,         // 0x20: DMA base address
    dma_len: u32,          // 0x24: DMA length
}

/// AST1060 I2C controller driver
pub struct Ast1060I2c {
    regs: *mut Registers,
    controller_id: u8,
    slave_buffer: [u8; 256],
    slave_data_len: usize,
}

impl Ast1060I2c {
    /// Create new driver instance
    /// 
    /// # Safety
    /// Caller must ensure base_addr points to valid I2C controller registers
    pub unsafe fn new(base_addr: usize, controller_id: u8) -> Self {
        Self {
            regs: base_addr as *mut Registers,
            controller_id,
            slave_buffer: [0u8; 256],
            slave_data_len: 0,
        }
    }
    
    /// Initialize controller
    pub fn init(&mut self) {
        unsafe {
            // Reset controller
            write_volatile(&mut (*self.regs).fun_ctrl, 0);
            
            // Configure timing for 400kHz
            write_volatile(&mut (*self.regs).ac_timing, 0x77743335);
            
            // Enable controller
            write_volatile(&mut (*self.regs).fun_ctrl, 1);
        }
    }
    
    fn wait_complete(&self) -> Result<()> {
        let mut timeout = 100_000;
        loop {
            let status = unsafe { read_volatile(&(*self.regs).intr_sts) };
            
            if status & (1 << 0) != 0 {  // TX done
                return Ok(());
            }
            if status & (1 << 1) != 0 {  // RX done
                return Ok(());
            }
            if status & (1 << 2) != 0 {  // NACK
                return Err(I2cError::Nack);
            }
            if status & (1 << 3) != 0 {  // Arbitration lost
                return Err(I2cError::ArbitrationLost);
            }
            
            timeout -= 1;
            if timeout == 0 {
                return Err(I2cError::Timeout);
            }
        }
    }
}

impl I2cHardware for Ast1060I2c {
    fn write_read(
        &mut self,
        device: &I2cDevice,
        write_buf: &[u8],
        read_buf: &mut [u8],
    ) -> Result<()> {
        // Select port/mux if needed
        if let Some((mux, seg)) = device.segment {
            self.select_mux_segment(mux, seg)?;
        }
        
        unsafe {
            // Set device address
            write_volatile(&mut (*self.regs).dev_addr, device.address as u32);
            
            // Write phase
            if !write_buf.is_empty() {
                for (i, byte) in write_buf.iter().enumerate() {
                    write_volatile(&mut (*self.regs).byte_buf, *byte as u32);
                }
                write_volatile(&mut (*self.regs).cmd_sts, 
                    (write_buf.len() as u32) | (1 << 8)); // TX command
                self.wait_complete()?;
            }
            
            // Read phase
            if !read_buf.is_empty() {
                write_volatile(&mut (*self.regs).cmd_sts,
                    (read_buf.len() as u32) | (1 << 9)); // RX command
                self.wait_complete()?;
                
                for i in 0..read_buf.len() {
                    read_buf[i] = read_volatile(&(*self.regs).byte_buf) as u8;
                }
            }
        }
        
        Ok(())
    }
    
    fn configure_slave_address(&mut self, address: u8) -> Result<()> {
        unsafe {
            let ctrl = read_volatile(&(*self.regs).fun_ctrl);
            write_volatile(&mut (*self.regs).fun_ctrl, ctrl | (1 << 16)); // Enable target
            write_volatile(&mut (*self.regs).dev_addr, 
                (address as u32) << 8 | 0x1); // Target address + enable
        }
        Ok(())
    }
    
    fn enable_slave_receive(&mut self) -> Result<()> {
        unsafe {
            let ctrl = read_volatile(&(*self.regs).intr_ctrl);
            write_volatile(&mut (*self.regs).intr_ctrl, ctrl | (1 << 8)); // Target RX IRQ
        }
        Ok(())
    }
    
    fn disable_slave_receive(&mut self) -> Result<()> {
        unsafe {
            let ctrl = read_volatile(&(*self.regs).intr_ctrl);
            write_volatile(&mut (*self.regs).intr_ctrl, ctrl & !(1 << 8));
        }
        Ok(())
    }
    
    fn has_slave_data(&self) -> bool {
        unsafe {
            read_volatile(&(*self.regs).intr_sts) & (1 << 8) != 0
        }
    }
    
    fn read_slave_data(&mut self, buf: &mut [u8]) -> usize {
        unsafe {
            let len = (read_volatile(&(*self.regs).buf_ctrl) & 0xFF) as usize;
            let len = len.min(buf.len());
            
            for i in 0..len {
                buf[i] = read_volatile(&(*self.regs).byte_buf) as u8;
            }
            
            // Clear interrupt
            write_volatile(&mut (*self.regs).intr_sts, 1 << 8);
            
            len
        }
    }
    
    fn reset_bus(&mut self) -> Result<()> {
        unsafe {
            // Toggle SCL to release stuck devices
            let ctrl = read_volatile(&(*self.regs).fun_ctrl);
            write_volatile(&mut (*self.regs).fun_ctrl, ctrl | (1 << 24)); // Bus recovery
            
            for _ in 0..9 {
                // Generate clock pulses
                core::hint::spin_loop();
            }
            
            write_volatile(&mut (*self.regs).fun_ctrl, ctrl);
        }
        Ok(())
    }
    
    fn status(&self) -> ControllerStatus {
        let sts = unsafe { read_volatile(&(*self.regs).intr_sts) };
        ControllerStatus {
            bus_busy: sts & (1 << 4) != 0,
            arbitration_lost: sts & (1 << 3) != 0,
            slave_addressed: sts & (1 << 7) != 0,
            data_ready: sts & (1 << 8) != 0,
        }
    }
}

impl Ast1060I2c {
    fn select_mux_segment(&mut self, mux: u8, segment: u8) -> Result<()> {
        // Write to I2C mux to select segment
        let mux_addr = 0x70 + mux;
        let segment_mask = 1u8 << segment;
        
        unsafe {
            write_volatile(&mut (*self.regs).dev_addr, mux_addr as u32);
            write_volatile(&mut (*self.regs).byte_buf, segment_mask as u32);
            write_volatile(&mut (*self.regs).cmd_sts, 1 | (1 << 8));
        }
        
        self.wait_complete()
    }
}
```

## System Configuration Mapping

### Hubris app.toml → pw_kernel system.json5

**Hubris Configuration:**

```toml
# app.toml
[tasks.i2c_driver]
name = "drv-openprot-i2c-server"
priority = 4
max-sizes = {flash = 16384, ram = 4096}
notifications = ["i2c-irq"]
interrupts = {"i2c.irq" = "i2c-irq"}
uses = ["i2c"]

[tasks.mctp_server]
name = "task-mctp-server"
priority = 5
task-slots = ["i2c_driver"]
notifications = ["i2c-rx", "timer"]
```

**pw_kernel Configuration (Actual Schema):**

```json5
// system.json5
// Based on actual pw_kernel/target/mps2_an505/ipc/user/system.json5
{
  arch: {
    type: "armv8m",
    vector_table_start_address: 0x10000000,
    vector_table_size_bytes: 2048,
  },
  kernel: {
    flash_start_address: 0x10000800,
    flash_size_bytes: 261120,
    ram_start_address: 0x38000000,
    ram_size_bytes: 65536,
    // interrupt_table: { ... } // Optional, for interrupt objects
  },
  apps: [
    {
      name: "mctp_server",
      flash_size_bytes: 16384,
      ram_size_bytes: 4096,
      process: {
        name: "mctp_server process",
        objects: [
          {
            // This creates a channel_initiator that connects to i2c_driver
            name: "I2C",
            type: "channel_initiator",
            handler_app: "i2c_driver",
            handler_object_name: "IPC",
          },
        ],
        threads: [
          {
            name: "main thread",
            stack_size_bytes: 1024,
          },
        ],
      },
    },
    {
      name: "i2c_driver",
      flash_size_bytes: 16384,
      ram_size_bytes: 4096,
      process: {
        name: "i2c_driver process",
        objects: [
          {
            // This is the handler side of the IPC channel
            name: "IPC",
            type: "channel_handler",
          },
          // Note: interrupt objects would be defined here when supported
          // { name: "I2C_IRQ", type: "interrupt", irq: 76 }
        ],
        threads: [
          {
            name: "main thread",
            stack_size_bytes: 1024,
          },
        ],
      },
    },
  ],
}
```

**Key Differences from Hubris:**

- Uses array syntax `apps: [...]` not object syntax `"apps": {...}`
- No `priority`, `binary`, `notifications`, `endpoints`, `peripherals` fields yet
- Channel connections defined via `handler_app` + `handler_object_name` references
- Memory sizes at app level, not nested under `memory` object
- No `shared_memory` or `platform` top-level keys (not yet implemented)
- Interrupt objects exist (`50616a91c`) but peripheral mapping is not in manifest yet

### Code Generation Mapping

**Hubris (build.rs):**

```rust
// Generates notifications.rs with:
// pub const I2C_IRQ_MASK: u32 = 1 << 0;
include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
```

**pw_kernel (system_codegen):**

```rust
// Generated code provides object handles, not string endpoints or notification masks
// The exact format depends on system_generator tooling

// Example generated.rs (conceptual - actual format may vary):
pub mod handle {
    /// Channel initiator handle for I2C communication
    pub const I2C: u32 = 0;
    /// Channel handler handle for receiving IPC requests
    pub const IPC: u32 = 0;
    /// Interrupt object handle (when interrupt objects are configured)
    pub const I2C_IRQ: u32 = 1;
}

// Handles are u32 values used directly in syscalls:
// syscall::channel_transact(handle::I2C, &send, &mut recv, deadline)
// syscall::object_wait(handle::IPC, Signals::READABLE, deadline)
```

## References

- **Hubris I2C Guide:** See other sections in this documentation
- **pw_kernel documentation:** <https://pigweed.dev/pw_kernel/>
- **pw_kernel source:** <https://pigweed.googlesource.com/pigweed/pigweed/+/main/pw_kernel/>
- **Key commits:**
  - `7c6aa2cb8` - pw_kernel: Add IPC channels (Sep 11, 2025)
  - `7fe7fde95` - pw_kernel: Add syscall wrappers and concrete time types to userspace (Sep 15, 2025)
  - `e7ccdcf30` - pw_i2c: Add responder APIs (Sep 16, 2025)
  - `50616a91c` - pw_kernel: Add Interrupt Object (Nov 19, 2025)
- **Key source files:**
  - `pw_kernel/userspace/syscall.rs` - Userspace syscall wrappers
  - `pw_kernel/kernel/object/channel.rs` - IPC channel implementation
  - `pw_kernel/syscall/syscall_defs.rs` - Syscall IDs and signal definitions
