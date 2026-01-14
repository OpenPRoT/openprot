# Overview

The OpenPRoT I2C subsystem provides a vendor-agnostic I2C driver framework supporting both controller and target modes. It is designed around a microkernel-based architecture, using IPC to communicate with separate I2C driver tasks that control the actual hardware.

## Audience

This guide serves three primary audiences:

| Audience | Goal | Start Here |
|----------|------|------------|
| **Application Developer** | Use I2C to communicate with devices | [Quick Start](#quick-start), then [I2C API](#i2c-api) |
| **Driver Implementer** | Add support for new I2C hardware | [Architecture](#architecture), then [Hardware Implementations](#hardware-implementations) |
| **System Integrator** | Configure I2C in a system deployment | [System Integration](#system-integration), then [MCTP Integration](#mctp-integration) |

## Quick Start

**Reading from an I2C device (controller mode):**

```rust
use drv_i2c_api::*;

// Connect to driver task, controller 1, port 0, device at address 0x50
let device = I2cDevice::new(
    I2C.get_task_id(),
    Controller::I2C1,
    PortIndex(0),
    None,
    0x50,
);

// Read 2 bytes from register 0x00
let value: u16 = device.read_reg(0x00)?;

// Write-then-read: send [0x10], read 8 bytes back
let mut buffer = [0u8; 8];
device.write_read(&[0x10], &mut buffer)?;
```

**Receiving messages as an I2C target:**

```rust
use drv_i2c_api::*;

const NOTIF_MASK: u32 = 0x0001;

let device = I2cDevice::new(I2C.get_task_id(), Controller::I2C1, PortIndex(0), None, 0x1D);

// Configure as target at address 0x1D
device.configure_slave_address(0x1D)?;
device.enable_slave_receive()?;
device.enable_slave_notification(NOTIF_MASK)?;

// Wait for incoming message
loop {
    let msg = sys_recv_open(&mut buf, NOTIF_MASK);
    if msg.sender == TaskId::KERNEL {
        let slave_msg = device.get_slave_message()?;
        // Process slave_msg.data()
    }
}
```

## Key Features

| Feature | Description |
|---------|-------------|
| **Hardware Abstraction** | Single API works across AST1060, STM32, LPC55, and other I2C controllers |
| **Controller + Target Modes** | Initiate transactions or respond to external controllers |
| **Interrupt-Driven** | Target mode uses async notifications—no polling required |
| **Memory Efficient** | ~270 bytes driver state; zero-copy where possible |
| **Type Safe** | Rust's type system prevents common I2C programming errors |
| **Task Isolation** | MPU-enforced separation between driver and application tasks |

## Use Cases

| Use Case | Mode | Typical Protocol |
|----------|------|------------------|
| BMC/host management communication | Target | MCTP-over-I2C |
| Sensor reading | Controller | Raw I2C / SMBus |
| EEPROM access | Controller | Raw I2C |
| Power management | Controller | PMBus |
| Device authentication | Target | SPDM over MCTP |
| Multi-board communication | Both | Custom / MCTP |
