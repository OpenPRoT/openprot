# pw_kernel IPC: Channel Objects

This guide explains how to wire up cross-process communication between two
`pw_kernel` userspace processes using channel objects.

A worked example lives at `target/veer/ipc/` (system image) and upstream at
`@pigweed//pw_kernel/tests/ipc/user/` (the two processes themselves) — refer
to those files alongside this document.

## Conceptual model

A channel in `pw_kernel` is a kernel object with two endpoints owned by
distinct processes:

- A **handler** endpoint — the server side. It waits for an incoming
  message, reads the request, and sends a single response.
- An **initiator** endpoint — the client side. It performs a single
  `channel_transact` call that sends a request and blocks until the handler
  responds.

The kernel handles the rendezvous, copies the request and response between
process address spaces, and wakes the appropriate thread on each side.

## Declaring the channel in `system.json5`

Channel endpoints are declared as `objects` inside the `processes` list of
the `system.json5` file driving the system image. Each side names its local
object; the initiator names the handler process and its handler-side object
name to wire the two together.

```json5
{
  apps: [
    {
      name: "ipc",
      flash_size_bytes: 32768,
      ram_size_bytes: 8192,
      processes: [
        {
          name: "initiator",
          objects: [
            {
              name: "IPC",
              type: "channel_initiator",
              handler_process: "handler",
              handler_object_name: "IPC",
            },
          ],
          threads: [
            { name: "initiator thread", stack_size_bytes: 1024 },
          ],
        },
        {
          name: "handler",
          objects: [
            { name: "IPC", type: "channel_handler" },
          ],
          threads: [
            { name: "handler thread", stack_size_bytes: 1024 },
          ],
        },
      ],
    },
  ],
}
```

The `system_generator` codegen step turns the `name` field into a
`handle::IPC` constant inside each process's generated `*_codegen` crate.
The two `name` strings do not need to match across processes; only the
`handler_object_name` linkage on the initiator does.

## Initiator side

The initiator performs a synchronous `channel_transact`: it provides a send
buffer, a receive buffer, and a deadline, and the syscall returns the number
of bytes the handler wrote.

```rust
use initiator_codegen::handle;
use pw_status::Result;
use userspace::time::Instant;
use userspace::{process_entry, syscall};

fn send_one(c: char) -> Result<()> {
    let mut send_buf = [0u8; size_of::<char>()];
    let mut recv_buf = [0u8; size_of::<char>() * 2];

    c.encode_utf8(&mut send_buf);
    let _len: usize = syscall::channel_transact(
        handle::IPC,
        &send_buf,
        &mut recv_buf,
        Instant::MAX,
    )?;
    Ok(())
}

#[process_entry("initiator")]
fn entry() -> ! {
    let _ = send_one('a');
    loop {}
}
```

`Instant::MAX` waits indefinitely. Pass a finite `Instant` to bound the
transaction — the kernel returns a deadline-exceeded error rather than
blocking forever.

## Handler side

The handler waits for the channel to become readable, reads the request,
and writes a response. The pattern is `object_wait` → `channel_read` →
`channel_respond`, looped.

```rust
use handler_codegen::handle;
use pw_status::{Error, Result};
use userspace::process_entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

fn handle_messages() -> Result<()> {
    loop {
        let wait = syscall::object_wait(handle::IPC, Signals::READABLE, Instant::MAX)
            .map_err(|_| Error::Internal)?;
        if !wait.pending_signals.contains(Signals::READABLE) {
            return Err(Error::Internal);
        }

        let mut request = [0u8; size_of::<char>()];
        let _len = syscall::channel_read(handle::IPC, 0, &mut request)?;

        let c = char::from_u32(u32::from_ne_bytes(request))
            .ok_or(Error::InvalidArgument)?;
        let upper = c.to_ascii_uppercase();

        let mut response = [0u8; size_of::<char>() * 2];
        upper.encode_utf8(&mut response[0..size_of::<char>()]);
        c.encode_utf8(&mut response[size_of::<char>()..]);
        syscall::channel_respond(handle::IPC, &response)?;
    }
}

#[process_entry("handler")]
fn entry() -> ! {
    if let Err(e) = handle_messages() {
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}
```

`channel_respond` must be called exactly once per request; the next
`object_wait` then unblocks for the following request.

## Build wiring on the openprot side

On a target like veer, the `system_image` macro consumes both the
`system.json5` and the upstream `multi_process_app` target that bundles the
two process binaries. See `target/veer/ipc/user/BUILD.bazel`:

```python
system_image(
    name = "ipc",
    apps = ["@pigweed//pw_kernel/tests/ipc/user:ipc"],
    kernel = ":target",
    platform = "//target/veer",
    system_config = ":system_config",
    tags = ["kernel"],
)

target_codegen(
    name = "codegen",
    arch = "@pigweed//pw_kernel/arch/riscv:arch_riscv",
    system_config = ":system_config",
)
```

Each process's own `BUILD.bazel` (upstream, in
`@pigweed//pw_kernel/tests/ipc/user/BUILD.bazel`) uses the `rust_process`
macro and names its codegen crate via `codegen_crate_name`, which is what
allows `use initiator_codegen::handle` and `use handler_codegen::handle` to
resolve from within the source files.

## Designing your own channel protocol

The kernel does not impose a wire format on the request or response — both
are byte buffers. A few rules of thumb that follow from that:

- Pick a fixed-size request/response layout (or a length-prefixed one) so
  the handler can size its read buffer up front. Keep the buffers on the
  stack — they live inside `no_std` userspace.
- Validate everything coming off the wire: length, range, enum variants.
  Treat the request side as untrusted — it is in a different process and may
  be running independently maintained code.
- Keep the handler loop free of `unwrap` / panic paths. The handler is the
  service for every other process holding an initiator endpoint, so a
  panic-on-malformed-input becomes a denial-of-service.
- For typed payloads, define a small request/response enum in a shared
  crate that both processes depend on, and use a byte-stable encoding
  (e.g. `zerocopy`) — there is no built-in IDL.

## Limitations to know about

- A channel has exactly one initiator and one handler. To fan in multiple
  clients, declare multiple channel pairs all pointing at the same handler
  process and have the handler `object_wait` on each in turn.
- The handler must call `channel_respond` exactly once per `channel_read`
  before reading again. Skipping the response leaves the initiator blocked.
- Bounded-deadline transactions require the caller to pass a finite
  `Instant`; the kernel does not impose a default timeout.
