# USART Integration Test

## Overview

This directory contains a full system integration test for the USART (Universal Synchronous/Asynchronous Receiver-Transmitter) driver on the AST10x0 platform. The test validates that the USART server and client communicate correctly over IPC channels in a multi-process QEMU-based simulation.

## Test Structure

### Components

- **`target.rs`**: Kernel entry point for the AST10x0 virt target
- **`server_main.rs`**: USART server process - handles requests from clients
- **`client_main.rs`**: USART client process - sends configuration and write requests
- **`system.json5`**: System configuration file defining memory layout and process definitions
- **`BUILD.bazel`**: Bazel build rules and test targets

### Memory Layout

```
ARM Cortex-M4 @ 200 MHz, 768KB SRAM (0x00000000 - 0x000BFFFF)

0x00000000 - 0x00000500: Vector table (1280 bytes)
0x00000500 - 0x00020000: Kernel code (~126KB, ends at 128KB boundary)
0x00020000 - 0x00060000: ipc multi-process app code (256KB, two processes)
0x00060000 - 0x00080000: Kernel RAM (128KB)
0x00080000 - 0x000A0000: App RAM (128KB)
```

### Process Definition

The test instantiates two processes:
1. **USART Server** (`usart_server_bin`):
   - Waits on IPC channel for client requests
   - Services read/write operations via USART backend
   - Manages UART5 IRQ and data flow
   - Depends on `//drivers/usart/server:usart_server`

2. **USART Client** (`usart_client_app`):
   - Configures USART to 1,500,000 baud
   - Writes test message: `"usart client online\r\n"`
   - Validates communication path
   - Depends on `//drivers/usart/client:usart_client`

## Running the Test

### Prerequisites

- `bazelisk` installed and in PATH
- AST10x0 QEMU binaries available
- Working Rust/Bazel build environment

### Run the Test

```bash
# Run with virt_ast10x0 platform config
bazelisk test //target/ast10x0/tests/usart:usart_test --config=virt_ast10x0

# With verbose output
bazelisk test //target/ast10x0/tests/usart:usart_test --config=virt_ast10x0 -s

# Show all test output
bazelisk test //target/ast10x0/tests/usart:usart_test --config=virt_ast10x0 --test_output=all
```

### Expected Output

```
INFO: Found 1 test target...
Target //target/ast10x0/tests/usart:usart_test up-to-date:
  bazel-bin/target/ast10x0/tests/usart/usart_test
INFO: Build completed successfully, X total actions
//target/ast10x0/tests/usart:usart_test                                  PASSED in 0.4s

Executed 1 out of 1 test: 1 test passes.
```

## Test Flow

1. **System Initialization**:
   - Kernel boots with multi-process IPC support
   - Codegen establishes wait groups and channel handles
   - Both processes start concurrently

2. **Server Setup**:
   - Registers IPC channel with `Signals::READABLE`
   - Registers UART5 IRQ with custom signal mask
   - Enters dispatch loop in `runtime::run()`

3. **Client Execution**:
   - Constructs `UsartClient` with IPC handle
   - Issues `configure(1_500_000)` call
   - Issues `write(b"usart client online\r\n")` call
   - Gracefully shuts down

4. **Server Processing**:
   - Receives client requests via `object_wait()`
   - Dispatches to backend (AST10x0 UART5 driver)
   - Responds via `channel_respond()`

5. **Test Completion**:
   - Client calls `debug_shutdown(Ok(()))`
   - QEMU halts with exit code 0 (success)

## Architecture

### USART Backend

The test uses the concrete AST10x0 USART backend:
- **Location**: `//target/ast10x0/backend/usart:usart_backend_ast10x0`
- **Hardware**: UART5 at 0x7e78_4000 (AST10x0 MMIO base)
- **Features**: Blocking read/write, IRQ masking, line status

### Wire Protocol

Defined in `//drivers/usart/api`:
- `UsartOp` enum: Configure, Write, Read, GetLineStatus, (Enable/Disable)Interrupts
- `UsartError` variants: Success, InvalidOperation, BufferTooSmall, Busy, Timeout, etc.
- Max payload per IPC request/response: 256 bytes

### Server Runtime

Dispatch loop in `//drivers/usart/server:runtime`:
1. Wait for IRQ or client channel readable
2. Route wake-ups by `user_data`:
   - If IRQ: acknowledge and clear pending signals
   - If channel: read request, dispatch, respond
3. Back to wait (infinite loop)

## Debugging

### Build Issues

If Bazel build fails with incompatibility error:
```
ERROR: Target is incompatible and cannot be built
```
**Solution**: Use `--config=virt_ast10x0` to set the correct platform.

### QEMU Timeout

If test hangs or times out:
- Check system.json5 for memory/process configuration correctness
- Verify UART5 device node exists in QEMU model
- Inspect server startup logs with `--test_output=all`

### Common Failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| Process crashes at startup | Missing codegen or wait group | Rebuild with clean cache |
| IPC channel not found | Incorrect process/object names | Match system.json5 and codegen |
| USART timeout | Backend not driving UART5 | Check backend initialization |
| Data corruption | Buffer overflow or alignment | Verify MAX_PAYLOAD_SIZE contract |

## Future Extensions

This test is a foundation for:
- [ ] Non-blocking read/write (`try_read`, `try_write`)
- [ ] Async result polling (`get_async_result`)
- [ ] Multi-client concurrent operations
- [ ] Error injection and recovery testing
- [ ] Performance benchmarking

See [ASYNC_NONBLOCKING_IMPLEMENTATION_PLAN.md](../../drivers/usart/api/ASYNC_NONBLOCKING_IMPLEMENTATION_PLAN.md) for planned enhancements.

## Related Documentation

- [USART Driver API](../../drivers/usart/api/README.md) - Protocol and backend trait
- [USART Client](../../drivers/usart/client/README.md) - IPC wrapper usage
- [USART Server](../../drivers/usart/server/README.md) - Dispatch and runtime
- [AST10x0 Backend](../../target/ast10x0/backend/usart/README.md) - Hardware driver
- [Async Non-Blocking Plan](../../drivers/usart/api/ASYNC_NONBLOCKING_IMPLEMENTATION_PLAN.md) - Future work

## Build Artifacts

After a successful build, the test artifacts are located at:
```
bazel-bin/target/ast10x0/tests/usart/
├── usart                    # System image (kernel + rootfs)
├── usart_test               # Test runner
├── usart_server_bin         # Compiled server binary
└── usart_client_app         # Compiled client binary
```

## Contributing

When modifying this test:
1. Keep system.json5 in sync with process/handle definitions
2. Update both server_main.rs and client_main.rs if changing IPC contract
3. Run test locally before committing
4. Document any new operations or error codes in USART API

## License

Licensed under the Apache-2.0 license. See LICENSE file in repository root.
