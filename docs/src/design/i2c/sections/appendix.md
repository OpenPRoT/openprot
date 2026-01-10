# Appendix

## Implementation Files

| File | Description |
|------|-------------|
| `drv/i2c-api/src/lib.rs` | Client API with `I2cDevice` methods |
| `drv/i2c-types/src/lib.rs` | Type definitions (`SlaveMessage`, `SlaveConfig`, `ResponseCode`) |
| `drv/i2c-types/src/traits.rs` | `I2cHardware` trait for hardware abstraction |
| `drv/openprot-i2c-server/src/main.rs` | IPC server handling slave operations |
| `drv/ast1060-i2c/src/slave.rs` | AST1060-specific slave mode implementation |
| `drv/ast1060-i2c/src/server_driver.rs` | AST1060 I2cHardware trait implementation |
| `task/mctp-server/src/main.rs` | MCTP server with I2C transport |

## Build System Files

| File | Purpose |
|------|---------|
| `build/xtask/src/dist.rs` | Processes `uses` field, creates region configs |
| `sys/kern/build.rs` | Generates region table with MPU encodings |
| `sys/kern/src/descs.rs` | RegionDesc and TaskDesc structures |
| `sys/kern/src/arch/arm_m.rs` | MPU configuration on context switch |

## Current Limitations

| Limitation | Current State | Notes |
|------------|---------------|-------|
| Notification subscribers | 1 per driver instance | Per-controller subscribers may be needed |
| Message buffer | 1 message per driver | Overwrites on overflow |
| Overflow handling | Silent overwrite | No error signaling |
| SlaveMessage controller ID | Not included | Cannot identify source controller |

## Comparisons with Other Systems

- **[Caliptra-MCU (Tock) Comparison](./appendix_caliptra_comparison.md)** - OS-level comparison of subscriber models and notification mechanisms

## Future Enhancements

1. **Message Queuing**: Replace single buffer with `heapless::Deque<SlaveMessage, N>`
2. **Buffer Overflow Signaling**: Add `ResponseCode::SlaveBufferFull`
4. **Multiple Transports**: Concurrent I2C + SPI transports for MCTP
5. **Hardware Flow Control**: Leverage clock stretching where available

