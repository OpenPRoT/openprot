# util_service

The seams every IPC service is built from. `#![no_std]`, host-buildable, no
kernel dependencies.

A service splits into three parts: wire marshalling in the service's own `api`
crate, a server that turns one request frame into one response frame, and a
transport that carries frames between them. This crate holds the traits that
seam sits on, so a service defines its wire format and nothing else.

## Traits

### [`Transport`](lib.rs)

One round-trip, caller waits for the response. For a thread dedicated to one
service, or an in-process path before IPC exists.

```rust
pub trait Transport {
    fn transact(&mut self, req: &[u8], resp: &mut [u8]) -> Result<usize, TransportError>;
}
```

### [`AsyncTransport`](lib.rs)

The same round-trip split so the caller never blocks. For a caller running in
an event loop. `poll` returns `Ok(None)` while the response is outstanding.

```rust
pub trait AsyncTransport {
    fn start(&mut self, req: &[u8]) -> Result<(), TransportError>;
    fn poll(&mut self, resp: &mut [u8]) -> Result<Option<usize>, TransportError>;
    fn cancel(&mut self) -> Result<(), TransportError>;
}
```

`poll` never waits. The caller parks its event loop until the response to this
round-trip arrives (for a kernel transport, `Signals::READABLE` on its channel
handle, registered with a WaitGroup) and polls once when it does. The handle
comes from the concrete transport at wiring time, not through the trait: a
loopback has no handle to give.

A server nudging the client out of band, with no round-trip outstanding, is a
separate mechanism and not part of this seam. The nudge carries no payload: the
client answers it by starting a round-trip and asking what happened.

Neither transport trait is the fallback for the other: a blocking transport has
no way to poll, and an event loop cannot wait. A type implements whichever it
can serve, or both.

### Using a transport from an event loop

Two pieces of code, and only one of them is generic.

Wiring runs once at startup and knows the concrete transport, because what to
park on is a property of that transport and not of the seam. A kernel
transport hands out its channel handle; a loopback has none to give, and needs
none, because its response is ready the moment the request is.

```rust
// Wiring: concrete type, once at startup.
let mut transport = AsyncChannelTransport::new(handle, send_buf, recv_buf);
wait_group.add(transport.as_raw(), Signals::READABLE)?;
```

The client is generic over `AsyncTransport` and never asks what it is talking
to. It encodes a request, starts the round-trip, and returns to the loop.

```rust
// Client: generic over T: AsyncTransport.
let len = encode_request(&mut req, op)?;
transport.start(&req[..len])?;

// On each wake, poll each round-trip still in flight. Ok(None) means this
// one's response has not arrived; the wake was for something else.
match transport.poll(&mut resp)? {
    Some(len) => handle(decode_response(&resp[..len])?),
    None => {}
}
```

That split is why registration is not on the trait. Everything a service
writes once and reuses is in the second block; only the composition root needs
the first, and it is concrete by nature.

A server nudge (no round-trip outstanding) wakes the same loop through its own
signal. The client answers it by starting a round-trip and asking what changed.

### [`Dispatch`](lib.rs)

The server end. One request frame in, one response frame out, no state between
calls beyond what the service itself owns. The same impl backs the production
channel and the in-process loopback, so host tests exercise the real server.

```rust
pub trait Dispatch {
    fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, DispatchError>;
}
```

A service encodes its own errors into the response frame, so a failed operation
is still a frame and still `Ok`. `DispatchError::ResponseTooSmall` is the one
case with nothing to send back: the response buffer cannot hold even an error
frame.
