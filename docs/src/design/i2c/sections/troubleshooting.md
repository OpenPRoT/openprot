# Troubleshooting

This section covers common problems and their solutions.

## Bus Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| `BusLocked` error | Device holding SDA low | Power cycle the device, or use bus recovery sequence |
| `NoDevice` on known device | Address mismatch | Verify 7-bit vs 8-bit address format |
| Intermittent `BusError` | Electrical noise | Check pull-up resistor values, reduce bus speed |
| `ControllerBusy` | Incomplete transaction | Reset controller, check for interrupt conflicts |

## Target Mode Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| No notifications received | Notification not enabled | Call `enable_slave_notification()` |
| `NoSlaveMessage` after notification | Message already retrieved | Normal if multiple notifications queued |
| `SlaveNotEnabled` error | Forgot setup step | Call `enable_slave_receive()` first |
| Messages getting dropped | Buffer overflow | Process messages faster, check for blocking calls |

## Configuration Issues

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| `BadController` error | Controller not in `uses` list | Update app.toml `uses` field |
| `MemManageFault` | MPU region violation | Check peripheral assignments in app.toml |
| `SlaveAddressInUse` | Duplicate configuration | Only configure target address once per controller |
| Task crashes on I2C access | Missing peripheral | Verify `uses` includes required peripherals |

## Debugging Tips

1. **Enable logging**: Use `serial_log` feature for debug output
2. **Check hardware**: Use logic analyzer to verify I2C signals
3. **Verify addresses**: I2C uses 7-bit addresses; some tools show 8-bit
4. **Test incrementally**: Verify controller mode works before target mode
5. **Monitor timing**: Use `sys_get_timer()` to measure latencies

## Bus Recovery

If the bus becomes locked (device holding SDA low):

```rust
// Attempt bus recovery
match device.reset_bus() {
    Ok(()) => {
        // Re-initialize
        device.configure_slave_address(addr)?;
        device.enable_slave_receive()?;
    }
    Err(e) => {
        // Hardware intervention may be needed
        log::error!("Bus recovery failed: {:?}", e);
    }
}
```
