# Testing

## Hardware Validation with Aardvark

I2C target mode requires an external controller device for testing. The codebase includes support for testing with an Aardvark I2C adapter:

| File | Purpose |
|------|---------|
| `drv/ast1060-i2c/aardvark_test.py` | Python test automation |
| `drv/ast1060-i2c/TEST-PLAN.md` | Hardware validation test cases |
| `drv/ast1060-i2c/AARDVARK-BRINGUP.md` | Hardware setup guide |

```bash
# Run target receive test with Aardvark
python3 aardvark_test.py --test target_receive --address 0x1D
```

## Example Test Application

```rust
// In app/ast1060-i2c-test/src/main.rs

#[export_name = "main"]
fn main() -> ! {
    let device = I2cDevice::new(
        i2c_task,
        Controller::I2C0,
        PortIndex(0),
        None,
        0x50,
    );

    // Test controller mode
    let mut buffer = [0u8; 16];
    device.read_reg(0x00u8, &mut buffer).unwrap_lite();

    // Test target mode
    device.configure_slave_address(0x1D).unwrap_lite();
    device.enable_slave_receive().unwrap_lite();
    device.enable_slave_notification(0x0001).unwrap_lite();

    loop {
        let msg = sys_recv_open(&mut buffer, 0x0001);
        if msg.sender == TaskId::KERNEL {
            if let Ok(slave_msg) = device.get_slave_message() {
                // Process message
            }
        }
    }
}
```

## Test Checklist

- [ ] Controller mode read from known device
- [ ] Controller mode write to known device
- [ ] Controller mode write-then-read sequence
- [ ] Target address configuration
- [ ] Target mode receive single message
- [ ] Target mode receive multiple messages
- [ ] Notification delivery and timing
- [ ] Error handling for reserved addresses
- [ ] Bus recovery after lockup
