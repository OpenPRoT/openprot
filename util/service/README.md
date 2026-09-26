# util_service

Transport and dispatch traits for IPC services. `#![no_std]`, no kernel
dependencies.

## Traits

`Transport` blocks until the response arrives. `AsyncTransport` splits the
round-trip so the caller never blocks: `start`, poll `Ok(None)` until
`Ok(Some(len))`. `Dispatch` is the server end, one request frame in, one
response frame out.

Wake-up registration (channel handle, WaitGroup) needs the concrete
transport, so it happens at wiring time, not through the trait.

## Loopback

`Loopback<D, N>` wraps a `Dispatch` and answers in-process. Both transport
traits work because the response is ready the moment the request is. `N`
sizes the async response buffer. The blocking path writes into the
caller's buffer directly. A loopback has no send buffer, so it never refuses a request at `start`.
If the request is too large, the server sees it and answers with a
protocol error frame instead.

Because `poll` never returns `Ok(None)`, a loopback cannot test not-ready
handling on its own. Wrap it in `Delayed` (below) for that.

## Delayed

`Delayed<T>` wraps any `AsyncTransport` and returns `Ok(None)` for a set
number of polls before forwarding to the inner transport. This is how a
host test reaches a client's not-ready path when the underlying transport
(like `Loopback`) always answers immediately.
