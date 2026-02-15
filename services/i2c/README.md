# I2C Service

IPC-based I2C service for the Pigweed kernel, modeled after the crypto service.

## Architecture

```
┌────────────────────┐
│   I2C Client       │  (userspace task)
│   (tests, apps)    │
└────────┬───────────┘
         │ IPC (channel)
         ▼
┌────────────────────┐
│   I2C Server       │  (userspace task)
│   services/i2c/    │
│   server/          │
└────────┬───────────┘
         │ calls
         ▼
┌────────────────────┐
│   I2C Backend      │  (library)
│   backend-aspeed/  │
└────────┬───────────┘
         │ uses
         ▼
┌────────────────────┐
│ aspeed-ddk i2c_core│  (portable driver)
│   @oot_crates_no_  │
│   std//aspeed-ddk  │
└────────────────────┘
```

## Components

- **api/** - Shared protocol definitions (op codes, error codes, wire format)
- **backend-aspeed/** - AST1060-specific backend using `aspeed-ddk::i2c_core`
- **client/** - IPC client library for userspace tasks
- **server/** - I2C server process handling IPC requests
- **tests/** - Integration tests

## Build

```bash
# Build the i2c_core dependency check
bazel build --config=k_qemu_ast1060 //services/i2c/backend-aspeed:i2c_backend_aspeed

# Build and run tests
bazel test --config=k_qemu_ast1060 //target/ast1060/i2c:i2c_test --test_output=all
```

## Operations

| Op Code | Operation | Description |
|---------|-----------|-------------|
| 0x01 | Write | Master write to device |
| 0x02 | Read | Master read from device |
| 0x03 | WriteRead | Combined write-then-read |
| 0x10 | Configure | Set I2C speed, mode |

## Status

- [ ] API protocol definitions
- [ ] Backend wrapper for aspeed-ddk::i2c_core
- [ ] Server implementation
- [ ] Client library
- [ ] QEMU tests
