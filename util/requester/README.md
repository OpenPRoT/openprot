<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# util_requester

The requester side of a client that never blocks. `util_service` is the loop
for a server that answers channels; this crate is its counterpart for a client
that asks them. `#![no_std]`, no kernel dependencies, no heap.

A kernel channel carries one transaction at a time. A client that talks to
several services therefore keeps, per service channel, the command in flight,
the commands waiting behind it, and the deadline the one in flight must meet.
A `Lane` is exactly that, over any `util_transport::AsyncTransport`.

## Public seams

- `Lane<T, C, I, Q>` owns a transport `T`, a command type `C`, an instant type
  `I`, and a queue of `Q` waiting commands.
  - `push(cmd)` queues a command, or gives it back if the queue is full.
  - `start_next(deadline_of)` starts the next command if the channel is free and
    returns the command that started, so the caller can register for its
    wake-up. `deadline_of` gives that command's deadline, if it has one.
  - `poll(resp)` collects the response if it has landed, as a `Done { cmd, len }`.
  - `cancel()` abandons the command in flight and returns it.
  - `expired(&now)` and `deadline()` say whether the command in flight is late.
  - `transport()` hands out the transport, so the caller can register its handle
    with a WaitGroup.
- `Command` is a request a lane can send: `encode(&self, out)` writes the frame
  (at most `MAX_REQUEST` bytes) or returns `None` if it does not fit.
- `Failed { error, cmd }` hands a command back when it did not get through, so
  the caller decides what its failure means. `LaneError` is `Encode` or
  `Transport(TransportError)`.

## What it does not do

A lane makes no syscall and reads no clock. The caller passes deadlines in and
asks whether one has passed; registering a handle with a WaitGroup, sleeping
until the earliest deadline, and deciding what a response means all stay with
the caller. That is what lets the lane run on a host against a fake transport.

## Invariants

- **One command in flight.** A second `start_next` while one is in flight does
  nothing; commands start in the order they were pushed.
- **A failure returns the command and leaves the lane idle.** A failed start, a
  failed encode and a failed poll all hand the command back in `Failed`. A start
  that fails is not retried: the caller fails it, or fails closed.
- **Cancel always frees the lane.** `cancel` returns the command and leaves the
  lane idle even if the transport reports an error, and the waiting commands are
  kept.
- **A deadline belongs to the command in flight,** and a command with none never
  expires.
- **Nothing panics.** Indexing uses `get`, and the queue is fixed-size.

## Host tests

```sh
bazel test //util/requester:requester_test
```

The tests run against a fake `AsyncTransport` that follows the trait's contract
and answers only when told to. They cover ordering and the one-in-flight rule, a
full queue, an empty lane, polling before and after the response, a failed start,
a command that does not encode, a failed poll, cancel (including a failed cancel
and a cancel with nothing in flight), deadline expiry at and after its instant,
and a command with no deadline.
